use std::collections::BTreeMap;

use crate::{
    domain::{
        ActionType, HandCyclePhase, HandParticipationState, HandResult, ParticipantState,
        PlacementEntry, StreetPhase, TournamentSeatState,
    },
    engine::{evaluate_best_holdem_hand, settle_showdown, HandCategory, HandStrength},
};

use super::*;

impl TournamentController {
    pub(crate) fn apply_action(
        &mut self,
        player_id: String,
        action_type: ActionType,
        raise_to_amount: Option<u32>,
        now_ms: u64,
    ) -> Result<(), TournamentError> {
        let window = self
            .state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .cloned()
            .ok_or_else(|| TournamentError::new("no action window is currently open"))?;

        if !window.legal_actions.contains(&action_type) {
            return Err(TournamentError::new(
                "action rejected: not legal for current action window",
            ));
        }

        if window.player_id != player_id {
            return Err(TournamentError::new(
                "action rejected: wrong acting participant",
            ));
        }

        if let Some(hand) = self.state.current_hand.as_mut() {
            hand.action_window = None;
        }

        let street_contribution = self.street_contribution(&player_id)?;
        let current_bet = self
            .state
            .current_hand
            .as_ref()
            .map(|hand| hand.betting_round.current_bet)
            .unwrap_or_default();
        let player_stack = self.player_stack(&player_id)?;
        let to_call = current_bet.saturating_sub(street_contribution);
        let min_full_raise_to = if current_bet == 0 {
            self.active_hand
                .as_ref()
                .map(|runtime| runtime.full_raise_increment)
                .unwrap_or(self.current_blind_level().big_blind)
        } else {
            current_bet
                + self
                    .active_hand
                    .as_ref()
                    .map(|runtime| runtime.full_raise_increment)
                    .unwrap_or(self.current_blind_level().big_blind)
        };

        match action_type {
            ActionType::Fold => {
                self.set_participation(&player_id, HandParticipationState::Folded)?;
                self.record_non_reopening_action(&player_id)?;
            }
            ActionType::Check => {
                if to_call != 0 {
                    return Err(TournamentError::new(
                        "cannot check when chips are required to call",
                    ));
                }
                self.record_non_reopening_action(&player_id)?;
            }
            ActionType::Call => {
                if player_stack <= to_call {
                    return Err(TournamentError::new(
                        "call is not legal when the player can only move all-in",
                    ));
                }
                self.commit_total_wager(&player_id, to_call, true)?;
                self.record_non_reopening_action(&player_id)?;
            }
            ActionType::Bet | ActionType::Raise => {
                let raise_to_amount = raise_to_amount.ok_or_else(|| {
                    TournamentError::new("bet and raise actions must include raise_to_amount")
                })?;
                if raise_to_amount < min_full_raise_to {
                    return Err(TournamentError::new(
                        "raise must satisfy minimum full raise sizing",
                    ));
                }
                let additional_amount = raise_to_amount.saturating_sub(street_contribution);
                if additional_amount > player_stack {
                    return Err(TournamentError::new("raise exceeds remaining stack"));
                }
                let prior_bet = current_bet;
                self.commit_total_wager(&player_id, additional_amount, true)?;
                self.handle_full_raise(&player_id, raise_to_amount - prior_bet)?;
            }
            ActionType::AllIn => {
                let all_in_to = street_contribution + player_stack;
                if player_stack == 0 {
                    return Err(TournamentError::new("player is already all-in"));
                }
                self.commit_total_wager(&player_id, player_stack, true)?;
                if all_in_to > current_bet
                    && all_in_to >= min_full_raise_to
                    && self.player_may_raise(&player_id)
                {
                    self.handle_full_raise(&player_id, all_in_to - current_bet)?;
                } else {
                    self.record_non_reopening_action(&player_id)?;
                }
                self.set_participation(&player_id, HandParticipationState::AllIn)?;
            }
        }

        if self.player_stack(&player_id)? == 0
            && self.participation(&player_id) == Some(HandParticipationState::Active)
        {
            self.set_participation(&player_id, HandParticipationState::AllIn)?;
        }

        if let Some(runtime) = self.active_hand.as_mut() {
            runtime.last_actor_id = Some(player_id);
        }

        self.refresh_betting_round_bounds();
        self.refresh_action_window(now_ms)?;
        Ok(())
    }

    pub(crate) fn record_non_reopening_action(
        &mut self,
        player_id: &str,
    ) -> Result<(), TournamentError> {
        let runtime = self
            .active_hand
            .as_mut()
            .ok_or_else(|| TournamentError::new("hand runtime missing while committing action"))?;
        runtime
            .acted_since_last_full_raise
            .insert(player_id.to_string());
        Ok(())
    }

    pub(crate) fn handle_full_raise(
        &mut self,
        player_id: &str,
        raise_increment: u32,
    ) -> Result<(), TournamentError> {
        let runtime = self
            .active_hand
            .as_mut()
            .ok_or_else(|| TournamentError::new("hand runtime missing while handling raise"))?;
        runtime.full_raise_increment = raise_increment;
        runtime.acted_since_last_full_raise.clear();
        runtime
            .acted_since_last_full_raise
            .insert(player_id.to_string());
        Ok(())
    }

    pub(crate) fn commit_timeout(&mut self, now_ms: u64) -> Result<(), TournamentError> {
        let action_window = self
            .state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .cloned()
            .ok_or_else(|| TournamentError::new("no action window available to timeout"))?;

        let timeout_action = if action_window.legal_actions.contains(&ActionType::Check) {
            ActionType::Check
        } else {
            ActionType::Fold
        };
        self.apply_action(action_window.player_id, timeout_action, None, now_ms)
    }

    pub(crate) fn advance_street_or_showdown(
        &mut self,
        now_ms: u64,
    ) -> Result<(), TournamentError> {
        match self
            .state
            .current_hand
            .as_ref()
            .map(|hand| hand.street)
            .ok_or_else(|| TournamentError::new("cannot advance a missing hand"))?
        {
            StreetPhase::Preflop => self.advance_to_street(StreetPhase::Flop, now_ms),
            StreetPhase::Flop => self.advance_to_street(StreetPhase::Turn, now_ms),
            StreetPhase::Turn => self.advance_to_street(StreetPhase::River, now_ms),
            StreetPhase::River | StreetPhase::Showdown => self.settle_current_hand(now_ms),
        }
    }

    pub(crate) fn advance_to_street(
        &mut self,
        next_street: StreetPhase,
        now_ms: u64,
    ) -> Result<(), TournamentError> {
        if let Some(hand) = self.state.current_hand.as_mut() {
            hand.cycle_phase = HandCyclePhase::StreetReveal;
        }

        match next_street {
            StreetPhase::Flop => {
                let cards = self
                    .active_hand
                    .as_mut()
                    .ok_or_else(|| TournamentError::new("missing hand runtime deck"))?
                    .deck
                    .reveal_flop()?;
                if let Some(hand) = self.state.current_hand.as_mut() {
                    hand.board_cards.extend(cards);
                }
            }
            StreetPhase::Turn => {
                let card = self
                    .active_hand
                    .as_mut()
                    .ok_or_else(|| TournamentError::new("missing hand runtime deck"))?
                    .deck
                    .reveal_turn()?;
                if let Some(hand) = self.state.current_hand.as_mut() {
                    hand.board_cards.push(card);
                }
            }
            StreetPhase::River => {
                let card = self
                    .active_hand
                    .as_mut()
                    .ok_or_else(|| TournamentError::new("missing hand runtime deck"))?
                    .deck
                    .reveal_river()?;
                if let Some(hand) = self.state.current_hand.as_mut() {
                    hand.board_cards.push(card);
                }
            }
            StreetPhase::Preflop | StreetPhase::Showdown => {}
        }

        let active_player_ids = self.active_player_ids_in_current_hand();
        let big_blind = self.current_blind_level().big_blind;
        let runtime = self
            .active_hand
            .as_mut()
            .ok_or_else(|| TournamentError::new("missing hand runtime while advancing street"))?;
        runtime.street_contributions_by_player_id = active_player_ids
            .iter()
            .map(|player_id| (player_id.clone(), 0_u32))
            .collect();
        runtime.acted_since_last_full_raise.clear();
        runtime.full_raise_increment = big_blind;

        if let Some(hand) = self.state.current_hand.as_mut() {
            hand.street = next_street;
            hand.cycle_phase = HandCyclePhase::AwaitingAction;
            hand.betting_round.street = next_street;
            hand.betting_round.current_bet = 0;
            hand.action_window = None;
        }
        self.refresh_betting_round_bounds();

        let first_actor_seat_index = self.next_active_seat_after(
            self.state
                .current_hand
                .as_ref()
                .map(|hand| hand.dealer_seat_index)
                .ok_or_else(|| TournamentError::new("current hand missing dealer seat"))?,
            &self.active_player_seats_in_current_hand(),
        )?;
        self.reset_turn_cursor(first_actor_seat_index)?;
        self.refresh_action_window(now_ms)
    }

    pub(crate) fn reveal_remaining_board(&mut self) -> Result<(), TournamentError> {
        while self
            .state
            .current_hand
            .as_ref()
            .is_some_and(|hand| hand.board_cards.len() < 5)
        {
            let board_len = self
                .state
                .current_hand
                .as_ref()
                .map(|hand| hand.board_cards.len())
                .unwrap_or_default();
            match board_len {
                0 => {
                    let cards = self
                        .active_hand
                        .as_mut()
                        .ok_or_else(|| TournamentError::new("missing hand runtime deck"))?
                        .deck
                        .reveal_flop()?;
                    if let Some(hand) = self.state.current_hand.as_mut() {
                        hand.board_cards.extend(cards);
                    }
                }
                3 => {
                    let card = self
                        .active_hand
                        .as_mut()
                        .ok_or_else(|| TournamentError::new("missing hand runtime deck"))?
                        .deck
                        .reveal_turn()?;
                    if let Some(hand) = self.state.current_hand.as_mut() {
                        hand.board_cards.push(card);
                    }
                }
                4 => {
                    let card = self
                        .active_hand
                        .as_mut()
                        .ok_or_else(|| TournamentError::new("missing hand runtime deck"))?
                        .deck
                        .reveal_river()?;
                    if let Some(hand) = self.state.current_hand.as_mut() {
                        hand.board_cards.push(card);
                    }
                }
                _ => break,
            }
        }

        if let Some(hand) = self.state.current_hand.as_mut() {
            hand.street = StreetPhase::Showdown;
            hand.cycle_phase = HandCyclePhase::Showdown;
            hand.action_window = None;
            hand.betting_round.street = StreetPhase::Showdown;
        }
        Ok(())
    }

    pub(crate) fn settle_current_hand(&mut self, now_ms: u64) -> Result<(), TournamentError> {
        let hand = self
            .state
            .current_hand
            .clone()
            .ok_or_else(|| TournamentError::new("cannot settle a missing hand"))?;
        let contenders = self.remaining_contenders();
        let total_contributions = hand.betting_round.contributions_by_player_id.clone();
        let odd_chip_order = self.player_order_starting_with(
            self.next_active_seat_after(
                hand.dealer_seat_index,
                &self.active_player_seats_in_current_hand(),
            )?,
            &self.active_player_seats_in_current_hand(),
        );

        let mut revealed_hands_by_player_id = BTreeMap::new();
        let mut hand_strengths_by_player_id = BTreeMap::new();

        if contenders.len() > 1 {
            let mut board_cards = hand.board_cards.clone();
            while board_cards.len() < 5 {
                match board_cards.len() {
                    0 => board_cards.extend(
                        self.active_hand
                            .as_mut()
                            .ok_or_else(|| {
                                TournamentError::new("missing deck while settling hand")
                            })?
                            .deck
                            .reveal_flop()?,
                    ),
                    3 => board_cards.push(
                        self.active_hand
                            .as_mut()
                            .ok_or_else(|| {
                                TournamentError::new("missing deck while settling hand")
                            })?
                            .deck
                            .reveal_turn()?,
                    ),
                    4 => board_cards.push(
                        self.active_hand
                            .as_mut()
                            .ok_or_else(|| {
                                TournamentError::new("missing deck while settling hand")
                            })?
                            .deck
                            .reveal_river()?,
                    ),
                    _ => break,
                }
            }
            if let Some(current_hand) = self.state.current_hand.as_mut() {
                current_hand.board_cards = board_cards.clone();
                current_hand.street = StreetPhase::Showdown;
                current_hand.cycle_phase = HandCyclePhase::Showdown;
            }

            for player_id in &contenders {
                let hole_cards = hand
                    .hole_cards_by_player_id
                    .get(player_id)
                    .cloned()
                    .ok_or_else(|| {
                        TournamentError::new("missing hole cards for showdown contender")
                    })?;
                let strength = evaluate_best_holdem_hand(&hole_cards, &board_cards)?;
                revealed_hands_by_player_id.insert(player_id.clone(), hole_cards);
                hand_strengths_by_player_id.insert(player_id.clone(), strength);
            }
        } else {
            let sole_winner = contenders
                .first()
                .cloned()
                .ok_or_else(|| TournamentError::new("cannot settle a hand without a winner"))?;
            hand_strengths_by_player_id.insert(
                sole_winner,
                HandStrength {
                    category: HandCategory::HighCard,
                    key_ranks: [0, 0, 0, 0, 0],
                },
            );
        }

        let settlement = settle_showdown(
            &total_contributions,
            &hand_strengths_by_player_id,
            &odd_chip_order,
        )?;
        for (player_id, payout) in &settlement.payouts_by_player_id {
            let stack = self.player_stack(player_id)?;
            self.set_player_stack(player_id, stack + payout)?;
        }

        let eliminated_player_ids = self.process_eliminations(hand.hand_number);
        let final_stack_by_player_id = self
            .state
            .seats
            .iter()
            .filter_map(|seat| {
                seat.participant_id
                    .as_ref()
                    .zip(seat.chip_count)
                    .map(|(player_id, chip_count)| (player_id.clone(), chip_count))
            })
            .collect();
        let result = HandResult {
            hand_number: hand.hand_number,
            winning_player_ids: settlement.winning_player_ids.clone(),
            pot_summaries: settlement.pot_summaries,
            board_cards: self
                .state
                .current_hand
                .as_ref()
                .map(|current_hand| current_hand.board_cards.clone())
                .unwrap_or_else(|| hand.board_cards.clone()),
            revealed_hands_by_player_id,
            eliminated_player_ids: eliminated_player_ids.clone(),
            final_stack_by_player_id,
        };
        self.state.hand_results.push(result);
        self.active_hand = None;

        if self.active_player_seats().len() <= 1 {
            self.complete_tournament();
        } else if let Some(current_hand) = self.state.current_hand.as_mut() {
            current_hand.cycle_phase = HandCyclePhase::BetweenHands;
            current_hand.action_window = None;
            self.intermission_deadline_ms = Some(now_ms + BETWEEN_HANDS_DELAY_MS);
        }

        self.sort_placements();
        Ok(())
    }

    pub(crate) fn process_eliminations(&mut self, hand_number: u32) -> Vec<String> {
        let mut eliminated = self
            .active_player_seats_in_current_hand()
            .into_iter()
            .filter_map(|(_, player_id)| {
                (self.player_stack(&player_id).ok() == Some(0)).then_some(player_id)
            })
            .collect::<Vec<_>>();
        eliminated.sort_by_key(|player_id| self.player_seat_index(player_id).unwrap_or(u8::MAX));

        let mut next_place = self
            .competitor_count()
            .saturating_sub(self.state.placements.len() as u8);
        for player_id in &eliminated {
            if let Some(participant) = self.state.participants.get_mut(player_id) {
                participant.state = ParticipantState::EliminatedObserver;
            }
            if let Ok(seat_index) = self.player_seat_index(player_id) {
                if let Some(seat) = self.state.seats.get_mut(seat_index as usize) {
                    seat.tournament_state = TournamentSeatState::EliminatedObserver;
                    seat.chip_count = Some(0);
                    seat.marker = None;
                }
            }
            self.state.placements.push(PlacementEntry {
                player_id: player_id.clone(),
                place: next_place,
                busted_at_hand_number: Some(hand_number),
            });
            next_place = next_place.saturating_sub(1);
        }

        eliminated
    }
}
