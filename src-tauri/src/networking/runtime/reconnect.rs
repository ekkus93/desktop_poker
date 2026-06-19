use std::sync::{Arc, Mutex};

use base64::Engine as _;
use rand_core::{OsRng, RngCore};

use crate::domain::{
    ConnectionState, ParticipantRegistryEntry, ParticipantState, TournamentPhase, TournamentState,
};

use super::*;

pub(crate) fn mark_participant_reconnect_eligible(
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

pub(crate) fn reconnect_window_ms(
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

pub(crate) fn restore_participant_after_reconnect(
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

pub(crate) fn is_reconnectable_participant(participant: &ParticipantRegistryEntry) -> bool {
    matches!(
        participant.state,
        ParticipantState::Seated
            | ParticipantState::Active
            | ParticipantState::EliminatedObserver
            | ParticipantState::Reconnecting
            | ParticipantState::Admitted
    ) && participant.state != ParticipantState::Removed
}

pub(crate) fn issue_reconnect_token() -> String {
    let mut bytes = [0_u8; 24];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub(crate) fn missing_reconnect_identity_message() -> String {
    "original reconnect identity is unavailable; v1 requires the original ephemeral signing/encryption keypair"
        .to_string()
}

pub(crate) fn is_stale_server_sequence(
    last_seen_server_sequence: Option<u64>,
    next_server_sequence: Option<u64>,
) -> bool {
    matches!(
        (last_seen_server_sequence, next_server_sequence),
        (Some(last_seen), Some(next_sequence)) if next_sequence <= last_seen
    )
}
