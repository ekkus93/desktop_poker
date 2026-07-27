use std::{
    net::TcpStream,
    sync::{Arc, Mutex},
    thread::{self},
    time::Duration,
};

use serde_json::Value;

use crate::{
    crypto::ProtocolCryptoProvider,
    domain::JoinPayload,
    networking::{read_json_frame, write_json_frame},
    protocol::{
        join_request_envelope, JoinTournamentRequest, ProtocolErrorMessage, ProtocolMessageType,
        ReconnectTournamentRequest, ResyncRequest, SignedEnvelope, SnapshotEvent, PROTOCOL_VERSION,
    },
};

use super::*;

pub(crate) fn connect_to_host(join_payload: &JoinPayload) -> Result<TcpStream, NetworkingError> {
    let stream =
        TcpStream::connect((join_payload.host_address.as_str(), join_payload.host_port))
            .map_err(|error| NetworkingError::new(format!("failed to connect to host: {error}")))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| NetworkingError::new(format!("failed to set read timeout: {error}")))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| NetworkingError::new(format!("failed to set write timeout: {error}")))?;
    Ok(stream)
}

pub(crate) fn connect_and_join(
    crypto_provider: &impl ProtocolCryptoProvider,
    join_payload: &JoinPayload,
    player_id: &str,
    display_name: &str,
    reconnect_identity: &Arc<Mutex<ClientReconnectIdentity>>,
) -> Result<(TcpStream, SignedEnvelope<SnapshotEvent>), NetworkingError> {
    let mut stream = connect_to_host(join_payload)?;
    let identity = reconnect_identity
        .lock()
        .map_err(|_| NetworkingError::new("client reconnect identity lock poisoned"))?;
    let signing_keys = identity
        .signing_keys
        .as_ref()
        .ok_or_else(|| NetworkingError::new(missing_reconnect_identity_message()))?;
    let encryption_keys = identity
        .encryption_keys
        .as_ref()
        .ok_or_else(|| NetworkingError::new(missing_reconnect_identity_message()))?;

    let mut join_envelope = join_request_envelope(
        join_payload.table_id.clone(),
        join_payload.session_epoch,
        player_id.to_string(),
        1,
        format!("join-{}", now_epoch_ms()),
        JoinTournamentRequest {
            display_name: display_name.to_string(),
            join_token: join_payload.join_token.clone(),
            signing_public_key: signing_keys.public_key_base64(),
            encryption_public_key: encryption_keys.public_key_base64(),
        },
    );
    join_envelope
        .sign(crypto_provider, signing_keys)
        .map_err(|error| NetworkingError::new(error.to_string()))?;
    drop(identity);

    write_json_frame(&mut stream, &join_envelope)?;
    let snapshot = read_snapshot_response(crypto_provider, &mut stream, join_payload)?;
    clear_established_read_timeout(&stream, "established client session")?;

    Ok((stream, snapshot))
}

pub(crate) fn reconnect_after_disconnect(
    crypto_provider: &impl ProtocolCryptoProvider,
    join_payload: &JoinPayload,
    player_id: &str,
    reconnect_identity: &Arc<Mutex<ClientReconnectIdentity>>,
    reconnect_token: Option<&str>,
    last_known_server_sequence: u64,
    next_counter: &mut u64,
) -> Result<(TcpStream, SignedEnvelope<SnapshotEvent>), NetworkingError> {
    let reconnect_token = reconnect_token
        .ok_or_else(|| NetworkingError::new("reconnect token is unavailable for this session"))?;
    let identity = reconnect_identity
        .lock()
        .map_err(|_| NetworkingError::new("client reconnect identity lock poisoned"))?;
    let signing_keys = identity
        .signing_keys
        .as_ref()
        .ok_or_else(|| NetworkingError::new(missing_reconnect_identity_message()))?;

    let mut reconnect_envelope = SignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::ReconnectTournamentRequest,
        table_id: join_payload.table_id.clone(),
        session_epoch: join_payload.session_epoch,
        sender_id: player_id.to_string(),
        counter: *next_counter,
        message_id: format!("reconnect-{}", now_epoch_ms()),
        server_sequence: None,
        payload: ReconnectTournamentRequest {
            player_id: player_id.to_string(),
            reconnect_token: reconnect_token.to_string(),
            last_known_server_seq: Some(last_known_server_sequence),
        },
        signature: None,
    };
    reconnect_envelope
        .sign(crypto_provider, signing_keys)
        .map_err(|error| NetworkingError::new(error.to_string()))?;
    *next_counter += 1;
    drop(identity);

    const MAX_ALREADY_CONNECTED_RETRIES: u8 = 10;

    for attempt in 0..=MAX_ALREADY_CONNECTED_RETRIES {
        let mut stream = connect_to_host(join_payload)?;
        write_json_frame(&mut stream, &reconnect_envelope)?;

        match read_snapshot_response(crypto_provider, &mut stream, join_payload) {
            Ok(snapshot) => {
                clear_established_read_timeout(&stream, "reconnected client session")?;
                return Ok((stream, snapshot));
            }
            Err(error)
                if error
                    .to_string()
                    .contains("participant is already connected") =>
            {
                if attempt == MAX_ALREADY_CONNECTED_RETRIES {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(error),
        }
    }

    Err(NetworkingError::new(
        "reconnect retries exhausted while waiting for prior connection cleanup",
    ))
}

pub(crate) fn request_resync_snapshot(
    crypto_provider: &impl ProtocolCryptoProvider,
    stream: &mut TcpStream,
    join_payload: &JoinPayload,
    player_id: &str,
    reconnect_identity: &Arc<Mutex<ClientReconnectIdentity>>,
    last_seen_server_sequence: u64,
    next_counter: &mut u64,
) -> Result<SignedEnvelope<SnapshotEvent>, NetworkingError> {
    let identity = reconnect_identity
        .lock()
        .map_err(|_| NetworkingError::new("client reconnect identity lock poisoned"))?;
    let signing_keys = identity
        .signing_keys
        .as_ref()
        .ok_or_else(|| NetworkingError::new(missing_reconnect_identity_message()))?;

    let mut resync_envelope = SignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::ResyncRequest,
        table_id: join_payload.table_id.clone(),
        session_epoch: join_payload.session_epoch,
        sender_id: player_id.to_string(),
        counter: *next_counter,
        message_id: format!("resync-{}", now_epoch_ms()),
        server_sequence: None,
        payload: ResyncRequest {
            last_seen_server_sequence: Some(last_seen_server_sequence),
        },
        signature: None,
    };
    resync_envelope
        .sign(crypto_provider, signing_keys)
        .map_err(|error| NetworkingError::new(error.to_string()))?;
    *next_counter += 1;
    drop(identity);

    write_json_frame(stream, &resync_envelope)?;
    read_snapshot_response(crypto_provider, stream, join_payload)
}

pub(crate) fn read_snapshot_response(
    crypto_provider: &impl ProtocolCryptoProvider,
    stream: &mut TcpStream,
    join_payload: &JoinPayload,
) -> Result<SignedEnvelope<SnapshotEvent>, NetworkingError> {
    let response_value: Value = read_json_frame(stream)?;
    let message_type = response_value
        .get("messageType")
        .and_then(Value::as_str)
        .ok_or_else(|| NetworkingError::new("host response missing messageType"))?;

    if message_type == "SNAPSHOT_EVENT" {
        let envelope: SignedEnvelope<SnapshotEvent> = serde_json::from_value(response_value)
            .map_err(|error| NetworkingError::new(format!("invalid snapshot envelope: {error}")))?;
        envelope
            .verify(crypto_provider, &join_payload.host_signing_public_key)
            .map_err(|error| NetworkingError::new(error.to_string()))?;

        if envelope.server_sequence.is_none() {
            return Err(NetworkingError::new(
                "snapshot missing authoritative serverSequence",
            ));
        }

        return Ok(envelope);
    }

    let envelope: SignedEnvelope<ProtocolErrorMessage> = serde_json::from_value(response_value)
        .map_err(|error| NetworkingError::new(format!("invalid rejection envelope: {error}")))?;
    envelope
        .verify(crypto_provider, &join_payload.host_signing_public_key)
        .map_err(|error| NetworkingError::new(error.to_string()))?;
    Err(NetworkingError::new(envelope.payload.message))
}
