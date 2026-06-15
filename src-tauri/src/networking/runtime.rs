use std::{
    collections::{BTreeMap, HashMap},
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
        counted_capacity, ConnectionState, HandCyclePhase, HandParticipationState, JoinPayload,
        ParticipantRegistryEntry, ParticipantState, PlayerIdentity, SeatOccupancyState, SeatState,
        StateProjector, TournamentPhase, TournamentSeatState, TournamentState,
    },
    networking::{read_json_frame, write_json_frame},
    protocol::{
        decode_join_payload, encode_join_payload, join_request_envelope, validate_join_payload,
        ActionWindowOpened, EliminationEvent, EncryptedPrivateEnvelope, HandResultCommitted,
        HandStartingEvent, JoinTournamentRequest, JsonSignedEnvelope, PlayerActionCommitted,
        PlayerActionSubmission, PrivateEnvelopeMetadata, PrivateHoleCardsEvent,
        ProtocolErrorMessage, ProtocolMessageType, ReadyStateRequest, RecipientHandSnapshot,
        RecipientSnapshotState, ReconnectTournamentRequest, ResyncRequest, SeatClaimRequest,
        SignedEnvelope, SnapshotEvent, SnapshotParticipant, StreetRevealed,
        TournamentCompleteEvent, TournamentStartedEvent, PROTOCOL_VERSION,
    },
    tournament::{ActionRequest, RegisteredPlayer, TournamentController},
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicEventLogEntry {
    pub sequence: u64,
    pub message_type: ProtocolMessageType,
    pub payload: Value,
}

pub struct HostServer {
    listener_addr: SocketAddr,
    join_payload: JoinPayload,
    encoded_join_payload: String,
    authoritative_state: Arc<Mutex<TournamentState>>,
    tournament_runtime: Arc<Mutex<Option<TournamentController>>>,
    clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
    server_sequence: Arc<AtomicU64>,
    stop_signal: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
    tick_thread: Option<JoinHandle<()>>,
    host_signing_keys: Arc<SigningKeyMaterial>,
    host_encryption_keys: Arc<Mutex<EncryptionKeyMaterial>>,
    public_events: Arc<Mutex<Vec<PublicEventLogEntry>>>,
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
        let tournament_runtime = Arc::new(Mutex::new(None));
        let clients = Arc::new(Mutex::new(HashMap::new()));
        let stop_signal = Arc::new(AtomicBool::new(false));
        let server_sequence = Arc::new(AtomicU64::new(0));
        let public_events = Arc::new(Mutex::new(Vec::new()));

        let accept_thread = {
            let authoritative_state = Arc::clone(&authoritative_state);
            let tournament_runtime = Arc::clone(&tournament_runtime);
            let clients = Arc::clone(&clients);
            let stop_signal = Arc::clone(&stop_signal);
            let server_sequence = Arc::clone(&server_sequence);
            let host_signing_keys = Arc::clone(&config.host_signing_keys);
            let host_encryption_keys = Arc::clone(&config.host_encryption_keys);
            let public_events = Arc::clone(&public_events);
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
                        let tournament_runtime = Arc::clone(&tournament_runtime);
                        let server_sequence = Arc::clone(&server_sequence);
                        let host_signing_keys = Arc::clone(&host_signing_keys);
                        let host_encryption_keys = Arc::clone(&host_encryption_keys);
                        let public_events = Arc::clone(&public_events);
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
                                            tournament_runtime,
                                            clients,
                                            join_payload,
                                            server_sequence,
                                            host_signing_keys,
                                            host_encryption_keys,
                                            public_events,
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

        let tick_thread = {
            let authoritative_state = Arc::clone(&authoritative_state);
            let tournament_runtime = Arc::clone(&tournament_runtime);
            let clients = Arc::clone(&clients);
            let stop_signal = Arc::clone(&stop_signal);
            let server_sequence = Arc::clone(&server_sequence);
            let host_signing_keys = Arc::clone(&config.host_signing_keys);
            let host_encryption_keys = Arc::clone(&config.host_encryption_keys);
            let public_events = Arc::clone(&public_events);
            let join_payload = join_payload.clone();

            thread::Builder::new()
                .name("desktop-poker-host-tick".to_string())
                .spawn(move || {
                    while !stop_signal.load(Ordering::SeqCst) {
                        let next_state = {
                            let mut runtime = match tournament_runtime.lock() {
                                Ok(runtime) => runtime,
                                Err(_) => break,
                            };

                            let Some(controller) = runtime.as_mut() else {
                                thread::sleep(Duration::from_millis(50));
                                continue;
                            };

                            let before = controller.state().clone();
                            if controller.advance_time(now_epoch_ms()).is_err() {
                                thread::sleep(Duration::from_millis(50));
                                continue;
                            }

                            let after = controller.state().clone();
                            (after != before).then_some(after)
                        };

                        if let Some(state) = next_state {
                            let previous_state = authoritative_state
                                .lock()
                                .map(|authoritative| authoritative.clone())
                                .unwrap_or_else(|_| state.clone());
                            let _ = authoritative_state.lock().map(|mut authoritative| {
                                *authoritative = state.clone();
                            });
                            let _ = publish_runtime_transition(
                                &join_payload,
                                &authoritative_state,
                                &previous_state,
                                &state,
                                &clients,
                                &server_sequence,
                                &host_signing_keys,
                                &host_encryption_keys,
                                &public_events,
                            );
                        }

                        thread::sleep(Duration::from_millis(50));
                    }
                })
                .map_err(|error| {
                    NetworkingError::new(format!("failed to start host tick loop: {error}"))
                })?
        };

        Ok(Self {
            listener_addr,
            join_payload,
            encoded_join_payload,
            authoritative_state,
            tournament_runtime,
            clients,
            server_sequence,
            stop_signal,
            accept_thread: Some(accept_thread),
            tick_thread: Some(tick_thread),
            host_signing_keys: config.host_signing_keys,
            host_encryption_keys: config.host_encryption_keys,
            public_events,
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

    pub fn public_events(&self) -> Result<Vec<PublicEventLogEntry>, NetworkingError> {
        self.public_events
            .lock()
            .map(|events| events.clone())
            .map_err(|_| NetworkingError::new("public event log lock poisoned"))
    }

    /// Register a local NPC participant without requiring a TCP connection or real crypto keys.
    ///
    /// Must be called before `claim_seat` for the same player ID.
    pub fn register_npc_participant(
        &self,
        player_id: &str,
        display_name: &str,
    ) -> Result<(), NetworkingError> {
        use crate::domain::{
            ConnectionState, ParticipantRegistryEntry, ParticipantState, PlayerIdentity,
        };
        self.update_lobby_state(|state| {
            if state.participants.contains_key(player_id) {
                return Err(NetworkingError::new(format!(
                    "participant {player_id} is already registered"
                )));
            }
            state.participants.insert(
                player_id.to_string(),
                ParticipantRegistryEntry {
                    identity: PlayerIdentity {
                        player_id: player_id.to_string(),
                        display_name: display_name.to_string(),
                        // NPCs use placeholder keys — they never sign or encrypt messages.
                        signing_public_key: "npc".to_string(),
                        encryption_public_key: "npc".to_string(),
                        signing_key_fingerprint: "npc".to_string(),
                    },
                    state: ParticipantState::Admitted,
                    connection_state: ConnectionState::Connected,
                    seat_index: None,
                    admitted_at_ms: now_epoch_ms(),
                    reconnect_token: None,
                    reconnect_expiry_ms: None,
                    is_host: false,
                },
            );
            Ok(())
        })
    }

    pub fn claim_seat(&self, player_id: &str, seat_index: u8) -> Result<(), NetworkingError> {
        self.update_lobby_state(|state| apply_seat_claim(state, player_id, seat_index))?;
        self.sync_snapshots_to_clients()
    }

    pub fn set_ready_state(&self, player_id: &str, is_ready: bool) -> Result<(), NetworkingError> {
        self.update_lobby_state(|state| apply_ready_state(state, player_id, is_ready))?;
        self.sync_snapshots_to_clients()
    }

    pub fn start_tournament(&self) -> Result<(), NetworkingError> {
        let before_state = self.authoritative_state()?;
        let controller = self
            .authoritative_state
            .lock()
            .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))
            .and_then(|mut state| apply_start_tournament(&mut state))?;

        self.tournament_runtime
            .lock()
            .map_err(|_| NetworkingError::new("tournament runtime lock poisoned"))?
            .replace(controller);
        let after_state = self.authoritative_state()?;
        publish_runtime_transition(
            &self.join_payload,
            &self.authoritative_state,
            &before_state,
            &after_state,
            &self.clients,
            &self.server_sequence,
            &self.host_signing_keys,
            &self.host_encryption_keys,
            &self.public_events,
        )
    }

    pub fn submit_action(
        &self,
        player_id: &str,
        action_window_id: String,
        action_type: crate::domain::ActionType,
        raise_to_amount: Option<u32>,
    ) -> Result<(), NetworkingError> {
        let before_state = self.authoritative_state()?;
        let next_state = {
            let mut runtime = self
                .tournament_runtime
                .lock()
                .map_err(|_| NetworkingError::new("tournament runtime lock poisoned"))?;
            let controller = runtime
                .as_mut()
                .ok_or_else(|| NetworkingError::new("live tournament runtime is unavailable"))?;

            controller
                .submit_action(
                    ActionRequest {
                        player_id: player_id.to_string(),
                        action_window_id,
                        action_type,
                        raise_to_amount,
                    },
                    now_epoch_ms(),
                )
                .map_err(|error| NetworkingError::new(error.to_string()))?;
            controller.state().clone()
        };

        self.authoritative_state
            .lock()
            .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))
            .map(|mut authoritative_state| {
                *authoritative_state = next_state;
            })?;
        let after_state = self.authoritative_state()?;
        publish_runtime_transition(
            &self.join_payload,
            &self.authoritative_state,
            &before_state,
            &after_state,
            &self.clients,
            &self.server_sequence,
            &self.host_signing_keys,
            &self.host_encryption_keys,
            &self.public_events,
        )
    }

    pub fn sync_snapshots_to_clients(&self) -> Result<(), NetworkingError> {
        let player_ids = self
            .clients
            .lock()
            .map_err(|_| NetworkingError::new("client registry lock poisoned"))?
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let mut failed_clients = Vec::new();

        for player_id in player_ids {
            let snapshot_envelope = build_snapshot_envelope(
                &DefaultCryptoProvider,
                &self.join_payload,
                &self.authoritative_state,
                &self.server_sequence,
                &self.host_signing_keys,
                &self.host_encryption_keys,
                &player_id,
            )?;

            let write_result = self
                .clients
                .lock()
                .map_err(|_| NetworkingError::new("client registry lock poisoned"))?
                .get(&player_id)
                .map(|client| {
                    client
                        .stream
                        .lock()
                        .map_err(|_| NetworkingError::new("client stream lock poisoned"))?
                        .try_clone()
                        .map_err(|error| {
                            NetworkingError::new(format!("failed to clone client stream: {error}"))
                        })
                        .and_then(|mut stream| write_json_frame(&mut stream, &snapshot_envelope))
                })
                .unwrap_or_else(|| Err(NetworkingError::new("unknown connected client")));

            if write_result.is_err() {
                failed_clients.push(player_id);
            }
        }

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

    fn update_lobby_state(
        &self,
        transform: impl FnOnce(&mut TournamentState) -> Result<(), NetworkingError>,
    ) -> Result<(), NetworkingError> {
        self.authoritative_state
            .lock()
            .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))
            .and_then(|mut state| transform(&mut state))
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
            payload: payload_value.clone(),
            signature: None,
        };

        let crypto_provider = DefaultCryptoProvider;
        envelope
            .sign(&crypto_provider, &self.host_signing_keys)
            .map_err(|error| NetworkingError::new(error.to_string()))?;
        append_public_event_log(
            &self.public_events,
            PublicEventLogEntry {
                sequence: server_sequence,
                message_type,
                payload: payload_value,
            },
        )?;

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

        if let Some(join_handle) = self.tick_thread.take() {
            let _ = join_handle.join();
        }
    }
}

#[derive(Debug)]
struct ClientReconnectIdentity {
    signing_keys: Option<SigningKeyMaterial>,
    encryption_keys: Option<EncryptionKeyMaterial>,
}

#[derive(Debug)]
struct ClientCommandConnection {
    player_id: String,
    table_id: String,
    session_epoch: u64,
    next_counter: u64,
    stream: Option<Arc<Mutex<TcpStream>>>,
}

pub struct ClientRuntime {
    incoming: Receiver<ClientRuntimeEvent>,
    reconnect_identity: Arc<Mutex<ClientReconnectIdentity>>,
    command_connection: Arc<Mutex<ClientCommandConnection>>,
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
        let stream_handle =
            Arc::new(Mutex::new(stream.try_clone().map_err(|error| {
                NetworkingError::new(format!("failed to clone stream: {error}"))
            })?));
        let command_connection = Arc::new(Mutex::new(ClientCommandConnection {
            player_id: player_id.clone(),
            table_id: join_payload.table_id.clone(),
            session_epoch: join_payload.session_epoch,
            next_counter: 2,
            stream: Some(Arc::clone(&stream_handle)),
        }));

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
        let command_connection_for_thread = Arc::clone(&command_connection);

        thread::spawn(move || {
            loop {
                let frame_value = match read_json_frame::<Value>(&mut stream) {
                    Ok(frame_value) => frame_value,
                    Err(_) => {
                        if let Ok(mut connection) = command_connection_for_thread.lock() {
                            connection.stream = None;
                        }
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
                                if let Ok(cloned_stream) = reconnected_stream.try_clone() {
                                    if let Ok(mut connection) = command_connection_for_thread.lock()
                                    {
                                        connection.stream =
                                            Some(Arc::new(Mutex::new(cloned_stream)));
                                    }
                                }
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
                                if let Ok(mut connection) = command_connection_for_thread.lock() {
                                    connection.stream = None;
                                }
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

                        if envelope.message_type == ProtocolMessageType::ProtocolError {
                            let protocol_error =
                                serde_json::from_value::<ProtocolErrorMessage>(envelope.payload)
                                    .map_err(|error| error.to_string());

                            match protocol_error {
                                Ok(message) => {
                                    let _ = sender.send(ClientRuntimeEvent::SafeError {
                                        player_id: player_id.clone(),
                                        message: message.message,
                                    });
                                }
                                Err(message) => {
                                    let _ = sender.send(ClientRuntimeEvent::SafeError {
                                        player_id: player_id.clone(),
                                        message,
                                    });
                                }
                            }
                            continue;
                        }

                        if matches!(
                            envelope.message_type,
                            ProtocolMessageType::HandStartingEvent
                                | ProtocolMessageType::ActionWindowOpenedEvent
                                | ProtocolMessageType::PlayerActionCommittedEvent
                                | ProtocolMessageType::StreetRevealedEvent
                                | ProtocolMessageType::EliminationEvent
                                | ProtocolMessageType::TournamentCompleteEvent
                                | ProtocolMessageType::HandResultCommittedEvent
                                | ProtocolMessageType::TournamentStartedEvent
                        ) {
                            let _ = sender.send(ClientRuntimeEvent::PublicEvent {
                                message_type: envelope.message_type,
                                server_sequence: envelope.server_sequence.unwrap_or_default(),
                                payload: envelope.payload,
                            });
                        }
                    }
                }
            }

            let _ = sender.send(ClientRuntimeEvent::Disconnected { player_id });
        });

        Ok(Self {
            incoming: receiver,
            reconnect_identity,
            command_connection,
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

    pub fn claim_seat(&self, seat_index: u8) -> Result<(), NetworkingError> {
        self.send_signed_request(
            ProtocolMessageType::SeatClaimRequest,
            SeatClaimRequest { seat_index },
        )
    }

    pub fn set_ready_state(&self, is_ready: bool) -> Result<(), NetworkingError> {
        self.send_signed_request(
            ProtocolMessageType::ReadyStateRequest,
            ReadyStateRequest { is_ready },
        )
    }

    pub fn submit_action(
        &self,
        action_window_id: String,
        seat_index: u8,
        action_type: crate::domain::ActionType,
        raise_to_amount: Option<u32>,
    ) -> Result<(), NetworkingError> {
        self.send_signed_request(
            ProtocolMessageType::ActionSubmissionRequest,
            PlayerActionSubmission {
                action_window_id,
                seat_index,
                action_type,
                raise_to_amount,
            },
        )
    }

    fn send_signed_request<TPayload: serde::Serialize + Clone>(
        &self,
        message_type: ProtocolMessageType,
        payload: TPayload,
    ) -> Result<(), NetworkingError> {
        let (player_id, table_id, session_epoch, counter, stream_handle) = {
            let mut connection = self
                .command_connection
                .lock()
                .map_err(|_| NetworkingError::new("client command connection lock poisoned"))?;
            let stream_handle = connection.stream.as_ref().cloned().ok_or_else(|| {
                NetworkingError::new("client is not currently connected to the host")
            })?;
            let counter = connection.next_counter;
            connection.next_counter += 1;
            (
                connection.player_id.clone(),
                connection.table_id.clone(),
                connection.session_epoch,
                counter,
                stream_handle,
            )
        };

        let mut envelope = SignedEnvelope {
            protocol_version: PROTOCOL_VERSION,
            message_type,
            table_id,
            session_epoch,
            sender_id: player_id,
            counter,
            message_id: format!("client-{counter}"),
            server_sequence: None,
            payload,
            signature: None,
        };
        {
            let reconnect_identity = self
                .reconnect_identity
                .lock()
                .map_err(|_| NetworkingError::new("client reconnect identity lock poisoned"))?;
            let signing_key = reconnect_identity
                .signing_keys
                .as_ref()
                .ok_or_else(|| NetworkingError::new(missing_reconnect_identity_message()))?;
            envelope
                .sign(&DefaultCryptoProvider, signing_key)
                .map_err(|error| NetworkingError::new(error.to_string()))?;
        }

        let mut stream = stream_handle
            .lock()
            .map_err(|_| NetworkingError::new("client command stream lock poisoned"))?
            .try_clone()
            .map_err(|error| NetworkingError::new(format!("failed to clone stream: {error}")))?;
        write_json_frame(&mut stream, &envelope)
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
        server_sequence: u64,
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

        ensure_joinable_phase(state.phase)?;
        ensure_join_capacity(&state)?;

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

fn ensure_joinable_phase(phase: TournamentPhase) -> Result<(), NetworkingError> {
    match phase {
        TournamentPhase::WaitingForPlayers => Ok(()),
        TournamentPhase::ReadyCheck => {
            Err(NetworkingError::new("joins are closed after roster freeze"))
        }
        TournamentPhase::Running => Err(NetworkingError::new(
            "joins are unavailable after the tournament starts",
        )),
        TournamentPhase::Complete | TournamentPhase::Cancelled => Err(NetworkingError::new(
            "joins are unavailable for closed sessions",
        )),
    }
}

fn ensure_join_capacity(state: &TournamentState) -> Result<(), NetworkingError> {
    let participant_count = counted_capacity(&state.participants);
    if participant_count >= state.config.max_players as usize {
        return Err(NetworkingError::new(format!(
            "table is full: {participant_count} participants already admitted for {} seats",
            state.config.max_players
        )));
    }

    Ok(())
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
    tournament_runtime: Arc<Mutex<Option<TournamentController>>>,
    clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
    join_payload: JoinPayload,
    server_sequence: Arc<AtomicU64>,
    host_signing_keys: Arc<SigningKeyMaterial>,
    host_encryption_keys: Arc<Mutex<EncryptionKeyMaterial>>,
    public_events: Arc<Mutex<Vec<PublicEventLogEntry>>>,
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
                ProtocolMessageType::SeatClaimRequest => {
                    let rejected_message_id = request_envelope.message_id.clone();
                    let response = handle_seat_claim_request(
                        &crypto_provider,
                        request_envelope,
                        &authoritative_state,
                    );

                    match response {
                        Ok(()) => {
                            if sync_host_client_snapshots(
                                &join_payload,
                                &authoritative_state,
                                &clients,
                                &server_sequence,
                                &host_signing_keys,
                                &host_encryption_keys,
                            )
                            .is_err()
                            {
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
                                "SEAT_CLAIM_REJECTED",
                                error.to_string(),
                                Some(rejected_message_id),
                            ) {
                                let _ = write_json_frame(&mut stream, &envelope);
                            }
                        }
                    }
                }
                ProtocolMessageType::ReadyStateRequest => {
                    let rejected_message_id = request_envelope.message_id.clone();
                    let response = handle_ready_state_request(
                        &crypto_provider,
                        request_envelope,
                        &authoritative_state,
                    );

                    match response {
                        Ok(()) => {
                            if sync_host_client_snapshots(
                                &join_payload,
                                &authoritative_state,
                                &clients,
                                &server_sequence,
                                &host_signing_keys,
                                &host_encryption_keys,
                            )
                            .is_err()
                            {
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
                                "READY_STATE_REJECTED",
                                error.to_string(),
                                Some(rejected_message_id),
                            ) {
                                let _ = write_json_frame(&mut stream, &envelope);
                            }
                        }
                    }
                }
                ProtocolMessageType::ActionSubmissionRequest => {
                    let rejected_message_id = request_envelope.message_id.clone();
                    let previous_state = authoritative_state
                        .lock()
                        .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))
                        .map(|state| state.clone());
                    let response = handle_action_submission_request(
                        &crypto_provider,
                        request_envelope,
                        &authoritative_state,
                        &tournament_runtime,
                    );

                    match response {
                        Ok(()) => {
                            let previous_state = match previous_state {
                                Ok(state) => state,
                                Err(error) => {
                                    if let Ok(envelope) = build_protocol_error_envelope(
                                        &crypto_provider,
                                        &join_payload,
                                        &server_sequence,
                                        &host_signing_keys,
                                        "ACTION_SUBMISSION_REJECTED",
                                        error.to_string(),
                                        Some(rejected_message_id.clone()),
                                    ) {
                                        let _ = write_json_frame(&mut stream, &envelope);
                                    }
                                    break;
                                }
                            };
                            let next_state = match authoritative_state
                                .lock()
                                .map_err(|_| {
                                    NetworkingError::new("authoritative state lock poisoned")
                                })
                                .map(|state| state.clone())
                            {
                                Ok(state) => state,
                                Err(_) => {
                                    let _ = clients.lock().map(|mut connected_clients| {
                                        connected_clients.remove(&player_id);
                                    });
                                    let _ = mark_participant_reconnect_eligible(
                                        &authoritative_state,
                                        &player_id,
                                    );
                                    break;
                                }
                            };
                            if publish_runtime_transition(
                                &join_payload,
                                &authoritative_state,
                                &previous_state,
                                &next_state,
                                &clients,
                                &server_sequence,
                                &host_signing_keys,
                                &host_encryption_keys,
                                &public_events,
                            )
                            .is_err()
                            {
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
                                "ACTION_SUBMISSION_REJECTED",
                                error.to_string(),
                                Some(rejected_message_id),
                            ) {
                                let _ = write_json_frame(&mut stream, &envelope);
                            }
                        }
                    }
                }
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

fn handle_seat_claim_request(
    crypto_provider: &impl ProtocolCryptoProvider,
    request_envelope: JsonSignedEnvelope,
    authoritative_state: &Arc<Mutex<TournamentState>>,
) -> Result<(), NetworkingError> {
    let request: SeatClaimRequest = serde_json::from_value(request_envelope.payload.clone())
        .map_err(|error| NetworkingError::new(format!("invalid seat claim payload: {error}")))?;
    let mut state = authoritative_state
        .lock()
        .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))?;
    let participant = state
        .participants
        .get(&request_envelope.sender_id)
        .ok_or_else(|| NetworkingError::new("seat claim requester is not registered"))?;

    request_envelope
        .verify(crypto_provider, &participant.identity.signing_public_key)
        .map_err(|error| NetworkingError::new(error.to_string()))?;

    apply_seat_claim(&mut state, &request_envelope.sender_id, request.seat_index)
}

fn handle_ready_state_request(
    crypto_provider: &impl ProtocolCryptoProvider,
    request_envelope: JsonSignedEnvelope,
    authoritative_state: &Arc<Mutex<TournamentState>>,
) -> Result<(), NetworkingError> {
    let request: ReadyStateRequest = serde_json::from_value(request_envelope.payload.clone())
        .map_err(|error| NetworkingError::new(format!("invalid ready-state payload: {error}")))?;
    let mut state = authoritative_state
        .lock()
        .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))?;
    let participant = state
        .participants
        .get(&request_envelope.sender_id)
        .ok_or_else(|| NetworkingError::new("ready-state requester is not registered"))?;

    request_envelope
        .verify(crypto_provider, &participant.identity.signing_public_key)
        .map_err(|error| NetworkingError::new(error.to_string()))?;

    apply_ready_state(&mut state, &request_envelope.sender_id, request.is_ready)
}

fn handle_action_submission_request(
    crypto_provider: &impl ProtocolCryptoProvider,
    request_envelope: JsonSignedEnvelope,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    tournament_runtime: &Arc<Mutex<Option<TournamentController>>>,
) -> Result<(), NetworkingError> {
    let request: PlayerActionSubmission = serde_json::from_value(request_envelope.payload.clone())
        .map_err(|error| {
            NetworkingError::new(format!("invalid action submission payload: {error}"))
        })?;
    {
        let state = authoritative_state
            .lock()
            .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))?;
        let participant = state
            .participants
            .get(&request_envelope.sender_id)
            .ok_or_else(|| NetworkingError::new("action requester is not registered"))?;

        request_envelope
            .verify(crypto_provider, &participant.identity.signing_public_key)
            .map_err(|error| NetworkingError::new(error.to_string()))?;
    }

    let next_state = {
        let mut runtime = tournament_runtime
            .lock()
            .map_err(|_| NetworkingError::new("tournament runtime lock poisoned"))?;
        let controller = runtime
            .as_mut()
            .ok_or_else(|| NetworkingError::new("live tournament runtime is unavailable"))?;
        controller
            .submit_action(
                ActionRequest {
                    player_id: request_envelope.sender_id.clone(),
                    action_window_id: request.action_window_id,
                    action_type: request.action_type,
                    raise_to_amount: request.raise_to_amount,
                },
                now_epoch_ms(),
            )
            .map_err(|error| NetworkingError::new(error.to_string()))?;
        controller.state().clone()
    };

    authoritative_state
        .lock()
        .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))
        .map(|mut state| {
            *state = next_state;
        })?;

    Ok(())
}

fn append_public_event_log(
    public_events: &Arc<Mutex<Vec<PublicEventLogEntry>>>,
    event: PublicEventLogEntry,
) -> Result<(), NetworkingError> {
    let mut entries = public_events
        .lock()
        .map_err(|_| NetworkingError::new("public event log lock poisoned"))?;
    entries.push(event);
    if entries.len() > 64 {
        let overflow = entries.len() - 64;
        entries.drain(0..overflow);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn broadcast_public_event_to_clients<TPayload: serde::Serialize>(
    join_payload: &JoinPayload,
    clients: &Arc<Mutex<HashMap<String, ConnectedClient>>>,
    server_sequence: &Arc<AtomicU64>,
    host_signing_keys: &Arc<SigningKeyMaterial>,
    public_events: &Arc<Mutex<Vec<PublicEventLogEntry>>>,
    message_type: ProtocolMessageType,
    payload: &TPayload,
    on_failed_client: impl Fn(&str) -> Result<(), NetworkingError>,
) -> Result<u64, NetworkingError> {
    let payload_value =
        serde_json::to_value(payload).map_err(|error| NetworkingError::new(error.to_string()))?;
    let next_server_sequence = server_sequence.fetch_add(1, Ordering::SeqCst) + 1;

    let mut envelope = SignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type,
        table_id: join_payload.table_id.clone(),
        session_epoch: join_payload.session_epoch,
        sender_id: "host".to_string(),
        counter: next_server_sequence,
        message_id: format!("host-{next_server_sequence}"),
        server_sequence: Some(next_server_sequence),
        payload: payload_value.clone(),
        signature: None,
    };
    envelope
        .sign(&DefaultCryptoProvider, host_signing_keys)
        .map_err(|error| NetworkingError::new(error.to_string()))?;

    append_public_event_log(
        public_events,
        PublicEventLogEntry {
            sequence: next_server_sequence,
            message_type,
            payload: payload_value,
        },
    )?;

    let player_ids = clients
        .lock()
        .map_err(|_| NetworkingError::new("client registry lock poisoned"))?
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let mut failed_clients = Vec::new();

    for player_id in player_ids {
        let write_result = clients
            .lock()
            .map_err(|_| NetworkingError::new("client registry lock poisoned"))?
            .get(&player_id)
            .map(|client| {
                client
                    .stream
                    .lock()
                    .map_err(|_| NetworkingError::new("client stream lock poisoned"))?
                    .try_clone()
                    .map_err(|error| {
                        NetworkingError::new(format!("failed to clone client stream: {error}"))
                    })
                    .and_then(|mut stream| write_json_frame(&mut stream, &envelope))
            })
            .unwrap_or_else(|| Err(NetworkingError::new("unknown connected client")));

        if write_result.is_err() {
            failed_clients.push(player_id);
        }
    }

    if !failed_clients.is_empty() {
        let mut connected_clients = clients
            .lock()
            .map_err(|_| NetworkingError::new("client registry lock poisoned"))?;
        for player_id in failed_clients {
            connected_clients.remove(&player_id);
            on_failed_client(&player_id)?;
        }
    }

    Ok(next_server_sequence)
}

fn infer_committed_action(
    before: &TournamentState,
    after: &TournamentState,
) -> Option<PlayerActionCommitted> {
    let before_window = before
        .current_hand
        .as_ref()
        .and_then(|hand| hand.action_window.as_ref())?;
    let before_hand = before.current_hand.as_ref()?;
    let after_hand = after.current_hand.as_ref()?;
    let before_contribution = before_hand
        .betting_round
        .contributions_by_player_id
        .get(&before_window.player_id)
        .copied()
        .unwrap_or_default();
    let after_contribution = after_hand
        .betting_round
        .contributions_by_player_id
        .get(&before_window.player_id)
        .copied()
        .unwrap_or_default();
    let after_participation = after_hand
        .participation_by_player_id
        .get(&before_window.player_id)
        .copied();
    let additional_amount = after_contribution.saturating_sub(before_contribution);
    let action_type = if matches!(
        after_participation,
        Some(crate::domain::HandParticipationState::Folded)
    ) {
        crate::domain::ActionType::Fold
    } else if additional_amount == 0 {
        crate::domain::ActionType::Check
    } else if matches!(
        after_participation,
        Some(crate::domain::HandParticipationState::AllIn)
    ) {
        crate::domain::ActionType::AllIn
    } else if before_window.call_amount > 0 && additional_amount == before_window.call_amount {
        crate::domain::ActionType::Call
    } else if before_hand.betting_round.current_bet == 0 {
        crate::domain::ActionType::Bet
    } else {
        crate::domain::ActionType::Raise
    };

    Some(PlayerActionCommitted {
        hand_number: before_hand.hand_number,
        seat_index: before_window.seat_index,
        player_id: before_window.player_id.clone(),
        action_type,
        raise_to_amount: matches!(
            action_type,
            crate::domain::ActionType::Bet
                | crate::domain::ActionType::Raise
                | crate::domain::ActionType::AllIn
        )
        .then_some(after_contribution),
    })
}

#[allow(clippy::too_many_arguments)]
fn publish_runtime_transition(
    join_payload: &JoinPayload,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    before: &TournamentState,
    after: &TournamentState,
    clients: &Arc<Mutex<HashMap<String, ConnectedClient>>>,
    server_sequence: &Arc<AtomicU64>,
    host_signing_keys: &Arc<SigningKeyMaterial>,
    host_encryption_keys: &Arc<Mutex<EncryptionKeyMaterial>>,
    public_events: &Arc<Mutex<Vec<PublicEventLogEntry>>>,
) -> Result<(), NetworkingError> {
    if before.phase != TournamentPhase::Running && after.phase == TournamentPhase::Running {
        let _ = broadcast_public_event_to_clients(
            join_payload,
            clients,
            server_sequence,
            host_signing_keys,
            public_events,
            ProtocolMessageType::TournamentStartedEvent,
            &build_tournament_started_event(after),
            |player_id| mark_participant_reconnect_eligible(authoritative_state, player_id),
        )?;
    }

    let before_hand_number = before.current_hand.as_ref().map(|hand| hand.hand_number);
    if let Some(after_hand) = after.current_hand.as_ref() {
        if before_hand_number != Some(after_hand.hand_number) {
            let _ = broadcast_public_event_to_clients(
                join_payload,
                clients,
                server_sequence,
                host_signing_keys,
                public_events,
                ProtocolMessageType::HandStartingEvent,
                &HandStartingEvent {
                    hand_number: after_hand.hand_number,
                    hand_phase: format!("{:?}", after_hand.cycle_phase),
                    dealer_seat_index: after_hand.dealer_seat_index,
                    small_blind_seat_index: after_hand.small_blind_seat_index,
                    big_blind_seat_index: after_hand.big_blind_seat_index,
                    board_cards: after_hand.board_cards.clone(),
                },
                |player_id| mark_participant_reconnect_eligible(authoritative_state, player_id),
            )?;
        }
    }

    if let Some(committed_action) = infer_committed_action(before, after) {
        let _ = broadcast_public_event_to_clients(
            join_payload,
            clients,
            server_sequence,
            host_signing_keys,
            public_events,
            ProtocolMessageType::PlayerActionCommittedEvent,
            &committed_action,
            |player_id| mark_participant_reconnect_eligible(authoritative_state, player_id),
        )?;
    }

    let before_board = before
        .current_hand
        .as_ref()
        .map(|hand| hand.board_cards.clone())
        .unwrap_or_default();
    let after_board = after
        .current_hand
        .as_ref()
        .map(|hand| hand.board_cards.clone())
        .unwrap_or_default();
    if let Some(after_hand) = after.current_hand.as_ref() {
        if after_board.len() > before_board.len() {
            let _ = broadcast_public_event_to_clients(
                join_payload,
                clients,
                server_sequence,
                host_signing_keys,
                public_events,
                ProtocolMessageType::StreetRevealedEvent,
                &StreetRevealed {
                    hand_number: after_hand.hand_number,
                    street: format!("{:?}", after_hand.street),
                    board_cards: after_board.clone(),
                },
                |player_id| mark_participant_reconnect_eligible(authoritative_state, player_id),
            )?;
        }
    }

    if after.hand_results.len() > before.hand_results.len() {
        for result in &after.hand_results[before.hand_results.len()..] {
            let _ = broadcast_public_event_to_clients(
                join_payload,
                clients,
                server_sequence,
                host_signing_keys,
                public_events,
                ProtocolMessageType::HandResultCommittedEvent,
                &HandResultCommitted {
                    hand_number: result.hand_number,
                    result: result.clone(),
                },
                |player_id| mark_participant_reconnect_eligible(authoritative_state, player_id),
            )?;
        }
    }

    if after.placements.len() > before.placements.len() {
        for placement in &after.placements[before.placements.len()..] {
            let _ = broadcast_public_event_to_clients(
                join_payload,
                clients,
                server_sequence,
                host_signing_keys,
                public_events,
                ProtocolMessageType::EliminationEvent,
                &EliminationEvent {
                    player_id: placement.player_id.clone(),
                    place: placement.place,
                },
                |player_id| mark_participant_reconnect_eligible(authoritative_state, player_id),
            )?;
        }
    }

    if after.phase == TournamentPhase::Complete && before.phase != TournamentPhase::Complete {
        if let Some(winner_player_id) = after
            .placements
            .iter()
            .find(|entry| entry.place == 1)
            .map(|entry| entry.player_id.clone())
        {
            let _ = broadcast_public_event_to_clients(
                join_payload,
                clients,
                server_sequence,
                host_signing_keys,
                public_events,
                ProtocolMessageType::TournamentCompleteEvent,
                &TournamentCompleteEvent {
                    winner_player_id,
                    placements: after.placements.clone(),
                },
                |player_id| mark_participant_reconnect_eligible(authoritative_state, player_id),
            )?;
        }
    }

    if let Some(after_hand) = after.current_hand.as_ref() {
        for (player_id, cards) in &after_hand.hole_cards_by_player_id {
            let before_cards = before
                .current_hand
                .as_ref()
                .and_then(|hand| hand.hole_cards_by_player_id.get(player_id));
            let has_live_recipient = clients
                .lock()
                .map(|connected_clients| connected_clients.contains_key(player_id))
                .unwrap_or(false);
            if before_cards != Some(cards) && has_live_recipient {
                let _ = send_private_hole_cards_to_recipient(
                    join_payload,
                    clients,
                    server_sequence,
                    host_signing_keys,
                    host_encryption_keys,
                    player_id,
                    &PrivateHoleCardsEvent {
                        recipient_player_id: player_id.clone(),
                        hole_cards: cards.clone(),
                    },
                    |player_id| mark_participant_reconnect_eligible(authoritative_state, player_id),
                )?;
            }
        }

        if let Some(window) = after_hand.action_window.as_ref() {
            let before_window_id = before
                .current_hand
                .as_ref()
                .and_then(|hand| hand.action_window.as_ref())
                .map(|window| window.action_window_id.as_str());
            if before_window_id != Some(window.action_window_id.as_str()) {
                let _ = broadcast_public_event_to_clients(
                    join_payload,
                    clients,
                    server_sequence,
                    host_signing_keys,
                    public_events,
                    ProtocolMessageType::ActionWindowOpenedEvent,
                    &ActionWindowOpened {
                        hand_number: after_hand.hand_number,
                        hand_phase: format!("{:?}", after_hand.cycle_phase),
                        action_window_id: window.action_window_id.clone(),
                        player_id: window.player_id.clone(),
                        seat_index: window.seat_index,
                        legal_actions: window.legal_actions.clone(),
                        call_amount: window.call_amount,
                        min_raise_to: window.min_raise_to,
                        max_raise_to: window.max_raise_to,
                        deadline_epoch_ms: window.deadline_epoch_ms,
                    },
                    |player_id| mark_participant_reconnect_eligible(authoritative_state, player_id),
                )?;
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn sync_host_client_snapshots(
    join_payload: &JoinPayload,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    clients: &Arc<Mutex<HashMap<String, ConnectedClient>>>,
    server_sequence: &Arc<AtomicU64>,
    host_signing_keys: &Arc<SigningKeyMaterial>,
    host_encryption_keys: &Arc<Mutex<EncryptionKeyMaterial>>,
) -> Result<(), NetworkingError> {
    let player_ids = clients
        .lock()
        .map_err(|_| NetworkingError::new("client registry lock poisoned"))?
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let mut failed_clients = Vec::new();

    for player_id in player_ids {
        let snapshot_envelope = build_snapshot_envelope(
            &DefaultCryptoProvider,
            join_payload,
            authoritative_state,
            server_sequence,
            host_signing_keys,
            host_encryption_keys,
            &player_id,
        )?;
        let write_result = clients
            .lock()
            .map_err(|_| NetworkingError::new("client registry lock poisoned"))?
            .get(&player_id)
            .map(|client| {
                client
                    .stream
                    .lock()
                    .map_err(|_| NetworkingError::new("client stream lock poisoned"))?
                    .try_clone()
                    .map_err(|error| {
                        NetworkingError::new(format!("failed to clone client stream: {error}"))
                    })
                    .and_then(|mut stream| write_json_frame(&mut stream, &snapshot_envelope))
            })
            .unwrap_or_else(|| Err(NetworkingError::new("unknown connected client")));

        if write_result.is_err() {
            failed_clients.push(player_id);
        }
    }

    if !failed_clients.is_empty() {
        let mut connected_clients = clients
            .lock()
            .map_err(|_| NetworkingError::new("client registry lock poisoned"))?;
        for player_id in failed_clients {
            connected_clients.remove(&player_id);
            mark_participant_reconnect_eligible(authoritative_state, &player_id)?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn send_private_hole_cards_to_recipient(
    join_payload: &JoinPayload,
    clients: &Arc<Mutex<HashMap<String, ConnectedClient>>>,
    server_sequence: &Arc<AtomicU64>,
    host_signing_keys: &Arc<SigningKeyMaterial>,
    host_encryption_keys: &Arc<Mutex<EncryptionKeyMaterial>>,
    recipient_id: &str,
    hole_cards_event: &PrivateHoleCardsEvent,
    on_failed_client: impl Fn(&str) -> Result<(), NetworkingError>,
) -> Result<u64, NetworkingError> {
    let crypto_provider = DefaultCryptoProvider;

    let (client_stream, recipient_encryption_public_key) = {
        let connected_clients = clients
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

    let next_server_sequence = server_sequence.fetch_add(1, Ordering::SeqCst) + 1;
    let payload_bytes = serde_json::to_vec(hole_cards_event).map_err(|error| {
        NetworkingError::new(format!("failed to serialize private payload: {error}"))
    })?;

    let host_encryption_keys = host_encryption_keys
        .lock()
        .map_err(|_| NetworkingError::new("host encryption key lock poisoned"))?;

    let aad_metadata = PrivateEnvelopeMetadata {
        sender_id: "host".to_string(),
        table_id: join_payload.table_id.clone(),
        session_epoch: join_payload.session_epoch,
        counter: next_server_sequence,
        message_id: format!("private-{next_server_sequence}"),
        server_sequence: next_server_sequence,
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
        .sign(&crypto_provider, host_signing_keys)
        .map_err(|error| NetworkingError::new(error.to_string()))?;

    let mut stream = client_stream
        .lock()
        .map_err(|_| NetworkingError::new("client stream lock poisoned"))?
        .try_clone()
        .map_err(|error| NetworkingError::new(format!("failed to clone client stream: {error}")))?;

    match write_json_frame(&mut stream, &envelope) {
        Ok(()) => Ok(next_server_sequence),
        Err(error) => {
            if let Ok(mut connected_clients) = clients.lock() {
                connected_clients.remove(recipient_id);
            }
            on_failed_client(recipient_id)?;
            Err(error)
        }
    }
}

fn apply_seat_claim(
    state: &mut TournamentState,
    player_id: &str,
    seat_index: u8,
) -> Result<(), NetworkingError> {
    ensure_lobby_phase(state.phase)?;
    ensure_lobby_seat_map(state);

    if seat_index >= state.config.max_players {
        return Err(NetworkingError::new("seat index is out of range"));
    }

    let current_seat_index = state
        .participants
        .get(player_id)
        .ok_or_else(|| NetworkingError::new("participant is not registered"))?
        .seat_index;

    if state
        .seats
        .get(seat_index as usize)
        .is_some_and(|seat| seat.occupancy == SeatOccupancyState::Occupied)
        && state.seats[seat_index as usize].participant_id.as_deref() != Some(player_id)
    {
        return Err(NetworkingError::new("seat is already occupied"));
    }

    if let Some(previous_seat_index) = current_seat_index.filter(|index| *index != seat_index) {
        state.seats[previous_seat_index as usize] = SeatState {
            seat_index: previous_seat_index,
            occupancy: SeatOccupancyState::Empty,
            tournament_state: TournamentSeatState::Open,
            participant_id: None,
            display_name: None,
            chip_count: None,
            is_ready: false,
            marker: None,
        };
    }

    let participant = state
        .participants
        .get_mut(player_id)
        .ok_or_else(|| NetworkingError::new("participant is not registered"))?;
    participant.state = ParticipantState::Seated;
    participant.seat_index = Some(seat_index);
    participant.connection_state = ConnectionState::Connected;
    state.seats[seat_index as usize] = SeatState {
        seat_index,
        occupancy: SeatOccupancyState::Occupied,
        tournament_state: TournamentSeatState::Lobby,
        participant_id: Some(player_id.to_string()),
        display_name: Some(participant.identity.display_name.clone()),
        chip_count: None,
        is_ready: false,
        marker: None,
    };
    recompute_lobby_phase(state);
    Ok(())
}

fn apply_ready_state(
    state: &mut TournamentState,
    player_id: &str,
    is_ready: bool,
) -> Result<(), NetworkingError> {
    ensure_lobby_phase(state.phase)?;
    ensure_lobby_seat_map(state);

    let seat_index = state
        .participants
        .get(player_id)
        .and_then(|participant| participant.seat_index)
        .ok_or_else(|| NetworkingError::new("ready state requires a claimed seat"))?;
    let seat = state
        .seats
        .get_mut(seat_index as usize)
        .ok_or_else(|| NetworkingError::new("participant seat is out of range"))?;

    if seat.participant_id.as_deref() != Some(player_id) {
        return Err(NetworkingError::new(
            "seat ownership does not match the participant",
        ));
    }

    seat.is_ready = is_ready;
    seat.tournament_state = if is_ready {
        TournamentSeatState::Ready
    } else {
        TournamentSeatState::Lobby
    };
    recompute_lobby_phase(state);
    Ok(())
}

fn ensure_lobby_phase(phase: TournamentPhase) -> Result<(), NetworkingError> {
    match phase {
        TournamentPhase::WaitingForPlayers | TournamentPhase::ReadyCheck => Ok(()),
        TournamentPhase::Running => Err(NetworkingError::new(
            "lobby actions are unavailable after the tournament starts",
        )),
        TournamentPhase::Complete | TournamentPhase::Cancelled => Err(NetworkingError::new(
            "lobby actions are unavailable for closed sessions",
        )),
    }
}

fn recompute_lobby_phase(state: &mut TournamentState) {
    if !matches!(
        state.phase,
        TournamentPhase::WaitingForPlayers | TournamentPhase::ReadyCheck
    ) {
        return;
    }

    let occupied_seats = state
        .seats
        .iter()
        .filter(|seat| seat.occupancy == SeatOccupancyState::Occupied)
        .collect::<Vec<_>>();
    let all_ready = occupied_seats.iter().all(|seat| seat.is_ready);

    state.phase = if occupied_seats.len() >= 2 && all_ready {
        TournamentPhase::ReadyCheck
    } else {
        TournamentPhase::WaitingForPlayers
    };
}

fn apply_start_tournament(
    state: &mut TournamentState,
) -> Result<TournamentController, NetworkingError> {
    ensure_lobby_phase(state.phase)?;
    ensure_lobby_seat_map(state);

    let original_participants = state.participants.clone();
    let registered_players = state
        .seats
        .iter()
        .filter(|seat| seat.occupancy == SeatOccupancyState::Occupied)
        .map(|seat| {
            let participant_id = seat
                .participant_id
                .as_ref()
                .ok_or_else(|| NetworkingError::new("occupied seat is missing a participant id"))?;
            let participant = original_participants.get(participant_id).ok_or_else(|| {
                NetworkingError::new("occupied seat references an unknown participant")
            })?;

            Ok(RegisteredPlayer {
                identity: participant.identity.clone(),
                seat_index: seat.seat_index,
                is_host: participant.is_host,
                is_ready: seat.is_ready,
            })
        })
        .collect::<Result<Vec<_>, NetworkingError>>()?;
    let mut controller = TournamentController::new(
        state.table_id.clone(),
        state.session_epoch,
        state.config.clone(),
        registered_players,
    )
    .map_err(|error| NetworkingError::new(error.to_string()))?;
    controller
        .start_tournament(now_epoch_ms())
        .map_err(|error| NetworkingError::new(error.to_string()))?;

    let mut next_state = controller.state().clone();
    for (player_id, original_participant) in original_participants {
        if let Some(next_participant) = next_state.participants.get_mut(&player_id) {
            next_participant.admitted_at_ms = original_participant.admitted_at_ms;
            next_participant.connection_state = original_participant.connection_state;
            next_participant.reconnect_token = original_participant.reconnect_token;
            next_participant.reconnect_expiry_ms = original_participant.reconnect_expiry_ms;
        } else if original_participant.state == ParticipantState::Admitted {
            next_state
                .participants
                .insert(player_id, original_participant);
        }
    }

    *state = next_state;
    Ok(controller)
}

fn build_tournament_started_event(state: &TournamentState) -> TournamentStartedEvent {
    TournamentStartedEvent {
        tournament_name: state.config.tournament_name.clone(),
        starting_stack: state.config.starting_stack,
        blind_schedule_preset: state
            .blind_schedule
            .levels
            .first()
            .map(|level| level.label.clone())
            .unwrap_or_else(|| "Level 1".to_string()),
        frozen_player_ids: state
            .seats
            .iter()
            .filter_map(|seat| seat.participant_id.clone())
            .collect(),
    }
}

fn ensure_lobby_seat_map(state: &mut TournamentState) {
    if state.seats.len() >= state.config.max_players as usize {
        return;
    }

    state.seats = (0..state.config.max_players)
        .map(|seat_index| {
            state
                .seats
                .get(seat_index as usize)
                .cloned()
                .unwrap_or(SeatState {
                    seat_index,
                    occupancy: SeatOccupancyState::Empty,
                    tournament_state: TournamentSeatState::Open,
                    participant_id: None,
                    display_name: None,
                    chip_count: None,
                    is_ready: false,
                    marker: None,
                })
        })
        .collect();
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
    let (projected_snapshot_state, private_hole_cards) =
        build_recipient_snapshot_state(&authoritative_snapshot, player_id)?;

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
            state: projected_snapshot_state,
            local_player_id: player_id.to_string(),
            private_hole_cards,
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

fn build_recipient_snapshot_state(
    authoritative_state: &TournamentState,
    player_id: &str,
) -> Result<(RecipientSnapshotState, Vec<crate::domain::Card>), NetworkingError> {
    let mut projected_state = authoritative_state.clone();
    let occupied_seat_links: BTreeMap<u8, String> = projected_state
        .seats
        .iter()
        .filter(|seat| seat.occupancy == SeatOccupancyState::Occupied)
        .filter_map(|seat| {
            seat.participant_id
                .as_ref()
                .map(|participant_id| (seat.seat_index, participant_id.clone()))
        })
        .collect();
    for (participant_id, participant) in &mut projected_state.participants {
        if participant
            .seat_index
            .is_some_and(|seat_index| occupied_seat_links.get(&seat_index) != Some(participant_id))
        {
            participant.seat_index = None;
        }
    }
    let projection = StateProjector::project(&projected_state)
        .map_err(|error| NetworkingError::new(error.to_string()))?;
    let private_state = projection
        .private_states
        .get(player_id)
        .ok_or_else(|| NetworkingError::new("snapshot target is not registered"))?;
    let participants = projected_state
        .participants
        .values()
        .map(|participant| SnapshotParticipant {
            player_id: participant.identity.player_id.clone(),
            display_name: participant.identity.display_name.clone(),
            seat_index: participant.seat_index,
            is_host: participant.is_host,
            is_ready: participant
                .seat_index
                .and_then(|seat_index| projected_state.seats.get(seat_index as usize))
                .map(|seat| seat.is_ready)
                .unwrap_or(false),
            connection_state: participant.connection_state,
            participant_state: participant.state,
        })
        .map(|participant| (participant.player_id.clone(), participant))
        .collect();
    let current_hand = projected_state
        .current_hand
        .as_ref()
        .map(|hand| RecipientHandSnapshot {
            hand_number: hand.hand_number,
            cycle_phase: hand.cycle_phase,
            street: hand.street,
            dealer_seat_index: hand.dealer_seat_index,
            small_blind_seat_index: hand.small_blind_seat_index,
            big_blind_seat_index: hand.big_blind_seat_index,
            board_cards: hand.board_cards.clone(),
            public_hole_cards_by_player_id: public_revealed_hole_cards(hand),
            participation_by_player_id: hand.participation_by_player_id.clone(),
            betting_round: hand.betting_round.clone(),
            action_window: hand.action_window.clone(),
        });

    Ok((
        RecipientSnapshotState {
            table_id: projected_state.table_id,
            session_epoch: projected_state.session_epoch,
            phase: projected_state.phase,
            config: projected_state.config,
            blind_schedule: projected_state.blind_schedule,
            blind_level_index: projected_state.blind_level_index,
            participants,
            seats: projected_state.seats,
            current_hand,
            hand_results: projected_state.hand_results,
            placements: projected_state.placements,
        },
        private_state.private_hole_cards.clone(),
    ))
}

fn public_revealed_hole_cards(
    hand: &crate::domain::HandState,
) -> BTreeMap<String, Vec<crate::domain::Card>> {
    if !matches!(
        hand.cycle_phase,
        HandCyclePhase::Showdown | HandCyclePhase::Settlement
    ) {
        return BTreeMap::new();
    }

    hand.hole_cards_by_player_id
        .iter()
        .filter(|(player_id, _)| {
            hand.participation_by_player_id
                .get(player_id.as_str())
                .is_some_and(|participation| {
                    !matches!(
                        participation,
                        HandParticipationState::Folded
                            | HandParticipationState::Out
                            | HandParticipationState::EliminatedObserver
                    )
                })
        })
        .map(|(player_id, cards)| (player_id.clone(), cards.clone()))
        .collect()
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
            Ok(snapshot) => return Ok((stream, snapshot)),
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
        build_snapshot_envelope, connect_and_join, handle_join_request, handle_reconnect_request,
        handle_resync_request, now_epoch_ms, reconnect_after_disconnect,
        validate_production_host_ip, ClientReconnectIdentity,
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
            ActionWindowOpened, EliminationEvent, JoinTournamentRequest, JsonSignedEnvelope,
            PrivateHoleCardsEvent, ProtocolMessageType, ReconnectTournamentRequest, ResyncRequest,
            SignedEnvelope, SnapshotEvent, TournamentCompleteEvent, TournamentStartedEvent,
            PROTOCOL_VERSION,
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
        let deadline = Instant::now() + Duration::from_secs(10);

        loop {
            match client.next_event(Duration::from_millis(200)) {
                Ok(ClientRuntimeEvent::PublicEvent {
                    message_type,
                    payload,
                    ..
                }) => {
                    assert_eq!(message_type, expected_message_type);
                    return payload;
                }
                Ok(ClientRuntimeEvent::Snapshot(_)) => {}
                Ok(ClientRuntimeEvent::Reconnecting { .. }) => {}
                Ok(ClientRuntimeEvent::Disconnected { .. }) => {}
                Ok(ClientRuntimeEvent::ResyncRequested { .. }) => {}
                Ok(ClientRuntimeEvent::PrivateHoleCards(other)) => {
                    panic!("expected public event, got private hole cards: {other:?}");
                }
                Ok(ClientRuntimeEvent::SafeError { message, .. }) => {
                    panic!("expected public event, got safe error: {message}");
                }
                // A poll-window timeout is not a failure under load; keep waiting
                // until the outer deadline rather than panicking on the first gap.
                Err(_) => {}
            }

            assert!(
                Instant::now() < deadline,
                "expected public event before timeout"
            );
        }
    }

    fn wait_for_public_event(
        client: &ClientRuntime,
        expected_message_type: ProtocolMessageType,
    ) -> serde_json::Value {
        wait_for_public_event_where(client, expected_message_type, |_| true)
    }

    fn wait_for_public_event_where(
        client: &ClientRuntime,
        expected_message_type: ProtocolMessageType,
        predicate: impl Fn(&serde_json::Value) -> bool,
    ) -> serde_json::Value {
        let deadline = Instant::now() + Duration::from_secs(10);

        loop {
            match client.next_event(Duration::from_millis(200)) {
                Ok(ClientRuntimeEvent::PublicEvent {
                    message_type,
                    payload,
                    ..
                }) if message_type == expected_message_type && predicate(&payload) => {
                    return payload
                }
                Ok(ClientRuntimeEvent::Snapshot(_)) => {}
                Ok(ClientRuntimeEvent::PublicEvent { .. }) => {}
                Ok(ClientRuntimeEvent::PrivateHoleCards(_)) => {}
                Ok(ClientRuntimeEvent::Reconnecting { .. }) => {}
                Ok(ClientRuntimeEvent::Disconnected { .. }) => {}
                Ok(ClientRuntimeEvent::ResyncRequested { .. }) => {}
                Ok(ClientRuntimeEvent::SafeError { message, .. }) => {
                    panic!("expected public event, got safe error: {message}");
                }
                // A poll-window timeout is not a failure under load; keep waiting
                // until the outer deadline rather than panicking on the first gap.
                Err(_) => {}
            }

            assert!(
                Instant::now() < deadline,
                "expected matching public event before timeout"
            );
        }
    }

    fn wait_for_private_hole_cards(client: &ClientRuntime) -> PrivateHoleCardsEvent {
        let deadline = Instant::now() + Duration::from_secs(10);

        loop {
            match client.next_event(Duration::from_millis(200)) {
                Ok(ClientRuntimeEvent::PrivateHoleCards(payload)) => return payload,
                Ok(ClientRuntimeEvent::Snapshot(_)) => {}
                Ok(ClientRuntimeEvent::PublicEvent { .. }) => {}
                Ok(ClientRuntimeEvent::Reconnecting { .. }) => {}
                Ok(ClientRuntimeEvent::Disconnected { .. }) => {}
                Ok(ClientRuntimeEvent::ResyncRequested { .. }) => {}
                Ok(ClientRuntimeEvent::SafeError { message, .. }) => {
                    panic!("expected private hole cards, got safe error: {message}");
                }
                // A poll-window timeout is not a failure under load; keep waiting
                // until the outer deadline rather than panicking on the first gap.
                Err(_) => {}
            }

            assert!(
                Instant::now() < deadline,
                "expected private hole cards before timeout"
            );
        }
    }

    fn expect_snapshot_where(
        client: &ClientRuntime,
        predicate: impl Fn(&SnapshotEvent) -> bool,
    ) -> SnapshotEvent {
        let deadline = Instant::now() + Duration::from_secs(10);

        loop {
            // A poll-window timeout is not a failure under load; keep waiting until the
            // outer deadline rather than panicking on the first gap.
            if let Ok(ClientRuntimeEvent::Snapshot(snapshot)) =
                client.next_event(Duration::from_millis(200))
            {
                if predicate(&snapshot) {
                    return *snapshot;
                }
            }

            assert!(
                Instant::now() < deadline,
                "expected matching snapshot before timeout"
            );
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

    fn wait_for_client_command_connection(client: &ClientRuntime) {
        let deadline = Instant::now() + Duration::from_secs(10);

        loop {
            if client
                .command_connection
                .lock()
                .expect("client command connection")
                .stream
                .is_some()
            {
                return;
            }

            assert!(
                Instant::now() < deadline,
                "client command connection should become available"
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

    fn signed_join_envelope(
        provider: &DefaultCryptoProvider,
        signing_keys: &crate::crypto::SigningKeyMaterial,
        encryption_keys: &crate::crypto::EncryptionKeyMaterial,
        join_payload: &JoinPayload,
        player_id: &str,
        display_name: &str,
        join_token: &str,
    ) -> JsonSignedEnvelope {
        let request = JoinTournamentRequest {
            display_name: display_name.to_string(),
            join_token: join_token.to_string(),
            signing_public_key: signing_keys.public_key_base64(),
            encryption_public_key: encryption_keys.public_key_base64(),
        };
        let mut envelope = SignedEnvelope {
            protocol_version: PROTOCOL_VERSION,
            message_type: ProtocolMessageType::JoinTournamentRequest,
            table_id: join_payload.table_id.clone(),
            session_epoch: join_payload.session_epoch,
            sender_id: player_id.to_string(),
            counter: 1,
            message_id: format!("join-test-{player_id}"),
            server_sequence: None,
            payload: serde_json::to_value(request).expect("join request payload"),
            signature: None,
        };
        envelope
            .sign(provider, signing_keys)
            .expect("join request should sign");

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
    fn recipient_snapshots_strip_other_players_private_data_from_serialized_json() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let server_sequence = Arc::new(AtomicU64::new(0));
        let join_payload = sample_join_payload_for_tests(
            "table-private-snapshot",
            80,
            host_signing_keys.public_key_base64(),
        );
        let mut state = sample_tournament_state("table-private-snapshot", 80);
        let player_a_keys = provider.generate_signing_keypair();
        let player_b_keys = provider.generate_signing_keypair();
        state.phase = TournamentPhase::Running;
        state.participants.insert(
            "player-a".to_string(),
            sample_participant_entry(
                "player-a",
                "Alice",
                player_a_keys.public_key_base64(),
                provider.generate_encryption_keypair().public_key_base64(),
                ParticipantState::Active,
                ConnectionState::Connected,
                Some(0),
                "token-a",
            ),
        );
        state.participants.insert(
            "player-b".to_string(),
            sample_participant_entry(
                "player-b",
                "Bob",
                player_b_keys.public_key_base64(),
                provider.generate_encryption_keypair().public_key_base64(),
                ParticipantState::Active,
                ConnectionState::Connected,
                Some(1),
                "token-b",
            ),
        );
        state.seats = vec![
            SeatState {
                seat_index: 0,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: TournamentSeatState::Active,
                participant_id: Some("player-a".to_string()),
                display_name: Some("Alice".to_string()),
                chip_count: Some(1200),
                is_ready: true,
                marker: None,
            },
            SeatState {
                seat_index: 1,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: TournamentSeatState::Active,
                participant_id: Some("player-b".to_string()),
                display_name: Some("Bob".to_string()),
                chip_count: Some(1300),
                is_ready: true,
                marker: None,
            },
        ];
        state.current_hand = Some(crate::domain::HandState {
            hand_number: 4,
            cycle_phase: HandCyclePhase::AwaitingAction,
            street: crate::domain::StreetPhase::Turn,
            dealer_seat_index: 0,
            small_blind_seat_index: 0,
            big_blind_seat_index: 1,
            board_cards: Vec::new(),
            hole_cards_by_player_id: [
                (
                    "player-a".to_string(),
                    vec![
                        crate::domain::Card {
                            rank: crate::domain::Rank::Ace,
                            suit: crate::domain::Suit::Clubs,
                        },
                        crate::domain::Card {
                            rank: crate::domain::Rank::Ace,
                            suit: crate::domain::Suit::Hearts,
                        },
                    ],
                ),
                (
                    "player-b".to_string(),
                    vec![
                        crate::domain::Card {
                            rank: crate::domain::Rank::Jack,
                            suit: crate::domain::Suit::Diamonds,
                        },
                        crate::domain::Card {
                            rank: crate::domain::Rank::Nine,
                            suit: crate::domain::Suit::Spades,
                        },
                    ],
                ),
            ]
            .into_iter()
            .collect(),
            participation_by_player_id: [
                (
                    "player-a".to_string(),
                    crate::domain::HandParticipationState::Active,
                ),
                (
                    "player-b".to_string(),
                    crate::domain::HandParticipationState::Active,
                ),
            ]
            .into_iter()
            .collect(),
            betting_round: crate::domain::BettingRoundState {
                street: crate::domain::StreetPhase::Turn,
                current_bet: 40,
                min_raise_to: Some(80),
                max_raise_to: Some(200),
                pot_size: 120,
                contributions_by_player_id: BTreeMap::new(),
            },
            action_window: None,
        });
        let authoritative_state = Arc::new(Mutex::new(state));

        let snapshot = build_snapshot_envelope(
            &provider,
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
            "player-a",
        )
        .expect("snapshot envelope");

        let json = serde_json::to_string(&snapshot).expect("snapshot json");
        assert!(!json.contains("token-b"));
        assert!(!json.contains("\"rank\":\"JACK\""));
        assert_eq!(snapshot.payload.private_hole_cards.len(), 2);
        assert!(snapshot
            .payload
            .state
            .current_hand
            .as_ref()
            .is_some_and(|hand| hand.public_hole_cards_by_player_id.is_empty()));
    }

    #[test]
    fn observer_snapshot_contains_no_private_cards() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let server_sequence = Arc::new(AtomicU64::new(0));
        let join_payload = sample_join_payload_for_tests(
            "table-observer-snapshot",
            81,
            host_signing_keys.public_key_base64(),
        );
        let mut state = sample_tournament_state("table-observer-snapshot", 81);
        let observer_keys = provider.generate_signing_keypair();
        let active_keys = provider.generate_signing_keypair();
        state.phase = TournamentPhase::Running;
        state.participants.insert(
            "observer".to_string(),
            sample_participant_entry(
                "observer",
                "Observer",
                observer_keys.public_key_base64(),
                provider.generate_encryption_keypair().public_key_base64(),
                ParticipantState::EliminatedObserver,
                ConnectionState::Connected,
                Some(0),
                "token-observer",
            ),
        );
        state.participants.insert(
            "active".to_string(),
            sample_participant_entry(
                "active",
                "Active",
                active_keys.public_key_base64(),
                provider.generate_encryption_keypair().public_key_base64(),
                ParticipantState::Active,
                ConnectionState::Connected,
                Some(1),
                "token-active",
            ),
        );
        state.seats = vec![
            SeatState {
                seat_index: 0,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: TournamentSeatState::EliminatedObserver,
                participant_id: Some("observer".to_string()),
                display_name: Some("Observer".to_string()),
                chip_count: Some(0),
                is_ready: true,
                marker: None,
            },
            SeatState {
                seat_index: 1,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: TournamentSeatState::Active,
                participant_id: Some("active".to_string()),
                display_name: Some("Active".to_string()),
                chip_count: Some(1500),
                is_ready: true,
                marker: None,
            },
        ];
        state.current_hand = Some(crate::domain::HandState {
            hand_number: 1,
            cycle_phase: HandCyclePhase::AwaitingAction,
            street: crate::domain::StreetPhase::Preflop,
            dealer_seat_index: 0,
            small_blind_seat_index: 0,
            big_blind_seat_index: 1,
            board_cards: Vec::new(),
            hole_cards_by_player_id: [(
                "active".to_string(),
                vec![
                    crate::domain::Card {
                        rank: crate::domain::Rank::King,
                        suit: crate::domain::Suit::Clubs,
                    },
                    crate::domain::Card {
                        rank: crate::domain::Rank::Queen,
                        suit: crate::domain::Suit::Clubs,
                    },
                ],
            )]
            .into_iter()
            .collect(),
            participation_by_player_id: [(
                "active".to_string(),
                crate::domain::HandParticipationState::Active,
            )]
            .into_iter()
            .collect(),
            betting_round: crate::domain::BettingRoundState {
                street: crate::domain::StreetPhase::Preflop,
                current_bet: 20,
                min_raise_to: Some(40),
                max_raise_to: None,
                pot_size: 30,
                contributions_by_player_id: BTreeMap::new(),
            },
            action_window: None,
        });
        let authoritative_state = Arc::new(Mutex::new(state));

        let snapshot = build_snapshot_envelope(
            &provider,
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
            "observer",
        )
        .expect("observer snapshot envelope");

        assert!(snapshot.payload.private_hole_cards.is_empty());
        assert!(snapshot
            .payload
            .state
            .current_hand
            .as_ref()
            .is_some_and(|hand| hand.public_hole_cards_by_player_id.is_empty()));
    }

    #[test]
    fn join_requests_are_rejected_after_roster_freeze() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let join_payload = sample_join_payload_for_tests(
            "table-join-ready-check",
            82,
            host_signing_keys.public_key_base64(),
        );
        let server_sequence = Arc::new(AtomicU64::new(0));
        let mut state = sample_tournament_state("table-join-ready-check", 82);
        state.phase = TournamentPhase::ReadyCheck;
        let authoritative_state = Arc::new(Mutex::new(state));
        let player_signing_keys = provider.generate_signing_keypair();
        let player_encryption_keys = provider.generate_encryption_keypair();

        let result = handle_join_request(
            &provider,
            signed_join_envelope(
                &provider,
                &player_signing_keys,
                &player_encryption_keys,
                &join_payload,
                "player-ready-check",
                "Ready Check",
                &join_payload.join_token,
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        );

        assert!(result.is_err());
        assert!(result
            .expect_err("join should fail")
            .to_string()
            .contains("roster freeze"));
    }

    #[test]
    fn joins_allow_up_to_capacity_then_reject_max_plus_one_even_with_open_seats() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let join_payload = sample_join_payload_for_tests(
            "table-max-capacity",
            87,
            host_signing_keys.public_key_base64(),
        );
        let server_sequence = Arc::new(AtomicU64::new(0));
        let mut state = sample_tournament_state("table-max-capacity", 87);
        state.config.max_players = 2;
        state.seats = (0..4)
            .map(|seat_index| SeatState {
                seat_index,
                occupancy: SeatOccupancyState::Empty,
                tournament_state: TournamentSeatState::Open,
                participant_id: None,
                display_name: None,
                chip_count: None,
                is_ready: false,
                marker: None,
            })
            .collect();
        let authoritative_state = Arc::new(Mutex::new(state));
        let first_signing_keys = provider.generate_signing_keypair();
        let first_encryption_keys = provider.generate_encryption_keypair();
        let second_signing_keys = provider.generate_signing_keypair();
        let second_encryption_keys = provider.generate_encryption_keypair();
        let third_signing_keys = provider.generate_signing_keypair();
        let third_encryption_keys = provider.generate_encryption_keypair();

        let first = handle_join_request(
            &provider,
            signed_join_envelope(
                &provider,
                &first_signing_keys,
                &first_encryption_keys,
                &join_payload,
                "player-one",
                "One",
                &join_payload.join_token,
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        );
        let second = handle_join_request(
            &provider,
            signed_join_envelope(
                &provider,
                &second_signing_keys,
                &second_encryption_keys,
                &join_payload,
                "player-two",
                "Two",
                &join_payload.join_token,
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        );
        let third = handle_join_request(
            &provider,
            signed_join_envelope(
                &provider,
                &third_signing_keys,
                &third_encryption_keys,
                &join_payload,
                "player-three",
                "Three",
                &join_payload.join_token,
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        );

        assert!(first.is_ok());
        assert!(second.is_ok());
        assert!(third.is_err());
        assert!(third
            .expect_err("join should fail")
            .to_string()
            .contains("table is full"));
    }

    #[test]
    fn admitted_unseated_participants_count_toward_join_capacity() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let join_payload = sample_join_payload_for_tests(
            "table-capacity",
            83,
            host_signing_keys.public_key_base64(),
        );
        let server_sequence = Arc::new(AtomicU64::new(0));
        let mut state = sample_tournament_state("table-capacity", 83);
        state.config.max_players = 2;
        let host_keys = provider.generate_signing_keypair();
        let waiting_keys = provider.generate_signing_keypair();
        state.participants.insert(
            "host".to_string(),
            sample_participant_entry(
                "host",
                "Host",
                host_keys.public_key_base64(),
                provider.generate_encryption_keypair().public_key_base64(),
                ParticipantState::Active,
                ConnectionState::Connected,
                Some(0),
                "token-host",
            ),
        );
        state.participants.insert(
            "waiting".to_string(),
            sample_participant_entry(
                "waiting",
                "Waiting",
                waiting_keys.public_key_base64(),
                provider.generate_encryption_keypair().public_key_base64(),
                ParticipantState::Admitted,
                ConnectionState::Disconnected,
                None,
                "token-waiting",
            ),
        );
        state.seats = vec![
            SeatState {
                seat_index: 0,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: TournamentSeatState::Active,
                participant_id: Some("host".to_string()),
                display_name: Some("Host".to_string()),
                chip_count: Some(1500),
                is_ready: true,
                marker: None,
            },
            SeatState {
                seat_index: 1,
                occupancy: SeatOccupancyState::Empty,
                tournament_state: TournamentSeatState::Open,
                participant_id: None,
                display_name: None,
                chip_count: None,
                is_ready: false,
                marker: None,
            },
        ];
        let authoritative_state = Arc::new(Mutex::new(state));
        let player_signing_keys = provider.generate_signing_keypair();
        let player_encryption_keys = provider.generate_encryption_keypair();

        let result = handle_join_request(
            &provider,
            signed_join_envelope(
                &provider,
                &player_signing_keys,
                &player_encryption_keys,
                &join_payload,
                "player-over-capacity",
                "Capacity",
                &join_payload.join_token,
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        );

        assert!(result.is_err());
        assert!(result
            .expect_err("join should fail")
            .to_string()
            .contains("table is full"));
    }

    #[test]
    fn reconnect_eligible_disconnected_participants_count_toward_join_capacity() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let join_payload = sample_join_payload_for_tests(
            "table-reconnect-capacity",
            86,
            host_signing_keys.public_key_base64(),
        );
        let server_sequence = Arc::new(AtomicU64::new(0));
        let mut state = sample_tournament_state("table-reconnect-capacity", 86);
        state.config.max_players = 2;
        let host_keys = provider.generate_signing_keypair();
        let reconnecting_keys = provider.generate_signing_keypair();
        state.participants.insert(
            "host".to_string(),
            sample_participant_entry(
                "host",
                "Host",
                host_keys.public_key_base64(),
                provider.generate_encryption_keypair().public_key_base64(),
                ParticipantState::Active,
                ConnectionState::Connected,
                Some(0),
                "token-host",
            ),
        );
        state.participants.insert(
            "reconnecting".to_string(),
            sample_participant_entry(
                "reconnecting",
                "Reconnect",
                reconnecting_keys.public_key_base64(),
                provider.generate_encryption_keypair().public_key_base64(),
                ParticipantState::Reconnecting,
                ConnectionState::Disconnected,
                Some(1),
                "token-reconnecting",
            ),
        );
        let authoritative_state = Arc::new(Mutex::new(state));
        let player_signing_keys = provider.generate_signing_keypair();
        let player_encryption_keys = provider.generate_encryption_keypair();

        let result = handle_join_request(
            &provider,
            signed_join_envelope(
                &provider,
                &player_signing_keys,
                &player_encryption_keys,
                &join_payload,
                "player-after-reconnect-slot",
                "Reconnect Slot",
                &join_payload.join_token,
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        );

        assert!(result.is_err());
        assert!(result
            .expect_err("join should fail")
            .to_string()
            .contains("table is full"));
    }

    #[test]
    fn eliminated_observers_do_not_block_join_capacity() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let join_payload = sample_join_payload_for_tests(
            "table-observer-capacity",
            84,
            host_signing_keys.public_key_base64(),
        );
        let server_sequence = Arc::new(AtomicU64::new(0));
        let mut state = sample_tournament_state("table-observer-capacity", 84);
        state.config.max_players = 2;
        let host_keys = provider.generate_signing_keypair();
        let observer_keys = provider.generate_signing_keypair();
        state.participants.insert(
            "host".to_string(),
            sample_participant_entry(
                "host",
                "Host",
                host_keys.public_key_base64(),
                provider.generate_encryption_keypair().public_key_base64(),
                ParticipantState::Active,
                ConnectionState::Connected,
                Some(0),
                "token-host",
            ),
        );
        state.participants.insert(
            "observer".to_string(),
            sample_participant_entry(
                "observer",
                "Observer",
                observer_keys.public_key_base64(),
                provider.generate_encryption_keypair().public_key_base64(),
                ParticipantState::EliminatedObserver,
                ConnectionState::Connected,
                Some(1),
                "token-observer",
            ),
        );
        state.seats = vec![
            SeatState {
                seat_index: 0,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: TournamentSeatState::Active,
                participant_id: Some("host".to_string()),
                display_name: Some("Host".to_string()),
                chip_count: Some(1500),
                is_ready: true,
                marker: None,
            },
            SeatState {
                seat_index: 1,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: TournamentSeatState::EliminatedObserver,
                participant_id: Some("observer".to_string()),
                display_name: Some("Observer".to_string()),
                chip_count: Some(0),
                is_ready: true,
                marker: None,
            },
        ];
        let authoritative_state = Arc::new(Mutex::new(state));
        let player_signing_keys = provider.generate_signing_keypair();
        let player_encryption_keys = provider.generate_encryption_keypair();

        let result = handle_join_request(
            &provider,
            signed_join_envelope(
                &provider,
                &player_signing_keys,
                &player_encryption_keys,
                &join_payload,
                "player-new",
                "New Player",
                &join_payload.join_token,
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn join_requests_reject_the_wrong_join_token() {
        let provider = DefaultCryptoProvider;
        let host_signing_keys = provider.generate_signing_keypair();
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let join_payload = sample_join_payload_for_tests(
            "table-wrong-token",
            85,
            host_signing_keys.public_key_base64(),
        );
        let server_sequence = Arc::new(AtomicU64::new(0));
        let authoritative_state =
            Arc::new(Mutex::new(sample_tournament_state("table-wrong-token", 85)));
        let player_signing_keys = provider.generate_signing_keypair();
        let player_encryption_keys = provider.generate_encryption_keypair();

        let result = handle_join_request(
            &provider,
            signed_join_envelope(
                &provider,
                &player_signing_keys,
                &player_encryption_keys,
                &join_payload,
                "player-wrong-token",
                "Wrong Token",
                "not-the-live-token",
            ),
            &join_payload,
            &authoritative_state,
            &server_sequence,
            &host_signing_keys,
            &host_encryption_keys,
        );

        assert!(result.is_err());
        assert!(result
            .expect_err("join should fail")
            .to_string()
            .contains("join token mismatch"));
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
                    ..
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
                ..
            } => {
                assert_eq!(message_type, ProtocolMessageType::TournamentStartedEvent);
                assert_eq!(payload.get("tournamentName"), Some(&json!("LAN Test")));
            }
            other => panic!("expected public event, got {other:?}"),
        }
    }

    #[test]
    fn client_lobby_requests_sync_seat_and_ready_state_across_host_and_clients() {
        let provider = DefaultCryptoProvider;
        let host = bind_test_host(&provider, "table-lobby-sync", 60);
        let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
        let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
        let _ = expect_snapshot_event(&alice);
        let _ = expect_snapshot_event(&bob);

        alice.claim_seat(1).expect("alice should claim a seat");
        let alice_seated = expect_snapshot_where(&alice, |snapshot| {
            snapshot
                .state
                .participants
                .get("player-alice")
                .and_then(|participant| participant.seat_index)
                == Some(1)
        });
        assert_eq!(alice_seated.state.phase, TournamentPhase::WaitingForPlayers);

        bob.claim_seat(2).expect("bob should claim a seat");
        let host_view_after_bob = expect_snapshot_where(&alice, |snapshot| {
            snapshot
                .state
                .participants
                .get("player-bob")
                .and_then(|participant| participant.seat_index)
                == Some(2)
        });
        let bob_view_after_claim = expect_snapshot_where(&bob, |snapshot| {
            snapshot
                .state
                .participants
                .get("player-bob")
                .and_then(|participant| participant.seat_index)
                == Some(2)
        });
        assert_eq!(
            host_view_after_bob.state.phase,
            TournamentPhase::WaitingForPlayers
        );
        assert_eq!(
            bob_view_after_claim.state.phase,
            TournamentPhase::WaitingForPlayers
        );

        alice
            .set_ready_state(true)
            .expect("alice should toggle ready");
        let alice_ready = expect_snapshot_where(&alice, |snapshot| {
            snapshot
                .state
                .seats
                .iter()
                .find(|seat| seat.seat_index == 1)
                .is_some_and(|seat| seat.is_ready)
        });
        let bob_sees_alice_ready = expect_snapshot_where(&bob, |snapshot| {
            snapshot
                .state
                .seats
                .iter()
                .find(|seat| seat.seat_index == 1)
                .is_some_and(|seat| seat.is_ready)
        });
        assert_eq!(alice_ready.state.phase, TournamentPhase::WaitingForPlayers);
        assert_eq!(
            bob_sees_alice_ready.state.phase,
            TournamentPhase::WaitingForPlayers
        );

        bob.set_ready_state(true).expect("bob should toggle ready");
        let alice_ready_check = expect_snapshot_where(&alice, |snapshot| {
            snapshot.state.phase == TournamentPhase::ReadyCheck
        });
        let bob_ready_check = expect_snapshot_where(&bob, |snapshot| {
            snapshot.state.phase == TournamentPhase::ReadyCheck
        });
        assert_eq!(alice_ready_check.state.phase, TournamentPhase::ReadyCheck);
        assert_eq!(bob_ready_check.state.phase, TournamentPhase::ReadyCheck);

        let host_state = host.authoritative_state().expect("host state");
        assert_eq!(host_state.phase, TournamentPhase::ReadyCheck);
        assert!(host_state
            .seats
            .iter()
            .filter(|seat| seat.occupancy == SeatOccupancyState::Occupied)
            .all(|seat| seat.is_ready));
    }

    #[test]
    fn host_start_tournament_emits_running_public_and_private_events_to_connected_clients() {
        let provider = DefaultCryptoProvider;
        let host = bind_test_host(&provider, "table-host-start", 61);
        let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
        let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
        let _ = expect_snapshot_event(&alice);
        let _ = expect_snapshot_event(&bob);

        host.claim_seat("player-alice", 0)
            .expect("alice seat claim should succeed");
        let _ = expect_snapshot_where(&alice, |snapshot| {
            snapshot
                .state
                .participants
                .get("player-alice")
                .and_then(|participant| participant.seat_index)
                == Some(0)
        });
        let _ = expect_snapshot_where(&bob, |snapshot| {
            snapshot
                .state
                .participants
                .get("player-alice")
                .and_then(|participant| participant.seat_index)
                == Some(0)
        });

        host.claim_seat("player-bob", 1)
            .expect("bob seat claim should succeed");
        let _ = expect_snapshot_where(&alice, |snapshot| {
            snapshot
                .state
                .participants
                .get("player-bob")
                .and_then(|participant| participant.seat_index)
                == Some(1)
        });
        let _ = expect_snapshot_where(&bob, |snapshot| {
            snapshot
                .state
                .participants
                .get("player-bob")
                .and_then(|participant| participant.seat_index)
                == Some(1)
        });

        host.set_ready_state("player-alice", true)
            .expect("alice ready should succeed");
        let _ = expect_snapshot_where(&alice, |snapshot| {
            snapshot
                .state
                .seats
                .iter()
                .find(|seat| seat.seat_index == 0)
                .is_some_and(|seat| seat.is_ready)
        });
        let _ = expect_snapshot_where(&bob, |snapshot| {
            snapshot
                .state
                .seats
                .iter()
                .find(|seat| seat.seat_index == 0)
                .is_some_and(|seat| seat.is_ready)
        });

        host.set_ready_state("player-bob", true)
            .expect("bob ready should succeed");
        let _ = expect_snapshot_where(&alice, |snapshot| {
            snapshot.state.phase == TournamentPhase::ReadyCheck
        });
        let _ = expect_snapshot_where(&bob, |snapshot| {
            snapshot.state.phase == TournamentPhase::ReadyCheck
        });

        host.start_tournament()
            .expect("host should start the tournament");
        for client in [&alice, &bob] {
            let _ = wait_for_public_event(client, ProtocolMessageType::TournamentStartedEvent);
            let hand_start_payload =
                wait_for_public_event(client, ProtocolMessageType::HandStartingEvent);
            assert_eq!(hand_start_payload.get("handNumber"), Some(&json!(1)));

            assert_eq!(wait_for_private_hole_cards(client).hole_cards.len(), 2);

            let action_window_payload =
                wait_for_public_event(client, ProtocolMessageType::ActionWindowOpenedEvent);
            assert_eq!(action_window_payload.get("handNumber"), Some(&json!(1)));
        }

        let host_state = host.authoritative_state().expect("host state");
        assert_eq!(host_state.phase, TournamentPhase::Running);
        assert!(host_state.current_hand.is_some());
    }

    #[test]
    fn client_action_submission_syncs_running_state_across_the_live_runtime() {
        let provider = DefaultCryptoProvider;
        let host = bind_test_host(&provider, "table-action-sync", 62);
        let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
        let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
        let _ = expect_snapshot_event(&alice);
        let _ = expect_snapshot_event(&bob);

        host.claim_seat("player-alice", 0)
            .expect("alice seat claim should succeed");
        let _ = expect_snapshot_where(&alice, |snapshot| {
            snapshot
                .state
                .participants
                .get("player-alice")
                .and_then(|participant| participant.seat_index)
                == Some(0)
        });
        let _ = expect_snapshot_where(&bob, |snapshot| {
            snapshot
                .state
                .participants
                .get("player-alice")
                .and_then(|participant| participant.seat_index)
                == Some(0)
        });

        host.claim_seat("player-bob", 1)
            .expect("bob seat claim should succeed");
        let _ = expect_snapshot_where(&alice, |snapshot| {
            snapshot
                .state
                .participants
                .get("player-bob")
                .and_then(|participant| participant.seat_index)
                == Some(1)
        });
        let _ = expect_snapshot_where(&bob, |snapshot| {
            snapshot
                .state
                .participants
                .get("player-bob")
                .and_then(|participant| participant.seat_index)
                == Some(1)
        });

        host.set_ready_state("player-alice", true)
            .expect("alice ready should succeed");
        let _ = expect_snapshot_where(&alice, |snapshot| {
            snapshot
                .state
                .seats
                .iter()
                .find(|seat| seat.seat_index == 0)
                .is_some_and(|seat| seat.is_ready)
        });
        let _ = expect_snapshot_where(&bob, |snapshot| {
            snapshot
                .state
                .seats
                .iter()
                .find(|seat| seat.seat_index == 0)
                .is_some_and(|seat| seat.is_ready)
        });

        host.set_ready_state("player-bob", true)
            .expect("bob ready should succeed");
        let _ = expect_snapshot_where(&alice, |snapshot| {
            snapshot.state.phase == TournamentPhase::ReadyCheck
        });
        let _ = expect_snapshot_where(&bob, |snapshot| {
            snapshot.state.phase == TournamentPhase::ReadyCheck
        });

        host.start_tournament()
            .expect("host should start the tournament");
        let deadline = Instant::now() + Duration::from_secs(2);
        let action_window = loop {
            if let Some(window) = host
                .authoritative_state()
                .expect("host state after start")
                .current_hand
                .as_ref()
                .and_then(|hand| hand.action_window.clone())
            {
                break window;
            }

            assert!(
                Instant::now() < deadline,
                "running hand should expose an action window"
            );
            thread::sleep(Duration::from_millis(20));
        };
        let acting_client = if action_window.player_id == "player-alice" {
            &alice
        } else {
            &bob
        };
        wait_for_client_command_connection(acting_client);
        let action_type = if action_window
            .legal_actions
            .contains(&crate::domain::ActionType::Check)
        {
            crate::domain::ActionType::Check
        } else {
            crate::domain::ActionType::Call
        };

        acting_client
            .submit_action(
                action_window.action_window_id.clone(),
                action_window.seat_index,
                action_type,
                None,
            )
            .expect("client action should succeed");

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let next_action_window_id = host
                .authoritative_state()
                .expect("host state")
                .current_hand
                .as_ref()
                .and_then(|hand| hand.action_window.as_ref())
                .map(|window| window.action_window_id.clone());
            if next_action_window_id != Some(action_window.action_window_id.clone()) {
                break;
            }

            assert!(
                Instant::now() < deadline,
                "host should advance beyond the submitted action window"
            );
            thread::sleep(Duration::from_millis(20));
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
    fn reconnect_after_disconnect_retries_until_prior_connection_cleans_up() {
        let provider = DefaultCryptoProvider;
        let host = bind_test_host(&provider, "table-reconnect-race", 31);
        let join_payload = crate::protocol::decode_join_payload(host.encoded_join_payload())
            .expect("join payload should decode");
        let reconnect_identity = Arc::new(Mutex::new(ClientReconnectIdentity {
            signing_keys: Some(provider.generate_signing_keypair()),
            encryption_keys: Some(provider.generate_encryption_keypair()),
        }));
        let (original_stream, snapshot) = connect_and_join(
            &provider,
            &join_payload,
            "player-race",
            "Race",
            &reconnect_identity,
        )
        .expect("client should join");
        let reconnect_token = snapshot
            .payload
            .reconnect_token
            .clone()
            .expect("reconnect token");

        wait_for_host_participant_state(
            &host,
            "player-race",
            ParticipantState::Admitted,
            ConnectionState::Connected,
        );

        let mut next_counter = 2;
        let (reconnected_stream, reconnected_snapshot) = thread::scope(|scope| {
            scope.spawn(|| {
                thread::sleep(Duration::from_millis(60));
                original_stream
                    .shutdown(Shutdown::Both)
                    .expect("shutdown original stream");
            });

            reconnect_after_disconnect(
                &provider,
                &join_payload,
                "player-race",
                &reconnect_identity,
                Some(reconnect_token.as_str()),
                snapshot.server_sequence.unwrap_or(0),
                &mut next_counter,
            )
        })
        .expect("reconnect should succeed after cleanup");

        assert_eq!(reconnected_snapshot.payload.local_player_id, "player-race");
        wait_for_host_participant_state(
            &host,
            "player-race",
            ParticipantState::Admitted,
            ConnectionState::Connected,
        );
        let state = host.authoritative_state().expect("authoritative state");
        let participant = state.participants.get("player-race").expect("participant");
        assert!(participant.reconnect_expiry_ms.is_none());
        assert_eq!(host.clients.lock().expect("client registry").len(), 1);
        let _ = reconnected_stream.shutdown(Shutdown::Both);
    }

    #[test]
    fn reconnect_after_disconnect_reports_retry_exhaustion_without_creating_a_duplicate_session() {
        let provider = DefaultCryptoProvider;
        let host = bind_test_host(&provider, "table-reconnect-exhausted", 31);
        let join_payload = crate::protocol::decode_join_payload(host.encoded_join_payload())
            .expect("join payload should decode");
        let reconnect_identity = Arc::new(Mutex::new(ClientReconnectIdentity {
            signing_keys: Some(provider.generate_signing_keypair()),
            encryption_keys: Some(provider.generate_encryption_keypair()),
        }));
        let (original_stream, snapshot) = connect_and_join(
            &provider,
            &join_payload,
            "player-exhausted",
            "Exhausted",
            &reconnect_identity,
        )
        .expect("client should join");
        let reconnect_token = snapshot
            .payload
            .reconnect_token
            .clone()
            .expect("reconnect token");

        wait_for_host_participant_state(
            &host,
            "player-exhausted",
            ParticipantState::Admitted,
            ConnectionState::Connected,
        );

        let mut next_counter = 2;
        let error = reconnect_after_disconnect(
            &provider,
            &join_payload,
            "player-exhausted",
            &reconnect_identity,
            Some(reconnect_token.as_str()),
            snapshot.server_sequence.unwrap_or(0),
            &mut next_counter,
        )
        .expect_err("reconnect should exhaust retries while the original stream stays connected");

        assert_eq!(
            error.to_string(),
            "reconnect retries exhausted while waiting for prior connection cleanup"
        );
        let state = host.authoritative_state().expect("authoritative state");
        let participant = state
            .participants
            .get("player-exhausted")
            .expect("participant");
        assert_eq!(participant.connection_state, ConnectionState::Connected);
        assert_eq!(participant.state, ParticipantState::Admitted);
        assert_eq!(host.clients.lock().expect("client registry").len(), 1);
        let _ = original_stream.shutdown(Shutdown::Both);
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
            second_snapshot.payload.state.config.tournament_name,
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
        assert_eq!(snapshot.state.phase, TournamentPhase::Running);
        let action_window = snapshot
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
        assert_eq!(snapshot.state.config.tournament_name, "Resync Continue");

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
                assert_eq!(snapshot.state.phase, TournamentPhase::ReadyCheck);
                assert_eq!(snapshot.state.config.tournament_name, "Resynced Tournament");
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
