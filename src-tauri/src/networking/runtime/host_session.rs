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

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_host_client_session(
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
