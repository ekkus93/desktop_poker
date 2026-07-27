use std::collections::BTreeMap;

use super::super::{
    apply_private_hole_cards_to_snapshot, apply_public_event_to_snapshot,
    blind_schedule_for_preset, build_table_history_for_state, build_table_view_snapshot,
    ensure_legal, format_marker, format_phase, format_street, issue_join_token,
    parse_protocol_street, resolve_action_request, screen_catalog, DesktopAppState,
    DesktopTableActionKind, TableViewerMode,
};
use crate::{
    domain::{
        ActionType, BettingRoundState, Card, ConnectionState, HandCyclePhase,
        HandParticipationState, HandResult, ParticipantRegistryEntry, ParticipantState,
        PlayerIdentity, PotSummary, Rank, SeatMarker, SeatOccupancyState, SeatState, StreetPhase,
        Suit, TournamentConfig, TournamentPhase, TournamentSeatState, TournamentState,
    },
    protocol::{self},
};

use super::support::*;

#[test]
fn resolve_action_request_covers_fold_check_call_bet_raise_and_all_in_paths() {
    let check_window = sample_action_window(vec![ActionType::Check, ActionType::Bet]);
    let call_window = sample_action_window(vec![ActionType::Call, ActionType::Raise]);
    let all_in_window = sample_action_window(vec![ActionType::Fold, ActionType::AllIn]);

    assert_eq!(
        resolve_action_request(&all_in_window, DesktopTableActionKind::Fold, None)
            .expect("fold should resolve"),
        (ActionType::Fold, None, "Fold"),
    );
    assert_eq!(
        resolve_action_request(&check_window, DesktopTableActionKind::CheckOrCall, None)
            .expect("check should resolve"),
        (ActionType::Check, None, "Check"),
    );
    assert_eq!(
        resolve_action_request(&call_window, DesktopTableActionKind::CheckOrCall, None)
            .expect("call should resolve"),
        (ActionType::Call, None, "Call"),
    );
    assert_eq!(
        resolve_action_request(&check_window, DesktopTableActionKind::BetOrRaise, Some(120))
            .expect("bet should resolve"),
        (ActionType::Bet, Some(120), "Bet"),
    );
    assert_eq!(
        resolve_action_request(&call_window, DesktopTableActionKind::BetOrRaise, Some(120))
            .expect("raise should resolve"),
        (ActionType::Raise, Some(120), "Raise"),
    );
    assert_eq!(
        resolve_action_request(&all_in_window, DesktopTableActionKind::AllIn, None)
            .expect("all-in should resolve"),
        (ActionType::AllIn, None, "All-in"),
    );
    assert_eq!(
        resolve_action_request(&call_window, DesktopTableActionKind::BetOrRaise, None)
            .expect_err("missing raise amount should fail"),
        "raise amount is required for bet / raise"
    );
}

#[test]
fn ensure_legal_and_format_helpers_return_expected_contract_strings() {
    let window = sample_action_window(vec![ActionType::Call, ActionType::Raise]);

    assert!(ensure_legal(&window, ActionType::Raise).is_ok());
    assert_eq!(
        ensure_legal(&window, ActionType::Fold).expect_err("fold should be illegal"),
        "Fold is not legal in the current action window"
    );
    assert_eq!(
        format_phase(TournamentPhase::WaitingForPlayers),
        "Waiting for players"
    );
    assert_eq!(format_phase(TournamentPhase::Running), "Running");
    assert_eq!(format_street(StreetPhase::Preflop), "Preflop");
    assert_eq!(format_street(StreetPhase::River), "River");
    assert_eq!(format_marker(SeatMarker::Dealer), "Dealer");
    assert_eq!(format_marker(SeatMarker::BigBlind), "Big blind");
}

#[test]
fn blind_schedule_presets_match_the_canonical_v1_structure() {
    let fast = blind_schedule_for_preset("fast").expect("fast blind schedule");
    let normal = blind_schedule_for_preset("normal").expect("normal blind schedule");
    let slow = blind_schedule_for_preset("slow").expect("slow blind schedule");

    for (schedule, expected_duration) in [(&fast, 180), (&normal, 300), (&slow, 480)] {
        assert_eq!(schedule.levels.len(), 12);
        assert_eq!(
            schedule.levels.first().map(|level| level.small_blind),
            Some(10)
        );
        assert_eq!(
            schedule.levels.first().map(|level| level.big_blind),
            Some(20)
        );
        assert_eq!(
            schedule.levels.last().map(|level| level.small_blind),
            Some(800)
        );
        assert_eq!(
            schedule.levels.last().map(|level| level.big_blind),
            Some(1600)
        );
        assert!(schedule
            .levels
            .iter()
            .all(|level| level.duration_seconds == expected_duration));
    }
}

#[test]
fn join_tokens_are_random_and_url_safe() {
    let first = issue_join_token();
    let second = issue_join_token();

    assert!(!first.is_empty());
    assert!(!second.is_empty());
    assert_ne!(first, second);
    assert!(!first.contains(':'));
    assert!(first
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_')));
}

#[test]
fn debug_state_is_blocked_when_debug_tools_are_disabled() {
    let mut state = DesktopAppState::detect();
    state.bootstrap.debug_tools_enabled = false;
    state.bootstrap.screens = screen_catalog(false);

    assert_eq!(
        state
            .debug_state(TableViewerMode::Local)
            .expect_err("release mode should block debug state"),
        "debug tools are unavailable in release builds"
    );
}

fn snapshot_test_state() -> TournamentState {
    let blind_schedule = blind_schedule_for_preset("normal").expect("normal blind schedule");
    TournamentState {
        table_id: "table-test".to_string(),
        session_epoch: 7,
        phase: TournamentPhase::Running,
        config: TournamentConfig {
            tournament_name: "Observer Table".to_string(),
            table_name: Some("Main Table".to_string()),
            max_players: 3,
            starting_stack: 1_500,
            turn_timer_seconds: 30,
            blind_schedule: blind_schedule.clone(),
        },
        blind_schedule,
        blind_level_index: 0,
        participants: [
            (
                "observer".to_string(),
                ParticipantRegistryEntry {
                    identity: PlayerIdentity {
                        player_id: "observer".to_string(),
                        display_name: "Observer".to_string(),
                        signing_public_key: "sign-observer".to_string(),
                        encryption_public_key: "enc-observer".to_string(),
                        signing_key_fingerprint: "fp-observer".to_string(),
                    },
                    state: ParticipantState::EliminatedObserver,
                    connection_state: ConnectionState::Connected,
                    seat_index: Some(0),
                    admitted_at_ms: 1,
                    reconnect_token: None,
                    reconnect_expiry_ms: None,
                    is_host: false,
                },
            ),
            (
                "showdown".to_string(),
                ParticipantRegistryEntry {
                    identity: PlayerIdentity {
                        player_id: "showdown".to_string(),
                        display_name: "Showdown".to_string(),
                        signing_public_key: "sign-showdown".to_string(),
                        encryption_public_key: "enc-showdown".to_string(),
                        signing_key_fingerprint: "fp-showdown".to_string(),
                    },
                    state: ParticipantState::Active,
                    connection_state: ConnectionState::Connected,
                    seat_index: Some(1),
                    admitted_at_ms: 1,
                    reconnect_token: None,
                    reconnect_expiry_ms: None,
                    is_host: false,
                },
            ),
            (
                "folded".to_string(),
                ParticipantRegistryEntry {
                    identity: PlayerIdentity {
                        player_id: "folded".to_string(),
                        display_name: "Folded".to_string(),
                        signing_public_key: "sign-folded".to_string(),
                        encryption_public_key: "enc-folded".to_string(),
                        signing_key_fingerprint: "fp-folded".to_string(),
                    },
                    state: ParticipantState::Active,
                    connection_state: ConnectionState::Connected,
                    seat_index: Some(2),
                    admitted_at_ms: 1,
                    reconnect_token: None,
                    reconnect_expiry_ms: None,
                    is_host: false,
                },
            ),
        ]
        .into_iter()
        .collect(),
        seats: vec![
            SeatState {
                seat_index: 0,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: TournamentSeatState::EliminatedObserver,
                participant_id: Some("observer".to_string()),
                display_name: Some("Observer".to_string()),
                chip_count: Some(0),
                is_ready: true,
                marker: None,
            },
            SeatState {
                seat_index: 1,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: TournamentSeatState::Active,
                participant_id: Some("showdown".to_string()),
                display_name: Some("Showdown".to_string()),
                chip_count: Some(1_250),
                is_ready: false,
                marker: None,
            },
            SeatState {
                seat_index: 2,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: TournamentSeatState::Active,
                participant_id: Some("folded".to_string()),
                display_name: Some("Folded".to_string()),
                chip_count: Some(250),
                is_ready: false,
                marker: None,
            },
        ],
        current_hand: Some(crate::domain::HandState {
            hand_number: 9,
            cycle_phase: HandCyclePhase::Settlement,
            street: StreetPhase::Showdown,
            dealer_seat_index: 0,
            small_blind_seat_index: 1,
            big_blind_seat_index: 2,
            board_cards: vec![Card {
                rank: Rank::Ace,
                suit: Suit::Clubs,
            }],
            hole_cards_by_player_id: [
                (
                    "showdown".to_string(),
                    vec![
                        Card {
                            rank: Rank::King,
                            suit: Suit::Hearts,
                        },
                        Card {
                            rank: Rank::King,
                            suit: Suit::Diamonds,
                        },
                    ],
                ),
                (
                    "folded".to_string(),
                    vec![
                        Card {
                            rank: Rank::Two,
                            suit: Suit::Spades,
                        },
                        Card {
                            rank: Rank::Three,
                            suit: Suit::Spades,
                        },
                    ],
                ),
            ]
            .into_iter()
            .collect(),
            participation_by_player_id: [
                ("showdown".to_string(), HandParticipationState::AllIn),
                ("folded".to_string(), HandParticipationState::Folded),
            ]
            .into_iter()
            .collect(),
            betting_round: BettingRoundState {
                street: StreetPhase::Showdown,
                current_bet: 0,
                min_raise_to: None,
                max_raise_to: None,
                pot_size: 200,
                contributions_by_player_id: BTreeMap::new(),
            },
            action_window: None,
        }),
        hand_results: vec![HandResult {
            hand_number: 8,
            winning_player_ids: vec!["showdown".to_string()],
            pot_summaries: vec![PotSummary {
                pot_index: 0,
                amount: 120,
                eligible_player_ids: vec!["showdown".to_string()],
                winner_player_ids: vec!["showdown".to_string()],
                odd_chip_count: 0,
                odd_chip_awarded_to: None,
            }],
            board_cards: vec![Card {
                rank: Rank::Queen,
                suit: Suit::Clubs,
            }],
            revealed_hands_by_player_id: BTreeMap::new(),
            eliminated_player_ids: Vec::new(),
            final_stack_by_player_id: [("showdown".to_string(), 1_620)].into_iter().collect(),
        }],
        placements: Vec::new(),
    }
}

#[test]
fn observer_table_view_shows_public_showdown_cards_but_not_folded_private_cards() {
    let state = snapshot_test_state();
    let view = build_table_view_snapshot(
        &state,
        "observer",
        TableViewerMode::Observer,
        false,
        Vec::new(),
    )
    .expect("observer table view");

    let showdown_seat = view
        .seats
        .iter()
        .find(|seat| seat.display_name == "Showdown")
        .expect("showdown seat");
    let folded_seat = view
        .seats
        .iter()
        .find(|seat| seat.display_name == "Folded")
        .expect("folded seat");

    assert!(!showdown_seat.cards_hidden);
    assert_eq!(showdown_seat.hole_cards.len(), 2);
    assert!(folded_seat.cards_hidden);
    assert!(folded_seat.hole_cards.is_empty());
}

#[test]
fn hand_history_uses_stored_completed_hand_boards() {
    let mut state = snapshot_test_state();
    state
        .current_hand
        .as_mut()
        .expect("current hand")
        .board_cards = vec![Card {
        rank: Rank::Ten,
        suit: Suit::Hearts,
    }];

    let history = build_table_history_for_state(&state);

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].board_cards[0].label, "Queen of Clubs");
}

#[test]
fn hand_result_application_uses_final_stack_values_when_present() {
    let mut state = snapshot_test_state();
    let event = serde_json::to_value(protocol::HandResultCommitted {
        hand_number: 9,
        result: HandResult {
            hand_number: 9,
            winning_player_ids: vec!["showdown".to_string()],
            pot_summaries: vec![PotSummary {
                pot_index: 0,
                amount: 101,
                eligible_player_ids: vec!["showdown".to_string()],
                winner_player_ids: vec!["showdown".to_string()],
                odd_chip_count: 0,
                odd_chip_awarded_to: None,
            }],
            board_cards: vec![Card {
                rank: Rank::Ace,
                suit: Suit::Clubs,
            }],
            revealed_hands_by_player_id: BTreeMap::new(),
            eliminated_player_ids: Vec::new(),
            final_stack_by_player_id: [
                ("showdown".to_string(), 1_351),
                ("folded".to_string(), 249),
            ]
            .into_iter()
            .collect(),
        },
    })
    .expect("hand result payload");

    apply_public_event_to_snapshot(
        &mut state,
        "observer",
        protocol::ProtocolMessageType::HandResultCommittedEvent,
        &event,
    );

    assert_eq!(state.seats[1].chip_count, Some(1_351));
    assert_eq!(state.seats[2].chip_count, Some(249));
}

#[test]
fn odd_chip_fallback_awards_the_extra_chip_to_the_declared_recipient() {
    let mut state = snapshot_test_state();
    state.seats[1].chip_count = Some(10);
    state.seats[2].chip_count = Some(10);
    let event = serde_json::to_value(protocol::HandResultCommitted {
        hand_number: 9,
        result: HandResult {
            hand_number: 9,
            winning_player_ids: vec!["showdown".to_string(), "folded".to_string()],
            pot_summaries: vec![PotSummary {
                pot_index: 0,
                amount: 5,
                eligible_player_ids: vec!["showdown".to_string(), "folded".to_string()],
                winner_player_ids: vec!["showdown".to_string(), "folded".to_string()],
                odd_chip_count: 1,
                odd_chip_awarded_to: Some("showdown".to_string()),
            }],
            board_cards: vec![Card {
                rank: Rank::Ace,
                suit: Suit::Clubs,
            }],
            revealed_hands_by_player_id: BTreeMap::new(),
            eliminated_player_ids: Vec::new(),
            final_stack_by_player_id: BTreeMap::new(),
        },
    })
    .expect("hand result payload");

    apply_public_event_to_snapshot(
        &mut state,
        "observer",
        protocol::ProtocolMessageType::HandResultCommittedEvent,
        &event,
    );

    assert_eq!(state.seats[1].chip_count, Some(13));
    assert_eq!(state.seats[2].chip_count, Some(12));
}

// T9.1 — parse_protocol_street all valid values and case variants

#[test]
fn parse_protocol_street_all_valid_variants_and_case_insensitive() {
    assert_eq!(parse_protocol_street("PREFLOP"), Some(StreetPhase::Preflop));
    assert_eq!(parse_protocol_street("flop"), Some(StreetPhase::Flop));
    assert_eq!(parse_protocol_street("Turn"), Some(StreetPhase::Turn));
    assert_eq!(parse_protocol_street("RIVER"), Some(StreetPhase::River));
    assert_eq!(
        parse_protocol_street("SHOWDOWN"),
        Some(StreetPhase::Showdown)
    );
}

#[test]
fn parse_protocol_street_returns_none_for_unknown_and_empty() {
    assert_eq!(parse_protocol_street(""), None);
    assert_eq!(parse_protocol_street("DEAL"), None);
}

#[test]
fn parse_protocol_street_trims_whitespace() {
    assert_eq!(parse_protocol_street(" FLOP "), Some(StreetPhase::Flop));
}

// T9.2 — apply_private_hole_cards_to_snapshot inserts cards into hand

#[test]
fn apply_private_hole_cards_inserts_recipient_cards_into_current_hand() {
    use crate::domain::{
        BettingRoundState, BlindLevel, BlindSchedule, HandCyclePhase, HandState, TournamentConfig,
    };
    use crate::protocol::PrivateHoleCardsEvent;

    let config = TournamentConfig {
        tournament_name: "Test".to_string(),
        table_name: None,
        max_players: 2,
        starting_stack: 1000,
        turn_timer_seconds: 10,
        blind_schedule: BlindSchedule {
            levels: vec![BlindLevel {
                level_index: 1,
                label: "L1".to_string(),
                small_blind: 10,
                big_blind: 20,
                ante: 0,
                duration_seconds: 60,
            }],
        },
    };

    let mut state = TournamentState {
        table_id: "t".to_string(),
        session_epoch: 1,
        phase: TournamentPhase::Running,
        config: config.clone(),
        blind_schedule: config.blind_schedule.clone(),
        blind_level_index: 0,
        participants: BTreeMap::new(),
        seats: vec![],
        hand_results: vec![],
        placements: vec![],
        current_hand: Some(HandState {
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
            action_window: None,
        }),
    };

    let cards = vec![
        Card {
            rank: Rank::Ace,
            suit: Suit::Spades,
        },
        Card {
            rank: Rank::King,
            suit: Suit::Hearts,
        },
    ];
    let event = PrivateHoleCardsEvent {
        recipient_player_id: "player-a".to_string(),
        hole_cards: cards.clone(),
    };

    apply_private_hole_cards_to_snapshot(&mut state, &event);

    let hand = state.current_hand.as_ref().unwrap();
    assert_eq!(hand.hole_cards_by_player_id.get("player-a"), Some(&cards));
}

// T9.3 — apply_private_hole_cards_to_snapshot no-op when no current hand

#[test]
fn apply_private_hole_cards_no_op_when_no_current_hand() {
    use crate::domain::{BlindLevel, BlindSchedule, TournamentConfig};
    use crate::protocol::PrivateHoleCardsEvent;

    let config = TournamentConfig {
        tournament_name: "Test".to_string(),
        table_name: None,
        max_players: 2,
        starting_stack: 1000,
        turn_timer_seconds: 10,
        blind_schedule: BlindSchedule {
            levels: vec![BlindLevel {
                level_index: 1,
                label: "L1".to_string(),
                small_blind: 10,
                big_blind: 20,
                ante: 0,
                duration_seconds: 60,
            }],
        },
    };

    let mut state = TournamentState {
        table_id: "t".to_string(),
        session_epoch: 1,
        phase: TournamentPhase::Running,
        config: config.clone(),
        blind_schedule: config.blind_schedule.clone(),
        blind_level_index: 0,
        participants: BTreeMap::new(),
        seats: vec![],
        hand_results: vec![],
        placements: vec![],
        current_hand: None,
    };

    let event = PrivateHoleCardsEvent {
        recipient_player_id: "player-a".to_string(),
        hole_cards: vec![Card {
            rank: Rank::Ace,
            suit: Suit::Spades,
        }],
    };

    // Must not panic
    apply_private_hole_cards_to_snapshot(&mut state, &event);
    assert!(state.current_hand.is_none());
}

#[test]
fn tournament_complete_clears_hand_and_reconciles_final_placements() {
    let mut state = snapshot_test_state();
    let mut final_result = state.hand_results[0].clone();
    final_result.hand_number = 9;
    final_result.final_stack_by_player_id =
        [("showdown".to_string(), 1_500), ("folded".to_string(), 0)]
            .into_iter()
            .collect();
    state.hand_results.push(final_result);

    state.participants.get_mut("showdown").unwrap().state = ParticipantState::EliminatedObserver;
    state.seats[1].tournament_state = TournamentSeatState::EliminatedObserver;
    state.seats[1].chip_count = Some(0);

    let placements = vec![
        crate::domain::PlacementEntry {
            player_id: "showdown".to_string(),
            place: 1,
            busted_at_hand_number: None,
        },
        crate::domain::PlacementEntry {
            player_id: "folded".to_string(),
            place: 2,
            busted_at_hand_number: Some(9),
        },
    ];
    let payload = serde_json::to_value(protocol::TournamentCompleteEvent {
        winner_player_id: "showdown".to_string(),
        placements: placements.clone(),
    })
    .expect("completion payload");

    apply_public_event_to_snapshot(
        &mut state,
        "showdown",
        protocol::ProtocolMessageType::TournamentCompleteEvent,
        &payload,
    );

    assert_eq!(state.phase, TournamentPhase::Complete);
    assert!(state.current_hand.is_none());
    assert_eq!(state.placements, placements);
    assert_eq!(
        state.participants["showdown"].state,
        ParticipantState::Active
    );
    assert_eq!(
        state.participants["folded"].state,
        ParticipantState::EliminatedObserver
    );
    assert_eq!(state.seats[1].tournament_state, TournamentSeatState::Active);
    assert_eq!(state.seats[1].chip_count, Some(1_500));
    assert_eq!(state.seats[1].marker, Some(SeatMarker::Dealer));
    assert_eq!(
        state.seats[2].tournament_state,
        TournamentSeatState::EliminatedObserver
    );
    assert_eq!(state.seats[2].chip_count, Some(0));
    assert_eq!(state.seats[2].marker, None);
}
