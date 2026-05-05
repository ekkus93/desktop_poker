use std::collections::{BTreeMap, BTreeSet};

use crate::{
    app_state::ModuleDescriptor,
    domain::{
        validate_tournament_state, ActionType, ActionWindow, BettingRoundState, BlindLevel,
        ConnectionState, HandCyclePhase, HandParticipationState, HandResult, HandState,
        ParticipantRegistryEntry, ParticipantState, PlacementEntry, PlayerIdentity, SeatMarker,
        SeatOccupancyState, SeatState, StreetPhase, TournamentConfig, TournamentPhase,
        TournamentSeatState, TournamentState,
    },
    engine::{
        evaluate_best_holdem_hand, legal_actions, settle_showdown, Deck, EngineError, HandCategory,
        HandStrength, LegalActionContext,
    },
};

pub const BETWEEN_HANDS_DELAY_MS: u64 = 1_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TournamentError {
    message: String,
}

impl TournamentError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TournamentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TournamentError {}

impl From<crate::domain::DomainError> for TournamentError {
    fn from(value: crate::domain::DomainError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<EngineError> for TournamentError {
    fn from(value: EngineError) -> Self {
        Self::new(value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredPlayer {
    pub identity: PlayerIdentity,
    pub seat_index: u8,
    pub is_host: bool,
    pub is_ready: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionRequest {
    pub player_id: String,
    pub action_window_id: String,
    pub action_type: ActionType,
    pub raise_to_amount: Option<u32>,
}

#[derive(Clone, Debug)]
struct ActiveHandRuntime {
    deck: Deck,
    street_contributions_by_player_id: BTreeMap<String, u32>,
    acted_since_last_full_raise: BTreeSet<String>,
    full_raise_increment: u32,
    last_actor_id: Option<String>,
}

pub struct TournamentController {
    state: TournamentState,
    roster_frozen: bool,
    started_at_ms: Option<u64>,
    next_blind_deadline_ms: Option<u64>,
    intermission_deadline_ms: Option<u64>,
    dealer_button_seat_index: Option<u8>,
    next_action_window_id: u64,
    pending_deck: Option<Deck>,
    active_hand: Option<ActiveHandRuntime>,
}

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "tournament",
        responsibility:
            "Coordinates roster freeze, blind scheduling, hand loops, eliminations, and tournament completion.",
    }
}

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
        let current_window = self
            .state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .cloned()
            .ok_or_else(|| TournamentError::new("stale action window rejected"))?;

        if now_ms >= current_window.deadline_epoch_ms {
            self.commit_timeout(now_ms)?;
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

        self.apply_action(
            request.player_id,
            request.action_type,
            request.raise_to_amount,
            now_ms,
        )?;
        self.validate_state()?;
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

    fn validate_state(&self) -> Result<(), TournamentError> {
        validate_tournament_state(&self.state)?;
        Ok(())
    }

    fn current_blind_level(&self) -> &BlindLevel {
        &self.state.blind_schedule.levels[self.state.blind_level_index]
    }

    fn start_next_hand(&mut self, now_ms: u64) -> Result<(), TournamentError> {
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

    fn post_blind(&mut self, seat_index: u8, blind_amount: u32) -> Result<(), TournamentError> {
        let player_id = self.player_id_at_seat(seat_index)?;
        let posted_amount = blind_amount.min(self.player_stack(&player_id)?);
        self.commit_total_wager(&player_id, posted_amount, true)?;
        if self.player_stack(&player_id)? == 0 {
            self.set_participation(&player_id, HandParticipationState::AllIn)?;
        }
        Ok(())
    }

    fn commit_total_wager(
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

    fn refresh_betting_round_bounds(&mut self) {
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

    fn refresh_action_window(&mut self, now_ms: u64) -> Result<(), TournamentError> {
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

    fn apply_action(
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

    fn record_non_reopening_action(&mut self, player_id: &str) -> Result<(), TournamentError> {
        let runtime = self
            .active_hand
            .as_mut()
            .ok_or_else(|| TournamentError::new("hand runtime missing while committing action"))?;
        runtime
            .acted_since_last_full_raise
            .insert(player_id.to_string());
        Ok(())
    }

    fn handle_full_raise(
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

    fn commit_timeout(&mut self, now_ms: u64) -> Result<(), TournamentError> {
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

    fn advance_street_or_showdown(&mut self, now_ms: u64) -> Result<(), TournamentError> {
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

    fn advance_to_street(
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

    fn reveal_remaining_board(&mut self) -> Result<(), TournamentError> {
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

    fn settle_current_hand(&mut self, now_ms: u64) -> Result<(), TournamentError> {
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
        let result = HandResult {
            hand_number: hand.hand_number,
            winning_player_ids: settlement.winning_player_ids.clone(),
            pot_summaries: settlement.pot_summaries,
            revealed_hands_by_player_id,
            eliminated_player_ids: eliminated_player_ids.clone(),
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

    fn process_eliminations(&mut self, hand_number: u32) -> Vec<String> {
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

    fn complete_tournament(&mut self) {
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

    fn advance_blind_levels_if_due(&mut self, now_ms: u64) {
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

    fn reset_turn_cursor(&mut self, first_actor_seat_index: u8) -> Result<(), TournamentError> {
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

    fn next_actor_id(&self) -> Result<Option<String>, TournamentError> {
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

    fn player_needs_action(&self, player_id: &str) -> Result<bool, TournamentError> {
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

    fn player_may_raise(&self, player_id: &str) -> bool {
        matches!(
            self.participation(player_id),
            Some(HandParticipationState::Active)
        ) && !self
            .active_hand
            .as_ref()
            .is_some_and(|runtime| runtime.acted_since_last_full_raise.contains(player_id))
    }

    fn players_who_can_act(&self) -> Vec<String> {
        self.active_player_ids_in_current_hand()
            .into_iter()
            .filter(|player_id| self.player_needs_action(player_id).unwrap_or(false))
            .collect()
    }

    fn remaining_contenders(&self) -> Vec<String> {
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

    fn active_player_seats(&self) -> Vec<(u8, String)> {
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

    fn active_player_seats_in_current_hand(&self) -> Vec<(u8, String)> {
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

    fn active_player_ids_in_current_hand(&self) -> Vec<String> {
        self.active_player_seats_in_current_hand()
            .into_iter()
            .map(|(_, player_id)| player_id)
            .collect()
    }

    fn competitor_count(&self) -> u8 {
        self.state
            .participants
            .values()
            .filter(|participant| participant.seat_index.is_some())
            .count() as u8
    }

    fn player_order_starting_with(
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

    fn next_dealer_seat_index(
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

    fn next_active_seat_after(
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

    fn player_id_at_seat(&self, seat_index: u8) -> Result<String, TournamentError> {
        self.state
            .seats
            .get(seat_index as usize)
            .and_then(|seat| seat.participant_id.clone())
            .ok_or_else(|| TournamentError::new("seat does not currently hold a participant"))
    }

    fn player_seat_index(&self, player_id: &str) -> Result<u8, TournamentError> {
        self.state
            .participants
            .get(player_id)
            .and_then(|participant| participant.seat_index)
            .ok_or_else(|| TournamentError::new("participant seat index is unavailable"))
    }

    fn player_stack(&self, player_id: &str) -> Result<u32, TournamentError> {
        let seat_index = self.player_seat_index(player_id)?;
        self.state
            .seats
            .get(seat_index as usize)
            .and_then(|seat| seat.chip_count)
            .ok_or_else(|| TournamentError::new("participant stack is unavailable"))
    }

    fn set_player_stack(
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

    fn participation(&self, player_id: &str) -> Option<HandParticipationState> {
        self.state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.participation_by_player_id.get(player_id).copied())
    }

    fn set_participation(
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

    fn street_contribution(&self, player_id: &str) -> Result<u32, TournamentError> {
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

    fn clear_markers(&mut self) {
        for seat in &mut self.state.seats {
            seat.marker = None;
        }
    }

    fn assign_markers(&mut self, dealer: u8, small_blind: u8, big_blind: u8) {
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

    fn sort_placements(&mut self) {
        self.state.placements.sort_by_key(|entry| entry.place);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{BlindSchedule, Card, Rank, Suit};

    fn sample_config(starting_stack: u32) -> TournamentConfig {
        TournamentConfig {
            tournament_name: "M4 Test".to_string(),
            table_name: Some("Table One".to_string()),
            max_players: 3,
            starting_stack,
            turn_timer_seconds: 10,
            blind_schedule: BlindSchedule {
                levels: vec![
                    BlindLevel {
                        level_index: 1,
                        label: "Level 1".to_string(),
                        small_blind: 50,
                        big_blind: 100,
                        ante: 0,
                        duration_seconds: 5,
                    },
                    BlindLevel {
                        level_index: 2,
                        label: "Level 2".to_string(),
                        small_blind: 100,
                        big_blind: 200,
                        ante: 0,
                        duration_seconds: 5,
                    },
                ],
            },
        }
    }

    fn player(player_id: &str, seat_index: u8) -> RegisteredPlayer {
        RegisteredPlayer {
            identity: PlayerIdentity {
                player_id: player_id.to_string(),
                display_name: format!("Player {player_id}"),
                signing_public_key: format!("sign-{player_id}"),
                encryption_public_key: format!("enc-{player_id}"),
                signing_key_fingerprint: format!("fp-{player_id}"),
            },
            seat_index,
            is_host: player_id == "p1",
            is_ready: true,
        }
    }

    fn card(rank: Rank, suit: Suit) -> Card {
        Card { rank, suit }
    }

    fn stacked_deck(prefix: Vec<Card>) -> Deck {
        let mut cards = prefix.clone();
        cards.extend(
            Deck::standard_52()
                .cards()
                .iter()
                .copied()
                .filter(|card| !prefix.contains(card)),
        );
        Deck::from_cards(cards).expect("stacked deck should be unique")
    }

    fn action_window(controller: &TournamentController) -> ActionWindow {
        controller
            .state()
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.clone())
            .expect("expected open action window")
    }

    #[test]
    fn custom_deck_sequences_drive_real_controller_hole_cards() {
        let mut controller = TournamentController::new(
            "table-deck-contract",
            1,
            sample_config(1_000),
            vec![player("p1", 0), player("p2", 1)],
        )
        .expect("controller should build");
        controller.set_next_deck(stacked_deck(vec![
            card(Rank::Ace, Suit::Spades),
            card(Rank::King, Suit::Hearts),
            card(Rank::Queen, Suit::Clubs),
            card(Rank::Jack, Suit::Diamonds),
            card(Rank::Ten, Suit::Spades),
            card(Rank::Nine, Suit::Hearts),
            card(Rank::Eight, Suit::Clubs),
            card(Rank::Seven, Suit::Diamonds),
            card(Rank::Six, Suit::Spades),
        ]));

        controller
            .start_tournament(0)
            .expect("tournament should start");

        let hand = controller
            .state()
            .current_hand
            .as_ref()
            .expect("current hand should exist");

        assert_eq!(hand.board_cards.len(), 0);
        assert_eq!(hand.hole_cards_by_player_id.len(), 2);
        assert_eq!(
            hand.hole_cards_by_player_id
                .get("p1")
                .expect("p1 should have hole cards")
                .len(),
            2
        );
        assert_eq!(
            hand.hole_cards_by_player_id
                .get("p2")
                .expect("p2 should have hole cards")
                .len(),
            2
        );
    }

    #[test]
    fn hand_progresses_from_start_to_completion() {
        let mut controller = TournamentController::new(
            "table-1",
            1,
            sample_config(1_000),
            vec![player("p1", 0), player("p2", 1)],
        )
        .expect("controller should build");
        controller.set_next_deck(stacked_deck(vec![
            card(Rank::Ace, Suit::Spades),
            card(Rank::King, Suit::Spades),
            card(Rank::Queen, Suit::Hearts),
            card(Rank::Jack, Suit::Hearts),
            card(Rank::Two, Suit::Clubs),
            card(Rank::Three, Suit::Clubs),
            card(Rank::Four, Suit::Diamonds),
            card(Rank::Five, Suit::Diamonds),
            card(Rank::Six, Suit::Diamonds),
            card(Rank::Seven, Suit::Diamonds),
            card(Rank::Eight, Suit::Diamonds),
        ]));
        controller
            .start_tournament(0)
            .expect("tournament should start");

        for now_ms in 1..=8 {
            let window = action_window(&controller);
            let action_type = if window.legal_actions.contains(&ActionType::Check) {
                ActionType::Check
            } else {
                ActionType::Call
            };
            controller
                .submit_action(
                    ActionRequest {
                        player_id: window.player_id,
                        action_window_id: window.action_window_id,
                        action_type,
                        raise_to_amount: None,
                    },
                    now_ms,
                )
                .expect("scripted action should succeed");
            if controller
                .state()
                .current_hand
                .as_ref()
                .is_some_and(|hand| hand.cycle_phase == HandCyclePhase::BetweenHands)
            {
                break;
            }
        }

        let current_hand = controller
            .state()
            .current_hand
            .as_ref()
            .expect("hand should remain visible during intermission");
        assert_eq!(current_hand.cycle_phase, HandCyclePhase::BetweenHands);
        assert_eq!(current_hand.board_cards.len(), 5);
        assert_eq!(controller.state().hand_results.len(), 1);
    }

    #[test]
    fn between_hands_auto_progression_works() {
        let mut controller = TournamentController::new(
            "table-2",
            1,
            sample_config(1_000),
            vec![player("p1", 0), player("p2", 1)],
        )
        .expect("controller should build");
        controller.set_next_deck(stacked_deck(vec![
            card(Rank::Ace, Suit::Spades),
            card(Rank::King, Suit::Spades),
            card(Rank::Queen, Suit::Hearts),
            card(Rank::Jack, Suit::Hearts),
            card(Rank::Two, Suit::Clubs),
            card(Rank::Three, Suit::Clubs),
            card(Rank::Four, Suit::Diamonds),
            card(Rank::Five, Suit::Diamonds),
            card(Rank::Six, Suit::Diamonds),
            card(Rank::Seven, Suit::Diamonds),
            card(Rank::Eight, Suit::Diamonds),
        ]));
        controller
            .start_tournament(0)
            .expect("tournament should start");

        for now_ms in 1..=8 {
            let window = action_window(&controller);
            let action_type = if window.legal_actions.contains(&ActionType::Check) {
                ActionType::Check
            } else {
                ActionType::Call
            };
            controller
                .submit_action(
                    ActionRequest {
                        player_id: window.player_id,
                        action_window_id: window.action_window_id,
                        action_type,
                        raise_to_amount: None,
                    },
                    now_ms,
                )
                .expect("scripted action should resolve the hand");
            if controller
                .state()
                .current_hand
                .as_ref()
                .is_some_and(|hand| hand.cycle_phase == HandCyclePhase::BetweenHands)
            {
                break;
            }
        }
        assert_eq!(controller.state().hand_results.len(), 1);
        assert_eq!(
            controller
                .state()
                .current_hand
                .as_ref()
                .map(|hand| hand.cycle_phase),
            Some(HandCyclePhase::BetweenHands)
        );

        controller.set_next_deck(stacked_deck(vec![
            card(Rank::Nine, Suit::Clubs),
            card(Rank::Eight, Suit::Clubs),
            card(Rank::Seven, Suit::Hearts),
            card(Rank::Six, Suit::Hearts),
            card(Rank::Five, Suit::Clubs),
            card(Rank::Four, Suit::Clubs),
            card(Rank::Three, Suit::Diamonds),
            card(Rank::Two, Suit::Diamonds),
            card(Rank::Ace, Suit::Diamonds),
            card(Rank::King, Suit::Diamonds),
            card(Rank::Queen, Suit::Diamonds),
        ]));
        controller
            .advance_time(2_000)
            .expect("intermission should auto-start next hand");

        assert_eq!(controller.state().hand_results.len(), 1);
        assert_eq!(
            controller
                .state()
                .current_hand
                .as_ref()
                .map(|hand| hand.hand_number),
            Some(2)
        );
    }

    #[test]
    fn blind_levels_increase_only_between_hands() {
        let mut controller = TournamentController::new(
            "table-3",
            1,
            sample_config(1_000),
            vec![player("p1", 0), player("p2", 1)],
        )
        .expect("controller should build");
        controller
            .start_tournament(0)
            .expect("tournament should start");

        controller
            .advance_time(6_000)
            .expect("time advance should succeed");
        assert_eq!(controller.state().blind_level_index, 0);

        for now_ms in 1..=8 {
            let window = action_window(&controller);
            let action_type = if window.legal_actions.contains(&ActionType::Check) {
                ActionType::Check
            } else {
                ActionType::Call
            };
            controller
                .submit_action(
                    ActionRequest {
                        player_id: window.player_id,
                        action_window_id: window.action_window_id,
                        action_type,
                        raise_to_amount: None,
                    },
                    now_ms,
                )
                .expect("action should succeed");
            if controller
                .state()
                .current_hand
                .as_ref()
                .is_some_and(|hand| hand.cycle_phase == HandCyclePhase::BetweenHands)
            {
                break;
            }
        }

        controller.set_next_deck(stacked_deck(vec![
            card(Rank::Nine, Suit::Clubs),
            card(Rank::Eight, Suit::Clubs),
            card(Rank::Seven, Suit::Hearts),
            card(Rank::Six, Suit::Hearts),
            card(Rank::Five, Suit::Clubs),
            card(Rank::Four, Suit::Clubs),
            card(Rank::Three, Suit::Diamonds),
            card(Rank::Two, Suit::Diamonds),
            card(Rank::Ace, Suit::Diamonds),
            card(Rank::King, Suit::Diamonds),
            card(Rank::Queen, Suit::Diamonds),
        ]));
        controller
            .advance_time(6_000 + BETWEEN_HANDS_DELAY_MS)
            .expect("next hand should start at the higher blind level");
        assert_eq!(controller.state().blind_level_index, 1);
    }

    #[test]
    fn short_all_in_does_not_reopen_action() {
        let mut controller = TournamentController::new(
            "table-4",
            1,
            sample_config(1_000),
            vec![player("p1", 0), player("p2", 1), player("p3", 2)],
        )
        .expect("controller should build");
        controller
            .start_tournament(0)
            .expect("tournament should start");
        controller
            .set_player_stack("p3", 150)
            .expect("stack override should apply");

        let first_window = action_window(&controller);
        controller
            .submit_action(
                ActionRequest {
                    player_id: first_window.player_id,
                    action_window_id: first_window.action_window_id,
                    action_type: ActionType::Raise,
                    raise_to_amount: Some(200),
                },
                1,
            )
            .expect("open raise should succeed");

        let second_window = action_window(&controller);
        controller
            .submit_action(
                ActionRequest {
                    player_id: second_window.player_id,
                    action_window_id: second_window.action_window_id,
                    action_type: ActionType::Call,
                    raise_to_amount: None,
                },
                2,
            )
            .expect("call should succeed");

        let third_window = action_window(&controller);
        assert_eq!(third_window.player_id, "p3");
        controller
            .submit_action(
                ActionRequest {
                    player_id: third_window.player_id,
                    action_window_id: third_window.action_window_id,
                    action_type: ActionType::AllIn,
                    raise_to_amount: None,
                },
                3,
            )
            .expect("short all-in should succeed");

        let reopened_window = action_window(&controller);
        assert_eq!(reopened_window.player_id, "p1");
        assert!(!reopened_window.legal_actions.contains(&ActionType::Raise));
        assert!(!reopened_window.legal_actions.contains(&ActionType::AllIn));
        assert_eq!(
            reopened_window.legal_actions,
            vec![ActionType::Fold, ActionType::Call]
        );
    }

    #[test]
    fn heads_up_uses_button_as_small_blind_and_rotates_next_hand() {
        let mut controller = TournamentController::new(
            "table-heads-up",
            1,
            sample_config(1_000),
            vec![player("p1", 0), player("p2", 1)],
        )
        .expect("controller should build");
        controller
            .start_tournament(0)
            .expect("tournament should start");

        let first_hand = controller
            .state()
            .current_hand
            .clone()
            .expect("first hand should be active");
        assert_eq!(
            first_hand.dealer_seat_index,
            first_hand.small_blind_seat_index
        );
        assert_ne!(
            first_hand.dealer_seat_index,
            first_hand.big_blind_seat_index
        );

        let first_window = action_window(&controller);
        assert_eq!(first_window.player_id, "p1");
        controller
            .submit_action(
                ActionRequest {
                    player_id: first_window.player_id,
                    action_window_id: first_window.action_window_id,
                    action_type: ActionType::Fold,
                    raise_to_amount: None,
                },
                1,
            )
            .expect("heads-up fold should settle the hand");

        controller
            .advance_time(BETWEEN_HANDS_DELAY_MS + 2)
            .expect("next hand should start after intermission");

        let second_hand = controller
            .state()
            .current_hand
            .clone()
            .expect("second hand should be active");
        assert_eq!(second_hand.hand_number, 2);
        assert_eq!(
            second_hand.dealer_seat_index,
            second_hand.small_blind_seat_index
        );
        assert_ne!(second_hand.dealer_seat_index, first_hand.dealer_seat_index);
        assert_ne!(
            second_hand.big_blind_seat_index,
            second_hand.small_blind_seat_index
        );
    }

    #[test]
    fn timeout_commits_and_stale_actions_are_rejected() {
        let mut controller = TournamentController::new(
            "table-5",
            1,
            sample_config(1_000),
            vec![player("p1", 0), player("p2", 1)],
        )
        .expect("controller should build");
        controller
            .start_tournament(0)
            .expect("tournament should start");

        let expired_window = action_window(&controller);
        controller
            .advance_time(expired_window.deadline_epoch_ms)
            .expect("timeout should auto-commit");
        assert_eq!(controller.state().hand_results.len(), 1);

        let stale_result = controller.submit_action(
            ActionRequest {
                player_id: expired_window.player_id,
                action_window_id: expired_window.action_window_id,
                action_type: ActionType::Call,
                raise_to_amount: None,
            },
            expired_window.deadline_epoch_ms + 1,
        );
        assert!(stale_result.is_err());
        assert!(stale_result
            .expect_err("stale action should be rejected")
            .to_string()
            .contains("stale"));
    }

    #[test]
    fn simultaneous_eliminations_follow_seat_order_for_placements() {
        let mut controller = TournamentController::new(
            "table-placement",
            1,
            sample_config(1_000),
            vec![player("p1", 0), player("p2", 1), player("p3", 2)],
        )
        .expect("controller should build");
        controller
            .start_tournament(0)
            .expect("tournament should start");
        controller
            .set_player_stack("p1", 0)
            .expect("stack override should apply");
        controller
            .set_player_stack("p2", 0)
            .expect("stack override should apply");
        controller
            .set_player_stack("p3", 1_000)
            .expect("stack override should apply");

        let eliminated = controller.process_eliminations(7);
        controller.sort_placements();

        assert_eq!(eliminated, vec!["p1".to_string(), "p2".to_string()]);
        assert_eq!(controller.state().placements.len(), 2);
        let places_by_player = controller
            .state()
            .placements
            .iter()
            .map(|entry| (entry.player_id.as_str(), entry.place))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(places_by_player.get("p1"), Some(&3));
        assert_eq!(places_by_player.get("p2"), Some(&2));
    }

    #[test]
    fn tournament_ends_correctly() {
        let mut controller = TournamentController::new(
            "table-6",
            1,
            sample_config(100),
            vec![player("p1", 0), player("p2", 1)],
        )
        .expect("controller should build");
        controller.set_next_deck(stacked_deck(vec![
            card(Rank::Ace, Suit::Spades),
            card(Rank::King, Suit::Spades),
            card(Rank::Queen, Suit::Hearts),
            card(Rank::Jack, Suit::Hearts),
            card(Rank::Two, Suit::Clubs),
            card(Rank::Three, Suit::Clubs),
            card(Rank::Four, Suit::Diamonds),
            card(Rank::Five, Suit::Diamonds),
            card(Rank::Six, Suit::Diamonds),
            card(Rank::Seven, Suit::Diamonds),
            card(Rank::Eight, Suit::Diamonds),
        ]));
        controller
            .start_tournament(0)
            .expect("tournament should start");

        let window = action_window(&controller);
        controller
            .submit_action(
                ActionRequest {
                    player_id: window.player_id,
                    action_window_id: window.action_window_id,
                    action_type: ActionType::AllIn,
                    raise_to_amount: None,
                },
                1,
            )
            .expect("all-in call should complete the tournament");

        assert_eq!(controller.state().phase, TournamentPhase::Complete);
        assert_eq!(controller.state().placements.len(), 2);
        assert_eq!(controller.state().placements[0].place, 1);
        assert_eq!(controller.state().placements[1].place, 2);
    }
}
