use std::{
    collections::HashMap,
    net::TcpStream,
    sync::{atomic::AtomicU64, Arc, Mutex},
    thread::{self},
};

use crate::{
    crypto::{DefaultCryptoProvider, SigningKeyMaterial},
    domain::{JoinPayload, TournamentState},
    networking::{read_json_frame, write_json_frame},
    protocol::{JsonSignedEnvelope, ProtocolMessageType},
};

use super::*;

fn mark_reconnect_or_record_health(
    authoritative_state: &Arc<Mutex<TournamentState>>,
    runtime_health: &Arc<Mutex<HostRuntimeHealth>>,
    player_id: &str,
) {
    if let Err(error) = mark_participant_reconnect_eligible(authoritative_state, player_id) {
        update_health(runtime_health, |h| {
            h.record_reconnect_mark_error(player_id, &error);
        });
    }
}

/// Session-exit cleanup. A session whose registration was already retired or
/// replaced must not demote a newer connection for the same player (N-3).
fn disconnect_client(
    clients: &Arc<Mutex<HashMap<String, ConnectedClient>>>,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    runtime_health: &Arc<Mutex<HostRuntimeHealth>>,
    player_id: &str,
    generation: u64,
) {
    match take_client_if_current(clients, player_id, generation) {
        Ok(Some(client)) => {
            mark_reconnect_or_record_health(authoritative_state, runtime_health, player_id);
            close_client(&client);
        }
        // Already retired by a failed writer, or replaced by a newer connection.
        Ok(None) => {}
        Err(_) => {
            // Ownership cannot be checked; fail toward reconnect-eligible rather
            // than leaving a `Connected` participant without a session.
            update_health(runtime_health, |h| {
                h.record_client_registry_error();
            });
            mark_reconnect_or_record_health(authoritative_state, runtime_health, player_id);
        }
    }
}

fn record_state_lock_error(
    runtime_health: &Arc<Mutex<HostRuntimeHealth>>,
    context: impl Into<String>,
) {
    let context = context.into();
    update_health(runtime_health, |health| {
        health.state_lock_error_count += 1;
        health.record_error(context);
    });
}

/// Serialize every session-originated frame with broadcasts and snapshot sync
/// for this connection. The read loop owns a separate socket clone and never
/// holds this writer lock while waiting for input.
fn write_client_frame<T: serde::Serialize>(
    writer: &Arc<Mutex<TcpStream>>,
    payload: &T,
) -> Result<(), NetworkingError> {
    let mut stream = writer
        .lock()
        .map_err(|_| NetworkingError::new("client stream lock poisoned"))?;
    write_json_frame(&mut stream, payload)
}

#[allow(clippy::too_many_arguments)]
fn send_action_rejection_best_effort(
    writer: &Arc<Mutex<TcpStream>>,
    crypto_provider: &DefaultCryptoProvider,
    join_payload: &JoinPayload,
    server_sequence: &Arc<AtomicU64>,
    host_signing_keys: &SigningKeyMaterial,
    rejected_message_id: String,
    error: &NetworkingError,
) {
    if let Ok(envelope) = build_protocol_error_envelope(
        crypto_provider,
        join_payload,
        server_sequence,
        host_signing_keys,
        "ACTION_SUBMISSION_REJECTED",
        error.to_string(),
        Some(rejected_message_id),
    ) {
        // Best-effort rejection reply. Authoritative state and transition
        // publication have already been handled before this helper is called.
        let _ = write_client_frame(writer, &envelope);
    }
}

/// Start the session thread for a registered client. On spawn failure the
/// stream and slot guard are dropped and the error is returned so the caller
/// can retire the registration.
pub(crate) fn spawn_host_client_session(
    player_id: String,
    generation: u64,
    mut stream: TcpStream,
    writer: Arc<Mutex<TcpStream>>,
    context: HostSessionContext,
    active_client_slot_guard: ActiveClientSlotGuard,
) -> Result<(), NetworkingError> {
    let HostSessionContext {
        authoritative_state,
        tournament_runtime,
        transition_lock,
        clients,
        join_payload,
        server_sequence,
        host_signing_keys,
        host_encryption_keys,
        public_events,
        runtime_health,
    } = context;
    let runtime_health_for_error = Arc::clone(&runtime_health);
    let player_id_for_error = player_id.clone();
    if let Err(error) = thread::Builder::new()
        .name(format!("host-client-{player_id}"))
        .spawn(move || {
            let _active_client_slot_guard = active_client_slot_guard;
            let crypto_provider = DefaultCryptoProvider;

            loop {
                let next_request = read_json_frame::<JsonSignedEnvelope>(&mut stream);
                let request_envelope = match next_request {
                    Ok(request_envelope) => request_envelope,
                    Err(_) => {
                        disconnect_client(
                            &clients,
                            &authoritative_state,
                            &runtime_health,
                            &player_id,
                            generation,
                        );
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
                                    disconnect_client(
                                        &clients,
                                        &authoritative_state,
                                        &runtime_health,
                                        &player_id,
                                        generation,
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
                                    // Best-effort rejection reply; if the write fails the
                                    // client will time out and reconnect.
                                    let _ = write_client_frame(&writer, &envelope);
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
                                    disconnect_client(
                                        &clients,
                                        &authoritative_state,
                                        &runtime_health,
                                        &player_id,
                                        generation,
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
                                    // Best-effort rejection reply; if the write fails the
                                    // client will time out and reconnect.
                                    let _ = write_client_frame(&writer, &envelope);
                                }
                            }
                        }
                    }
                    ProtocolMessageType::ActionSubmissionRequest => {
                        let _transition_guard = match transition_lock.lock() {
                            Ok(guard) => guard,
                            Err(_) => {
                                record_state_lock_error(
                                    &runtime_health,
                                    "host transition lock poisoned during remote action",
                                );
                                break;
                            }
                        };
                        let rejected_message_id = request_envelope.message_id.clone();
                        let response = handle_action_submission_request(
                            &crypto_provider,
                            request_envelope,
                            &authoritative_state,
                            &tournament_runtime,
                        );

                        match response {
                            Ok(RemoteActionSubmissionOutcome::Committed {
                                previous_state,
                                after_state,
                            }) => {
                                if publish_runtime_transition(
                                    &join_payload,
                                    &authoritative_state,
                                    &previous_state,
                                    &after_state,
                                    &clients,
                                    &server_sequence,
                                    &host_signing_keys,
                                    &host_encryption_keys,
                                    &public_events,
                                )
                                .is_err()
                                {
                                    disconnect_client(
                                        &clients,
                                        &authoritative_state,
                                        &runtime_health,
                                        &player_id,
                                        generation,
                                    );
                                    break;
                                }
                            }
                            Ok(RemoteActionSubmissionOutcome::TimeoutAdvancedThenRejected {
                                previous_state,
                                after_state,
                                error,
                            }) => {
                                if publish_runtime_transition(
                                    &join_payload,
                                    &authoritative_state,
                                    &previous_state,
                                    &after_state,
                                    &clients,
                                    &server_sequence,
                                    &host_signing_keys,
                                    &host_encryption_keys,
                                    &public_events,
                                )
                                .is_err()
                                {
                                    disconnect_client(
                                        &clients,
                                        &authoritative_state,
                                        &runtime_health,
                                        &player_id,
                                        generation,
                                    );
                                    break;
                                }
                                send_action_rejection_best_effort(
                                    &writer,
                                    &crypto_provider,
                                    &join_payload,
                                    &server_sequence,
                                    &host_signing_keys,
                                    rejected_message_id,
                                    &error,
                                );
                            }
                            Ok(RemoteActionSubmissionOutcome::RejectedNoStateChange { error })
                            | Err(error) => {
                                send_action_rejection_best_effort(
                                    &writer,
                                    &crypto_provider,
                                    &join_payload,
                                    &server_sequence,
                                    &host_signing_keys,
                                    rejected_message_id,
                                    &error,
                                );
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
                                if write_client_frame(&writer, &snapshot_envelope).is_err() {
                                    disconnect_client(
                                        &clients,
                                        &authoritative_state,
                                        &runtime_health,
                                        &player_id,
                                        generation,
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
                                    // Best-effort rejection reply; if the write fails the
                                    // client will time out and reconnect.
                                    let _ = write_client_frame(&writer, &envelope);
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
                            let _ = write_client_frame(&writer, &envelope);
                        }
                    }
                }
            }
        })
    {
        update_health(&runtime_health_for_error, |h| {
            h.record_error(format!(
                "failed to spawn host-client-{player_id_for_error} thread: {error}"
            ));
        });
        Err(NetworkingError::new(format!(
            "failed to spawn host-client session: {error}"
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, net::TcpListener};

    #[test]
    fn record_state_lock_error_increments_counter_and_sets_last_error() {
        let health = Arc::new(Mutex::new(HostRuntimeHealth::default()));

        record_state_lock_error(&health, "test state lock failure");

        let h = health.lock().unwrap();
        assert_eq!(h.state_lock_error_count, 1);
        assert_eq!(h.last_error.as_deref(), Some("test state lock failure"));
    }

    #[test]
    fn concurrent_session_writes_remain_individual_frames() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let sender = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut receiver, _) = listener.accept().unwrap();
        let writer = Arc::new(Mutex::new(sender));
        let mut writers = Vec::new();

        for thread_id in 0..8_u32 {
            let writer = Arc::clone(&writer);
            writers.push(thread::spawn(move || {
                for index in 0..16_u32 {
                    write_client_frame(&writer, &(thread_id * 16 + index)).unwrap();
                }
            }));
        }

        for writer_thread in writers {
            writer_thread.join().unwrap();
        }

        let received: HashSet<u32> = (0..128)
            .map(|_| read_json_frame(&mut receiver).unwrap())
            .collect();
        assert_eq!(received.len(), 128);
        assert!(received.contains(&0));
        assert!(received.contains(&127));
    }
}
