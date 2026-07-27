use std::collections::{BTreeMap, BTreeSet};

use crate::{
    domain::{
        validate_tournament_state, ActionWindow, BettingRoundState, BlindLevel, ConnectionState,
        HandCyclePhase, HandParticipationState, HandState, ParticipantRegistryEntry,
        ParticipantState, SeatOccupancyState, SeatState, StreetPhase, TournamentConfig,
        TournamentPhase, TournamentSeatState, TournamentState,
    },
    engine::{legal_actions, Deck, LegalActionContext},
};

use super::*;

impl TournamentController {
    pub fn new(
        table_id: impl Into<String>,
        session_epoch: u64,
        config: TournamentConfig,
        registered_players: Vec<RegisteredPlayer>,
    ) -> Result<Self, TournamentError> {
        if registered_players.len() < 2 {
            return Err(TournamentError::new(
                "at least two seated players are required to build a tournament",
            ));
        }

        let mut seats = (0..config.max_players)
            .map(|seat_index| SeatState {
                seat_index,
                occupancy: SeatOccupancyState::Empty,
                tournament_state: TournamentSeatState::Open,
                participant_id: None,
                display_name: None,
                chip_count: None,
                is_ready: false,
                marker: None,
            })
            .collect::<Vec<_>>();
        let mut participants = BTreeMap::new();
        let mut occupied_seats = BTreeSet::new();

        for registered_player in registered_players {
            if registered_player.seat_index >= config.max_players {
                return Err(TournamentError::new(
                    "registered player seat index is out of range",
                ));
            }
            if !occupied_seats.insert(registered_player.seat_index) {
                return Err(TournamentError::new("duplicate seat assignment detected"));
            }

            seats[registered_player.seat_index as usize] = SeatState {
                seat_index: registered_player.seat_index,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: if registered_player.is_ready {
                    TournamentSeatState::Ready
                } else {
                    TournamentSeatState::Lobby
                },
                participant_id: Some(registered_player.identity.player_id.clone()),
                display_name: Some(registered_player.identity.display_name.clone()),
                chip_count: None,
                is_ready: registered_player.is_ready,
                marker: None,
            };

            participants.insert(
                registered_player.identity.player_id.clone(),
                ParticipantRegistryEntry {
                    identity: registered_player.identity,
                    state: ParticipantState::Seated,
                    connection_state: ConnectionState::Connected,
                    seat_index: Some(registered_player.seat_index),
                    admitted_at_ms: 0,
                    reconnect_token: None,
                    reconnect_expiry_ms: None,
                    is_host: registered_player.is_host,
                },
            );
        }

        let state = TournamentState {
            table_id: table_id.into(),
            session_epoch,
            phase: TournamentPhase::ReadyCheck,
            config: config.clone(),
            blind_schedule: config.blind_schedule.clone(),
            blind_level_index: 0,
            participants,
            seats,
            current_hand: None,
            hand_results: Vec::new(),
            placements: Vec::new(),
        };
        validate_tournament_state(&state)?;

        Ok(Self {
            state,
            roster_frozen: false,
            started_at_ms: None,
            next_blind_deadline_ms: None,
            intermission_deadline_ms: None,
            dealer_button_seat_index: None,
            next_action_window_id: 1,
            pending_deck: None,
            active_hand: None,
        })
    }

    #[must_use]
    pub fn state(&self) -> &TournamentState {
        &self.state
    }

    pub fn set_next_deck(&mut self, deck: Deck) {
        self.pending_deck = Some(deck);
    }

    pub fn start_tournament(&mut self, now_ms: u64) -> Result<(), TournamentError> {
        if matches!(
            self.state.phase,
            TournamentPhase::Running | TournamentPhase::Complete
        ) {
            return Err(TournamentError::new("tournament has already started"));
        }

        let seated_count = self
            .state
            .seats
            .iter()
            .filter(|seat| seat.occupancy == SeatOccupancyState::Occupied)
            .count();
        if !(2..=10).contains(&seated_count) {
            return Err(TournamentError::new(
                "tournament start requires between two and ten seated players",
            ));
        }

        if self
            .state
            .seats
            .iter()
            .filter(|seat| seat.occupancy == SeatOccupancyState::Occupied)
            .any(|seat| !seat.is_ready)
        {
            return Err(TournamentError::new(
                "tournament start requires every occupied seat to be ready",
            ));
        }

        self.roster_frozen = true;
        self.started_at_ms = Some(now_ms);
        self.state.phase = TournamentPhase::Running;
        self.state.blind_level_index = 0;
        self.state.hand_results.clear();
        self.state.placements.clear();
        self.next_blind_deadline_ms =
            Some(now_ms + self.current_blind_level().duration_seconds as u64 * 1_000);

        for seat in &mut self.state.seats {
            if seat.occupancy == SeatOccupancyState::Occupied {
                seat.tournament_state = TournamentSeatState::Active;
                seat.chip_count = Some(self.state.config.starting_stack);
            }
        }

        for participant in self.state.participants.values_mut() {
            participant.state = ParticipantState::Active;
        }

        self.start_next_hand(now_ms)?;
        self.validate_state()?;
        Ok(())
    }

    pub fn submit_action(
        &mut self,
        request: ActionRequest,
        now_ms: u64,
    ) -> Result<(), TournamentError> {
        let rollback_state = self.clone();
        let current_window = self
            .state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .cloned()
            .ok_or_else(|| TournamentError::new("stale action window rejected"))?;

        if now_ms >= current_window.deadline_epoch_ms {
            if let Err(error) = self.commit_timeout(now_ms) {
                *self = rollback_state;
                return Err(error);
            }
            if let Err(error) = self.validate_state() {
                *self = rollback_state;
                return Err(error);
            }
            return Err(TournamentError::new("stale action window rejected"));
        }

        if request.player_id != current_window.player_id {
            return Err(TournamentError::new(
                "action rejected: player does not own the action window",
            ));
        }

        if request.action_window_id != current_window.action_window_id {
            return Err(TournamentError::new("stale action window rejected"));
        }

        if let Err(error) = self.apply_action(
            request.player_id,
            request.action_type,
            request.raise_to_amount,
            now_ms,
        ) {
            *self = rollback_state;
            return Err(error);
        }
        if let Err(error) = self.validate_state() {
            *self = rollback_state;
            return Err(error);
        }
        Ok(())
    }

    pub fn advance_time(&mut self, now_ms: u64) -> Result<(), TournamentError> {
        loop {
            if self
                .state
                .current_hand
                .as_ref()
                .and_then(|hand| hand.action_window.as_ref())
                .is_some_and(|window| now_ms >= window.deadline_epoch_ms)
            {
                self.commit_timeout(now_ms)?;
                continue;
            }

            if self
                .state
                .current_hand
                .as_ref()
                .is_some_and(|hand| hand.cycle_phase == HandCyclePhase::BetweenHands)
                && self
                    .intermission_deadline_ms
                    .is_some_and(|deadline_ms| now_ms >= deadline_ms)
            {
                self.advance_blind_levels_if_due(now_ms);
                if self.state.phase != TournamentPhase::Complete {
                    self.start_next_hand(now_ms)?;
                }
                continue;
            }

            break;
        }

        self.validate_state()?;
        Ok(())
    }

    pub(crate) fn validate_state(&self) -> Result<(), TournamentError> {
        validate_tournament_state(&self.state)?;
        Ok(())
    }

    pub(crate) fn current_blind_level(&self) -> &BlindLevel {
        &self.state.blind_schedule.levels[self.state.blind_level_index]
    }

    pub(crate) fn start_next_hand(&mut self, now_ms: u64) -> Result<(), TournamentError> {
        self.intermission_deadline_ms = None;
        self.active_hand = None;
        self.clear_markers();

        let active_players = self.active_player_seats();
        if active_players.len() <= 1 {
            self.complete_tournament();
            return Ok(());
        }

        let hand_number = self.state.hand_results.len() as u32 + 1;
        let dealer_seat_index = self.next_dealer_seat_index(&active_players)?;
        let is_heads_up = active_players.len() == 2;
        let small_blind_seat_index = if is_heads_up {
            dealer_seat_index
        } else {
            self.next_active_seat_after(dealer_seat_index, &active_players)?
        };
        let big_blind_seat_index =
            self.next_active_seat_after(small_blind_seat_index, &active_players)?;
        self.dealer_button_seat_index = Some(dealer_seat_index);
        self.assign_markers(
            dealer_seat_index,
            small_blind_seat_index,
            big_blind_seat_index,
        );

        let mut deck = self.pending_deck.take().unwrap_or_else(Deck::shuffled);
        let deal_order = self.player_order_starting_with(small_blind_seat_index, &active_players);
        let hole_cards_by_player_id = deck.deal_hole_cards(&deal_order)?;

        let participation_by_player_id = active_players
            .iter()
            .map(|(_, player_id)| (player_id.clone(), HandParticipationState::Active))
            .collect::<BTreeMap<_, _>>();
        let total_contributions_by_player_id = active_players
            .iter()
            .map(|(_, player_id)| (player_id.clone(), 0_u32))
            .collect::<BTreeMap<_, _>>();
        let street_contributions_by_player_id = total_contributions_by_player_id.clone();

        self.state.current_hand = Some(HandState {
            hand_number,
            cycle_phase: HandCyclePhase::Dealing,
            street: StreetPhase::Preflop,
            dealer_seat_index,
            small_blind_seat_index,
            big_blind_seat_index,
            board_cards: Vec::new(),
            hole_cards_by_player_id,
            participation_by_player_id,
            betting_round: BettingRoundState {
                street: StreetPhase::Preflop,
                current_bet: 0,
                min_raise_to: Some(self.current_blind_level().big_blind),
                max_raise_to: None,
                pot_size: 0,
                contributions_by_player_id: total_contributions_by_player_id,
            },
            action_window: None,
        });

        self.active_hand = Some(ActiveHandRuntime {
            deck,
            street_contributions_by_player_id,
            acted_since_last_full_raise: BTreeSet::new(),
            full_raise_increment: self.current_blind_level().big_blind,
            last_actor_id: None,
        });

        if self.current_blind_level().ante > 0 {
            let players = active_players
                .iter()
                .map(|(_, player_id)| player_id.clone())
                .collect::<Vec<_>>();
            for player_id in players {
                let ante = self
                    .current_blind_level()
                    .ante
                    .min(self.player_stack(&player_id)?);
                self.commit_total_wager(&player_id, ante, false)?;
                if self.player_stack(&player_id)? == 0 {
                    self.set_participation(&player_id, HandParticipationState::AllIn)?;
                }
            }
        }

        self.post_blind(
            small_blind_seat_index,
            self.current_blind_level().small_blind,
        )?;
        self.post_blind(big_blind_seat_index, self.current_blind_level().big_blind)?;

        let first_actor_seat_index = if is_heads_up {
            dealer_seat_index
        } else {
            self.next_active_seat_after(big_blind_seat_index, &active_players)?
        };
        self.reset_turn_cursor(first_actor_seat_index)?;
        self.refresh_action_window(now_ms)?;
        Ok(())
    }

    pub(crate) fn post_blind(
        &mut self,
        seat_index: u8,
        blind_amount: u32,
    ) -> Result<(), TournamentError> {
        let player_id = self.player_id_at_seat(seat_index)?;
        let posted_amount = blind_amount.min(self.player_stack(&player_id)?);
        self.commit_total_wager(&player_id, posted_amount, true)?;
        if self.player_stack(&player_id)? == 0 {
            self.set_participation(&player_id, HandParticipationState::AllIn)?;
        }
        Ok(())
    }

    pub(crate) fn commit_total_wager(
        &mut self,
        player_id: &str,
        wager_amount: u32,
        count_toward_street: bool,
    ) -> Result<(), TournamentError> {
        if wager_amount == 0 {
            return Ok(());
        }

        let stack = self.player_stack(player_id)?;
        if wager_amount > stack {
            return Err(TournamentError::new(
                "player cannot wager more chips than remain in stack",
            ));
        }
        self.set_player_stack(player_id, stack - wager_amount)?;

        if let Some(hand) = self.state.current_hand.as_mut() {
            let total_contribution = hand
                .betting_round
                .contributions_by_player_id
                .entry(player_id.to_string())
                .or_insert(0);
            *total_contribution += wager_amount;
            hand.betting_round.pot_size += wager_amount;
        }

        if count_toward_street {
            let runtime = self
                .active_hand
                .as_mut()
                .ok_or_else(|| TournamentError::new("hand runtime missing while posting wager"))?;
            let street_total = runtime
                .street_contributions_by_player_id
                .entry(player_id.to_string())
                .or_insert(0);
            *street_total += wager_amount;
            if let Some(hand) = self.state.current_hand.as_mut() {
                hand.betting_round.current_bet = hand.betting_round.current_bet.max(*street_total);
            }
        }

        self.refresh_betting_round_bounds();
        Ok(())
    }

    pub(crate) fn refresh_betting_round_bounds(&mut self) {
        let current_bet = self
            .state
            .current_hand
            .as_ref()
            .map(|hand| hand.betting_round.current_bet)
            .unwrap_or_default();
        let full_raise_increment = self
            .active_hand
            .as_ref()
            .map(|runtime| runtime.full_raise_increment)
            .unwrap_or(self.current_blind_level().big_blind);
        let min_raise_to = if current_bet == 0 {
            full_raise_increment
        } else {
            current_bet + full_raise_increment
        };

        let max_raise_to = self
            .active_player_ids_in_current_hand()
            .into_iter()
            .filter_map(|player_id| {
                self.street_contribution(&player_id)
                    .ok()
                    .zip(self.player_stack(&player_id).ok())
            })
            .map(|(street_contribution, stack)| street_contribution + stack)
            .max();

        if let Some(hand) = self.state.current_hand.as_mut() {
            hand.betting_round.min_raise_to = Some(min_raise_to);
            hand.betting_round.max_raise_to = max_raise_to;
        }
    }

    pub(crate) fn refresh_action_window(&mut self, now_ms: u64) -> Result<(), TournamentError> {
        let current_hand =
            self.state.current_hand.as_ref().ok_or_else(|| {
                TournamentError::new("cannot open an action window without a hand")
            })?;
        if current_hand.cycle_phase == HandCyclePhase::BetweenHands {
            return Ok(());
        }

        if self.remaining_contenders().len() <= 1 {
            self.settle_current_hand(now_ms)?;
            return Ok(());
        }

        if self.players_who_can_act().is_empty() {
            if self.remaining_contenders().iter().all(|player_id| {
                self.participation(player_id)
                    .is_some_and(|state| state == HandParticipationState::AllIn)
            }) {
                self.reveal_remaining_board()?;
                self.settle_current_hand(now_ms)?;
            } else {
                self.advance_street_or_showdown(now_ms)?;
            }
            return Ok(());
        }

        let next_player_id = self
            .next_actor_id()?
            .ok_or_else(|| TournamentError::new("no legal next actor found"))?;
        let street_contribution = self.street_contribution(&next_player_id)?;
        let player_stack = self.player_stack(&next_player_id)?;
        let current_bet = self
            .state
            .current_hand
            .as_ref()
            .map(|hand| hand.betting_round.current_bet)
            .unwrap_or_default();
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
        let legal = legal_actions(&LegalActionContext {
            player_stack,
            player_commit: street_contribution,
            current_bet,
            min_full_raise_to,
            may_raise: self.player_may_raise(&next_player_id),
        });

        let action_window = ActionWindow {
            action_window_id: format!("aw-{}", self.next_action_window_id),
            player_id: next_player_id.clone(),
            seat_index: self.player_seat_index(&next_player_id)?,
            legal_actions: legal.legal_actions,
            call_amount: legal.call_amount,
            min_raise_to: legal.min_raise_to,
            max_raise_to: legal.max_raise_to,
            deadline_epoch_ms: now_ms + self.state.config.turn_timer_seconds as u64 * 1_000,
        };
        self.next_action_window_id += 1;

        if let Some(hand) = self.state.current_hand.as_mut() {
            hand.cycle_phase = HandCyclePhase::AwaitingAction;
            hand.action_window = Some(action_window);
        }
        Ok(())
    }
}
