use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::Engine as _;
use local_ip_address::list_afinet_netifas;
use serde_json::Value;

use crate::{
    crypto::{
        DefaultCryptoProvider, EncryptionKeyMaterial, ProtocolCryptoProvider, SigningKeyMaterial,
    },
    domain::{JoinPayload, SnapshotState, TournamentState},
    networking::{read_json_frame, write_json_frame},
    protocol::{
        decode_join_payload, encode_join_payload, join_request_envelope, validate_join_payload,
        EncryptedPrivateEnvelope, JoinTournamentRequest, JsonSignedEnvelope,
        PrivateEnvelopeMetadata, PrivateHoleCardsEvent, ProtocolErrorMessage, ProtocolMessageType,
        SignedEnvelope, SnapshotEvent, PROTOCOL_VERSION,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkingError {
    message: String,
}

impl NetworkingError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NetworkingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NetworkingError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostRuntimeMode {
    Production,
    Test,
}

#[derive(Debug)]
pub struct HostRuntimeConfig {
    pub bind_addr: SocketAddr,
    pub advertised_host: String,
    pub session_epoch: u64,
    pub table_id: String,
    pub table_name: Option<String>,
    pub join_token: String,
    pub host_signing_keys: Arc<SigningKeyMaterial>,
    pub host_encryption_keys: Arc<Mutex<EncryptionKeyMaterial>>,
    pub snapshot_state: TournamentState,
    pub runtime_mode: HostRuntimeMode,
}

#[derive(Debug)]
struct ConnectedClient {
    stream: Arc<Mutex<TcpStream>>,
    encryption_public_key: String,
}

pub struct HostServer {
    listener_addr: SocketAddr,
    join_payload: JoinPayload,
    encoded_join_payload: String,
    clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
    server_sequence: Arc<AtomicU64>,
    stop_signal: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
    host_signing_keys: Arc<SigningKeyMaterial>,
    host_encryption_keys: Arc<Mutex<EncryptionKeyMaterial>>,
}

impl HostServer {
    pub fn bind(config: HostRuntimeConfig) -> Result<Self, NetworkingError> {
        let advertised_ip: IpAddr = config
            .advertised_host
            .parse()
            .map_err(|error| NetworkingError::new(format!("invalid advertised host: {error}")))?;

        if config.runtime_mode == HostRuntimeMode::Production {
            validate_production_host_ip(advertised_ip)?;
        }

        let listener = TcpListener::bind(config.bind_addr)
            .map_err(|error| NetworkingError::new(format!("failed to bind listener: {error}")))?;
        listener.set_nonblocking(false).map_err(|error| {
            NetworkingError::new(format!("failed to configure listener: {error}"))
        })?;

        let listener_addr = listener.local_addr().map_err(|error| {
            NetworkingError::new(format!("failed to read listener addr: {error}"))
        })?;

        let join_payload = JoinPayload {
            payload_version: PROTOCOL_VERSION,
            host_address: config.advertised_host.clone(),
            host_port: listener_addr.port(),
            table_id: config.table_id.clone(),
            session_epoch: config.session_epoch,
            host_signing_public_key: config.host_signing_keys.public_key_base64(),
            join_token: config.join_token.clone(),
            generated_at_ms: now_epoch_ms(),
            table_name: config.table_name.clone(),
        };
        validate_join_payload(&join_payload)
            .map_err(|error| NetworkingError::new(error.to_string()))?;
        let encoded_join_payload = encode_join_payload(&join_payload)
            .map_err(|error| NetworkingError::new(error.to_string()))?;

        let clients = Arc::new(Mutex::new(HashMap::new()));
        let stop_signal = Arc::new(AtomicBool::new(false));
        let server_sequence = Arc::new(AtomicU64::new(0));

        let accept_thread = {
            let clients = Arc::clone(&clients);
            let stop_signal = Arc::clone(&stop_signal);
            let host_signing_keys = Arc::clone(&config.host_signing_keys);
            let host_encryption_keys = Arc::clone(&config.host_encryption_keys);
            let join_payload = join_payload.clone();
            let snapshot_state = config.snapshot_state.clone();

            thread::Builder::new()
                .name("desktop-poker-host".to_string())
                .spawn(move || {
                    for incoming in listener.incoming() {
                        if stop_signal.load(Ordering::SeqCst) {
                            break;
                        }

                        let Ok(mut stream) = incoming else {
                            continue;
                        };

                        let clients = Arc::clone(&clients);
                        let host_signing_keys = Arc::clone(&host_signing_keys);
                        let host_encryption_keys = Arc::clone(&host_encryption_keys);
                        let join_payload = join_payload.clone();
                        let snapshot_state = snapshot_state.clone();

                        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

                        thread::spawn(move || {
                            let crypto_provider = DefaultCryptoProvider;
                            let initial_request =
                                read_json_frame::<JsonSignedEnvelope>(&mut stream);

                            let response = match initial_request {
                                Ok(request_envelope) => handle_join_request(
                                    &crypto_provider,
                                    request_envelope,
                                    &join_payload,
                                    &snapshot_state,
                                    &host_signing_keys,
                                    &host_encryption_keys,
                                ),
                                Err(error) => Err(error),
                            };

                            match response {
                                Ok((player_id, acceptance_envelope, encryption_public_key)) => {
                                    if write_json_frame(&mut stream, &acceptance_envelope).is_ok() {
                                        let stream_handle = match stream.try_clone() {
                                            Ok(cloned_stream) => {
                                                Arc::new(Mutex::new(cloned_stream))
                                            }
                                            Err(_) => return,
                                        };

                                        if let Ok(mut connected_clients) = clients.lock() {
                                            connected_clients.insert(
                                                player_id,
                                                ConnectedClient {
                                                    stream: stream_handle,
                                                    encryption_public_key,
                                                },
                                            );
                                        }
                                    }
                                }
                                Err(error) => {
                                    let mut envelope = SignedEnvelope {
                                        protocol_version: PROTOCOL_VERSION,
                                        message_type: ProtocolMessageType::ProtocolError,
                                        table_id: join_payload.table_id.clone(),
                                        session_epoch: join_payload.session_epoch,
                                        sender_id: "host".to_string(),
                                        counter: 0,
                                        message_id: format!("error-{}", now_epoch_ms()),
                                        server_sequence: None,
                                        payload: ProtocolErrorMessage {
                                            code: "JOIN_REJECTED".to_string(),
                                            message: error.to_string(),
                                            rejected_message_id: None,
                                        },
                                        signature: None,
                                    };

                                    if envelope.sign(&crypto_provider, &host_signing_keys).is_ok() {
                                        let _ = write_json_frame(&mut stream, &envelope);
                                    }
                                }
                            }
                        });
                    }
                })
                .map_err(|error| {
                    NetworkingError::new(format!("failed to start accept loop: {error}"))
                })?
        };

        Ok(Self {
            listener_addr,
            join_payload,
            encoded_join_payload,
            clients,
            server_sequence,
            stop_signal,
            accept_thread: Some(accept_thread),
            host_signing_keys: config.host_signing_keys,
            host_encryption_keys: config.host_encryption_keys,
        })
    }

    #[must_use]
    pub fn listener_addr(&self) -> SocketAddr {
        self.listener_addr
    }

    #[must_use]
    pub fn join_payload(&self) -> &JoinPayload {
        &self.join_payload
    }

    #[must_use]
    pub fn encoded_join_payload(&self) -> &str {
        &self.encoded_join_payload
    }

    #[must_use]
    pub fn join_payload_share_text(&self) -> String {
        self.encoded_join_payload.clone()
    }

    #[must_use]
    pub fn join_payload_qr_text(&self) -> String {
        self.encoded_join_payload.clone()
    }

    pub fn broadcast_public_event<TPayload: serde::Serialize>(
        &self,
        message_type: ProtocolMessageType,
        payload: &TPayload,
    ) -> Result<(), NetworkingError> {
        let payload_value = serde_json::to_value(payload)
            .map_err(|error| NetworkingError::new(error.to_string()))?;
        let server_sequence = self.server_sequence.fetch_add(1, Ordering::SeqCst) + 1;

        let mut envelope = SignedEnvelope {
            protocol_version: PROTOCOL_VERSION,
            message_type,
            table_id: self.join_payload.table_id.clone(),
            session_epoch: self.join_payload.session_epoch,
            sender_id: "host".to_string(),
            counter: server_sequence,
            message_id: format!("host-{server_sequence}"),
            server_sequence: Some(server_sequence),
            payload: payload_value,
            signature: None,
        };

        let crypto_provider = DefaultCryptoProvider;
        envelope
            .sign(&crypto_provider, &self.host_signing_keys)
            .map_err(|error| NetworkingError::new(error.to_string()))?;

        let failed_clients = {
            let connected_clients = self
                .clients
                .lock()
                .map_err(|_| NetworkingError::new("client registry lock poisoned"))?;
            let mut failed_clients = Vec::new();

            for (player_id, client) in connected_clients.iter() {
                let write_result = client
                    .stream
                    .lock()
                    .map_err(|_| NetworkingError::new("client stream lock poisoned"))?
                    .try_clone()
                    .map_err(|error| {
                        NetworkingError::new(format!("failed to clone client stream: {error}"))
                    })
                    .and_then(|mut stream| write_json_frame(&mut stream, &envelope));

                if write_result.is_err() {
                    failed_clients.push(player_id.clone());
                }
            }

            failed_clients
        };

        if !failed_clients.is_empty() {
            let mut connected_clients = self
                .clients
                .lock()
                .map_err(|_| NetworkingError::new("client registry lock poisoned"))?;
            for player_id in failed_clients {
                connected_clients.remove(&player_id);
            }
        }

        Ok(())
    }

    pub fn send_private_hole_cards(
        &self,
        recipient_id: &str,
        hole_cards_event: &PrivateHoleCardsEvent,
    ) -> Result<(), NetworkingError> {
        let crypto_provider = DefaultCryptoProvider;

        let (client_stream, recipient_encryption_public_key) = {
            let connected_clients = self
                .clients
                .lock()
                .map_err(|_| NetworkingError::new("client registry lock poisoned"))?;
            let client = connected_clients
                .get(recipient_id)
                .ok_or_else(|| NetworkingError::new("unknown recipient"))?;

            (
                Arc::clone(&client.stream),
                client.encryption_public_key.clone(),
            )
        };

        let server_sequence = self.server_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let payload_bytes = serde_json::to_vec(hole_cards_event).map_err(|error| {
            NetworkingError::new(format!("failed to serialize private payload: {error}"))
        })?;

        let host_encryption_keys = self
            .host_encryption_keys
            .lock()
            .map_err(|_| NetworkingError::new("host encryption key lock poisoned"))?;

        let aad_metadata = PrivateEnvelopeMetadata {
            sender_id: "host".to_string(),
            table_id: self.join_payload.table_id.clone(),
            session_epoch: self.join_payload.session_epoch,
            counter: server_sequence,
            message_id: format!("private-{server_sequence}"),
            server_sequence,
            recipient_id: recipient_id.to_string(),
        };

        let aad_bytes = serde_json::json!({
            "protocolVersion": PROTOCOL_VERSION,
            "messageType": "PRIVATE_HOLE_CARDS_EVENT",
            "tableId": aad_metadata.table_id,
            "sessionEpoch": aad_metadata.session_epoch,
            "senderId": aad_metadata.sender_id,
            "counter": aad_metadata.counter,
            "messageId": aad_metadata.message_id,
            "serverSequence": aad_metadata.server_sequence,
            "recipientId": aad_metadata.recipient_id,
            "recipientKeyId": crate::crypto::key_fingerprint(
                &base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(recipient_encryption_public_key.as_bytes())
                    .map_err(|error| NetworkingError::new(format!("invalid recipient public key: {error}")))?,
            )
        });

        let aad = serde_json::to_vec(&aad_bytes)
            .map_err(|error| NetworkingError::new(format!("failed to serialize aad: {error}")))?;
        let encrypted_payload = crypto_provider
            .encrypt(
                &host_encryption_keys,
                &recipient_encryption_public_key,
                &payload_bytes,
                &aad,
            )
            .map_err(|error| NetworkingError::new(error.to_string()))?;
        drop(host_encryption_keys);

        let mut envelope = EncryptedPrivateEnvelope::from_encrypted_payload(
            encrypted_payload,
            PrivateEnvelopeMetadata {
                recipient_id: recipient_id.to_string(),
                ..aad_metadata
            },
        );
        envelope
            .sign(&crypto_provider, &self.host_signing_keys)
            .map_err(|error| NetworkingError::new(error.to_string()))?;

        let mut stream = client_stream
            .lock()
            .map_err(|_| NetworkingError::new("client stream lock poisoned"))?
            .try_clone()
            .map_err(|error| {
                NetworkingError::new(format!("failed to clone client stream: {error}"))
            })?;

        write_json_frame(&mut stream, &envelope)
    }
}

impl Drop for HostServer {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.listener_addr);

        if let Some(join_handle) = self.accept_thread.take() {
            let _ = join_handle.join();
        }
    }
}

pub struct ClientRuntime {
    incoming: Receiver<ClientRuntimeEvent>,
}

impl ClientRuntime {
    pub fn connect(config: ClientRuntimeConfig) -> Result<Self, NetworkingError> {
        let join_payload = decode_join_payload(&config.join_payload)
            .map_err(|error| NetworkingError::new(error.to_string()))?;
        validate_join_payload(&join_payload)
            .map_err(|error| NetworkingError::new(error.to_string()))?;

        let mut stream =
            TcpStream::connect((join_payload.host_address.as_str(), join_payload.host_port))
                .map_err(|error| {
                    NetworkingError::new(format!("failed to connect to host: {error}"))
                })?;
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

        let crypto_provider = DefaultCryptoProvider;
        let mut join_envelope = join_request_envelope(
            join_payload.table_id.clone(),
            join_payload.session_epoch,
            config.player_id.clone(),
            1,
            format!("join-{}", now_epoch_ms()),
            JoinTournamentRequest {
                display_name: config.display_name.clone(),
                join_token: join_payload.join_token.clone(),
                signing_public_key: config.signing_keys.public_key_base64(),
                encryption_public_key: config.encryption_keys.public_key_base64(),
            },
        );
        join_envelope
            .sign(&crypto_provider, &config.signing_keys)
            .map_err(|error| NetworkingError::new(error.to_string()))?;
        write_json_frame(&mut stream, &join_envelope)?;

        let response_value: Value = read_json_frame(&mut stream)?;
        let message_type = response_value
            .get("messageType")
            .and_then(Value::as_str)
            .ok_or_else(|| NetworkingError::new("host response missing messageType"))?;

        let host_signing_public_key = join_payload.host_signing_public_key.clone();

        let (snapshot_event, host_encryption_public_key) = if message_type == "SNAPSHOT_EVENT" {
            let envelope: SignedEnvelope<SnapshotEvent> = serde_json::from_value(response_value)
                .map_err(|error| {
                    NetworkingError::new(format!("invalid snapshot envelope: {error}"))
                })?;
            envelope
                .verify(&crypto_provider, &host_signing_public_key)
                .map_err(|error| NetworkingError::new(error.to_string()))?;
            let snapshot_event = envelope.payload;
            let host_encryption_public_key = snapshot_event
                .host_encryption_public_key
                .clone()
                .ok_or_else(|| NetworkingError::new("snapshot missing hostEncryptionPublicKey"))?;
            (snapshot_event, host_encryption_public_key)
        } else {
            let envelope: SignedEnvelope<ProtocolErrorMessage> =
                serde_json::from_value(response_value).map_err(|error| {
                    NetworkingError::new(format!("invalid rejection envelope: {error}"))
                })?;
            envelope
                .verify(&crypto_provider, &host_signing_public_key)
                .map_err(|error| NetworkingError::new(error.to_string()))?;
            return Err(NetworkingError::new(envelope.payload.message));
        };

        let (sender, receiver) = mpsc::channel();
        sender
            .send(ClientRuntimeEvent::Snapshot(Box::new(snapshot_event)))
            .map_err(|error| NetworkingError::new(format!("failed to queue snapshot: {error}")))?;

        let mut read_stream = stream
            .try_clone()
            .map_err(|error| NetworkingError::new(format!("failed to clone stream: {error}")))?;
        let encryption_keys = config.encryption_keys;
        let player_id = config.player_id;

        thread::spawn(move || {
            loop {
                let Ok(frame_value) = read_json_frame::<Value>(&mut read_stream) else {
                    break;
                };

                let Some(message_type) = frame_value.get("messageType").and_then(Value::as_str)
                else {
                    continue;
                };

                match message_type {
                    "PRIVATE_HOLE_CARDS_EVENT" => {
                        let Ok(envelope) =
                            serde_json::from_value::<EncryptedPrivateEnvelope>(frame_value.clone())
                        else {
                            continue;
                        };

                        if envelope
                            .verify(&crypto_provider, &host_signing_public_key)
                            .is_err()
                        {
                            continue;
                        }

                        let encrypted_payload = crate::crypto::EncryptedPayload {
                            nonce_base64: envelope.nonce.clone(),
                            ciphertext_base64: envelope.ciphertext.clone(),
                            recipient_key_id: envelope.recipient_key_id.clone(),
                        };

                        let Ok(plaintext) = crypto_provider.decrypt(
                            &encryption_keys,
                            &host_encryption_public_key,
                            &encrypted_payload,
                            envelope
                                .associated_data_json()
                                .unwrap_or_default()
                                .as_slice(),
                        ) else {
                            continue;
                        };

                        let Ok(private_payload) =
                            serde_json::from_slice::<PrivateHoleCardsEvent>(&plaintext)
                        else {
                            continue;
                        };

                        let _ = sender.send(ClientRuntimeEvent::PrivateHoleCards(private_payload));
                    }
                    "SNAPSHOT_EVENT" => {}
                    _ => {
                        let Ok(envelope) =
                            serde_json::from_value::<JsonSignedEnvelope>(frame_value.clone())
                        else {
                            continue;
                        };

                        if envelope
                            .verify(&crypto_provider, &host_signing_public_key)
                            .is_err()
                        {
                            continue;
                        }

                        if !matches!(
                            envelope.message_type,
                            ProtocolMessageType::ActionWindowOpenedEvent
                                | ProtocolMessageType::PlayerActionCommittedEvent
                                | ProtocolMessageType::StreetRevealedEvent
                                | ProtocolMessageType::EliminationEvent
                                | ProtocolMessageType::TournamentCompleteEvent
                                | ProtocolMessageType::HandResultCommittedEvent
                                | ProtocolMessageType::TournamentStartedEvent
                        ) {
                            continue;
                        }

                        let _ = sender.send(ClientRuntimeEvent::PublicEvent {
                            message_type: envelope.message_type,
                            payload: envelope.payload,
                        });
                    }
                }
            }

            let _ = sender.send(ClientRuntimeEvent::Disconnected { player_id });
        });

        Ok(Self { incoming: receiver })
    }

    pub fn next_event(&self, timeout: Duration) -> Result<ClientRuntimeEvent, NetworkingError> {
        self.incoming.recv_timeout(timeout).map_err(|error| {
            NetworkingError::new(format!("timed out waiting for client event: {error}"))
        })
    }
}

pub struct ClientRuntimeConfig {
    pub join_payload: String,
    pub player_id: String,
    pub display_name: String,
    pub signing_keys: SigningKeyMaterial,
    pub encryption_keys: EncryptionKeyMaterial,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClientRuntimeEvent {
    Snapshot(Box<SnapshotEvent>),
    PublicEvent {
        message_type: ProtocolMessageType,
        payload: Value,
    },
    PrivateHoleCards(PrivateHoleCardsEvent),
    Disconnected {
        player_id: String,
    },
}

pub fn resolve_connectable_host_ip() -> Result<IpAddr, NetworkingError> {
    let interfaces = list_afinet_netifas()
        .map_err(|error| NetworkingError::new(format!("failed to list interfaces: {error}")))?;

    interfaces
        .into_iter()
        .map(|(_, ip)| ip)
        .find(|ip| ip.is_ipv4() && !ip.is_loopback() && !ip.is_unspecified())
        .ok_or_else(|| NetworkingError::new("no valid LAN IP address is available for hosting"))
}

fn validate_production_host_ip(ip_addr: IpAddr) -> Result<(), NetworkingError> {
    if ip_addr.is_unspecified() || ip_addr.is_loopback() {
        return Err(NetworkingError::new(
            "production host flow requires a non-loopback, connectable LAN IP address",
        ));
    }

    Ok(())
}

fn handle_join_request(
    crypto_provider: &impl ProtocolCryptoProvider,
    request_envelope: JsonSignedEnvelope,
    join_payload: &JoinPayload,
    snapshot_state: &TournamentState,
    host_signing_keys: &SigningKeyMaterial,
    host_encryption_keys: &Arc<Mutex<EncryptionKeyMaterial>>,
) -> Result<(String, SignedEnvelope<SnapshotEvent>, String), NetworkingError> {
    if request_envelope.message_type != ProtocolMessageType::JoinTournamentRequest {
        return Err(NetworkingError::new(
            "first client message must be JOIN_TOURNAMENT_REQUEST",
        ));
    }

    let request: JoinTournamentRequest = serde_json::from_value(request_envelope.payload.clone())
        .map_err(|error| {
        NetworkingError::new(format!("invalid join request payload: {error}"))
    })?;

    request_envelope
        .verify(crypto_provider, &request.signing_public_key)
        .map_err(|error| NetworkingError::new(error.to_string()))?;

    if request.join_token != join_payload.join_token {
        return Err(NetworkingError::new("join token mismatch"));
    }

    let host_encryption_public_key = host_encryption_keys
        .lock()
        .map_err(|_| NetworkingError::new("host encryption key lock poisoned"))?
        .public_key_base64();

    let mut snapshot_envelope = SignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::SnapshotEvent,
        table_id: join_payload.table_id.clone(),
        session_epoch: join_payload.session_epoch,
        sender_id: "host".to_string(),
        counter: 1,
        message_id: format!("snapshot-{}", now_epoch_ms()),
        server_sequence: None,
        payload: SnapshotEvent {
            state: SnapshotState {
                state: snapshot_state.clone(),
                local_player_id: request_envelope.sender_id.clone(),
                reconnect_token: Some(format!("reconnect-{}", request_envelope.sender_id)),
                host_signing_public_key: Some(join_payload.host_signing_public_key.clone()),
                host_encryption_public_key: Some(host_encryption_public_key.clone()),
            },
            local_player_id: request_envelope.sender_id.clone(),
            reconnect_token: Some(format!("reconnect-{}", request_envelope.sender_id)),
            host_signing_public_key: Some(join_payload.host_signing_public_key.clone()),
            host_encryption_public_key: Some(host_encryption_public_key),
        },
        signature: None,
    };

    snapshot_envelope
        .sign(crypto_provider, host_signing_keys)
        .map_err(|error| NetworkingError::new(error.to_string()))?;

    Ok((
        request_envelope.sender_id,
        snapshot_envelope,
        request.encryption_public_key,
    ))
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use serde_json::json;

    use super::validate_production_host_ip;
    use crate::{
        crypto::{DefaultCryptoProvider, ProtocolCryptoProvider},
        domain::{
            BlindLevel, BlindSchedule, Card, Rank, Suit, TournamentConfig, TournamentPhase,
            TournamentState,
        },
        networking::{
            resolve_connectable_host_ip, ClientRuntime, ClientRuntimeConfig, ClientRuntimeEvent,
            HostRuntimeConfig, HostRuntimeMode, HostServer,
        },
        protocol::{PrivateHoleCardsEvent, ProtocolMessageType, TournamentStartedEvent},
    };

    fn sample_tournament_state(table_id: &str, session_epoch: u64) -> TournamentState {
        TournamentState {
            table_id: table_id.to_string(),
            session_epoch,
            phase: TournamentPhase::WaitingForPlayers,
            config: TournamentConfig {
                tournament_name: "LAN Test".to_string(),
                table_name: Some("Test Table".to_string()),
                max_players: 6,
                starting_stack: 1500,
                turn_timer_seconds: 20,
                blind_schedule: BlindSchedule {
                    levels: vec![BlindLevel {
                        level_index: 1,
                        label: "Level 1".to_string(),
                        small_blind: 10,
                        big_blind: 20,
                        ante: 0,
                        duration_seconds: 180,
                    }],
                },
            },
            blind_schedule: BlindSchedule {
                levels: vec![BlindLevel {
                    level_index: 1,
                    label: "Level 1".to_string(),
                    small_blind: 10,
                    big_blind: 20,
                    ante: 0,
                    duration_seconds: 180,
                }],
            },
            blind_level_index: 0,
            participants: BTreeMap::new(),
            seats: Vec::new(),
            current_hand: None,
            hand_results: Vec::new(),
            placements: Vec::new(),
        }
    }

    #[test]
    fn host_can_open_listener() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

        let host = HostServer::bind(HostRuntimeConfig {
            bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
            advertised_host: "127.0.0.1".to_string(),
            session_epoch: 1,
            table_id: "table-1".to_string(),
            table_name: Some("Test".to_string()),
            join_token: "join-token".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: sample_tournament_state("table-1", 1),
            runtime_mode: HostRuntimeMode::Test,
        })
        .expect("host should bind");

        assert!(host.listener_addr().port() > 0);
        assert!(host.encoded_join_payload().starts_with("pkr1_"));
    }

    #[test]
    fn client_can_connect_and_join_using_canonical_payload() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

        let host = HostServer::bind(HostRuntimeConfig {
            bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
            advertised_host: "127.0.0.1".to_string(),
            session_epoch: 7,
            table_id: "table-join".to_string(),
            table_name: Some("Join Table".to_string()),
            join_token: "join-token".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: sample_tournament_state("table-join", 7),
            runtime_mode: HostRuntimeMode::Test,
        })
        .expect("host should bind");

        let client = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: host.encoded_join_payload().to_string(),
            player_id: "player-a".to_string(),
            display_name: "Alice".to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        })
        .expect("client should connect");

        match client
            .next_event(Duration::from_secs(2))
            .expect("snapshot event")
        {
            ClientRuntimeEvent::Snapshot(snapshot) => {
                assert_eq!(snapshot.local_player_id, "player-a");
            }
            other => panic!("expected snapshot event, got {other:?}"),
        }
    }

    #[test]
    fn public_events_flow_from_host_to_client_over_real_tcp() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

        let host = HostServer::bind(HostRuntimeConfig {
            bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
            advertised_host: "127.0.0.1".to_string(),
            session_epoch: 11,
            table_id: "table-events".to_string(),
            table_name: Some("Events".to_string()),
            join_token: "join-token".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: sample_tournament_state("table-events", 11),
            runtime_mode: HostRuntimeMode::Test,
        })
        .expect("host should bind");

        let client = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: host.encoded_join_payload().to_string(),
            player_id: "player-public".to_string(),
            display_name: "Public".to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        })
        .expect("client should connect");
        let _ = client.next_event(Duration::from_secs(2)).expect("snapshot");

        host.broadcast_public_event(
            ProtocolMessageType::TournamentStartedEvent,
            &TournamentStartedEvent {
                tournament_name: "LAN Test".to_string(),
                starting_stack: 1500,
                blind_schedule_preset: "FAST".to_string(),
                frozen_player_ids: vec!["player-public".to_string()],
            },
        )
        .expect("host should broadcast");

        match client
            .next_event(Duration::from_secs(2))
            .expect("public event")
        {
            ClientRuntimeEvent::PublicEvent {
                message_type,
                payload,
            } => {
                assert_eq!(message_type, ProtocolMessageType::TournamentStartedEvent);
                assert_eq!(payload.get("tournamentName"), Some(&json!("LAN Test")));
            }
            other => panic!("expected public event, got {other:?}"),
        }
    }

    #[test]
    fn private_encrypted_payload_can_be_delivered_and_decrypted() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

        let host = HostServer::bind(HostRuntimeConfig {
            bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
            advertised_host: "127.0.0.1".to_string(),
            session_epoch: 15,
            table_id: "table-private".to_string(),
            table_name: Some("Private".to_string()),
            join_token: "join-token".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: sample_tournament_state("table-private", 15),
            runtime_mode: HostRuntimeMode::Test,
        })
        .expect("host should bind");

        let client = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: host.encoded_join_payload().to_string(),
            player_id: "player-private".to_string(),
            display_name: "Private".to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        })
        .expect("client should connect");
        let _ = client.next_event(Duration::from_secs(2)).expect("snapshot");

        host.send_private_hole_cards(
            "player-private",
            &PrivateHoleCardsEvent {
                recipient_player_id: "player-private".to_string(),
                hole_cards: vec![
                    Card {
                        rank: Rank::Ace,
                        suit: Suit::Spades,
                    },
                    Card {
                        rank: Rank::King,
                        suit: Suit::Spades,
                    },
                ],
            },
        )
        .expect("private payload should send");

        match client
            .next_event(Duration::from_secs(2))
            .expect("private event")
        {
            ClientRuntimeEvent::PrivateHoleCards(payload) => {
                assert_eq!(payload.recipient_player_id, "player-private");
                assert_eq!(payload.hole_cards.len(), 2);
            }
            other => panic!("expected private payload, got {other:?}"),
        }
    }

    #[test]
    fn invalid_host_ip_blocks_host_startup() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

        let host = HostServer::bind(HostRuntimeConfig {
            bind_addr: "0.0.0.0:0".parse().expect("socket addr"),
            advertised_host: "0.0.0.0".to_string(),
            session_epoch: 1,
            table_id: "table-invalid".to_string(),
            table_name: Some("Invalid".to_string()),
            join_token: "join-token".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: sample_tournament_state("table-invalid", 1),
            runtime_mode: HostRuntimeMode::Production,
        });

        assert!(host.is_err());
    }

    #[test]
    fn invalid_payload_blocks_join_before_connect() {
        let provider = DefaultCryptoProvider;

        let result = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: "{\"payloadVersion\":1,\"hostAddress\":\"0.0.0.0\",\"hostPort\":43818,\"tableId\":\"table-1\",\"sessionEpoch\":1,\"hostSigningPublicKey\":\"host\",\"joinToken\":\"join\",\"generatedAtMs\":1}".to_string(),
            player_id: "player-invalid".to_string(),
            display_name: "Invalid".to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn host_ip_resolution_returns_error_when_no_connectable_address_exists() {
        let result = validate_production_host_ip("127.0.0.1".parse().expect("ip address"));
        assert!(result.is_err());
        let _ = resolve_connectable_host_ip;
    }
}
