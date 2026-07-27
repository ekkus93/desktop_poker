use std::{
    collections::HashMap,
    net::TcpStream,
    sync::{atomic::AtomicU64, Arc, Mutex},
    thread::{self},
};

use crate::{
    crypto::{DefaultCryptoProvider, EncryptionKeyMaterial, SigningKeyMaterial},
    domain::{JoinPayload, TournamentState},
    networking::{read_json_frame, write_json_frame},
    protocol::{JsonSignedEnvelope, ProtocolMessageType},
    tournament::TournamentController,
};

use super::*;

fn remove_client_or_record_health(
    clients: &Arc<Mutex<HashMap<String, ConnectedClient>>>,
    runtime_health: &Arc<Mutex<HostRuntimeHealth>>,
    player_id: &str,
) {
    match clients.lock() {
        Ok(mut connected_clients) => {
            connected_clients.remove(player_id);
        }
        Err(_) => update_health(runtime_health, |h| {
            h.record_client_registry_error();
        }),
    }
}

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

fn disconnect_client(
    clients: &Arc<Mutex<HashMap<String, ConnectedClient>>>,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    runtime_health: &Arc<Mutex<HostRuntimeHealth>>,
    player_id: &str,
) {
    remove_client_or_record_health(clients, runtime_health, player_id);
    mark_reconnect_or_record_health(authoritative_state, runtime_health, player_id);
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_host_client_session(
    player_id: String,
    mut stream: TcpStream,
    authoritative_state: Arc<Mutex<TournamentState>>,
    tournament_runtime: Arc<Mutex<Option<TournamentController>>>,
    transition_lock: Arc<Mutex<()>>,
    clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
    join_payload: JoinPayload,
    server_sequence: Arc<AtomicU64>,
    host_signing_keys: Arc<SigningKeyMaterial>,
    host_encryption_keys: Arc<Mutex<EncryptionKeyMaterial>>,
    public_events: Arc<Mutex<Vec<PublicEventLogEntry>>>,
    runtime_health: Arc<Mutex<HostRuntimeHealth>>,
) {
    let runtime_health_for_error = Arc::clone(&runtime_health);
    let player_id_for_error = player_id.clone();
    if let Err(error) = thread::Builder::new()
        .name(format!("host-client-{player_id}"))
        .spawn(move || {
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
                                    disconnect_client(
                                        &clients,
                                        &authoritative_state,
                                        &runtime_health,
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
                                    // Best-effort rejection reply; if the write fails the
                                    // client will time out and reconnect.
                                    let _ = write_json_frame(&mut stream, &envelope);
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
                                        record_state_lock_error(
                                            &runtime_health,
                                            format!(
                                                "authoritative state lock poisoned reading \
                                                 previous state for action by {player_id}: \
                                                 {error}"
                                            ),
                                        );
                                        if let Ok(envelope) = build_protocol_error_envelope(
                                            &crypto_provider,
                                            &join_payload,
                                            &server_sequence,
                                            &host_signing_keys,
                                            "ACTION_SUBMISSION_REJECTED",
                                            error.to_string(),
                                            Some(rejected_message_id.clone()),
                                        ) {
                                            // Best-effort rejection reply; if the write fails the
                                            // client will time out and reconnect.
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
                                        record_state_lock_error(
                                            &runtime_health,
                                            format!(
                                                "authoritative state lock poisoned reading \
                                                 next state after action by {player_id}"
                                            ),
                                        );
                                        disconnect_client(
                                            &clients,
                                            &authoritative_state,
                                            &runtime_health,
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
                                    disconnect_client(
                                        &clients,
                                        &authoritative_state,
                                        &runtime_health,
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
                                    // Best-effort rejection reply; if the write fails the
                                    // client will time out and reconnect.
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
                                    disconnect_client(
                                        &clients,
                                        &authoritative_state,
                                        &runtime_health,
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
                                    // Best-effort rejection reply; if the write fails the
                                    // client will time out and reconnect.
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
        })
    {
        update_health(&runtime_health_for_error, |h| {
            h.record_error(format!(
                "failed to spawn host-client-{player_id_for_error} thread: {error}"
            ));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_state_lock_error_increments_counter_and_sets_last_error() {
        let health = Arc::new(Mutex::new(HostRuntimeHealth::default()));

        record_state_lock_error(&health, "test state lock failure");

        let h = health.lock().unwrap();
        assert_eq!(h.state_lock_error_count, 1);
        assert_eq!(h.last_error.as_deref(), Some("test state lock failure"));
    }
}
