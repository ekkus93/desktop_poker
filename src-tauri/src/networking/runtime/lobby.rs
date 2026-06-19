use crate::{
    domain::{
        ConnectionState, ParticipantState, SeatOccupancyState, SeatState, TournamentPhase,
        TournamentSeatState, TournamentState,
    },
    protocol::TournamentStartedEvent,
    tournament::{RegisteredPlayer, TournamentController},
};

use super::*;

pub(crate) fn apply_seat_claim(
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

pub(crate) fn apply_ready_state(
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

pub(crate) fn ensure_lobby_phase(phase: TournamentPhase) -> Result<(), NetworkingError> {
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

pub(crate) fn recompute_lobby_phase(state: &mut TournamentState) {
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

pub(crate) fn apply_start_tournament(
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

pub(crate) fn build_tournament_started_event(state: &TournamentState) -> TournamentStartedEvent {
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

pub(crate) fn ensure_lobby_seat_map(state: &mut TournamentState) {
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
