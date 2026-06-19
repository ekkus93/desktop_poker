use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use base64::Engine as _;

use crate::{
    crypto::{
        DefaultCryptoProvider, EncryptionKeyMaterial, ProtocolCryptoProvider, SigningKeyMaterial,
    },
    domain::{JoinPayload, TournamentPhase, TournamentState},
    networking::write_json_frame,
    protocol::{
        ActionWindowOpened, EliminationEvent, EncryptedPrivateEnvelope, HandResultCommitted,
        HandStartingEvent, PlayerActionCommitted, PrivateEnvelopeMetadata, PrivateHoleCardsEvent,
        ProtocolMessageType, SignedEnvelope, StreetRevealed, TournamentCompleteEvent,
        PROTOCOL_VERSION,
    },
};

use super::*;

pub(crate) fn append_public_event_log(
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
pub(crate) fn broadcast_public_event_to_clients<TPayload: serde::Serialize>(
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

pub(crate) fn infer_committed_action(
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
pub(crate) fn publish_runtime_transition(
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
pub(crate) fn sync_host_client_snapshots(
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
pub(crate) fn send_private_hole_cards_to_recipient(
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
