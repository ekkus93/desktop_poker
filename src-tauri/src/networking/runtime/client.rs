use std::{
    sync::{
        mpsc::{self},
        Arc, Mutex,
    },
    thread::{self},
    time::Duration,
};

use serde_json::Value;

use crate::{
    crypto::{
        DefaultCryptoProvider, EncryptionKeyMaterial, ProtocolCryptoProvider, SigningKeyMaterial,
    },
    networking::{read_json_frame, write_json_frame},
    protocol::{
        decode_join_payload, validate_join_payload, EncryptedPrivateEnvelope, JsonSignedEnvelope,
        PlayerActionSubmission, PrivateHoleCardsEvent, ProtocolErrorMessage, ProtocolMessageType,
        ReadyStateRequest, SeatClaimRequest, SignedEnvelope, SnapshotEvent, PROTOCOL_VERSION,
    },
};

use super::*;

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

        // The client runtime thread is intentionally detached; shutdown is coordinated
        // by the stop flag and stream lifecycle.
        thread::Builder::new()
            .name(format!("desktop-poker-client-runtime-{player_id}"))
            .spawn(move || {
            let mut protocol_warning_counts: std::collections::BTreeMap<String, u64> =
                std::collections::BTreeMap::new();
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
                                let Some(snapshot_sequence) = snapshot_server_sequence_or_warn(
                                    &sender,
                                    &mut protocol_warning_counts,
                                    &player_id,
                                    snapshot_envelope.server_sequence,
                                ) else {
                                    let _ = sender.send(ClientRuntimeEvent::SafeError {
                                        player_id: player_id.clone(),
                                        message: "reconnect snapshot missing server sequence"
                                            .to_string(),
                                    });
                                    break;
                                };
                                reconnect_token = snapshot_envelope.payload.reconnect_token.clone();
                                last_seen_server_sequence = Some(snapshot_sequence);

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
                    emit_protocol_warning(
                        &sender,
                        &mut protocol_warning_counts,
                        &player_id,
                        "incoming frame missing messageType",
                    );
                    continue;
                };

                match message_type {
                    "PRIVATE_HOLE_CARDS_EVENT" => {
                        let Ok(envelope) =
                            serde_json::from_value::<EncryptedPrivateEnvelope>(frame_value.clone())
                        else {
                            emit_protocol_warning(
                                &sender,
                                &mut protocol_warning_counts,
                                &player_id,
                                "malformed private hole-card envelope",
                            );
                            continue;
                        };

                        if envelope
                            .verify(&crypto_provider, &host_signing_public_key)
                            .is_err()
                        {
                            emit_protocol_warning(
                                &sender,
                                &mut protocol_warning_counts,
                                &player_id,
                                "private hole-card envelope signature verification failed",
                            );
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
                                    let Some(snapshot_sequence) =
                                        snapshot_server_sequence_or_warn(
                                            &sender,
                                            &mut protocol_warning_counts,
                                            &player_id,
                                            snapshot_envelope.server_sequence,
                                        )
                                    else {
                                        let _ = sender.send(ClientRuntimeEvent::SafeError {
                                            player_id: player_id.clone(),
                                            message:
                                                "resync snapshot missing server sequence; cannot continue"
                                                    .to_string(),
                                        });
                                        break;
                                    };
                                    reconnect_token =
                                        snapshot_envelope.payload.reconnect_token.clone();
                                    last_seen_server_sequence = Some(snapshot_sequence);
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
                        let associated_data = match envelope.associated_data_json() {
                            Ok(associated_data) => associated_data,
                            Err(error) => {
                                emit_protocol_warning(
                                    &sender,
                                    &mut protocol_warning_counts,
                                    &player_id,
                                    format!(
                                        "private hole-card associated-data serialization failed: {error}"
                                    ),
                                );
                                continue;
                            }
                        };
                        let Ok(plaintext) = crypto_provider.decrypt(
                            encryption_keys,
                            &host_encryption_public_key,
                            &encrypted_payload,
                            associated_data.as_slice(),
                        ) else {
                            emit_protocol_warning(
                                &sender,
                                &mut protocol_warning_counts,
                                &player_id,
                                "private hole-card payload decrypt failed",
                            );
                            continue;
                        };

                        let Ok(private_payload) =
                            serde_json::from_slice::<PrivateHoleCardsEvent>(&plaintext)
                        else {
                            emit_protocol_warning(
                                &sender,
                                &mut protocol_warning_counts,
                                &player_id,
                                "private hole-card payload JSON was invalid",
                            );
                            continue;
                        };

                        let _ = sender.send(ClientRuntimeEvent::PrivateHoleCards(private_payload));
                    }
                    "SNAPSHOT_EVENT" => {
                        let Ok(envelope) =
                            serde_json::from_value::<SignedEnvelope<SnapshotEvent>>(frame_value)
                        else {
                            emit_protocol_warning(
                                &sender,
                                &mut protocol_warning_counts,
                                &player_id,
                                "malformed snapshot envelope",
                            );
                            continue;
                        };

                        if envelope
                            .verify(&crypto_provider, &host_signing_public_key)
                            .is_err()
                        {
                            emit_protocol_warning(
                                &sender,
                                &mut protocol_warning_counts,
                                &player_id,
                                "snapshot envelope signature verification failed",
                            );
                            continue;
                        }

                        let Some(snapshot_sequence) = snapshot_server_sequence_or_warn(
                            &sender,
                            &mut protocol_warning_counts,
                            &player_id,
                            envelope.server_sequence,
                        ) else {
                            continue;
                        };
                        reconnect_token = envelope.payload.reconnect_token.clone();
                        last_seen_server_sequence = Some(snapshot_sequence);
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
                            emit_protocol_warning(
                                &sender,
                                &mut protocol_warning_counts,
                                &player_id,
                                "malformed public envelope",
                            );
                            continue;
                        };

                        if envelope
                            .verify(&crypto_provider, &host_signing_public_key)
                            .is_err()
                        {
                            emit_protocol_warning(
                                &sender,
                                &mut protocol_warning_counts,
                                &player_id,
                                "public envelope signature verification failed",
                            );
                            continue;
                        }

                        // ProtocolError is intentionally unsequenced: it must not
                        // update last_seen_server_sequence or trigger resync.
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
                            // Validate sequence before any stale check or last-seen
                            // mutation: a missing sequence must not clear last_seen.
                            let Some(server_sequence) = public_event_server_sequence_or_warn(
                                &sender,
                                &mut protocol_warning_counts,
                                &player_id,
                                envelope.server_sequence,
                            ) else {
                                continue;
                            };

                            if is_stale_server_sequence(
                                last_seen_server_sequence,
                                Some(server_sequence),
                            ) {
                                let _ = sender.send(ClientRuntimeEvent::ResyncRequested {
                                    player_id: player_id.clone(),
                                    last_seen_server_sequence: last_seen_server_sequence
                                        .unwrap_or(0),
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
                                        let Some(snapshot_sequence) =
                                            snapshot_server_sequence_or_warn(
                                                &sender,
                                                &mut protocol_warning_counts,
                                                &player_id,
                                                snapshot_envelope.server_sequence,
                                            )
                                        else {
                                            let _ = sender.send(ClientRuntimeEvent::SafeError {
                                                player_id: player_id.clone(),
                                                message:
                                                    "resync snapshot missing server sequence; cannot continue"
                                                        .to_string(),
                                            });
                                            break;
                                        };
                                        reconnect_token =
                                            snapshot_envelope.payload.reconnect_token.clone();
                                        last_seen_server_sequence = Some(snapshot_sequence);
                                        if let Some(next_host_encryption_public_key) =
                                            snapshot_envelope
                                                .payload
                                                .host_encryption_public_key
                                                .clone()
                                        {
                                            host_encryption_public_key =
                                                next_host_encryption_public_key;
                                        }
                                        let _ = sender.send(ClientRuntimeEvent::Snapshot(
                                            Box::new(snapshot_envelope.payload),
                                        ));
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

                            last_seen_server_sequence = Some(server_sequence);
                            let _ = sender.send(ClientRuntimeEvent::PublicEvent {
                                message_type: envelope.message_type,
                                server_sequence,
                                payload: envelope.payload,
                            });
                        }
                    }
                }
            }

            let _ = sender.send(ClientRuntimeEvent::Disconnected { player_id });
        })
        .map_err(|error| {
            NetworkingError::new(format!(
                "failed to spawn client runtime thread: {error}"
            ))
        })?;

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

fn emit_protocol_warning(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    reason: impl Into<String>,
) {
    let reason = reason.into();
    let count = counts.entry(reason.clone()).or_insert(0);
    *count += 1;
    if *count == 1 || count.is_power_of_two() {
        let _ = sender.send(ClientRuntimeEvent::ProtocolWarning {
            player_id: player_id.to_string(),
            reason,
            count: *count,
        });
    }
}

fn required_server_sequence_or_warn(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    server_sequence: Option<u64>,
    reason: &'static str,
) -> Option<u64> {
    match server_sequence {
        Some(sequence) => Some(sequence),
        None => {
            emit_protocol_warning(sender, counts, player_id, reason);
            None
        }
    }
}

fn public_event_server_sequence_or_warn(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    server_sequence: Option<u64>,
) -> Option<u64> {
    required_server_sequence_or_warn(
        sender,
        counts,
        player_id,
        server_sequence,
        "public event envelope missing server sequence",
    )
}

fn snapshot_server_sequence_or_warn(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    server_sequence: Option<u64>,
) -> Option<u64> {
    required_server_sequence_or_warn(
        sender,
        counts,
        player_id,
        server_sequence,
        "snapshot envelope missing server sequence",
    )
}

// Test-only helper that mirrors the production sequencing order without the
// full resync path, so tests can assert last-seen preservation in isolation.
#[cfg(test)]
fn validate_live_public_event_sequence(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    last_seen_server_sequence: &mut Option<u64>,
    envelope_sequence: Option<u64>,
) -> Option<u64> {
    let sequence =
        public_event_server_sequence_or_warn(sender, counts, player_id, envelope_sequence)?;
    if is_stale_server_sequence(*last_seen_server_sequence, Some(sequence)) {
        return None;
    }
    *last_seen_server_sequence = Some(sequence);
    Some(sequence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_public_event_server_sequence_emits_warning_and_returns_none() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut warning_counts = std::collections::BTreeMap::new();

        let result =
            public_event_server_sequence_or_warn(&sender, &mut warning_counts, "player-1", None);

        assert_eq!(result, None);

        let warning = receiver.try_recv().expect("expected protocol warning");
        assert!(
            matches!(
                warning,
                ClientRuntimeEvent::ProtocolWarning {
                    ref player_id,
                    ref reason,
                    count
                } if player_id == "player-1"
                    && reason.contains("missing server sequence")
                    && count == 1
            ),
            "unexpected warning event: {warning:?}"
        );
    }

    #[test]
    fn present_public_event_server_sequence_returns_sequence_without_warning() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut warning_counts = std::collections::BTreeMap::new();

        let result = public_event_server_sequence_or_warn(
            &sender,
            &mut warning_counts,
            "player-1",
            Some(42),
        );

        assert_eq!(result, Some(42));
        assert!(receiver.try_recv().is_err(), "no warning should be emitted");
    }

    #[test]
    fn missing_public_event_sequence_warns_and_preserves_last_seen_sequence() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut warning_counts = std::collections::BTreeMap::new();
        let mut last_seen_server_sequence = Some(10u64);

        let result = validate_live_public_event_sequence(
            &sender,
            &mut warning_counts,
            "player-1",
            &mut last_seen_server_sequence,
            None,
        );

        assert_eq!(result, None);
        assert_eq!(
            last_seen_server_sequence,
            Some(10),
            "last_seen_server_sequence must not be cleared by a malformed frame"
        );

        let warning = receiver.try_recv().expect("expected protocol warning");
        assert!(
            matches!(
                warning,
                ClientRuntimeEvent::ProtocolWarning { ref reason, .. }
                    if reason.contains("missing server sequence")
            ),
            "unexpected event: {warning:?}"
        );
        assert!(
            receiver.try_recv().is_err(),
            "malformed public event should not emit additional events"
        );
    }

    #[test]
    fn present_public_event_sequence_updates_last_seen_sequence() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut warning_counts = std::collections::BTreeMap::new();
        let mut last_seen_server_sequence = Some(10u64);

        let result = validate_live_public_event_sequence(
            &sender,
            &mut warning_counts,
            "player-1",
            &mut last_seen_server_sequence,
            Some(11),
        );

        assert_eq!(result, Some(11));
        assert_eq!(last_seen_server_sequence, Some(11));
        assert!(
            receiver.try_recv().is_err(),
            "valid sequence should not warn"
        );
    }

    #[test]
    fn missing_snapshot_sequence_warns_and_preserves_last_seen_sequence() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut warning_counts = std::collections::BTreeMap::new();
        let mut last_seen_server_sequence = Some(42u64);

        let sequence =
            snapshot_server_sequence_or_warn(&sender, &mut warning_counts, "player-1", None);

        // Guard mirrors production: only update last_seen if Some.
        if let Some(s) = sequence {
            last_seen_server_sequence = Some(s);
        }

        assert_eq!(sequence, None);
        assert_eq!(
            last_seen_server_sequence,
            Some(42),
            "last_seen_server_sequence must not be cleared by a malformed snapshot"
        );

        let warning = receiver.try_recv().expect("expected protocol warning");
        assert!(
            matches!(
                warning,
                ClientRuntimeEvent::ProtocolWarning { ref reason, .. }
                    if reason.contains("snapshot envelope missing server sequence")
            ),
            "unexpected event: {warning:?}"
        );
        assert!(
            receiver.try_recv().is_err(),
            "malformed snapshot should not emit additional events"
        );
    }

    #[test]
    fn present_snapshot_sequence_updates_last_seen_sequence_after_validation() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut warning_counts = std::collections::BTreeMap::new();
        let mut last_seen_server_sequence = Some(42u64);

        let sequence =
            snapshot_server_sequence_or_warn(&sender, &mut warning_counts, "player-1", Some(43));

        if let Some(s) = sequence {
            last_seen_server_sequence = Some(s);
        }

        assert_eq!(sequence, Some(43));
        assert_eq!(last_seen_server_sequence, Some(43));
        assert!(
            receiver.try_recv().is_err(),
            "valid sequence should not warn"
        );
    }
}
