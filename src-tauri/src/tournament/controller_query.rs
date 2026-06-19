use crate::domain::{
    HandParticipationState, ParticipantState, PlacementEntry, SeatMarker, SeatOccupancyState,
    TournamentPhase,
};

use super::*;

impl TournamentController {
    pub(crate) fn complete_tournament(&mut self) {
        self.state.phase = TournamentPhase::Complete;
        self.state.current_hand = None;
        self.intermission_deadline_ms = None;
        self.active_hand = None;
        self.clear_markers();

        if let Some((seat_index, winner_player_id)) = self.active_player_seats().first().cloned() {
            if !self
                .state
                .placements
                .iter()
                .any(|entry| entry.player_id == winner_player_id)
            {
                self.state.placements.push(PlacementEntry {
                    player_id: winner_player_id.clone(),
                    place: 1,
                    busted_at_hand_number: None,
                });
            }
            if let Some(seat) = self.state.seats.get_mut(seat_index as usize) {
                seat.marker = Some(SeatMarker::Dealer);
            }
        }
        self.sort_placements();
    }

    pub(crate) fn advance_blind_levels_if_due(&mut self, now_ms: u64) {
        while self
            .next_blind_deadline_ms
            .is_some_and(|deadline_ms| now_ms >= deadline_ms)
            && self.state.blind_level_index + 1 < self.state.blind_schedule.levels.len()
        {
            let previous_deadline = self.next_blind_deadline_ms.unwrap_or(now_ms);
            self.state.blind_level_index += 1;
            self.next_blind_deadline_ms = Some(
                previous_deadline + self.current_blind_level().duration_seconds as u64 * 1_000,
            );
        }

        if self.state.blind_level_index + 1 >= self.state.blind_schedule.levels.len()
            && self
                .next_blind_deadline_ms
                .is_some_and(|deadline_ms| now_ms >= deadline_ms)
        {
            self.next_blind_deadline_ms = None;
        }
    }

    pub(crate) fn reset_turn_cursor(
        &mut self,
        first_actor_seat_index: u8,
    ) -> Result<(), TournamentError> {
        let ordered_player_ids = self.player_order_starting_with(
            first_actor_seat_index,
            &self.active_player_seats_in_current_hand(),
        );
        if let Some(previous_player_id) = ordered_player_ids.last().cloned() {
            let runtime = self.active_hand.as_mut().ok_or_else(|| {
                TournamentError::new("missing hand runtime while resetting cursor")
            })?;
            runtime.last_actor_id = Some(previous_player_id);
        }
        Ok(())
    }

    pub(crate) fn next_actor_id(&self) -> Result<Option<String>, TournamentError> {
        let ordered_player_ids = self
            .active_player_seats_in_current_hand()
            .into_iter()
            .map(|(_, player_id)| player_id)
            .collect::<Vec<_>>();
        if ordered_player_ids.is_empty() {
            return Ok(None);
        }

        let last_actor_id = self
            .active_hand
            .as_ref()
            .and_then(|runtime| runtime.last_actor_id.as_ref())
            .cloned();
        let start_index = last_actor_id
            .as_ref()
            .and_then(|player_id| {
                ordered_player_ids
                    .iter()
                    .position(|candidate| candidate == player_id)
            })
            .map_or(0, |index| index + 1);

        for offset in 0..ordered_player_ids.len() {
            let candidate = &ordered_player_ids[(start_index + offset) % ordered_player_ids.len()];
            if self.player_needs_action(candidate)? {
                return Ok(Some(candidate.clone()));
            }
        }

        Ok(None)
    }

    pub(crate) fn player_needs_action(&self, player_id: &str) -> Result<bool, TournamentError> {
        if !matches!(
            self.participation(player_id),
            Some(HandParticipationState::Active)
        ) {
            return Ok(false);
        }

        let current_bet = self
            .state
            .current_hand
            .as_ref()
            .map(|hand| hand.betting_round.current_bet)
            .unwrap_or_default();
        let street_contribution = self.street_contribution(player_id)?;
        let acted = self
            .active_hand
            .as_ref()
            .is_some_and(|runtime| runtime.acted_since_last_full_raise.contains(player_id));

        Ok(street_contribution < current_bet || !acted)
    }

    pub(crate) fn player_may_raise(&self, player_id: &str) -> bool {
        matches!(
            self.participation(player_id),
            Some(HandParticipationState::Active)
        ) && !self
            .active_hand
            .as_ref()
            .is_some_and(|runtime| runtime.acted_since_last_full_raise.contains(player_id))
    }

    pub(crate) fn players_who_can_act(&self) -> Vec<String> {
        self.active_player_ids_in_current_hand()
            .into_iter()
            .filter(|player_id| self.player_needs_action(player_id).unwrap_or(false))
            .collect()
    }

    pub(crate) fn remaining_contenders(&self) -> Vec<String> {
        self.active_player_ids_in_current_hand()
            .into_iter()
            .filter(|player_id| {
                matches!(
                    self.participation(player_id),
                    Some(HandParticipationState::Active | HandParticipationState::AllIn)
                )
            })
            .collect()
    }

    pub(crate) fn active_player_seats(&self) -> Vec<(u8, String)> {
        self.state
            .seats
            .iter()
            .filter_map(|seat| {
                let player_id = seat.participant_id.clone()?;
                let participant = self.state.participants.get(&player_id)?;
                let stack = seat.chip_count.unwrap_or(0);
                (seat.occupancy == SeatOccupancyState::Occupied
                    && participant.state != ParticipantState::EliminatedObserver
                    && stack > 0)
                    .then_some((seat.seat_index, player_id))
            })
            .collect()
    }

    pub(crate) fn active_player_seats_in_current_hand(&self) -> Vec<(u8, String)> {
        self.state
            .current_hand
            .as_ref()
            .map(|hand| {
                hand.participation_by_player_id
                    .keys()
                    .filter_map(|player_id| {
                        let seat_index = self
                            .state
                            .participants
                            .get(player_id)
                            .and_then(|participant| participant.seat_index)?;
                        let seat = self.state.seats.get(seat_index as usize)?;
                        (seat.occupancy == SeatOccupancyState::Occupied)
                            .then_some((seat_index, player_id.clone()))
                    })
                    .collect()
            })
            .unwrap_or_else(|| self.active_player_seats())
    }

    pub(crate) fn active_player_ids_in_current_hand(&self) -> Vec<String> {
        self.active_player_seats_in_current_hand()
            .into_iter()
            .map(|(_, player_id)| player_id)
            .collect()
    }

    pub(crate) fn competitor_count(&self) -> u8 {
        self.state
            .participants
            .values()
            .filter(|participant| participant.seat_index.is_some())
            .count() as u8
    }

    pub(crate) fn player_order_starting_with(
        &self,
        first_seat_index: u8,
        active_players: &[(u8, String)],
    ) -> Vec<String> {
        if active_players.is_empty() {
            return Vec::new();
        }

        let mut ordered_players = active_players.to_vec();
        ordered_players.sort_by_key(|(seat_index, _)| *seat_index);
        let start_index = ordered_players
            .iter()
            .position(|(seat_index, _)| *seat_index == first_seat_index)
            .unwrap_or(0);
        ordered_players.rotate_left(start_index);
        ordered_players
            .into_iter()
            .map(|(_, player_id)| player_id)
            .collect()
    }

    pub(crate) fn next_dealer_seat_index(
        &self,
        active_players: &[(u8, String)],
    ) -> Result<u8, TournamentError> {
        if let Some(previous_dealer) = self.dealer_button_seat_index {
            self.next_active_seat_after(previous_dealer, active_players)
        } else {
            active_players
                .first()
                .map(|(seat_index, _)| *seat_index)
                .ok_or_else(|| TournamentError::new("cannot determine the first dealer seat"))
        }
    }

    pub(crate) fn next_active_seat_after(
        &self,
        seat_index: u8,
        active_players: &[(u8, String)],
    ) -> Result<u8, TournamentError> {
        let ordered_seat_indexes = active_players
            .iter()
            .map(|(candidate_seat_index, _)| *candidate_seat_index)
            .collect::<Vec<_>>();
        let next_seat = ordered_seat_indexes
            .iter()
            .copied()
            .find(|candidate| *candidate > seat_index)
            .or_else(|| ordered_seat_indexes.first().copied())
            .ok_or_else(|| {
                TournamentError::new("no active seat exists after the requested seat")
            })?;
        Ok(next_seat)
    }

    pub(crate) fn player_id_at_seat(&self, seat_index: u8) -> Result<String, TournamentError> {
        self.state
            .seats
            .get(seat_index as usize)
            .and_then(|seat| seat.participant_id.clone())
            .ok_or_else(|| TournamentError::new("seat does not currently hold a participant"))
    }

    pub(crate) fn player_seat_index(&self, player_id: &str) -> Result<u8, TournamentError> {
        self.state
            .participants
            .get(player_id)
            .and_then(|participant| participant.seat_index)
            .ok_or_else(|| TournamentError::new("participant seat index is unavailable"))
    }

    pub(crate) fn player_stack(&self, player_id: &str) -> Result<u32, TournamentError> {
        let seat_index = self.player_seat_index(player_id)?;
        self.state
            .seats
            .get(seat_index as usize)
            .and_then(|seat| seat.chip_count)
            .ok_or_else(|| TournamentError::new("participant stack is unavailable"))
    }

    pub(crate) fn set_player_stack(
        &mut self,
        player_id: &str,
        chip_count: u32,
    ) -> Result<(), TournamentError> {
        let seat_index = self.player_seat_index(player_id)?;
        let seat = self
            .state
            .seats
            .get_mut(seat_index as usize)
            .ok_or_else(|| TournamentError::new("seat index out of range while setting stack"))?;
        seat.chip_count = Some(chip_count);
        Ok(())
    }

    pub(crate) fn participation(&self, player_id: &str) -> Option<HandParticipationState> {
        self.state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.participation_by_player_id.get(player_id).copied())
    }

    pub(crate) fn set_participation(
        &mut self,
        player_id: &str,
        participation: HandParticipationState,
    ) -> Result<(), TournamentError> {
        let hand =
            self.state.current_hand.as_mut().ok_or_else(|| {
                TournamentError::new("no current hand while updating participation")
            })?;
        hand.participation_by_player_id
            .insert(player_id.to_string(), participation);
        Ok(())
    }

    pub(crate) fn street_contribution(&self, player_id: &str) -> Result<u32, TournamentError> {
        Ok(self
            .active_hand
            .as_ref()
            .and_then(|runtime| {
                runtime
                    .street_contributions_by_player_id
                    .get(player_id)
                    .copied()
            })
            .unwrap_or_default())
    }

    pub(crate) fn clear_markers(&mut self) {
        for seat in &mut self.state.seats {
            seat.marker = None;
        }
    }

    pub(crate) fn assign_markers(&mut self, dealer: u8, small_blind: u8, big_blind: u8) {
        self.clear_markers();
        if let Some(seat) = self.state.seats.get_mut(dealer as usize) {
            seat.marker = Some(SeatMarker::Dealer);
        }
        if let Some(seat) = self.state.seats.get_mut(small_blind as usize) {
            seat.marker = Some(SeatMarker::SmallBlind);
        }
        if let Some(seat) = self.state.seats.get_mut(big_blind as usize) {
            seat.marker = Some(SeatMarker::BigBlind);
        }
    }

    pub(crate) fn sort_placements(&mut self) {
        self.state.placements.sort_by_key(|entry| entry.place);
    }
}
