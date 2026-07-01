use std::collections::BTreeMap;

use super::super::{
    active_seat_count_for_state, blind_schedule_for_preset, build_session_participants,
    client_snapshot_state_from_event, format_connection_state_value,
    format_participant_state_value, format_tournament_phase_value,
};
use crate::{
    domain::{
        BettingRoundState, BlindLevel, BlindSchedule, Card, ConnectionState, HandCyclePhase,
        ParticipantState, Rank, SeatOccupancyState, SeatState, StreetPhase, Suit, TournamentConfig,
        TournamentPhase, TournamentSeatState, TournamentState,
    },
    protocol::{RecipientHandSnapshot, RecipientSnapshotState, SnapshotEvent, SnapshotParticipant},
};

// ——— helpers ——————————————————————————————————————————————————

fn blank_tournament_state() -> TournamentState {
    TournamentState {
        table_id: "cfg-test".to_string(),
        session_epoch: 1,
        phase: TournamentPhase::WaitingForPlayers,
        config: TournamentConfig {
            tournament_name: "T".to_string(),
            table_name: None,
            max_players: 2,
            starting_stack: 1000,
            turn_timer_seconds: 10,
            blind_schedule: BlindSchedule { levels: vec![] },
        },
        blind_schedule: BlindSchedule { levels: vec![] },
        blind_level_index: 0,
        participants: BTreeMap::new(),
        seats: vec![],
        hand_results: vec![],
        placements: vec![],
        current_hand: None,
    }
}

fn blank_seat(index: u8, occupied: bool) -> SeatState {
    SeatState {
        seat_index: index,
        occupancy: if occupied {
            SeatOccupancyState::Occupied
        } else {
            SeatOccupancyState::Empty
        },
        tournament_state: TournamentSeatState::Active,
        participant_id: None,
        display_name: None,
        chip_count: None,
        is_ready: true,
        marker: None,
    }
}

fn blank_snapshot_state(_local_player_id: &str) -> RecipientSnapshotState {
    RecipientSnapshotState {
        table_id: "cfg-snap".to_string(),
        session_epoch: 1,
        phase: TournamentPhase::WaitingForPlayers,
        config: TournamentConfig {
            tournament_name: "T".to_string(),
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
        },
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
        blind_level_index: 0,
        participants: BTreeMap::new(),
        seats: vec![],
        current_hand: None,
        hand_results: vec![],
        placements: vec![],
    }
}

fn blank_snapshot_event(local_player_id: &str) -> SnapshotEvent {
    SnapshotEvent {
        state: blank_snapshot_state(local_player_id),
        local_player_id: local_player_id.to_string(),
        private_hole_cards: vec![],
        reconnect_token: None,
        host_signing_public_key: None,
        host_encryption_public_key: None,
    }
}

fn snapshot_participant(
    player_id: &str,
    display_name: &str,
    seat_index: Option<u8>,
) -> SnapshotParticipant {
    SnapshotParticipant {
        player_id: player_id.to_string(),
        display_name: display_name.to_string(),
        seat_index,
        is_host: false,
        is_ready: false,
        connection_state: ConnectionState::Connected,
        participant_state: ParticipantState::Active,
    }
}

fn blank_recipient_hand() -> RecipientHandSnapshot {
    RecipientHandSnapshot {
        hand_number: 1,
        cycle_phase: HandCyclePhase::AwaitingAction,
        street: StreetPhase::Preflop,
        dealer_seat_index: 0,
        small_blind_seat_index: 0,
        big_blind_seat_index: 1,
        board_cards: vec![],
        public_hole_cards_by_player_id: BTreeMap::new(),
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
    }
}

// T7.1 — blind_schedule_for_preset alias presets and unknown

#[test]
fn blind_schedule_for_preset_turbo_matches_fast() {
    let turbo = blind_schedule_for_preset("turbo").expect("turbo should work");
    let fast = blind_schedule_for_preset("fast").expect("fast should work");
    assert_eq!(turbo, fast);
}

#[test]
fn blind_schedule_for_preset_standard_matches_normal() {
    let standard = blind_schedule_for_preset("standard").expect("standard should work");
    let normal = blind_schedule_for_preset("normal").expect("normal should work");
    assert_eq!(standard, normal);
}

#[test]
fn blind_schedule_for_preset_deep_stack_matches_slow() {
    let deep = blind_schedule_for_preset("deep-stack").expect("deep-stack should work");
    let slow = blind_schedule_for_preset("slow").expect("slow should work");
    assert_eq!(deep, slow);
}

#[test]
fn blind_schedule_for_preset_unknown_returns_error() {
    let err = blind_schedule_for_preset("unknown-preset").expect_err("should fail for unknown");
    assert!(
        err.contains("unsupported blindPresetId"),
        "unexpected error: {err}"
    );
}

#[test]
fn blind_schedule_for_preset_trims_whitespace() {
    let padded = blind_schedule_for_preset(" fast ").expect("should trim whitespace");
    let clean = blind_schedule_for_preset("fast").expect("clean should work");
    assert_eq!(padded, clean);
}

// T7.2 — format_connection_state_value all three variants

#[test]
fn format_connection_state_value_all_variants() {
    assert_eq!(
        format_connection_state_value(ConnectionState::Connected),
        "connected"
    );
    assert_eq!(
        format_connection_state_value(ConnectionState::Disconnected),
        "disconnected"
    );
    assert_eq!(
        format_connection_state_value(ConnectionState::Reconnecting),
        "reconnecting"
    );
}

// T7.3 — format_participant_state_value all six variants

#[test]
fn format_participant_state_value_all_variants() {
    assert_eq!(
        format_participant_state_value(ParticipantState::Admitted),
        "admitted"
    );
    assert_eq!(
        format_participant_state_value(ParticipantState::Seated),
        "seated"
    );
    assert_eq!(
        format_participant_state_value(ParticipantState::Active),
        "active"
    );
    assert_eq!(
        format_participant_state_value(ParticipantState::Reconnecting),
        "reconnecting"
    );
    assert_eq!(
        format_participant_state_value(ParticipantState::EliminatedObserver),
        "eliminatedObserver"
    );
    assert_eq!(
        format_participant_state_value(ParticipantState::Removed),
        "removed"
    );
}

// T7.4 — format_tournament_phase_value all five variants

#[test]
fn format_tournament_phase_value_all_variants() {
    assert_eq!(
        format_tournament_phase_value(TournamentPhase::WaitingForPlayers),
        "waitingForPlayers"
    );
    assert_eq!(
        format_tournament_phase_value(TournamentPhase::ReadyCheck),
        "readyCheck"
    );
    assert_eq!(
        format_tournament_phase_value(TournamentPhase::Running),
        "running"
    );
    assert_eq!(
        format_tournament_phase_value(TournamentPhase::Complete),
        "complete"
    );
    assert_eq!(
        format_tournament_phase_value(TournamentPhase::Cancelled),
        "cancelled"
    );
}

// T7.5 — active_seat_count_for_state

#[test]
fn active_seat_count_counts_occupied_seats_only() {
    let mut state = blank_tournament_state();
    state.seats = vec![
        blank_seat(0, true),
        blank_seat(1, true),
        blank_seat(2, true),
        blank_seat(3, false),
        blank_seat(4, false),
    ];
    assert_eq!(active_seat_count_for_state(&state), 3);
}

#[test]
fn active_seat_count_returns_zero_when_all_empty() {
    let mut state = blank_tournament_state();
    state.seats = vec![blank_seat(0, false), blank_seat(1, false)];
    assert_eq!(active_seat_count_for_state(&state), 0);
}

// T7.6 — build_session_participants maps participant fields

#[test]
fn build_session_participants_maps_is_ready_from_linked_seat() {
    use crate::domain::{ParticipantRegistryEntry, PlayerIdentity};

    let mut state = blank_tournament_state();
    let mut seat = blank_seat(0, true);
    seat.participant_id = Some("alice".to_string());
    seat.is_ready = true;
    state.seats = vec![seat];
    state.participants.insert(
        "alice".to_string(),
        ParticipantRegistryEntry {
            identity: PlayerIdentity {
                player_id: "alice".to_string(),
                display_name: "Alice".to_string(),
                signing_public_key: "sign-alice".to_string(),
                encryption_public_key: "enc-alice".to_string(),
                signing_key_fingerprint: "fp-alice".to_string(),
            },
            state: ParticipantState::Active,
            connection_state: ConnectionState::Connected,
            seat_index: Some(0),
            admitted_at_ms: 0,
            reconnect_token: None,
            reconnect_expiry_ms: None,
            is_host: false,
        },
    );

    let participants = build_session_participants(&state);
    assert_eq!(participants.len(), 1);
    assert!(participants[0].is_ready);
    assert_eq!(participants[0].connection_state, "connected");
    assert_eq!(participants[0].participant_state, "active");
}

#[test]
fn build_session_participants_no_seat_index_is_not_ready() {
    use crate::domain::{ParticipantRegistryEntry, PlayerIdentity};

    let mut state = blank_tournament_state();
    state.participants.insert(
        "bob".to_string(),
        ParticipantRegistryEntry {
            identity: PlayerIdentity {
                player_id: "bob".to_string(),
                display_name: "Bob".to_string(),
                signing_public_key: "sign-bob".to_string(),
                encryption_public_key: "enc-bob".to_string(),
                signing_key_fingerprint: "fp-bob".to_string(),
            },
            state: ParticipantState::Admitted,
            connection_state: ConnectionState::Connected,
            seat_index: None,
            admitted_at_ms: 0,
            reconnect_token: None,
            reconnect_expiry_ms: None,
            is_host: false,
        },
    );

    let participants = build_session_participants(&state);
    assert_eq!(participants.len(), 1);
    assert!(!participants[0].is_ready);
    assert_eq!(participants[0].seat_index, None);
}

// T7.7 — client_snapshot_state_from_event

#[test]
fn client_snapshot_state_from_event_merges_private_hole_cards() {
    let mut event = blank_snapshot_event("player-a");
    event.private_hole_cards = vec![
        Card {
            rank: Rank::Ace,
            suit: Suit::Spades,
        },
        Card {
            rank: Rank::King,
            suit: Suit::Hearts,
        },
    ];

    let mut hand = blank_recipient_hand();
    // Bob has public cards (revealed at showdown)
    hand.public_hole_cards_by_player_id.insert(
        "player-b".to_string(),
        vec![Card {
            rank: Rank::Two,
            suit: Suit::Clubs,
        }],
    );
    event.state.current_hand = Some(hand);

    let snap = client_snapshot_state_from_event(&event);
    let hand = snap.state.current_hand.as_ref().unwrap();

    // Alice's private cards were merged
    assert_eq!(
        hand.hole_cards_by_player_id.get("player-a"),
        Some(&event.private_hole_cards)
    );
    // Bob's public cards are present
    assert!(hand.hole_cards_by_player_id.contains_key("player-b"));
}

#[test]
fn client_snapshot_state_from_event_no_private_cards_player_absent_from_hole_cards() {
    let mut event = blank_snapshot_event("player-a");
    event.state.current_hand = Some(blank_recipient_hand());
    // private_hole_cards is empty → player-a should not appear

    let snap = client_snapshot_state_from_event(&event);
    let hand = snap.state.current_hand.as_ref().unwrap();
    assert!(!hand.hole_cards_by_player_id.contains_key("player-a"));
}

#[test]
fn client_snapshot_state_from_event_normalizes_stale_seat_index() {
    let mut event = blank_snapshot_event("player-a");

    // Seat 0 is occupied by player-a
    let mut seat = blank_seat(0, true);
    seat.participant_id = Some("player-a".to_string());
    event.state.seats = vec![seat];

    // player-a has correct seat_index=0; ghost has stale seat_index=0 (links to player-a's seat)
    event.state.participants.insert(
        "player-a".to_string(),
        snapshot_participant("player-a", "Alice", Some(0)),
    );
    event.state.participants.insert(
        "ghost".to_string(),
        snapshot_participant("ghost", "Ghost", Some(0)), // stale: seat 0 belongs to player-a
    );

    let snap = client_snapshot_state_from_event(&event);

    // player-a keeps seat_index=0 (legitimate)
    let alice = snap.state.participants.get("player-a").unwrap();
    assert_eq!(alice.seat_index, Some(0));

    // ghost gets seat_index normalized to None
    let ghost = snap.state.participants.get("ghost").unwrap();
    assert_eq!(ghost.seat_index, None);
}
