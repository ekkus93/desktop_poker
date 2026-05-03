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
use rand_core::{OsRng, RngCore};
use serde_json::Value;

use crate::{
    crypto::{
        key_fingerprint, DefaultCryptoProvider, EncryptionKeyMaterial, ProtocolCryptoProvider,
        SigningKeyMaterial,
    },
    domain::{
        ConnectionState, JoinPayload, ParticipantRegistryEntry, ParticipantState, PlayerIdentity,
        SnapshotState, TournamentPhase, TournamentState,
    },
    networking::{read_json_frame, write_json_frame},
    protocol::{
        decode_join_payload, encode_join_payload, join_request_envelope, validate_join_payload,
        EncryptedPrivateEnvelope, JoinTournamentRequest, JsonSignedEnvelope,
        PrivateEnvelopeMetadata, PrivateHoleCardsEvent, ProtocolErrorMessage, ProtocolMessageType,
        ReconnectTournamentRequest, ResyncRequest, SignedEnvelope, SnapshotEvent, PROTOCOL_VERSION,
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
    authoritative_state: Arc<Mutex<TournamentState>>,
    clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
    server_sequence: Arc<AtomicU64>,
    stop_signal: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
    host_signing_keys: Arc<SigningKeyMaterial>,
    host_encryption_keys: Arc<Mutex<EncryptionKeyMaterial>>,
}

#[derive(Debug)]
struct InitialRequestAcceptance {
    player_id: String,
    snapshot_envelope: SignedEnvelope<SnapshotEvent>,
    encryption_public_key: String,
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

        let authoritative_state = Arc::new(Mutex::new(config.snapshot_state.clone()));
        let clients = Arc::new(Mutex::new(HashMap::new()));
        let stop_signal = Arc::new(AtomicBool::new(false));
        let server_sequence = Arc::new(AtomicU64::new(0));

        let accept_thread = {
            let authoritative_state = Arc::clone(&authoritative_state);
            let clients = Arc::clone(&clients);
            let stop_signal = Arc::clone(&stop_signal);
            let server_sequence = Arc::clone(&server_sequence);
            let host_signing_keys = Arc::clone(&config.host_signing_keys);
            let host_encryption_keys = Arc::clone(&config.host_encryption_keys);
            let join_payload = join_payload.clone();

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
                        let authoritative_state = Arc::clone(&authoritative_state);
                        let server_sequence = Arc::clone(&server_sequence);
                        let host_signing_keys = Arc::clone(&host_signing_keys);
                        let host_encryption_keys = Arc::clone(&host_encryption_keys);
                        let join_payload = join_payload.clone();

                        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

                        thread::spawn(move || {
                            let crypto_provider = DefaultCryptoProvider;
                            let initial_request =
                                read_json_frame::<JsonSignedEnvelope>(&mut stream);

                            let response = match initial_request {
                                Ok(request_envelope) => handle_initial_client_request(
                                    &crypto_provider,
                                    request_envelope,
                                    &join_payload,
                                    &authoritative_state,
                                    &server_sequence,
                                    &host_signing_keys,
                                    &host_encryption_keys,
                                ),
                                Err(error) => Err(error),
                            };

                            match response {
                                Ok(InitialRequestAcceptance {
                                    player_id,
                                    snapshot_envelope,
                                    encryption_public_key,
                                }) => {
                                    if write_json_frame(&mut stream, &snapshot_envelope).is_ok() {
                                        let stream_handle =
                                            match stream.try_clone().map(|cloned_stream| {
                                                Arc::new(Mutex::new(cloned_stream))
                                            }) {
                                                Ok(handle) => handle,
                                                Err(_) => return,
                                            };

                                        if let Ok(mut connected_clients) = clients.lock() {
                                            connected_clients.insert(
                                                player_id.clone(),
                                                ConnectedClient {
                                                    stream: Arc::clone(&stream_handle),
                                                    encryption_public_key,
                                                },
                                            );
                                        }

                                        spawn_host_client_session(
                                            player_id,
                                            stream,
                                            authoritative_state,
                                            clients,
                                            join_payload,
                                            server_sequence,
                                            host_signing_keys,
                                            host_encryption_keys,
                                        );
                                    }
                                }
                                Err(error) => {
                                    if let Ok(envelope) = build_protocol_error_envelope(
                                        &crypto_provider,
                                        &join_payload,
                                        &server_sequence,
                                        &host_signing_keys,
                                        "JOIN_REJECTED",
                                        error.to_string(),
                                        None,
                                    ) {
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
            authoritative_state,
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

    pub fn authoritative_state(&self) -> Result<TournamentState, NetworkingError> {
        self.authoritative_state
            .lock()
            .map(|state| state.clone())
            .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))
    }

    pub fn replace_authoritative_state(
        &self,
        next_state: TournamentState,
    ) -> Result<(), NetworkingError> {
        if next_state.table_id != self.join_payload.table_id
            || next_state.session_epoch != self.join_payload.session_epoch
        {
            return Err(NetworkingError::new(
                "replacement state must match the active table/session",
            ));
        }

        self.authoritative_state
            .lock()
            .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))
            .map(|mut state| {
                *state = next_state;
            })
    }

    #[must_use]
    pub fn current_server_sequence(&self) -> u64 {
        self.server_sequence.load(Ordering::SeqCst)
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
                mark_participant_reconnect_eligible(&self.authoritative_state, &player_id)?;
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

        match write_json_frame(&mut stream, &envelope) {
            Ok(()) => Ok(()),
            Err(error) => {
                if let Ok(mut connected_clients) = self.clients.lock() {
                    connected_clients.remove(recipient_id);
                }
                let _ =
                    mark_participant_reconnect_eligible(&self.authoritative_state, recipient_id);
                Err(error)
            }
        }
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

#[derive(Debug)]
struct ClientReconnectIdentity {
    signing_keys: Option<SigningKeyMaterial>,
    encryption_keys: Option<EncryptionKeyMaterial>,
}

pub struct ClientRuntime {
    incoming: Receiver<ClientRuntimeEvent>,
    reconnect_identity: Arc<Mutex<ClientReconnectIdentity>>,
}

impl ClientRuntime {
    pub fn connect(config: ClientRuntimeConfig) -> Result<Self, NetworkingError> {
        let join_payload = decode_join_payload(&config.join_payload)
            .map_err(|error| NetworkingError::new(error.to_string()))?;
        validate_join_payload(&join_payload)
            .map_err(|error| NetworkingError::new(error.to_string()))?;

        let crypto_provider = DefaultCryptoProvider;
        let player_id = config.player_id.clone();
        let display_name = config.display_name.clone();
        let reconnect_identity = Arc::new(Mutex::new(ClientReconnectIdentity {
            signing_keys: Some(config.signing_keys),
            encryption_keys: Some(config.encryption_keys),
        }));

        let (mut stream, snapshot_envelope) = connect_and_join(
            &crypto_provider,
            &join_payload,
            &player_id,
            &display_name,
            &reconnect_identity,
        )?;
        let snapshot_event = snapshot_envelope.payload.clone();
        let snapshot_sequence = snapshot_envelope.server_sequence;

        let (sender, receiver) = mpsc::channel();
        sender
            .send(ClientRuntimeEvent::Snapshot(Box::new(snapshot_event)))
            .map_err(|error| NetworkingError::new(format!("failed to queue snapshot: {error}")))?;

        let host_signing_public_key = join_payload.host_signing_public_key.clone();
        let mut reconnect_token = snapshot_envelope.payload.reconnect_token.clone();
        let mut host_encryption_public_key = snapshot_envelope
            .payload
            .host_encryption_public_key
            .clone()
            .ok_or_else(|| NetworkingError::new("snapshot missing hostEncryptionPublicKey"))?;
        let mut last_seen_server_sequence = snapshot_sequence;
        let mut next_counter = 2;
        let reconnect_identity_for_thread = Arc::clone(&reconnect_identity);

        thread::spawn(move || {
            loop {
                let frame_value = match read_json_frame::<Value>(&mut stream) {
                    Ok(frame_value) => frame_value,
                    Err(_) => {
                        let _ = sender.send(ClientRuntimeEvent::Reconnecting {
                            player_id: player_id.clone(),
                        });

                        match reconnect_after_disconnect(
                            &crypto_provider,
                            &join_payload,
                            &player_id,
                            &reconnect_identity_for_thread,
                            reconnect_token.as_deref(),
                            last_seen_server_sequence.unwrap_or(0),
                            &mut next_counter,
                        ) {
                            Ok((reconnected_stream, snapshot_envelope)) => {
                                stream = reconnected_stream;
                                reconnect_token = snapshot_envelope.payload.reconnect_token.clone();
                                last_seen_server_sequence = snapshot_envelope.server_sequence;

                                let Some(next_host_encryption_public_key) =
                                    snapshot_envelope.payload.host_encryption_public_key.clone()
                                else {
                                    let _ = sender.send(ClientRuntimeEvent::SafeError {
                                        player_id: player_id.clone(),
                                        message:
                                            "reconnect snapshot missing hostEncryptionPublicKey"
                                                .to_string(),
                                    });
                                    break;
                                };
                                host_encryption_public_key = next_host_encryption_public_key;

                                let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(
                                    snapshot_envelope.payload,
                                )));
                                continue;
                            }
                            Err(error) => {
                                let _ = sender.send(ClientRuntimeEvent::SafeError {
                                    player_id: player_id.clone(),
                                    message: error.to_string(),
                                });
                                break;
                            }
                        }
                    }
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

                        if is_stale_server_sequence(
                            last_seen_server_sequence,
                            Some(envelope.server_sequence),
                        ) {
                            match request_resync_snapshot(
                                &crypto_provider,
                                &mut stream,
                                &join_payload,
                                &player_id,
                                &reconnect_identity_for_thread,
                                last_seen_server_sequence.unwrap_or(0),
                                &mut next_counter,
                            ) {
                                Ok(snapshot_envelope) => {
                                    reconnect_token =
                                        snapshot_envelope.payload.reconnect_token.clone();
                                    last_seen_server_sequence = snapshot_envelope.server_sequence;
                                    if let Some(next_host_encryption_public_key) =
                                        snapshot_envelope.payload.host_encryption_public_key.clone()
                                    {
                                        host_encryption_public_key =
                                            next_host_encryption_public_key;
                                    }
                                    let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(
                                        snapshot_envelope.payload,
                                    )));
                                }
                                Err(error) => {
                                    let _ = sender.send(ClientRuntimeEvent::SafeError {
                                        player_id: player_id.clone(),
                                        message: error.to_string(),
                                    });
                                    break;
                                }
                            }
                            continue;
                        }
                        last_seen_server_sequence = Some(envelope.server_sequence);

                        let encrypted_payload = crate::crypto::EncryptedPayload {
                            nonce_base64: envelope.nonce.clone(),
                            ciphertext_base64: envelope.ciphertext.clone(),
                            recipient_key_id: envelope.recipient_key_id.clone(),
                        };

                        let Ok(identity) = reconnect_identity_for_thread.lock() else {
                            let _ = sender.send(ClientRuntimeEvent::SafeError {
                                player_id: player_id.clone(),
                                message: "client reconnect identity lock poisoned".to_string(),
                            });
                            break;
                        };
                        let Some(encryption_keys) = identity.encryption_keys.as_ref() else {
                            let _ = sender.send(ClientRuntimeEvent::SafeError {
                                player_id: player_id.clone(),
                                message: missing_reconnect_identity_message(),
                            });
                            break;
                        };
                        let Ok(plaintext) = crypto_provider.decrypt(
                            encryption_keys,
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
                    "SNAPSHOT_EVENT" => {
                        let Ok(envelope) =
                            serde_json::from_value::<SignedEnvelope<SnapshotEvent>>(frame_value)
                        else {
                            continue;
                        };

                        if envelope
                            .verify(&crypto_provider, &host_signing_public_key)
                            .is_err()
                        {
                            continue;
                        }

                        last_seen_server_sequence = envelope.server_sequence;
                        reconnect_token = envelope.payload.reconnect_token.clone();
                        if let Some(next_host_encryption_public_key) =
                            envelope.payload.host_encryption_public_key.clone()
                        {
                            host_encryption_public_key = next_host_encryption_public_key;
                        }

                        let _ =
                            sender.send(ClientRuntimeEvent::Snapshot(Box::new(envelope.payload)));
                    }
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

                        if is_stale_server_sequence(
                            last_seen_server_sequence,
                            envelope.server_sequence,
                        ) {
                            let _ = sender.send(ClientRuntimeEvent::ResyncRequested {
                                player_id: player_id.clone(),
                                last_seen_server_sequence: last_seen_server_sequence.unwrap_or(0),
                            });

                            match request_resync_snapshot(
                                &crypto_provider,
                                &mut stream,
                                &join_payload,
                                &player_id,
                                &reconnect_identity_for_thread,
                                last_seen_server_sequence.unwrap_or(0),
                                &mut next_counter,
                            ) {
                                Ok(snapshot_envelope) => {
                                    reconnect_token =
                                        snapshot_envelope.payload.reconnect_token.clone();
                                    last_seen_server_sequence = snapshot_envelope.server_sequence;
                                    if let Some(next_host_encryption_public_key) =
                                        snapshot_envelope.payload.host_encryption_public_key.clone()
                                    {
                                        host_encryption_public_key =
                                            next_host_encryption_public_key;
                                    }
                                    let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(
                                        snapshot_envelope.payload,
                                    )));
                                }
                                Err(error) => {
                                    let _ = sender.send(ClientRuntimeEvent::SafeError {
                                        player_id: player_id.clone(),
                                        message: error.to_string(),
                                    });
                                    break;
                                }
                            }
                            continue;
                        }
                        last_seen_server_sequence = envelope.server_sequence;

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

        Ok(Self {
            incoming: receiver,
            reconnect_identity,
        })
    }

    pub fn next_event(&self, timeout: Duration) -> Result<ClientRuntimeEvent, NetworkingError> {
        self.incoming.recv_timeout(timeout).map_err(|error| {
            NetworkingError::new(format!("timed out waiting for client event: {error}"))
        })
    }

    pub fn clear_reconnect_identity(&self) -> Result<(), NetworkingError> {
        self.reconnect_identity
            .lock()
            .map_err(|_| NetworkingError::new("client reconnect identity lock poisoned"))
            .map(|mut identity| {
                identity.signing_keys = None;
                identity.encryption_keys = None;
            })
    }

    pub fn replace_reconnect_identity(
        &self,
        signing_keys: SigningKeyMaterial,
        encryption_keys: EncryptionKeyMaterial,
    ) -> Result<(), NetworkingError> {
        self.reconnect_identity
            .lock()
            .map_err(|_| NetworkingError::new("client reconnect identity lock poisoned"))
            .map(|mut identity| {
                identity.signing_keys = Some(signing_keys);
                identity.encryption_keys = Some(encryption_keys);
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
    Reconnecting {
        player_id: String,
    },
    ResyncRequested {
        player_id: String,
        last_seen_server_sequence: u64,
    },
    SafeError {
        player_id: String,
        message: String,
    },
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

fn handle_initial_client_request(
    crypto_provider: &impl ProtocolCryptoProvider,
    request_envelope: JsonSignedEnvelope,
    join_payload: &JoinPayload,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    server_sequence: &Arc<AtomicU64>,
    host_signing_keys: &SigningKeyMaterial,
    host_encryption_keys: &Arc<Mutex<EncryptionKeyMaterial>>,
) -> Result<InitialRequestAcceptance, NetworkingError> {
    match request_envelope.message_type {
        ProtocolMessageType::JoinTournamentRequest => handle_join_request(
            crypto_provider,
            request_envelope,
            join_payload,
            authoritative_state,
            server_sequence,
            host_signing_keys,
            host_encryption_keys,
        ),
        ProtocolMessageType::ReconnectTournamentRequest => handle_reconnect_request(
            crypto_provider,
            request_envelope,
            join_payload,
            authoritative_state,
            server_sequence,
            host_signing_keys,
            host_encryption_keys,
        ),
        _ => Err(NetworkingError::new(
            "first client message must be JOIN_TOURNAMENT_REQUEST or RECONNECT_TOURNAMENT_REQUEST",
        )),
    }
}

fn handle_join_request(
    crypto_provider: &impl ProtocolCryptoProvider,
    request_envelope: JsonSignedEnvelope,
    join_payload: &JoinPayload,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    server_sequence: &Arc<AtomicU64>,
    host_signing_keys: &SigningKeyMaterial,
    host_encryption_keys: &Arc<Mutex<EncryptionKeyMaterial>>,
) -> Result<InitialRequestAcceptance, NetworkingError> {
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

    let player_id = request_envelope.sender_id.clone();
    let reconnect_token = issue_reconnect_token();
    {
        let mut state = authoritative_state
            .lock()
            .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))?;

        if state
            .participants
            .get(&player_id)
            .is_some_and(|participant| participant.state != ParticipantState::Removed)
        {
            return Err(NetworkingError::new(
                "playerId already exists; use reconnect with the original identity",
            ));
        }

        state.participants.insert(
            player_id.clone(),
            ParticipantRegistryEntry {
                identity: PlayerIdentity {
                    player_id: player_id.clone(),
                    display_name: request.display_name.clone(),
                    signing_public_key: request.signing_public_key.clone(),
                    encryption_public_key: request.encryption_public_key.clone(),
                    signing_key_fingerprint: key_fingerprint(
                        &base64::engine::general_purpose::URL_SAFE_NO_PAD
                            .decode(request.signing_public_key.as_bytes())
                            .map_err(|error| {
                                NetworkingError::new(format!("invalid signing public key: {error}"))
                            })?,
                    ),
                },
                state: ParticipantState::Admitted,
                connection_state: ConnectionState::Connected,
                seat_index: None,
                admitted_at_ms: now_epoch_ms(),
                reconnect_token: Some(reconnect_token),
                reconnect_expiry_ms: None,
                is_host: false,
            },
        );
    }

    let snapshot_envelope = build_snapshot_envelope(
        crypto_provider,
        join_payload,
        authoritative_state,
        server_sequence,
        host_signing_keys,
        host_encryption_keys,
        &player_id,
    )?;

    Ok(InitialRequestAcceptance {
        player_id,
        snapshot_envelope,
        encryption_public_key: request.encryption_public_key,
    })
}

fn handle_reconnect_request(
    crypto_provider: &impl ProtocolCryptoProvider,
    request_envelope: JsonSignedEnvelope,
    join_payload: &JoinPayload,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    server_sequence: &Arc<AtomicU64>,
    host_signing_keys: &SigningKeyMaterial,
    host_encryption_keys: &Arc<Mutex<EncryptionKeyMaterial>>,
) -> Result<InitialRequestAcceptance, NetworkingError> {
    let request: ReconnectTournamentRequest =
        serde_json::from_value(request_envelope.payload.clone()).map_err(|error| {
            NetworkingError::new(format!("invalid reconnect request payload: {error}"))
        })?;

    if request.player_id != request_envelope.sender_id {
        return Err(NetworkingError::new(
            "reconnect requires the same playerId in senderId and payload",
        ));
    }

    let player_id = request.player_id.clone();
    {
        let mut state = authoritative_state
            .lock()
            .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))?;
        let tournament_phase = state.phase;
        let participant = state
            .participants
            .get_mut(&player_id)
            .ok_or_else(|| NetworkingError::new("unknown reconnect playerId"))?;

        request_envelope
            .verify(crypto_provider, &participant.identity.signing_public_key)
            .map_err(|error| NetworkingError::new(error.to_string()))?;

        if participant.reconnect_token.as_deref() != Some(request.reconnect_token.as_str()) {
            return Err(NetworkingError::new("reconnect token mismatch"));
        }

        if participant.connection_state == ConnectionState::Connected {
            return Err(NetworkingError::new("participant is already connected"));
        }

        if !is_reconnectable_participant(participant) {
            return Err(NetworkingError::new(
                "participant is not reconnect-eligible",
            ));
        }

        if participant
            .reconnect_expiry_ms
            .is_some_and(|expiry_ms| expiry_ms < now_epoch_ms())
        {
            return Err(NetworkingError::new("reconnect token expired"));
        }

        let authoritative_sequence = server_sequence.load(Ordering::SeqCst);
        if request
            .last_known_server_seq
            .is_some_and(|sequence| sequence > authoritative_sequence)
        {
            return Err(NetworkingError::new(
                "reconnect lastKnownServerSeq is ahead of the host sequence",
            ));
        }

        participant.connection_state = ConnectionState::Connected;
        restore_participant_after_reconnect(participant, tournament_phase);
    }

    let encryption_public_key = authoritative_state
        .lock()
        .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))?
        .participants
        .get(&player_id)
        .map(|participant| participant.identity.encryption_public_key.clone())
        .ok_or_else(|| NetworkingError::new("participant registry missing after reconnect"))?;

    let snapshot_envelope = build_snapshot_envelope(
        crypto_provider,
        join_payload,
        authoritative_state,
        server_sequence,
        host_signing_keys,
        host_encryption_keys,
        &player_id,
    )?;

    Ok(InitialRequestAcceptance {
        player_id,
        snapshot_envelope,
        encryption_public_key,
    })
}

#[allow(clippy::too_many_arguments)]
fn spawn_host_client_session(
    player_id: String,
    mut stream: TcpStream,
    authoritative_state: Arc<Mutex<TournamentState>>,
    clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
    join_payload: JoinPayload,
    server_sequence: Arc<AtomicU64>,
    host_signing_keys: Arc<SigningKeyMaterial>,
    host_encryption_keys: Arc<Mutex<EncryptionKeyMaterial>>,
) {
    thread::spawn(move || {
        let crypto_provider = DefaultCryptoProvider;

        loop {
            let next_request = read_json_frame::<JsonSignedEnvelope>(&mut stream);
            let request_envelope = match next_request {
                Ok(request_envelope) => request_envelope,
                Err(_) => {
                    let _ = clients.lock().map(|mut connected_clients| {
                        connected_clients.remove(&player_id);
                    });
                    let _ = mark_participant_reconnect_eligible(&authoritative_state, &player_id);
                    break;
                }
            };

            match request_envelope.message_type {
                ProtocolMessageType::ResyncRequest => {
                    let rejected_message_id = request_envelope.message_id.clone();
                    let response = handle_resync_request(
                        &crypto_provider,
                        request_envelope,
                        &join_payload,
                        &authoritative_state,
                        &server_sequence,
                        &host_signing_keys,
                        &host_encryption_keys,
                    );

                    match response {
                        Ok(snapshot_envelope) => {
                            if write_json_frame(&mut stream, &snapshot_envelope).is_err() {
                                let _ = clients.lock().map(|mut connected_clients| {
                                    connected_clients.remove(&player_id);
                                });
                                let _ = mark_participant_reconnect_eligible(
                                    &authoritative_state,
                                    &player_id,
                                );
                                break;
                            }
                        }
                        Err(error) => {
                            if let Ok(envelope) = build_protocol_error_envelope(
                                &crypto_provider,
                                &join_payload,
                                &server_sequence,
                                &host_signing_keys,
                                "RESYNC_REJECTED",
                                error.to_string(),
                                Some(rejected_message_id),
                            ) {
                                let _ = write_json_frame(&mut stream, &envelope);
                            }
                        }
                    }
                }
                _ => {
                    if let Ok(envelope) = build_protocol_error_envelope(
                        &crypto_provider,
                        &join_payload,
                        &server_sequence,
                        &host_signing_keys,
                        "UNSUPPORTED_REQUEST",
                        "host runtime only supports RESYNC_REQUEST after connect".to_string(),
                        Some(request_envelope.message_id),
                    ) {
                        let _ = write_json_frame(&mut stream, &envelope);
                    }
                }
            }
        }
    });
}

fn handle_resync_request(
    crypto_provider: &impl ProtocolCryptoProvider,
    request_envelope: JsonSignedEnvelope,
    join_payload: &JoinPayload,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    server_sequence: &Arc<AtomicU64>,
    host_signing_keys: &SigningKeyMaterial,
    host_encryption_keys: &Arc<Mutex<EncryptionKeyMaterial>>,
) -> Result<SignedEnvelope<SnapshotEvent>, NetworkingError> {
    let request: ResyncRequest =
        serde_json::from_value(request_envelope.payload.clone()).map_err(|error| {
            NetworkingError::new(format!("invalid resync request payload: {error}"))
        })?;

    {
        let state = authoritative_state
            .lock()
            .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))?;
        let participant = state
            .participants
            .get(&request_envelope.sender_id)
            .ok_or_else(|| NetworkingError::new("resync requester is not registered"))?;

        request_envelope
            .verify(crypto_provider, &participant.identity.signing_public_key)
            .map_err(|error| NetworkingError::new(error.to_string()))?;
    }

    if request
        .last_seen_server_sequence
        .is_some_and(|sequence| sequence > server_sequence.load(Ordering::SeqCst))
    {
        return Err(NetworkingError::new(
            "resync lastSeenServerSequence is ahead of the host sequence",
        ));
    }

    build_snapshot_envelope(
        crypto_provider,
        join_payload,
        authoritative_state,
        server_sequence,
        host_signing_keys,
        host_encryption_keys,
        &request_envelope.sender_id,
    )
}

fn build_snapshot_envelope(
    crypto_provider: &impl ProtocolCryptoProvider,
    join_payload: &JoinPayload,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    server_sequence: &Arc<AtomicU64>,
    host_signing_keys: &SigningKeyMaterial,
    host_encryption_keys: &Arc<Mutex<EncryptionKeyMaterial>>,
    player_id: &str,
) -> Result<SignedEnvelope<SnapshotEvent>, NetworkingError> {
    let authoritative_snapshot = authoritative_state
        .lock()
        .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))?
        .clone();
    let reconnect_token = authoritative_snapshot
        .participants
        .get(player_id)
        .map(|participant| participant.reconnect_token.clone())
        .ok_or_else(|| NetworkingError::new("snapshot target is not registered"))?;
    let host_encryption_public_key = host_encryption_keys
        .lock()
        .map_err(|_| NetworkingError::new("host encryption key lock poisoned"))?
        .public_key_base64();
    let next_server_sequence = server_sequence.fetch_add(1, Ordering::SeqCst) + 1;

    let mut snapshot_envelope = SignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::SnapshotEvent,
        table_id: join_payload.table_id.clone(),
        session_epoch: join_payload.session_epoch,
        sender_id: "host".to_string(),
        counter: next_server_sequence,
        message_id: format!("snapshot-{next_server_sequence}"),
        server_sequence: Some(next_server_sequence),
        payload: SnapshotEvent {
            state: SnapshotState {
                state: authoritative_snapshot,
                local_player_id: player_id.to_string(),
                reconnect_token: reconnect_token.clone(),
                host_signing_public_key: Some(join_payload.host_signing_public_key.clone()),
                host_encryption_public_key: Some(host_encryption_public_key.clone()),
            },
            local_player_id: player_id.to_string(),
            reconnect_token,
            host_signing_public_key: Some(join_payload.host_signing_public_key.clone()),
            host_encryption_public_key: Some(host_encryption_public_key),
        },
        signature: None,
    };

    snapshot_envelope
        .sign(crypto_provider, host_signing_keys)
        .map_err(|error| NetworkingError::new(error.to_string()))?;

    Ok(snapshot_envelope)
}

fn build_protocol_error_envelope(
    crypto_provider: &impl ProtocolCryptoProvider,
    join_payload: &JoinPayload,
    server_sequence: &Arc<AtomicU64>,
    host_signing_keys: &SigningKeyMaterial,
    code: &str,
    message: String,
    rejected_message_id: Option<String>,
) -> Result<SignedEnvelope<ProtocolErrorMessage>, NetworkingError> {
    let next_server_sequence = server_sequence.fetch_add(1, Ordering::SeqCst) + 1;
    let mut envelope = SignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::ProtocolError,
        table_id: join_payload.table_id.clone(),
        session_epoch: join_payload.session_epoch,
        sender_id: "host".to_string(),
        counter: next_server_sequence,
        message_id: format!("error-{next_server_sequence}"),
        server_sequence: Some(next_server_sequence),
        payload: ProtocolErrorMessage {
            code: code.to_string(),
            message,
            rejected_message_id,
        },
        signature: None,
    };

    envelope
        .sign(crypto_provider, host_signing_keys)
        .map_err(|error| NetworkingError::new(error.to_string()))?;

    Ok(envelope)
}

fn connect_to_host(join_payload: &JoinPayload) -> Result<TcpStream, NetworkingError> {
    let stream =
        TcpStream::connect((join_payload.host_address.as_str(), join_payload.host_port))
            .map_err(|error| NetworkingError::new(format!("failed to connect to host: {error}")))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    Ok(stream)
}

fn connect_and_join(
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

    Ok((stream, snapshot))
}

fn reconnect_after_disconnect(
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
    let mut stream = connect_to_host(join_payload)?;
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

    write_json_frame(&mut stream, &reconnect_envelope)?;
    let snapshot = read_snapshot_response(crypto_provider, &mut stream, join_payload)?;

    Ok((stream, snapshot))
}

fn request_resync_snapshot(
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

fn read_snapshot_response(
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

fn mark_participant_reconnect_eligible(
    authoritative_state: &Arc<Mutex<TournamentState>>,
    player_id: &str,
) -> Result<(), NetworkingError> {
    let mut state = authoritative_state
        .lock()
        .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))?;
    let tournament_phase = state.phase;
    let hand_is_active = state.current_hand.is_some();
    if let Some(participant) = state.participants.get_mut(player_id) {
        if participant.state == ParticipantState::Removed {
            return Ok(());
        }

        participant.connection_state = ConnectionState::Reconnecting;
        if participant.state != ParticipantState::EliminatedObserver {
            participant.state = ParticipantState::Reconnecting;
        }
        let reconnect_state = participant.state;
        participant.reconnect_expiry_ms = Some(
            now_epoch_ms() + reconnect_window_ms(tournament_phase, reconnect_state, hand_is_active),
        );
    }

    Ok(())
}

fn reconnect_window_ms(
    tournament_phase: TournamentPhase,
    participant_state: ParticipantState,
    hand_is_active: bool,
) -> u64 {
    if participant_state == ParticipantState::EliminatedObserver {
        300_000
    } else if tournament_phase != TournamentPhase::Running {
        120_000
    } else if hand_is_active {
        30_000
    } else {
        120_000
    }
}

fn restore_participant_after_reconnect(
    participant: &mut ParticipantRegistryEntry,
    tournament_phase: TournamentPhase,
) {
    participant.connection_state = ConnectionState::Connected;
    participant.reconnect_expiry_ms = None;
    if participant.state == ParticipantState::EliminatedObserver {
        return;
    }

    participant.state = if participant.seat_index.is_some() {
        if tournament_phase == TournamentPhase::Running {
            ParticipantState::Active
        } else {
            ParticipantState::Seated
        }
    } else {
        ParticipantState::Admitted
    };
}

fn is_reconnectable_participant(participant: &ParticipantRegistryEntry) -> bool {
    matches!(
        participant.state,
        ParticipantState::Seated
            | ParticipantState::Active
            | ParticipantState::EliminatedObserver
            | ParticipantState::Reconnecting
            | ParticipantState::Admitted
    ) && participant.state != ParticipantState::Removed
}

fn issue_reconnect_token() -> String {
    let mut bytes = [0_u8; 24];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn missing_reconnect_identity_message() -> String {
    "original reconnect identity is unavailable; v1 requires the original ephemeral signing/encryption keypair"
        .to_string()
}

fn is_stale_server_sequence(
    last_seen_server_sequence: Option<u64>,
    next_server_sequence: Option<u64>,
) -> bool {
    matches!(
        (last_seen_server_sequence, next_server_sequence),
        (Some(last_seen), Some(next_sequence)) if next_sequence <= last_seen
    )
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
        net::Shutdown,
        sync::{
            atomic::{AtomicU64, Ordering},
            Arc, Mutex,
        },
        thread,
        time::{Duration, Instant},
    };

    use base64::Engine as _;
    use serde_json::json;

    use super::{
        handle_reconnect_request, handle_resync_request, now_epoch_ms, validate_production_host_ip,
    };
    use crate::{
        crypto::{key_fingerprint, DefaultCryptoProvider, ProtocolCryptoProvider},
        domain::{
            ActionType, ActionWindow, BettingRoundState, BlindLevel, BlindSchedule, Card,
            ConnectionState, HandCyclePhase, HandParticipationState, HandState, JoinPayload,
            ParticipantRegistryEntry, ParticipantState, PlacementEntry, PlayerIdentity, Rank,
            SeatOccupancyState, SeatState, StreetPhase, Suit, TournamentConfig, TournamentPhase,
            TournamentSeatState, TournamentState,
        },
        networking::{
            resolve_connectable_host_ip, ClientRuntime, ClientRuntimeConfig, ClientRuntimeEvent,
            HostRuntimeConfig, HostRuntimeMode, HostServer,
        },
        protocol::{
            test_support::{sample_reconnect_request, sample_resync_request},
            ActionWindowOpened, EliminationEvent, JsonSignedEnvelope, PrivateHoleCardsEvent,
            ProtocolMessageType, ReconnectTournamentRequest, ResyncRequest, SignedEnvelope,
            SnapshotEvent, TournamentCompleteEvent, TournamentStartedEvent, PROTOCOL_VERSION,
        },
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

    fn bind_test_host(
        provider: &DefaultCryptoProvider,
        table_id: &str,
        session_epoch: u64,
    ) -> HostServer {
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

        HostServer::bind(HostRuntimeConfig {
            bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
            advertised_host: "127.0.0.1".to_string(),
            session_epoch,
            table_id: table_id.to_string(),
            table_name: Some(format!("Table {session_epoch}")),
            join_token: "join-token".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: sample_tournament_state(table_id, session_epoch),
            runtime_mode: HostRuntimeMode::Test,
        })
        .expect("host should bind")
    }

    fn connect_test_client(
        provider: &DefaultCryptoProvider,
        host: &HostServer,
        player_id: &str,
        display_name: &str,
    ) -> ClientRuntime {
        ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: host.encoded_join_payload().to_string(),
            player_id: player_id.to_string(),
            display_name: display_name.to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        })
        .expect("client should connect")
    }

    fn expect_snapshot_event(client: &ClientRuntime) -> SnapshotEvent {
        match client
            .next_event(Duration::from_secs(2))
            .expect("snapshot event")
        {
            ClientRuntimeEvent::Snapshot(snapshot) => *snapshot,
            other => panic!("expected snapshot event, got {other:?}"),
        }
    }

    fn assert_public_event(
        client: &ClientRuntime,
        expected_message_type: ProtocolMessageType,
    ) -> serde_json::Value {
        match client
            .next_event(Duration::from_secs(2))
            .expect("public event")
        {
            ClientRuntimeEvent::PublicEvent {
                message_type,
                payload,
            } => {
                assert_eq!(message_type, expected_message_type);
                payload
            }
            other => panic!("expected public event, got {other:?}"),
        }
    }

    fn disconnect_client(host: &HostServer, player_id: &str) {
        let deadline = Instant::now() + Duration::from_secs(2);
        let stream = loop {
            if let Some(stream) = host
                .clients
                .lock()
                .expect("client registry")
                .get(player_id)
                .map(|client| {
                    client
                        .stream
                        .lock()
                        .expect("client stream")
                        .try_clone()
                        .expect("clone stream")
                })
            {
                break stream;
            }

            assert!(Instant::now() < deadline, "connected client");
            thread::sleep(Duration::from_millis(20));
        };
        stream
            .shutdown(Shutdown::Both)
            .expect("shutdown client stream");
    }

    fn wait_for_host_participant_state(
        host: &HostServer,
        player_id: &str,
        expected_state: ParticipantState,
        expected_connection_state: ConnectionState,
    ) {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let state = host.authoritative_state().expect("authoritative state");
            if let Some(participant) = state.participants.get(player_id) {
                if participant.state == expected_state
                    && participant.connection_state == expected_connection_state
                {
                    return;
                }
            }

            assert!(
                Instant::now() < deadline,
                "participant state did not converge"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn sample_join_payload_for_tests(
        table_id: &str,
        session_epoch: u64,
        host_signing_public_key: String,
    ) -> JoinPayload {
        JoinPayload {
            payload_version: PROTOCOL_VERSION,
            host_address: "127.0.0.1".to_string(),
            host_port: 43_818,
            table_id: table_id.to_string(),
            session_epoch,
            host_signing_public_key,
            join_token: "join-token".to_string(),
            generated_at_ms: 1,
            table_name: Some("Reconnect Test".to_string()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn sample_participant_entry(
        player_id: &str,
        display_name: &str,
        signing_public_key: String,
        encryption_public_key: String,
        state: ParticipantState,
        connection_state: ConnectionState,
        seat_index: Option<u8>,
        reconnect_token: &str,
    ) -> ParticipantRegistryEntry {
        let signing_key_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(signing_public_key.as_bytes())
            .expect("valid signing key bytes");

        ParticipantRegistryEntry {
            identity: PlayerIdentity {
                player_id: player_id.to_string(),
                display_name: display_name.to_string(),
                signing_public_key,
                encryption_public_key,
                signing_key_fingerprint: key_fingerprint(&signing_key_bytes),
            },
            state,
            connection_state,
            seat_index,
            admitted_at_ms: 1,
            reconnect_token: Some(reconnect_token.to_string()),
            reconnect_expiry_ms: Some(now_epoch_ms() + 60_000),
            is_host: false,
        }
    }

    fn signed_reconnect_envelope(
        provider: &DefaultCryptoProvider,
        signing_keys: &crate::crypto::SigningKeyMaterial,
        join_payload: &JoinPayload,
        request: ReconnectTournamentRequest,
        counter: u64,
        message_id: &str,
    ) -> JsonSignedEnvelope {
        let mut envelope = SignedEnvelope {
            protocol_version: PROTOCOL_VERSION,
            message_type: ProtocolMessageType::ReconnectTournamentRequest,
            table_id: join_payload.table_id.clone(),
            session_epoch: join_payload.session_epoch,
            sender_id: request.player_id.clone(),
            counter,
            message_id: message_id.to_string(),
            server_sequence: None,
            payload: serde_json::to_value(request).expect("reconnect request payload"),
            signature: None,
        };
        envelope
            .sign(provider, signing_keys)
            .expect("reconnect request should sign");

        envelope
    }

    fn signed_resync_envelope(
        provider: &DefaultCryptoProvider,
        signing_keys: &crate::crypto::SigningKeyMaterial,
        join_payload: &JoinPayload,
        sender_id: &str,
        request: ResyncRequest,
        counter: u64,
        message_id: &str,
    ) -> JsonSignedEnvelope {
        let mut envelope = SignedEnvelope {
            protocol_version: PROTOCOL_VERSION,
            message_type: ProtocolMessageType::ResyncRequest,
            table_id: join_payload.table_id.clone(),
            session_epoch: join_payload.session_epoch,
            sender_id: sender_id.to_string(),
            counter,
            message_id: message_id.to_string(),
            server_sequence: None,
            payload: serde_json::to_value(request).expect("resync request payload"),
            signature: None,
        };
        envelope
            .sign(provider, signing_keys)
            .expect("resync request should sign");

        envelope
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
    fn multiple_clients_keep_distinct_identity_and_reconnect_state() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

        let host = HostServer::bind(HostRuntimeConfig {
            bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
            advertised_host: "127.0.0.1".to_string(),
            session_epoch: 8,
            table_id: "table-multi-client".to_string(),
            table_name: Some("Multi Client".to_string()),
            join_token: "join-token".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: sample_tournament_state("table-multi-client", 8),
            runtime_mode: HostRuntimeMode::Test,
        })
        .expect("host should bind");

        let alice_signing_keys = provider.generate_signing_keypair();
        let alice_signing_public_key = alice_signing_keys.public_key_base64();
        let alice_encryption_keys = provider.generate_encryption_keypair();
        let alice_encryption_public_key = alice_encryption_keys.public_key_base64();
        let alice = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: host.encoded_join_payload().to_string(),
            player_id: "player-a".to_string(),
            display_name: "Alice".to_string(),
            signing_keys: alice_signing_keys,
            encryption_keys: alice_encryption_keys,
        })
        .expect("alice should connect");

        let bob_signing_keys = provider.generate_signing_keypair();
        let bob_signing_public_key = bob_signing_keys.public_key_base64();
        let bob_encryption_keys = provider.generate_encryption_keypair();
        let bob_encryption_public_key = bob_encryption_keys.public_key_base64();
        let bob = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: host.encoded_join_payload().to_string(),
            player_id: "player-b".to_string(),
            display_name: "Bob".to_string(),
            signing_keys: bob_signing_keys,
            encryption_keys: bob_encryption_keys,
        })
        .expect("bob should connect");

        let alice_reconnect_token = match alice
            .next_event(Duration::from_secs(2))
            .expect("alice snapshot")
        {
            ClientRuntimeEvent::Snapshot(snapshot) => snapshot
                .reconnect_token
                .clone()
                .expect("alice reconnect token"),
            other => panic!("expected alice snapshot, got {other:?}"),
        };
        let bob_reconnect_token = match bob
            .next_event(Duration::from_secs(2))
            .expect("bob snapshot")
        {
            ClientRuntimeEvent::Snapshot(snapshot) => snapshot
                .reconnect_token
                .clone()
                .expect("bob reconnect token"),
            other => panic!("expected bob snapshot, got {other:?}"),
        };

        assert_ne!(alice_reconnect_token, bob_reconnect_token);

        let state = host.authoritative_state().expect("authoritative state");
        assert_eq!(state.participants.len(), 2);
        let alice_participant = state.participants.get("player-a").expect("alice state");
        let bob_participant = state.participants.get("player-b").expect("bob state");
        assert_eq!(
            alice_participant.identity.signing_public_key,
            alice_signing_public_key
        );
        assert_eq!(
            alice_participant.identity.encryption_public_key,
            alice_encryption_public_key
        );
        assert_eq!(
            bob_participant.identity.signing_public_key,
            bob_signing_public_key
        );
        assert_eq!(
            bob_participant.identity.encryption_public_key,
            bob_encryption_public_key
        );
        assert_ne!(
            alice_participant.identity.signing_public_key,
            bob_participant.identity.signing_public_key
        );
    }

    #[test]
    fn two_local_clients_can_join_and_receive_live_table_events_on_one_machine() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

        let host = HostServer::bind(HostRuntimeConfig {
            bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
            advertised_host: "127.0.0.1".to_string(),
            session_epoch: 9,
            table_id: "table-local-play".to_string(),
            table_name: Some("Local Play".to_string()),
            join_token: "join-token".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: sample_tournament_state("table-local-play", 9),
            runtime_mode: HostRuntimeMode::Test,
        })
        .expect("host should bind");

        let alice = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: host.encoded_join_payload().to_string(),
            player_id: "player-local-a".to_string(),
            display_name: "Alice".to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        })
        .expect("alice should connect");
        let bob = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: host.encoded_join_payload().to_string(),
            player_id: "player-local-b".to_string(),
            display_name: "Bob".to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        })
        .expect("bob should connect");

        let _ = alice
            .next_event(Duration::from_secs(2))
            .expect("alice snapshot");
        let _ = bob
            .next_event(Duration::from_secs(2))
            .expect("bob snapshot");

        host.broadcast_public_event(
            ProtocolMessageType::TournamentStartedEvent,
            &TournamentStartedEvent {
                tournament_name: "Local Play".to_string(),
                starting_stack: 1500,
                blind_schedule_preset: "FAST".to_string(),
                frozen_player_ids: vec!["player-local-a".to_string(), "player-local-b".to_string()],
            },
        )
        .expect("broadcast tournament started");

        for client in [&alice, &bob] {
            match client
                .next_event(Duration::from_secs(2))
                .expect("public event")
            {
                ClientRuntimeEvent::PublicEvent {
                    message_type,
                    payload,
                } => {
                    assert_eq!(message_type, ProtocolMessageType::TournamentStartedEvent);
                    assert_eq!(payload.get("tournamentName"), Some(&json!("Local Play")));
                }
                other => panic!("expected public event, got {other:?}"),
            }
        }

        host.send_private_hole_cards(
            "player-local-a",
            &PrivateHoleCardsEvent {
                recipient_player_id: "player-local-a".to_string(),
                hole_cards: vec![
                    Card {
                        rank: Rank::Ace,
                        suit: Suit::Spades,
                    },
                    Card {
                        rank: Rank::Ace,
                        suit: Suit::Hearts,
                    },
                ],
            },
        )
        .expect("send alice private cards");
        host.send_private_hole_cards(
            "player-local-b",
            &PrivateHoleCardsEvent {
                recipient_player_id: "player-local-b".to_string(),
                hole_cards: vec![
                    Card {
                        rank: Rank::King,
                        suit: Suit::Clubs,
                    },
                    Card {
                        rank: Rank::Queen,
                        suit: Suit::Diamonds,
                    },
                ],
            },
        )
        .expect("send bob private cards");

        match alice
            .next_event(Duration::from_secs(2))
            .expect("alice private cards")
        {
            ClientRuntimeEvent::PrivateHoleCards(payload) => {
                assert_eq!(payload.recipient_player_id, "player-local-a");
                assert_eq!(payload.hole_cards.len(), 2);
            }
            other => panic!("expected alice private cards, got {other:?}"),
        }

        match bob
            .next_event(Duration::from_secs(2))
            .expect("bob private cards")
        {
            ClientRuntimeEvent::PrivateHoleCards(payload) => {
                assert_eq!(payload.recipient_player_id, "player-local-b");
                assert_eq!(payload.hole_cards.len(), 2);
            }
            other => panic!("expected bob private cards, got {other:?}"),
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
    fn reconnect_succeeds_only_with_original_keypair_and_valid_token() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let original_signing_keys = provider.generate_signing_keypair();
        let original_encryption_keys = provider.generate_encryption_keypair();

        let host = HostServer::bind(HostRuntimeConfig {
            bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
            advertised_host: "127.0.0.1".to_string(),
            session_epoch: 21,
            table_id: "table-reconnect-success".to_string(),
            table_name: Some("Reconnect Success".to_string()),
            join_token: "join-token".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: sample_tournament_state("table-reconnect-success", 21),
            runtime_mode: HostRuntimeMode::Test,
        })
        .expect("host should bind");

        let client = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: host.encoded_join_payload().to_string(),
            player_id: "player-reconnect".to_string(),
            display_name: "Reconnect".to_string(),
            signing_keys: original_signing_keys,
            encryption_keys: original_encryption_keys,
        })
        .expect("client should connect");

        let initial_snapshot = client
            .next_event(Duration::from_secs(2))
            .expect("initial snapshot");
        match initial_snapshot {
            ClientRuntimeEvent::Snapshot(snapshot) => {
                assert!(snapshot.reconnect_token.is_some());
            }
            other => panic!("expected snapshot event, got {other:?}"),
        }

        disconnect_client(&host, "player-reconnect");

        match client
            .next_event(Duration::from_secs(2))
            .expect("reconnecting event")
        {
            ClientRuntimeEvent::Reconnecting { player_id } => {
                assert_eq!(player_id, "player-reconnect");
            }
            other => panic!("expected reconnecting event, got {other:?}"),
        }

        match client
            .next_event(Duration::from_secs(2))
            .expect("reconnected snapshot")
        {
            ClientRuntimeEvent::Snapshot(snapshot) => {
                assert_eq!(snapshot.local_player_id, "player-reconnect");
                assert!(snapshot.reconnect_token.is_some());
            }
            other => panic!("expected reconnected snapshot, got {other:?}"),
        }

        wait_for_host_participant_state(
            &host,
            "player-reconnect",
            ParticipantState::Admitted,
            ConnectionState::Connected,
        );
    }

    #[test]
    fn reconnect_fails_with_regenerated_keypair() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

        let host = HostServer::bind(HostRuntimeConfig {
            bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
            advertised_host: "127.0.0.1".to_string(),
            session_epoch: 22,
            table_id: "table-reconnect-fail".to_string(),
            table_name: Some("Reconnect Fail".to_string()),
            join_token: "join-token".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: sample_tournament_state("table-reconnect-fail", 22),
            runtime_mode: HostRuntimeMode::Test,
        })
        .expect("host should bind");

        let client = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: host.encoded_join_payload().to_string(),
            player_id: "player-regenerated".to_string(),
            display_name: "Regenerated".to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        })
        .expect("client should connect");
        let _ = client.next_event(Duration::from_secs(2)).expect("snapshot");

        client
            .replace_reconnect_identity(
                provider.generate_signing_keypair(),
                provider.generate_encryption_keypair(),
            )
            .expect("replace reconnect identity");

        disconnect_client(&host, "player-regenerated");

        match client
            .next_event(Duration::from_secs(2))
            .expect("reconnecting event")
        {
            ClientRuntimeEvent::Reconnecting { player_id } => {
                assert_eq!(player_id, "player-regenerated");
            }
            other => panic!("expected reconnecting event, got {other:?}"),
        }

        match client
            .next_event(Duration::from_secs(2))
            .expect("safe error event")
        {
            ClientRuntimeEvent::SafeError { player_id, message } => {
                assert_eq!(player_id, "player-regenerated");
                assert!(message.contains("signature verification failed"));
            }
            other => panic!("expected safe error event, got {other:?}"),
        }
    }

    #[test]
    fn reconnect_rejects_stale_tokens() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let player_signing_keys = provider.generate_signing_keypair();
        let player_encryption_keys = provider.generate_encryption_keypair();
        let join_payload = sample_join_payload_for_tests(
            "table-reconnect-stale-token",
            31,
            host_signing_keys.public_key_base64(),
        );
        let server_sequence = Arc::new(AtomicU64::new(0));
        let mut state = sample_tournament_state("table-reconnect-stale-token", 31);
        state.participants.insert(
            "player-stale-token".to_string(),
            sample_participant_entry(
                "player-stale-token",
                "Stale Token",
                player_signing_keys.public_key_base64(),
                player_encryption_keys.public_key_base64(),
                ParticipantState::Reconnecting,
                ConnectionState::Reconnecting,
                Some(0),
                "expected-token",
            ),
        );
        let authoritative_state = Arc::new(Mutex::new(state));

        let result = handle_reconnect_request(
            &provider,
            signed_reconnect_envelope(
                &provider,
                &player_signing_keys,
                &join_payload,
                ReconnectTournamentRequest {
                    player_id: "player-stale-token".to_string(),
                    reconnect_token: "wrong-token".to_string(),
                    ..sample_reconnect_request(Some(1))
                },
                1,
                "reconnect-stale-token",
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        );

        assert!(result.is_err());
        assert!(result
            .expect_err("reconnect should fail")
            .to_string()
            .contains("reconnect token mismatch"));
    }

    #[test]
    fn reconnect_rejects_already_connected_participants() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let player_signing_keys = provider.generate_signing_keypair();
        let player_encryption_keys = provider.generate_encryption_keypair();
        let join_payload = sample_join_payload_for_tests(
            "table-reconnect-connected",
            32,
            host_signing_keys.public_key_base64(),
        );
        let server_sequence = Arc::new(AtomicU64::new(4));
        let mut state = sample_tournament_state("table-reconnect-connected", 32);
        state.phase = TournamentPhase::Running;
        state.participants.insert(
            "player-connected".to_string(),
            sample_participant_entry(
                "player-connected",
                "Connected",
                player_signing_keys.public_key_base64(),
                player_encryption_keys.public_key_base64(),
                ParticipantState::Active,
                ConnectionState::Connected,
                Some(0),
                "connected-token",
            ),
        );
        let authoritative_state = Arc::new(Mutex::new(state));

        let result = handle_reconnect_request(
            &provider,
            signed_reconnect_envelope(
                &provider,
                &player_signing_keys,
                &join_payload,
                ReconnectTournamentRequest {
                    player_id: "player-connected".to_string(),
                    reconnect_token: "connected-token".to_string(),
                    last_known_server_seq: Some(4),
                },
                1,
                "reconnect-connected",
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        );

        assert!(result.is_err());
        assert!(result
            .expect_err("reconnect should fail")
            .to_string()
            .contains("already connected"));
    }

    #[test]
    fn reconnect_rejects_removed_participants() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let player_signing_keys = provider.generate_signing_keypair();
        let player_encryption_keys = provider.generate_encryption_keypair();
        let join_payload = sample_join_payload_for_tests(
            "table-reconnect-removed",
            33,
            host_signing_keys.public_key_base64(),
        );
        let server_sequence = Arc::new(AtomicU64::new(2));
        let mut state = sample_tournament_state("table-reconnect-removed", 33);
        state.participants.insert(
            "player-removed".to_string(),
            sample_participant_entry(
                "player-removed",
                "Removed",
                player_signing_keys.public_key_base64(),
                player_encryption_keys.public_key_base64(),
                ParticipantState::Removed,
                ConnectionState::Reconnecting,
                Some(0),
                "removed-token",
            ),
        );
        let authoritative_state = Arc::new(Mutex::new(state));

        let result = handle_reconnect_request(
            &provider,
            signed_reconnect_envelope(
                &provider,
                &player_signing_keys,
                &join_payload,
                ReconnectTournamentRequest {
                    player_id: "player-removed".to_string(),
                    reconnect_token: "removed-token".to_string(),
                    last_known_server_seq: Some(2),
                },
                1,
                "reconnect-removed",
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        );

        assert!(result.is_err());
        assert!(result
            .expect_err("reconnect should fail")
            .to_string()
            .contains("not reconnect-eligible"));
    }

    #[test]
    fn reconnect_after_tournament_complete_restores_seated_state() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let player_signing_keys = provider.generate_signing_keypair();
        let player_encryption_keys = provider.generate_encryption_keypair();
        let join_payload = sample_join_payload_for_tests(
            "table-reconnect-complete",
            34,
            host_signing_keys.public_key_base64(),
        );
        let server_sequence = Arc::new(AtomicU64::new(8));
        let mut state = sample_tournament_state("table-reconnect-complete", 34);
        state.phase = TournamentPhase::Complete;
        state.participants.insert(
            "player-complete".to_string(),
            sample_participant_entry(
                "player-complete",
                "Complete",
                player_signing_keys.public_key_base64(),
                player_encryption_keys.public_key_base64(),
                ParticipantState::Reconnecting,
                ConnectionState::Reconnecting,
                Some(0),
                "complete-token",
            ),
        );
        let authoritative_state = Arc::new(Mutex::new(state));

        let accepted = handle_reconnect_request(
            &provider,
            signed_reconnect_envelope(
                &provider,
                &player_signing_keys,
                &join_payload,
                ReconnectTournamentRequest {
                    player_id: "player-complete".to_string(),
                    reconnect_token: "complete-token".to_string(),
                    last_known_server_seq: Some(8),
                },
                1,
                "reconnect-complete",
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        )
        .expect("reconnect should succeed");

        assert_eq!(accepted.player_id, "player-complete");
        let state = authoritative_state.lock().expect("authoritative state");
        let participant = state
            .participants
            .get("player-complete")
            .expect("participant after reconnect");
        assert_eq!(participant.state, ParticipantState::Seated);
        assert_eq!(participant.connection_state, ConnectionState::Connected);
    }

    #[test]
    fn reconnect_preserves_eliminated_observer_state() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let player_signing_keys = provider.generate_signing_keypair();
        let player_encryption_keys = provider.generate_encryption_keypair();
        let join_payload = sample_join_payload_for_tests(
            "table-reconnect-observer",
            35,
            host_signing_keys.public_key_base64(),
        );
        let server_sequence = Arc::new(AtomicU64::new(3));
        let mut state = sample_tournament_state("table-reconnect-observer", 35);
        state.phase = TournamentPhase::Running;
        state.participants.insert(
            "player-observer".to_string(),
            sample_participant_entry(
                "player-observer",
                "Observer",
                player_signing_keys.public_key_base64(),
                player_encryption_keys.public_key_base64(),
                ParticipantState::EliminatedObserver,
                ConnectionState::Reconnecting,
                Some(2),
                "observer-token",
            ),
        );
        let authoritative_state = Arc::new(Mutex::new(state));

        let accepted = handle_reconnect_request(
            &provider,
            signed_reconnect_envelope(
                &provider,
                &player_signing_keys,
                &join_payload,
                ReconnectTournamentRequest {
                    player_id: "player-observer".to_string(),
                    reconnect_token: "observer-token".to_string(),
                    last_known_server_seq: Some(3),
                },
                1,
                "reconnect-observer",
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        )
        .expect("observer reconnect should succeed");

        assert_eq!(accepted.player_id, "player-observer");
        let state = authoritative_state.lock().expect("authoritative state");
        let participant = state
            .participants
            .get("player-observer")
            .expect("participant after reconnect");
        assert_eq!(participant.state, ParticipantState::EliminatedObserver);
        assert_eq!(participant.connection_state, ConnectionState::Connected);
    }

    #[test]
    fn resync_accepts_missing_sequence_and_replays_latest_snapshot() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let player_signing_keys = provider.generate_signing_keypair();
        let player_encryption_keys = provider.generate_encryption_keypair();
        let join_payload = sample_join_payload_for_tests(
            "table-resync-repeat",
            36,
            host_signing_keys.public_key_base64(),
        );
        let server_sequence = Arc::new(AtomicU64::new(5));
        let mut state = sample_tournament_state("table-resync-repeat", 36);
        state.participants.insert(
            "player-resync-repeat".to_string(),
            sample_participant_entry(
                "player-resync-repeat",
                "Resync Repeat",
                player_signing_keys.public_key_base64(),
                player_encryption_keys.public_key_base64(),
                ParticipantState::Admitted,
                ConnectionState::Connected,
                None,
                "resync-token",
            ),
        );
        let authoritative_state = Arc::new(Mutex::new(state));

        let first_snapshot = handle_resync_request(
            &provider,
            signed_resync_envelope(
                &provider,
                &player_signing_keys,
                &join_payload,
                "player-resync-repeat",
                sample_resync_request(None),
                1,
                "resync-repeat-1",
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        )
        .expect("resync should succeed without a sequence");
        assert_eq!(
            first_snapshot.payload.local_player_id,
            "player-resync-repeat"
        );

        authoritative_state
            .lock()
            .expect("authoritative state")
            .config
            .tournament_name = "Resync Repeat Updated".to_string();

        let second_snapshot = handle_resync_request(
            &provider,
            signed_resync_envelope(
                &provider,
                &player_signing_keys,
                &join_payload,
                "player-resync-repeat",
                sample_resync_request(None),
                2,
                "resync-repeat-2",
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        )
        .expect("repeated resync should succeed");

        assert_eq!(
            second_snapshot.payload.state.state.config.tournament_name,
            "Resync Repeat Updated"
        );
    }

    #[test]
    fn resync_rejects_future_sequences() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let player_signing_keys = provider.generate_signing_keypair();
        let player_encryption_keys = provider.generate_encryption_keypair();
        let join_payload = sample_join_payload_for_tests(
            "table-resync-future",
            37,
            host_signing_keys.public_key_base64(),
        );
        let server_sequence = Arc::new(AtomicU64::new(2));
        let mut state = sample_tournament_state("table-resync-future", 37);
        state.participants.insert(
            "player-resync-future".to_string(),
            sample_participant_entry(
                "player-resync-future",
                "Resync Future",
                player_signing_keys.public_key_base64(),
                player_encryption_keys.public_key_base64(),
                ParticipantState::Admitted,
                ConnectionState::Connected,
                None,
                "resync-future-token",
            ),
        );
        let authoritative_state = Arc::new(Mutex::new(state));

        let result = handle_resync_request(
            &provider,
            signed_resync_envelope(
                &provider,
                &player_signing_keys,
                &join_payload,
                "player-resync-future",
                sample_resync_request(Some(99)),
                1,
                "resync-future",
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        );

        assert!(result.is_err());
        assert!(result
            .expect_err("future resync should fail")
            .to_string()
            .contains("ahead of the host sequence"));
    }

    #[test]
    fn public_and_private_events_stay_ordered_and_scoped_across_two_clients() {
        let provider = DefaultCryptoProvider;
        let host = bind_test_host(&provider, "table-public-private-ordered", 38);

        let alice = connect_test_client(&provider, &host, "player-a", "Alice");
        let bob = connect_test_client(&provider, &host, "player-b", "Bob");
        let _ = expect_snapshot_event(&alice);
        let _ = expect_snapshot_event(&bob);

        host.broadcast_public_event(
            ProtocolMessageType::TournamentStartedEvent,
            &TournamentStartedEvent {
                tournament_name: "Ordered Events".to_string(),
                starting_stack: 1500,
                blind_schedule_preset: "FAST".to_string(),
                frozen_player_ids: vec!["player-a".to_string(), "player-b".to_string()],
            },
        )
        .expect("tournament started event");
        host.broadcast_public_event(
            ProtocolMessageType::ActionWindowOpenedEvent,
            &ActionWindowOpened {
                hand_number: 1,
                hand_phase: "AWAITING_ACTION".to_string(),
                action_window_id: "window-1".to_string(),
                player_id: "player-a".to_string(),
                seat_index: 0,
                legal_actions: vec![ActionType::Fold, ActionType::Call, ActionType::Raise],
                call_amount: 20,
                min_raise_to: Some(40),
                max_raise_to: Some(200),
                deadline_epoch_ms: 123_456,
            },
        )
        .expect("action window event");

        for client in [&alice, &bob] {
            let started_payload =
                assert_public_event(client, ProtocolMessageType::TournamentStartedEvent);
            assert_eq!(
                started_payload.get("tournamentName"),
                Some(&json!("Ordered Events"))
            );

            let action_window_payload =
                assert_public_event(client, ProtocolMessageType::ActionWindowOpenedEvent);
            assert_eq!(
                action_window_payload.get("playerId"),
                Some(&json!("player-a"))
            );
        }

        host.send_private_hole_cards(
            "player-a",
            &PrivateHoleCardsEvent {
                recipient_player_id: "player-a".to_string(),
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
        .expect("alice private cards");

        match alice
            .next_event(Duration::from_secs(2))
            .expect("alice private event")
        {
            ClientRuntimeEvent::PrivateHoleCards(payload) => {
                assert_eq!(payload.recipient_player_id, "player-a");
                assert_eq!(payload.hole_cards.len(), 2);
            }
            other => panic!("expected alice private cards, got {other:?}"),
        }
        assert!(bob.next_event(Duration::from_millis(200)).is_err());

        host.send_private_hole_cards(
            "player-b",
            &PrivateHoleCardsEvent {
                recipient_player_id: "player-b".to_string(),
                hole_cards: vec![
                    Card {
                        rank: Rank::Queen,
                        suit: Suit::Hearts,
                    },
                    Card {
                        rank: Rank::Jack,
                        suit: Suit::Hearts,
                    },
                ],
            },
        )
        .expect("bob private cards");

        match bob
            .next_event(Duration::from_secs(2))
            .expect("bob private event")
        {
            ClientRuntimeEvent::PrivateHoleCards(payload) => {
                assert_eq!(payload.recipient_player_id, "player-b");
                assert_eq!(payload.hole_cards.len(), 2);
            }
            other => panic!("expected bob private cards, got {other:?}"),
        }
    }

    #[test]
    fn reconnect_restores_mid_hand_snapshot_with_the_original_action_owner() {
        let provider = DefaultCryptoProvider;
        let host = bind_test_host(&provider, "table-mid-hand-reconnect", 39);
        let client = connect_test_client(&provider, &host, "player-midhand", "Midhand");
        let _ = expect_snapshot_event(&client);
        wait_for_host_participant_state(
            &host,
            "player-midhand",
            ParticipantState::Admitted,
            ConnectionState::Connected,
        );

        let mut updated_state = host.authoritative_state().expect("authoritative state");
        updated_state.phase = TournamentPhase::Running;
        updated_state.seats = vec![SeatState {
            seat_index: 0,
            occupancy: SeatOccupancyState::Occupied,
            tournament_state: TournamentSeatState::Active,
            participant_id: Some("player-midhand".to_string()),
            display_name: Some("Midhand".to_string()),
            chip_count: Some(1500),
            is_ready: true,
            marker: None,
        }];
        let participant = updated_state
            .participants
            .get_mut("player-midhand")
            .expect("participant");
        participant.state = ParticipantState::Active;
        participant.connection_state = ConnectionState::Connected;
        participant.seat_index = Some(0);
        updated_state.current_hand = Some(HandState {
            hand_number: 12,
            cycle_phase: HandCyclePhase::AwaitingAction,
            street: StreetPhase::Turn,
            dealer_seat_index: 0,
            small_blind_seat_index: 0,
            big_blind_seat_index: 0,
            board_cards: vec![Card {
                rank: Rank::Ace,
                suit: Suit::Clubs,
            }],
            hole_cards_by_player_id: [(
                "player-midhand".to_string(),
                vec![
                    Card {
                        rank: Rank::King,
                        suit: Suit::Spades,
                    },
                    Card {
                        rank: Rank::King,
                        suit: Suit::Hearts,
                    },
                ],
            )]
            .into_iter()
            .collect(),
            participation_by_player_id: [(
                "player-midhand".to_string(),
                HandParticipationState::Active,
            )]
            .into_iter()
            .collect(),
            betting_round: BettingRoundState {
                street: StreetPhase::Turn,
                current_bet: 40,
                min_raise_to: Some(80),
                max_raise_to: Some(200),
                pot_size: 120,
                contributions_by_player_id: [("player-midhand".to_string(), 40)]
                    .into_iter()
                    .collect(),
            },
            action_window: Some(ActionWindow {
                action_window_id: "window-midhand".to_string(),
                player_id: "player-midhand".to_string(),
                seat_index: 0,
                legal_actions: vec![ActionType::Fold, ActionType::Call, ActionType::Raise],
                call_amount: 40,
                min_raise_to: Some(80),
                max_raise_to: Some(200),
                deadline_epoch_ms: 456_789,
            }),
        });
        host.replace_authoritative_state(updated_state)
            .expect("replace authoritative state");

        disconnect_client(&host, "player-midhand");

        match client
            .next_event(Duration::from_secs(2))
            .expect("reconnecting event")
        {
            ClientRuntimeEvent::Reconnecting { player_id } => {
                assert_eq!(player_id, "player-midhand");
            }
            other => panic!("expected reconnecting event, got {other:?}"),
        }

        let snapshot = expect_snapshot_event(&client);
        assert_eq!(snapshot.local_player_id, "player-midhand");
        assert_eq!(snapshot.state.state.phase, TournamentPhase::Running);
        let action_window = snapshot
            .state
            .state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .expect("action window after reconnect");
        assert_eq!(action_window.player_id, "player-midhand");
        assert_eq!(action_window.action_window_id, "window-midhand");
    }

    #[test]
    fn resync_after_a_sequence_gap_allows_followup_public_events_to_continue() {
        let provider = DefaultCryptoProvider;
        let host = bind_test_host(&provider, "table-resync-continue", 40);
        let client = connect_test_client(&provider, &host, "player-resync-continue", "Resync");
        let _ = expect_snapshot_event(&client);

        let mut updated_state = host.authoritative_state().expect("authoritative state");
        updated_state.phase = TournamentPhase::ReadyCheck;
        updated_state.config.tournament_name = "Resync Continue".to_string();
        host.replace_authoritative_state(updated_state)
            .expect("replace authoritative state");

        host.server_sequence.store(0, Ordering::SeqCst);
        host.broadcast_public_event(
            ProtocolMessageType::TournamentStartedEvent,
            &TournamentStartedEvent {
                tournament_name: "stale".to_string(),
                starting_stack: 1500,
                blind_schedule_preset: "FAST".to_string(),
                frozen_player_ids: vec!["player-resync-continue".to_string()],
            },
        )
        .expect("broadcast stale event");

        match client
            .next_event(Duration::from_secs(2))
            .expect("resync requested")
        {
            ClientRuntimeEvent::ResyncRequested {
                player_id,
                last_seen_server_sequence,
            } => {
                assert_eq!(player_id, "player-resync-continue");
                assert_eq!(last_seen_server_sequence, 1);
            }
            other => panic!("expected resync requested event, got {other:?}"),
        }

        let snapshot = expect_snapshot_event(&client);
        assert_eq!(
            snapshot.state.state.config.tournament_name,
            "Resync Continue"
        );

        host.broadcast_public_event(
            ProtocolMessageType::TournamentCompleteEvent,
            &TournamentCompleteEvent {
                winner_player_id: "player-resync-continue".to_string(),
                placements: vec![PlacementEntry {
                    player_id: "player-resync-continue".to_string(),
                    place: 1,
                    busted_at_hand_number: None,
                }],
            },
        )
        .expect("broadcast tournament complete");

        let completion_payload =
            assert_public_event(&client, ProtocolMessageType::TournamentCompleteEvent);
        assert_eq!(
            completion_payload.get("winnerPlayerId"),
            Some(&json!("player-resync-continue"))
        );
    }

    #[test]
    fn elimination_and_completion_events_stay_in_sync_across_clients() {
        let provider = DefaultCryptoProvider;
        let host = bind_test_host(&provider, "table-elimination-sync", 41);
        let alice = connect_test_client(&provider, &host, "player-a", "Alice");
        let bob = connect_test_client(&provider, &host, "player-b", "Bob");
        let _ = expect_snapshot_event(&alice);
        let _ = expect_snapshot_event(&bob);

        host.broadcast_public_event(
            ProtocolMessageType::EliminationEvent,
            &EliminationEvent {
                player_id: "player-b".to_string(),
                place: 2,
            },
        )
        .expect("elimination event");
        host.broadcast_public_event(
            ProtocolMessageType::TournamentCompleteEvent,
            &TournamentCompleteEvent {
                winner_player_id: "player-a".to_string(),
                placements: vec![
                    PlacementEntry {
                        player_id: "player-a".to_string(),
                        place: 1,
                        busted_at_hand_number: None,
                    },
                    PlacementEntry {
                        player_id: "player-b".to_string(),
                        place: 2,
                        busted_at_hand_number: Some(12),
                    },
                ],
            },
        )
        .expect("completion event");

        for client in [&alice, &bob] {
            let elimination_payload =
                assert_public_event(client, ProtocolMessageType::EliminationEvent);
            assert_eq!(
                elimination_payload.get("playerId"),
                Some(&json!("player-b"))
            );
            let completion_payload =
                assert_public_event(client, ProtocolMessageType::TournamentCompleteEvent);
            assert_eq!(
                completion_payload.get("winnerPlayerId"),
                Some(&json!("player-a"))
            );
        }
    }

    #[test]
    fn host_marks_disconnect_as_reconnect_eligible() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

        let host = HostServer::bind(HostRuntimeConfig {
            bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
            advertised_host: "127.0.0.1".to_string(),
            session_epoch: 23,
            table_id: "table-disconnect".to_string(),
            table_name: Some("Disconnect".to_string()),
            join_token: "join-token".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: sample_tournament_state("table-disconnect", 23),
            runtime_mode: HostRuntimeMode::Test,
        })
        .expect("host should bind");

        let client = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: host.encoded_join_payload().to_string(),
            player_id: "player-disconnect".to_string(),
            display_name: "Disconnect".to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        })
        .expect("client should connect");
        let _ = client.next_event(Duration::from_secs(2)).expect("snapshot");
        client
            .clear_reconnect_identity()
            .expect("clear reconnect identity");

        let mut updated_state = host.authoritative_state().expect("authoritative state");
        updated_state.phase = TournamentPhase::Running;
        updated_state.seats = vec![SeatState {
            seat_index: 0,
            occupancy: SeatOccupancyState::Occupied,
            tournament_state: TournamentSeatState::Active,
            participant_id: Some("player-disconnect".to_string()),
            display_name: Some("Disconnect".to_string()),
            chip_count: Some(1500),
            is_ready: true,
            marker: None,
        }];
        let participant = updated_state
            .participants
            .get_mut("player-disconnect")
            .expect("participant");
        participant.state = ParticipantState::Active;
        participant.connection_state = ConnectionState::Connected;
        participant.seat_index = Some(0);
        host.replace_authoritative_state(updated_state)
            .expect("replace authoritative state");

        disconnect_client(&host, "player-disconnect");

        wait_for_host_participant_state(
            &host,
            "player-disconnect",
            ParticipantState::Reconnecting,
            ConnectionState::Reconnecting,
        );

        let state = host.authoritative_state().expect("authoritative state");
        let participant = state
            .participants
            .get("player-disconnect")
            .expect("participant registry entry");
        assert_eq!(participant.seat_index, Some(0));
        assert!(participant.reconnect_expiry_ms.is_some());
        assert_eq!(
            state
                .seats
                .first()
                .and_then(|seat| seat.participant_id.as_deref()),
            Some("player-disconnect")
        );
    }

    #[test]
    fn resync_replaces_local_state_from_authoritative_snapshot() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

        let host = HostServer::bind(HostRuntimeConfig {
            bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
            advertised_host: "127.0.0.1".to_string(),
            session_epoch: 24,
            table_id: "table-resync".to_string(),
            table_name: Some("Resync".to_string()),
            join_token: "join-token".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: sample_tournament_state("table-resync", 24),
            runtime_mode: HostRuntimeMode::Test,
        })
        .expect("host should bind");

        let client = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: host.encoded_join_payload().to_string(),
            player_id: "player-resync".to_string(),
            display_name: "Resync".to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        })
        .expect("client should connect");
        let _ = client.next_event(Duration::from_secs(2)).expect("snapshot");

        let mut updated_state = host.authoritative_state().expect("authoritative state");
        updated_state.phase = TournamentPhase::ReadyCheck;
        updated_state.config.tournament_name = "Resynced Tournament".to_string();
        host.replace_authoritative_state(updated_state)
            .expect("replace authoritative state");

        host.server_sequence.store(0, Ordering::SeqCst);
        host.broadcast_public_event(
            ProtocolMessageType::TournamentStartedEvent,
            &TournamentStartedEvent {
                tournament_name: "stale".to_string(),
                starting_stack: 1500,
                blind_schedule_preset: "FAST".to_string(),
                frozen_player_ids: vec!["player-resync".to_string()],
            },
        )
        .expect("broadcast stale event");

        let _ = client
            .next_event(Duration::from_secs(2))
            .expect("resync requested");
        match client
            .next_event(Duration::from_secs(2))
            .expect("resynced snapshot")
        {
            ClientRuntimeEvent::Snapshot(snapshot) => {
                assert_eq!(snapshot.state.state.phase, TournamentPhase::ReadyCheck);
                assert_eq!(
                    snapshot.state.state.config.tournament_name,
                    "Resynced Tournament"
                );
            }
            other => panic!("expected snapshot event, got {other:?}"),
        }
    }

    #[test]
    fn event_sequence_mismatch_triggers_resync() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

        let host = HostServer::bind(HostRuntimeConfig {
            bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
            advertised_host: "127.0.0.1".to_string(),
            session_epoch: 25,
            table_id: "table-sequence".to_string(),
            table_name: Some("Sequence".to_string()),
            join_token: "join-token".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: sample_tournament_state("table-sequence", 25),
            runtime_mode: HostRuntimeMode::Test,
        })
        .expect("host should bind");

        let client = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: host.encoded_join_payload().to_string(),
            player_id: "player-sequence".to_string(),
            display_name: "Sequence".to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        })
        .expect("client should connect");
        let _ = client.next_event(Duration::from_secs(2)).expect("snapshot");

        host.server_sequence.store(0, Ordering::SeqCst);
        host.broadcast_public_event(
            ProtocolMessageType::TournamentStartedEvent,
            &TournamentStartedEvent {
                tournament_name: "stale".to_string(),
                starting_stack: 1500,
                blind_schedule_preset: "FAST".to_string(),
                frozen_player_ids: vec!["player-sequence".to_string()],
            },
        )
        .expect("broadcast stale event");

        match client
            .next_event(Duration::from_secs(2))
            .expect("resync requested")
        {
            ClientRuntimeEvent::ResyncRequested {
                player_id,
                last_seen_server_sequence,
            } => {
                assert_eq!(player_id, "player-sequence");
                assert_eq!(last_seen_server_sequence, 1);
            }
            other => panic!("expected resync requested event, got {other:?}"),
        }

        match client
            .next_event(Duration::from_secs(2))
            .expect("resynced snapshot")
        {
            ClientRuntimeEvent::Snapshot(snapshot) => {
                assert_eq!(snapshot.local_player_id, "player-sequence");
            }
            other => panic!("expected snapshot event, got {other:?}"),
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
