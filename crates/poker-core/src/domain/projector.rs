use std::collections::BTreeMap;

use super::{
    validate_tournament_state, DomainError, ObserverProjection, ParticipantState, PrivateState,
    PublicState, TournamentState,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionBundle {
    pub public_state: PublicState,
    pub private_states: BTreeMap<String, PrivateState>,
    pub observer_projection: ObserverProjection,
}

pub struct StateProjector;

impl StateProjector {
    pub fn project(state: &TournamentState) -> Result<ProjectionBundle, DomainError> {
        validate_tournament_state(state)?;

        let public_state = project_public_state(state);
        let action_window_player_id = public_state.action_window_player_id.clone();

        let private_states = state
            .participants
            .iter()
            .filter(|(_, participant)| participant.state != ParticipantState::Removed)
            .map(|(player_id, participant)| {
                let is_observer = participant.state == ParticipantState::EliminatedObserver;
                let private_hole_cards = if is_observer {
                    Vec::new()
                } else {
                    state
                        .current_hand
                        .as_ref()
                        .and_then(|hand| hand.hole_cards_by_player_id.get(player_id))
                        .cloned()
                        .unwrap_or_default()
                };

                let can_act = !is_observer
                    && action_window_player_id
                        .as_ref()
                        .is_some_and(|window_player_id| window_player_id == player_id);

                (
                    player_id.clone(),
                    PrivateState {
                        public_state: public_state.clone(),
                        local_player_id: player_id.clone(),
                        private_hole_cards,
                        can_act,
                        is_observer,
                        action_window_player_id: can_act.then(|| player_id.clone()),
                    },
                )
            })
            .collect();

        Ok(ProjectionBundle {
            public_state: public_state.clone(),
            private_states,
            observer_projection: ObserverProjection {
                public_state,
                private_hole_cards: Vec::new(),
                can_act: false,
                is_observer: true,
                action_window_player_id: None,
            },
        })
    }
}

fn project_public_state(state: &TournamentState) -> PublicState {
    PublicState {
        tournament_name: state.config.tournament_name.clone(),
        table_name: state.config.table_name.clone(),
        table_id: state.table_id.clone(),
        session_epoch: state.session_epoch,
        phase: state.phase,
        seats: state.seats.clone(),
        board_cards: state
            .current_hand
            .as_ref()
            .map(|hand| hand.board_cards.clone())
            .unwrap_or_default(),
        blind_level_label: state
            .blind_schedule
            .levels
            .get(state.blind_level_index)
            .map(|level| level.label.clone()),
        current_hand_number: state.current_hand.as_ref().map(|hand| hand.hand_number),
        placements: state.placements.clone(),
        action_window_player_id: state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .map(|window| window.player_id.clone()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::{
        ActionWindow, BettingRoundState, BlindLevel, BlindSchedule, Card, ConnectionState,
        HandCyclePhase, HandParticipationState, HandState, ParticipantRegistryEntry,
        PlayerIdentity, Rank, SeatOccupancyState, SeatState, StreetPhase, Suit, TournamentConfig,
        TournamentPhase, TournamentSeatState,
    };

    fn sample_blind_schedule() -> BlindSchedule {
        BlindSchedule {
            levels: vec![BlindLevel {
                level_index: 1,
                label: "L1".to_string(),
                small_blind: 10,
                big_blind: 20,
                ante: 0,
                duration_seconds: 60,
            }],
        }
    }

    fn sample_config() -> TournamentConfig {
        TournamentConfig {
            tournament_name: "Projector Test".to_string(),
            table_name: None,
            max_players: 2,
            starting_stack: 1000,
            turn_timer_seconds: 10,
            blind_schedule: sample_blind_schedule(),
        }
    }

    fn sample_participant(
        player_id: &str,
        state: ParticipantState,
        seat_index: Option<u8>,
        signing_suffix: &str,
    ) -> ParticipantRegistryEntry {
        ParticipantRegistryEntry {
            identity: PlayerIdentity {
                player_id: player_id.to_string(),
                display_name: player_id.to_string(),
                signing_public_key: format!("sign-{signing_suffix}"),
                encryption_public_key: format!("enc-{signing_suffix}"),
                signing_key_fingerprint: format!("fp-{signing_suffix}"),
            },
            state,
            connection_state: ConnectionState::Connected,
            seat_index,
            admitted_at_ms: 0,
            reconnect_token: None,
            reconnect_expiry_ms: None,
            is_host: false,
        }
    }

    fn occupied_seat(index: u8, player_id: &str) -> SeatState {
        SeatState {
            seat_index: index,
            occupancy: SeatOccupancyState::Occupied,
            tournament_state: TournamentSeatState::Active,
            participant_id: Some(player_id.to_string()),
            display_name: Some(player_id.to_string()),
            chip_count: Some(1000),
            is_ready: true,
            marker: None,
        }
    }

    fn minimal_state() -> TournamentState {
        let mut participants = BTreeMap::new();
        participants.insert(
            "p1".to_string(),
            sample_participant("p1", ParticipantState::Active, Some(0), "p1"),
        );
        participants.insert(
            "p2".to_string(),
            sample_participant("p2", ParticipantState::Active, Some(1), "p2"),
        );
        TournamentState {
            table_id: "proj-test".to_string(),
            session_epoch: 1,
            phase: TournamentPhase::Running,
            config: sample_config(),
            blind_schedule: sample_blind_schedule(),
            blind_level_index: 0,
            participants,
            seats: vec![occupied_seat(0, "p1"), occupied_seat(1, "p2")],
            current_hand: None,
            hand_results: vec![],
            placements: vec![],
        }
    }

    fn two_cards() -> Vec<Card> {
        vec![
            Card {
                rank: Rank::Ace,
                suit: Suit::Spades,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Hearts,
            },
        ]
    }

    // T8.1 — Removed participant excluded from private_states
    #[test]
    fn project_excludes_removed_participant_from_private_states() {
        let mut state = minimal_state();
        state.participants.insert(
            "removed".to_string(),
            sample_participant("removed", ParticipantState::Removed, None, "removed"),
        );

        let bundle = StateProjector::project(&state).expect("project should succeed");
        assert!(!bundle.private_states.contains_key("removed"));
    }

    // T8.2 — EliminatedObserver gets empty private_hole_cards
    #[test]
    fn project_eliminated_observer_gets_empty_private_hole_cards() {
        let mut state = minimal_state();
        state.participants.insert(
            "observer".to_string(),
            sample_participant(
                "observer",
                ParticipantState::EliminatedObserver,
                None,
                "obs",
            ),
        );

        let mut hole_cards = BTreeMap::new();
        hole_cards.insert("p1".to_string(), two_cards());
        hole_cards.insert("p2".to_string(), two_cards());
        hole_cards.insert("observer".to_string(), two_cards()); // won't matter — they're observer

        let mut participation = BTreeMap::new();
        participation.insert("p1".to_string(), HandParticipationState::Active);
        participation.insert("p2".to_string(), HandParticipationState::Active);
        participation.insert(
            "observer".to_string(),
            HandParticipationState::EliminatedObserver,
        );

        state.current_hand = Some(HandState {
            hand_number: 1,
            cycle_phase: HandCyclePhase::AwaitingAction,
            street: StreetPhase::Preflop,
            dealer_seat_index: 0,
            small_blind_seat_index: 0,
            big_blind_seat_index: 1,
            board_cards: vec![],
            hole_cards_by_player_id: hole_cards,
            participation_by_player_id: participation,
            betting_round: BettingRoundState {
                street: StreetPhase::Preflop,
                current_bet: 0,
                min_raise_to: None,
                max_raise_to: None,
                pot_size: 0,
                contributions_by_player_id: BTreeMap::new(),
            },
            action_window: None,
        });

        let bundle = StateProjector::project(&state).expect("project should succeed");
        let observer_state = bundle
            .private_states
            .get("observer")
            .expect("observer in private_states");
        assert!(
            observer_state.private_hole_cards.is_empty(),
            "eliminated observer should have no private cards"
        );

        let p1_state = bundle
            .private_states
            .get("p1")
            .expect("p1 in private_states");
        assert_eq!(
            p1_state.private_hole_cards.len(),
            2,
            "active player retains cards"
        );
    }

    // T8.3 — can_act flag set only for action window owner
    #[test]
    fn project_can_act_set_only_for_action_window_owner() {
        let mut state = minimal_state();
        state.current_hand = Some(HandState {
            hand_number: 1,
            cycle_phase: HandCyclePhase::AwaitingAction,
            street: StreetPhase::Preflop,
            dealer_seat_index: 0,
            small_blind_seat_index: 0,
            big_blind_seat_index: 1,
            board_cards: vec![],
            hole_cards_by_player_id: BTreeMap::new(),
            participation_by_player_id: BTreeMap::new(),
            betting_round: BettingRoundState {
                street: StreetPhase::Preflop,
                current_bet: 0,
                min_raise_to: None,
                max_raise_to: None,
                pot_size: 0,
                contributions_by_player_id: BTreeMap::new(),
            },
            action_window: Some(ActionWindow {
                action_window_id: "w1".to_string(),
                player_id: "p1".to_string(),
                seat_index: 0,
                legal_actions: vec![],
                call_amount: 0,
                min_raise_to: None,
                max_raise_to: None,
                deadline_epoch_ms: 99_999,
            }),
        });

        let bundle = StateProjector::project(&state).expect("project should succeed");

        let p1 = bundle.private_states.get("p1").expect("p1");
        assert!(
            p1.can_act,
            "p1 owns the action window so can_act must be true"
        );
        assert_eq!(p1.action_window_player_id, Some("p1".to_string()));

        let p2 = bundle.private_states.get("p2").expect("p2");
        assert!(!p2.can_act, "p2 does not own the window");
        assert_eq!(p2.action_window_player_id, None);
    }

    // T8.4 — observer_projection always has empty cards and no action
    #[test]
    fn project_observer_projection_has_empty_cards_and_no_action() {
        let state = minimal_state();
        let bundle = StateProjector::project(&state).expect("project should succeed");

        let obs = &bundle.observer_projection;
        assert!(obs.private_hole_cards.is_empty());
        assert!(!obs.can_act);
        assert_eq!(obs.action_window_player_id, None);
        assert!(obs.is_observer);
    }

    // T8.5 — project_public_state — public fields populated correctly
    #[test]
    fn project_public_state_fields_match_tournament_state() {
        let state = minimal_state();
        let bundle = StateProjector::project(&state).expect("project should succeed");
        let public = &bundle.public_state;

        assert_eq!(public.tournament_name, "Projector Test");
        assert_eq!(public.phase, TournamentPhase::Running);
        assert!(
            public.board_cards.is_empty(),
            "no current hand → no board cards"
        );
        assert_eq!(public.action_window_player_id, None);
    }

    #[test]
    fn project_public_state_board_cards_populated_from_hand() {
        let mut state = minimal_state();
        let board = vec![Card {
            rank: Rank::Two,
            suit: Suit::Clubs,
        }];
        state.current_hand = Some(HandState {
            hand_number: 1,
            cycle_phase: HandCyclePhase::AwaitingAction,
            street: StreetPhase::Flop,
            dealer_seat_index: 0,
            small_blind_seat_index: 0,
            big_blind_seat_index: 1,
            board_cards: board.clone(),
            hole_cards_by_player_id: BTreeMap::new(),
            participation_by_player_id: BTreeMap::new(),
            betting_round: BettingRoundState {
                street: StreetPhase::Flop,
                current_bet: 0,
                min_raise_to: None,
                max_raise_to: None,
                pot_size: 0,
                contributions_by_player_id: BTreeMap::new(),
            },
            action_window: None,
        });
        let bundle = StateProjector::project(&state).expect("project should succeed");
        assert_eq!(bundle.public_state.board_cards, board);
    }

    #[test]
    fn project_public_state_blind_level_label_in_range() {
        let state = minimal_state();
        let bundle = StateProjector::project(&state).expect("project should succeed");
        assert_eq!(
            bundle.public_state.blind_level_label,
            Some("L1".to_string())
        );
    }

    #[test]
    fn project_public_state_blind_level_label_out_of_range() {
        let mut state = minimal_state();
        state.blind_level_index = 99; // beyond the one level
        let bundle = StateProjector::project(&state).expect("project should succeed");
        assert_eq!(bundle.public_state.blind_level_label, None);
    }
}
