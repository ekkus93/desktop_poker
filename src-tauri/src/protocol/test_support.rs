#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::Serialize;

use crate::domain::{
    ActionType, ActionWindow, BettingRoundState, BlindLevel, BlindSchedule, Card, ConnectionState,
    HandCyclePhase, HandParticipationState, HandResult, HandState, ParticipantRegistryEntry,
    ParticipantState, PlayerIdentity, Rank, SeatOccupancyState, SeatState, StreetPhase, Suit,
    TournamentConfig, TournamentPhase, TournamentSeatState, TournamentState,
};

use super::{
    canonical_json_bytes, ActionRejectedEvent, ActionWindowOpened, CanonicalJsonFixture,
    JoinTournamentRequest, ReconnectTournamentRequest, ResyncRequest, SnapshotEvent,
};

pub(crate) fn assert_canonical_fixture<T: Serialize>(fixture: CanonicalJsonFixture, value: &T) {
    let actual = String::from_utf8(canonical_json_bytes(value).expect("canonical json"))
        .expect("utf8 fixture");

    assert_eq!(
        actual, fixture.expected_json,
        "{} fixture drifted",
        fixture.name
    );
}

pub(crate) fn sample_join_request() -> JoinTournamentRequest {
    JoinTournamentRequest {
        display_name: "Alice".to_string(),
        join_token: "join-token".to_string(),
        signing_public_key: "sign-key".to_string(),
        encryption_public_key: "enc-key".to_string(),
    }
}

pub(crate) fn sample_reconnect_request(
    last_known_server_seq: Option<u64>,
) -> ReconnectTournamentRequest {
    ReconnectTournamentRequest {
        player_id: "player-a".to_string(),
        reconnect_token: "reconnect-token".to_string(),
        last_known_server_seq,
    }
}

pub(crate) fn sample_resync_request(last_seen_server_sequence: Option<u64>) -> ResyncRequest {
    ResyncRequest {
        last_seen_server_sequence,
    }
}

pub(crate) fn sample_action_window_opened(
    min_raise_to: Option<u32>,
    max_raise_to: Option<u32>,
) -> ActionWindowOpened {
    ActionWindowOpened {
        hand_number: 7,
        hand_phase: "AWAITING_ACTION".to_string(),
        action_window_id: "window-7".to_string(),
        player_id: "player-a".to_string(),
        seat_index: 1,
        legal_actions: vec![ActionType::Fold, ActionType::Call, ActionType::Raise],
        call_amount: 40,
        min_raise_to,
        max_raise_to,
        deadline_epoch_ms: 1_700_000_000_000,
    }
}

pub(crate) fn sample_action_rejected() -> ActionRejectedEvent {
    ActionRejectedEvent {
        seat_index: 1,
        action_type: ActionType::Raise,
        reason: "raise too small".to_string(),
    }
}

pub(crate) fn sample_private_hole_cards_event() -> super::PrivateHoleCardsEvent {
    super::PrivateHoleCardsEvent {
        recipient_player_id: "player-a".to_string(),
        hole_cards: vec![
            Card {
                rank: Rank::Ace,
                suit: Suit::Spades,
            },
            Card {
                rank: Rank::Ace,
                suit: Suit::Hearts,
            },
        ],
    }
}

pub(crate) fn sample_tournament_started_event() -> super::TournamentStartedEvent {
    super::TournamentStartedEvent {
        tournament_name: "Protocol Table".to_string(),
        starting_stack: 1500,
        blind_schedule_preset: "FAST".to_string(),
        frozen_player_ids: vec!["player-a".to_string(), "player-b".to_string()],
    }
}

pub(crate) fn sample_showdown_started_event() -> super::ShowdownStarted {
    super::ShowdownStarted {
        hand_number: 7,
        board_cards: vec![Card {
            rank: Rank::Ace,
            suit: Suit::Clubs,
        }],
    }
}

pub(crate) fn sample_hand_result_committed() -> super::HandResultCommitted {
    super::HandResultCommitted {
        hand_number: 7,
        result: HandResult {
            hand_number: 7,
            winning_player_ids: vec!["player-a".to_string()],
            pot_summaries: Vec::new(),
            revealed_hands_by_player_id: BTreeMap::new(),
            eliminated_player_ids: Vec::new(),
        },
    }
}

pub(crate) fn sample_tournament_state() -> TournamentState {
    let blind_schedule = BlindSchedule {
        levels: vec![BlindLevel {
            level_index: 1,
            label: "Level 1".to_string(),
            small_blind: 10,
            big_blind: 20,
            ante: 0,
            duration_seconds: 180,
        }],
    };

    let participants = [
        (
            "player-a".to_string(),
            ParticipantRegistryEntry {
                identity: PlayerIdentity {
                    player_id: "player-a".to_string(),
                    display_name: "Alice".to_string(),
                    signing_public_key: "sign-a".to_string(),
                    encryption_public_key: "enc-a".to_string(),
                    signing_key_fingerprint: "fingerprint-a".to_string(),
                },
                state: ParticipantState::Active,
                connection_state: ConnectionState::Connected,
                seat_index: Some(0),
                admitted_at_ms: 1,
                reconnect_token: Some("reconnect-a".to_string()),
                reconnect_expiry_ms: None,
                is_host: false,
            },
        ),
        (
            "player-b".to_string(),
            ParticipantRegistryEntry {
                identity: PlayerIdentity {
                    player_id: "player-b".to_string(),
                    display_name: "Bob".to_string(),
                    signing_public_key: "sign-b".to_string(),
                    encryption_public_key: "enc-b".to_string(),
                    signing_key_fingerprint: "fingerprint-b".to_string(),
                },
                state: ParticipantState::Active,
                connection_state: ConnectionState::Connected,
                seat_index: Some(1),
                admitted_at_ms: 2,
                reconnect_token: Some("reconnect-b".to_string()),
                reconnect_expiry_ms: None,
                is_host: false,
            },
        ),
        (
            "player-c".to_string(),
            ParticipantRegistryEntry {
                identity: PlayerIdentity {
                    player_id: "player-c".to_string(),
                    display_name: "Casey".to_string(),
                    signing_public_key: "sign-c".to_string(),
                    encryption_public_key: "enc-c".to_string(),
                    signing_key_fingerprint: "fingerprint-c".to_string(),
                },
                state: ParticipantState::EliminatedObserver,
                connection_state: ConnectionState::Connected,
                seat_index: Some(2),
                admitted_at_ms: 3,
                reconnect_token: Some("reconnect-c".to_string()),
                reconnect_expiry_ms: None,
                is_host: false,
            },
        ),
    ]
    .into_iter()
    .collect();

    TournamentState {
        table_id: "table-1".to_string(),
        session_epoch: 7,
        phase: TournamentPhase::Running,
        config: TournamentConfig {
            tournament_name: "Protocol Table".to_string(),
            table_name: Some("Main".to_string()),
            max_players: 6,
            starting_stack: 1500,
            turn_timer_seconds: 20,
            blind_schedule: blind_schedule.clone(),
        },
        blind_schedule,
        blind_level_index: 0,
        participants,
        seats: vec![
            SeatState {
                seat_index: 0,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: TournamentSeatState::Active,
                participant_id: Some("player-a".to_string()),
                display_name: Some("Alice".to_string()),
                chip_count: Some(1200),
                is_ready: true,
                marker: None,
            },
            SeatState {
                seat_index: 1,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: TournamentSeatState::Active,
                participant_id: Some("player-b".to_string()),
                display_name: Some("Bob".to_string()),
                chip_count: Some(1600),
                is_ready: true,
                marker: None,
            },
            SeatState {
                seat_index: 2,
                occupancy: SeatOccupancyState::Occupied,
                tournament_state: TournamentSeatState::EliminatedObserver,
                participant_id: Some("player-c".to_string()),
                display_name: Some("Casey".to_string()),
                chip_count: Some(0),
                is_ready: true,
                marker: None,
            },
        ],
        current_hand: Some(HandState {
            hand_number: 7,
            cycle_phase: HandCyclePhase::AwaitingAction,
            street: StreetPhase::Turn,
            dealer_seat_index: 0,
            small_blind_seat_index: 0,
            big_blind_seat_index: 1,
            board_cards: vec![Card {
                rank: Rank::King,
                suit: Suit::Diamonds,
            }],
            hole_cards_by_player_id: [
                (
                    "player-a".to_string(),
                    vec![
                        Card {
                            rank: Rank::Ace,
                            suit: Suit::Spades,
                        },
                        Card {
                            rank: Rank::Ace,
                            suit: Suit::Hearts,
                        },
                    ],
                ),
                (
                    "player-b".to_string(),
                    vec![
                        Card {
                            rank: Rank::King,
                            suit: Suit::Spades,
                        },
                        Card {
                            rank: Rank::Queen,
                            suit: Suit::Hearts,
                        },
                    ],
                ),
            ]
            .into_iter()
            .collect(),
            participation_by_player_id: [
                ("player-a".to_string(), HandParticipationState::Active),
                ("player-b".to_string(), HandParticipationState::Active),
            ]
            .into_iter()
            .collect(),
            betting_round: BettingRoundState {
                street: StreetPhase::Turn,
                current_bet: 40,
                min_raise_to: Some(80),
                max_raise_to: Some(300),
                pot_size: 200,
                contributions_by_player_id: [
                    ("player-a".to_string(), 40),
                    ("player-b".to_string(), 40),
                ]
                .into_iter()
                .collect(),
            },
            action_window: Some(ActionWindow {
                action_window_id: "window-7".to_string(),
                player_id: "player-a".to_string(),
                seat_index: 0,
                legal_actions: vec![ActionType::Fold, ActionType::Call, ActionType::Raise],
                call_amount: 40,
                min_raise_to: Some(80),
                max_raise_to: Some(300),
                deadline_epoch_ms: 1_700_000_000_000,
            }),
        }),
        hand_results: vec![HandResult {
            hand_number: 6,
            winning_player_ids: vec!["player-b".to_string()],
            pot_summaries: Vec::new(),
            revealed_hands_by_player_id: BTreeMap::new(),
            eliminated_player_ids: Vec::new(),
        }],
        placements: Vec::new(),
    }
}

pub(crate) fn sample_snapshot_event(local_player_id: &str) -> SnapshotEvent {
    SnapshotEvent {
        state: crate::domain::SnapshotState {
            state: sample_tournament_state(),
            local_player_id: local_player_id.to_string(),
            reconnect_token: Some(format!("reconnect-{local_player_id}")),
            host_signing_public_key: Some("host-sign".to_string()),
            host_encryption_public_key: Some("host-enc".to_string()),
        },
        local_player_id: local_player_id.to_string(),
        reconnect_token: Some(format!("reconnect-{local_player_id}")),
        host_signing_public_key: Some("host-sign".to_string()),
        host_encryption_public_key: Some("host-enc".to_string()),
    }
}
