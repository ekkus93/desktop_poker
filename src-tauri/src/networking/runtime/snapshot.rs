use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use crate::{
    crypto::{EncryptionKeyMaterial, ProtocolCryptoProvider, SigningKeyMaterial},
    domain::{
        HandCyclePhase, HandParticipationState, JoinPayload, SeatOccupancyState, StateProjector,
        TournamentState,
    },
    protocol::{
        ProtocolErrorMessage, ProtocolMessageType, RecipientHandSnapshot, RecipientSnapshotState,
        SignedEnvelope, SnapshotEvent, SnapshotParticipant, PROTOCOL_VERSION,
    },
};

use super::*;

pub(crate) fn build_snapshot_envelope(
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

pub(crate) fn build_recipient_snapshot_state(
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

pub(crate) fn public_revealed_hole_cards(
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

pub(crate) fn build_protocol_error_envelope(
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
