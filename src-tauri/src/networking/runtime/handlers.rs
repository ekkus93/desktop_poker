use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

use base64::Engine as _;

use crate::{
    crypto::{key_fingerprint, EncryptionKeyMaterial, ProtocolCryptoProvider, SigningKeyMaterial},
    domain::{
        counted_capacity, ConnectionState, JoinPayload, ParticipantRegistryEntry, ParticipantState,
        PlayerIdentity, TournamentPhase, TournamentState,
    },
    protocol::{
        JoinTournamentRequest, JsonSignedEnvelope, PlayerActionSubmission, ProtocolMessageType,
        ReadyStateRequest, ReconnectTournamentRequest, ResyncRequest, SeatClaimRequest,
        SignedEnvelope, SnapshotEvent,
    },
    tournament::{ActionRequest, ActionSubmissionOutcome, TournamentController},
};

use super::*;

pub(crate) fn handle_initial_client_request(
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

pub(crate) fn handle_join_request(
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

pub(crate) fn ensure_joinable_phase(phase: TournamentPhase) -> Result<(), NetworkingError> {
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

pub(crate) fn ensure_join_capacity(state: &TournamentState) -> Result<(), NetworkingError> {
    let participant_count = counted_capacity(&state.participants);
    if participant_count >= state.config.max_players as usize {
        return Err(NetworkingError::new(format!(
            "table is full: {participant_count} participants already admitted for {} seats",
            state.config.max_players
        )));
    }

    Ok(())
}

pub(crate) fn handle_reconnect_request(
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
            return Err(NetworkingError::with_code(
                crate::protocol::ERROR_CODE_RECONNECT_ALREADY_CONNECTED,
                "participant is already connected",
            ));
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

pub(crate) fn handle_resync_request(
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

pub(crate) fn handle_seat_claim_request(
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

pub(crate) fn handle_ready_state_request(
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

#[derive(Debug)]
pub(crate) enum RemoteActionSubmissionOutcome {
    Committed {
        previous_state: TournamentState,
        after_state: TournamentState,
    },
    RejectedNoStateChange {
        error: NetworkingError,
    },
    TimeoutAdvancedThenRejected {
        previous_state: TournamentState,
        after_state: TournamentState,
        error: NetworkingError,
    },
}

pub(crate) fn commit_remote_action_state(
    controller: &mut TournamentController,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    next_state: TournamentState,
    rollback_controller: TournamentController,
) -> Result<(TournamentState, TournamentState), NetworkingError> {
    match commit_runtime_state(authoritative_state, next_state) {
        Ok(states) => Ok(states),
        Err(error) => {
            *controller = rollback_controller;
            Err(error)
        }
    }
}

pub(crate) fn handle_action_submission_request(
    crypto_provider: &impl ProtocolCryptoProvider,
    request_envelope: JsonSignedEnvelope,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    tournament_runtime: &Arc<Mutex<Option<TournamentController>>>,
) -> Result<RemoteActionSubmissionOutcome, NetworkingError> {
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

    let before_state = authoritative_state
        .lock()
        .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))?
        .clone();
    let action_request = ActionRequest {
        player_id: request_envelope.sender_id,
        action_window_id: request.action_window_id,
        action_type: request.action_type,
        raise_to_amount: request.raise_to_amount,
    };

    let mut runtime = tournament_runtime
        .lock()
        .map_err(|_| NetworkingError::new("tournament runtime lock poisoned"))?;
    let controller = runtime
        .as_mut()
        .ok_or_else(|| NetworkingError::new("live tournament runtime is unavailable"))?;
    let mut normalized_controller_state = controller.state().clone();
    merge_networking_state(&before_state, &mut normalized_controller_state);
    if normalized_controller_state != before_state {
        return Err(NetworkingError::new(
            "tournament runtime diverged from authoritative gameplay state before remote action",
        ));
    }
    let rollback_controller = controller.clone();
    let action_outcome = match controller.submit_action_with_outcome(action_request, now_epoch_ms())
    {
        Ok(outcome) => outcome,
        Err(error) => {
            *controller = rollback_controller;
            return Err(NetworkingError::new(error.to_string()));
        }
    };
    let next_state = controller.state().clone();

    let invalid_transition = match &action_outcome {
        ActionSubmissionOutcome::Committed => next_state == before_state,
        ActionSubmissionOutcome::RejectedNoStateChange { .. } => next_state != before_state,
        ActionSubmissionOutcome::TimeoutAdvancedThenRejected { .. } => next_state == before_state,
    };
    if invalid_transition {
        *controller = rollback_controller;
        return Err(NetworkingError::new(
            "remote action outcome did not match its controller state transition; mutation was rolled back",
        ));
    }

    match action_outcome {
        ActionSubmissionOutcome::Committed => {
            let (previous_state, after_state) = commit_remote_action_state(
                controller,
                authoritative_state,
                next_state,
                rollback_controller,
            )?;
            Ok(RemoteActionSubmissionOutcome::Committed {
                previous_state,
                after_state,
            })
        }
        ActionSubmissionOutcome::RejectedNoStateChange { error } => {
            Ok(RemoteActionSubmissionOutcome::RejectedNoStateChange {
                error: NetworkingError::new(error.to_string()),
            })
        }
        ActionSubmissionOutcome::TimeoutAdvancedThenRejected { error } => {
            let (previous_state, after_state) = commit_remote_action_state(
                controller,
                authoritative_state,
                next_state,
                rollback_controller,
            )?;
            Ok(RemoteActionSubmissionOutcome::TimeoutAdvancedThenRejected {
                previous_state,
                after_state,
                error: NetworkingError::new(error.to_string()),
            })
        }
    }
}
