use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::{self},
    time::Duration,
};

use crate::{
    crypto::DefaultCryptoProvider,
    domain::{JoinPayload, TournamentState},
    networking::{read_json_frame, write_json_frame},
    protocol::{encode_join_payload, validate_join_payload, JsonSignedEnvelope, PROTOCOL_VERSION},
    tournament::ActionRequest,
};

use super::*;

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
                        // Acquire the lock only for the duration of advance_time,
                        // NOT for the sleep, so that start_tournament() can install
                        // the controller without being starved by this idle loop.
                        let next_state = match tournament_runtime.lock() {
                            Err(_) => break,
                            Ok(mut runtime) => match runtime.as_mut() {
                                None => None,
                                Some(controller) => {
                                    let before = controller.state().clone();
                                    if controller.advance_time(now_epoch_ms()).is_err() {
                                        None
                                    } else {
                                        let after = controller.state().clone();
                                        (after != before).then_some(after)
                                    }
                                }
                            },
                            // MutexGuard `runtime` dropped here — lock released
                        };

                        if let Some(mut state) = next_state {
                            let previous_state = authoritative_state
                                .lock()
                                .map(|authoritative| authoritative.clone())
                                .unwrap_or_else(|_| state.clone());
                            merge_networking_state(&previous_state, &mut state);
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
                        // NPCs never sign or encrypt messages, so these keys are
                        // placeholders. They must still be unique per participant:
                        // `validate_tournament_state` rejects duplicate signing-key
                        // bindings, so seating two or more NPCs with a shared key
                        // would fail tournament start. Derive them from the
                        // (unique) player_id to keep every NPC distinct.
                        signing_public_key: format!("npc-signing-{player_id}"),
                        encryption_public_key: format!("npc-encryption-{player_id}"),
                        signing_key_fingerprint: format!("npc-fingerprint-{player_id}"),
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

    /// Register, seat, and ready every NPC in `assignments` under a single
    /// authoritative-state lock.  Either all succeed or none are applied.
    pub fn add_npc_participants_atomic(
        &self,
        assignments: Vec<super::NpcSeatAssignment>,
    ) -> Result<Vec<String>, NetworkingError> {
        use crate::domain::{
            ConnectionState, ParticipantRegistryEntry, ParticipantState, PlayerIdentity,
        };

        let player_ids: Vec<String> = assignments.iter().map(|a| a.player_id.clone()).collect();

        self.update_lobby_state(|state| {
            ensure_lobby_phase(state.phase)?;
            ensure_lobby_seat_map(state);

            // --- Validate all assignments before mutating anything ---

            // Duplicate player_ids within the batch.
            let mut seen_ids = std::collections::HashSet::new();
            for a in &assignments {
                if !seen_ids.insert(a.player_id.as_str()) {
                    return Err(NetworkingError::new(format!(
                        "duplicate player_id in NPC batch: {}",
                        a.player_id
                    )));
                }
            }

            // Duplicate seat_indices within the batch.
            let mut seen_seats = std::collections::HashSet::new();
            for a in &assignments {
                if !seen_seats.insert(a.seat_index) {
                    return Err(NetworkingError::new(format!(
                        "duplicate seat_index in NPC batch: {}",
                        a.seat_index
                    )));
                }
            }

            for a in &assignments {
                // Already registered?
                if state.participants.contains_key(&a.player_id) {
                    return Err(NetworkingError::new(format!(
                        "participant {} is already registered",
                        a.player_id
                    )));
                }
                // Seat in range?
                if a.seat_index >= state.config.max_players {
                    return Err(NetworkingError::new(format!(
                        "seat {} is out of range (max_players={})",
                        a.seat_index, state.config.max_players
                    )));
                }
                // Seat available?
                if state
                    .seats
                    .get(a.seat_index as usize)
                    .is_some_and(|s| s.occupancy == crate::domain::SeatOccupancyState::Occupied)
                {
                    return Err(NetworkingError::new(format!(
                        "seat {} is already occupied",
                        a.seat_index
                    )));
                }
            }

            // --- Apply all ---

            for a in &assignments {
                state.participants.insert(
                    a.player_id.clone(),
                    ParticipantRegistryEntry {
                        identity: PlayerIdentity {
                            player_id: a.player_id.clone(),
                            display_name: a.display_name.clone(),
                            signing_public_key: format!("npc-signing-{}", a.player_id),
                            encryption_public_key: format!("npc-encryption-{}", a.player_id),
                            signing_key_fingerprint: format!("npc-fingerprint-{}", a.player_id),
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
                apply_seat_claim(state, &a.player_id, a.seat_index)?;
                apply_ready_state(state, &a.player_id, true)?;
            }

            Ok(())
        })?;

        self.sync_snapshots_to_clients()?;
        Ok(player_ids)
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
                let mut merged = next_state;
                merge_networking_state(&authoritative_state, &mut merged);
                *authoritative_state = merged;
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
