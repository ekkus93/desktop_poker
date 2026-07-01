use std::collections::BTreeMap;

use super::super::{
    build_elimination_summary, build_table_history_for_state, build_table_standings_for_state,
    card_view, display_name_for_state, display_names_for_state, format_action,
    format_connection_state, format_marker, format_phase, format_phase_value, format_street,
    format_tournament_seat_state, status_label_for_seat,
};
use crate::domain::{
    ActionType, BlindSchedule, Card, ConnectionState, HandParticipationState, HandResult,
    ParticipantState, PlayerIdentity, PotSummary, Rank, SeatMarker, SeatOccupancyState, SeatState,
    StreetPhase, Suit, TournamentConfig, TournamentPhase, TournamentSeatState, TournamentState,
};

// — helpers ——————————————————————————————————————————————————

fn blank_state() -> TournamentState {
    TournamentState {
        table_id: "test".to_string(),
        session_epoch: 1,
        phase: TournamentPhase::Running,
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

fn blank_seat(index: u8) -> SeatState {
    SeatState {
        seat_index: index,
        occupancy: SeatOccupancyState::Occupied,
        tournament_state: TournamentSeatState::Active,
        participant_id: None,
        display_name: None,
        chip_count: None,
        is_ready: true,
        marker: None,
    }
}

fn participant(player_id: &str, display_name: &str) -> crate::domain::ParticipantRegistryEntry {
    crate::domain::ParticipantRegistryEntry {
        identity: PlayerIdentity {
            player_id: player_id.to_string(),
            display_name: display_name.to_string(),
            signing_public_key: format!("sign-{player_id}"),
            encryption_public_key: format!("enc-{player_id}"),
            signing_key_fingerprint: format!("fp-{player_id}"),
        },
        state: ParticipantState::Active,
        connection_state: ConnectionState::Connected,
        seat_index: None,
        admitted_at_ms: 0,
        reconnect_token: None,
        reconnect_expiry_ms: None,
        is_host: false,
    }
}

fn pot_summary(amount: u32, winner: &str) -> PotSummary {
    PotSummary {
        pot_index: 0,
        amount,
        eligible_player_ids: vec![winner.to_string()],
        winner_player_ids: vec![winner.to_string()],
        odd_chip_count: 0,
        odd_chip_awarded_to: None,
    }
}

fn hand_result(hand_number: u32, winner_id: &str, pot_amount: u32) -> HandResult {
    HandResult {
        hand_number,
        winning_player_ids: vec![winner_id.to_string()],
        pot_summaries: vec![pot_summary(pot_amount, winner_id)],
        board_cards: vec![],
        revealed_hands_by_player_id: BTreeMap::new(),
        eliminated_player_ids: vec![],
        final_stack_by_player_id: BTreeMap::new(),
    }
}

// T5.1 — format_phase remaining variants

#[test]
fn format_phase_ready_check() {
    assert_eq!(format_phase(TournamentPhase::ReadyCheck), "Ready check");
}

#[test]
fn format_phase_complete() {
    assert_eq!(format_phase(TournamentPhase::Complete), "Complete");
}

#[test]
fn format_phase_cancelled() {
    assert_eq!(format_phase(TournamentPhase::Cancelled), "Cancelled");
}

// T5.2 — format_phase_value all five variants

#[test]
fn format_phase_value_all_variants() {
    assert_eq!(
        format_phase_value(TournamentPhase::WaitingForPlayers),
        "waitingForPlayers"
    );
    assert_eq!(
        format_phase_value(TournamentPhase::ReadyCheck),
        "readyCheck"
    );
    assert_eq!(format_phase_value(TournamentPhase::Running), "running");
    assert_eq!(format_phase_value(TournamentPhase::Complete), "complete");
    assert_eq!(format_phase_value(TournamentPhase::Cancelled), "cancelled");
}

// T5.3 — format_street remaining variants

#[test]
fn format_street_flop() {
    assert_eq!(format_street(StreetPhase::Flop), "Flop");
}

#[test]
fn format_street_turn() {
    assert_eq!(format_street(StreetPhase::Turn), "Turn");
}

#[test]
fn format_street_showdown() {
    assert_eq!(format_street(StreetPhase::Showdown), "Showdown");
}

// T5.4 — format_marker SmallBlind

#[test]
fn format_marker_small_blind() {
    assert_eq!(format_marker(SeatMarker::SmallBlind), "Small blind");
}

// T5.5 — format_connection_state all three variants

#[test]
fn format_connection_state_all_variants() {
    assert_eq!(
        format_connection_state(ConnectionState::Connected),
        "Connected"
    );
    assert_eq!(
        format_connection_state(ConnectionState::Disconnected),
        "Disconnected"
    );
    assert_eq!(
        format_connection_state(ConnectionState::Reconnecting),
        "Reconnecting"
    );
}

// T5.6 — format_tournament_seat_state all six variants

#[test]
fn format_tournament_seat_state_all_variants() {
    assert_eq!(
        format_tournament_seat_state(TournamentSeatState::Open),
        "Open"
    );
    assert_eq!(
        format_tournament_seat_state(TournamentSeatState::Lobby),
        "Lobby"
    );
    assert_eq!(
        format_tournament_seat_state(TournamentSeatState::Ready),
        "Ready"
    );
    assert_eq!(
        format_tournament_seat_state(TournamentSeatState::Active),
        "Active"
    );
    assert_eq!(
        format_tournament_seat_state(TournamentSeatState::EliminatedObserver),
        "Eliminated observer"
    );
    assert_eq!(
        format_tournament_seat_state(TournamentSeatState::Closed),
        "Closed"
    );
}

// T5.7 — format_action all six variants

#[test]
fn format_action_all_variants() {
    assert_eq!(format_action(ActionType::Fold), "Fold");
    assert_eq!(format_action(ActionType::Check), "Check");
    assert_eq!(format_action(ActionType::Call), "Call");
    assert_eq!(format_action(ActionType::Bet), "Bet");
    assert_eq!(format_action(ActionType::Raise), "Raise");
    assert_eq!(format_action(ActionType::AllIn), "All-in");
}

// T5.8 — card_view representative combinations

#[test]
fn card_view_ace_of_spades() {
    let view = card_view(&Card {
        rank: Rank::Ace,
        suit: Suit::Spades,
    });
    assert_eq!(view.label, "Ace of Spades");
    assert_eq!(view.compact_label, "A♠");
    assert_eq!(view.tone, "dark");
}

#[test]
fn card_view_two_of_hearts() {
    let view = card_view(&Card {
        rank: Rank::Two,
        suit: Suit::Hearts,
    });
    assert_eq!(view.label, "Two of Hearts");
    assert_eq!(view.compact_label, "2♥");
    assert_eq!(view.tone, "red");
}

#[test]
fn card_view_ten_of_diamonds() {
    let view = card_view(&Card {
        rank: Rank::Ten,
        suit: Suit::Diamonds,
    });
    assert_eq!(view.compact_label, "10♦");
    assert_eq!(view.tone, "red");
}

#[test]
fn card_view_jack_of_clubs() {
    let view = card_view(&Card {
        rank: Rank::Jack,
        suit: Suit::Clubs,
    });
    assert_eq!(view.compact_label, "J♣");
    assert_eq!(view.tone, "dark");
}

// T5.9 — status_label_for_seat all participation states and None

#[test]
fn status_label_folded_returns_folded_this_hand() {
    let seat = blank_seat(0);
    assert_eq!(
        status_label_for_seat(&seat, Some(HandParticipationState::Folded)),
        "Folded this hand"
    );
}

#[test]
fn status_label_all_in_returns_all_in() {
    let seat = blank_seat(0);
    assert_eq!(
        status_label_for_seat(&seat, Some(HandParticipationState::AllIn)),
        "All-in"
    );
}

#[test]
fn status_label_eliminated_observer_returns_eliminated_observer() {
    let seat = blank_seat(0);
    assert_eq!(
        status_label_for_seat(&seat, Some(HandParticipationState::EliminatedObserver)),
        "Eliminated observer"
    );
}

#[test]
fn status_label_out_returns_out_of_this_hand() {
    let seat = blank_seat(0);
    assert_eq!(
        status_label_for_seat(&seat, Some(HandParticipationState::Out)),
        "Out of this hand"
    );
}

#[test]
fn status_label_active_delegates_to_seat_state() {
    let seat = blank_seat(0);
    let label = status_label_for_seat(&seat, Some(HandParticipationState::Active));
    assert!(!label.is_empty());
}

#[test]
fn status_label_none_delegates_to_seat_state() {
    let seat = blank_seat(0);
    let label = status_label_for_seat(&seat, None);
    assert!(!label.is_empty());
}

// T5.10 — build_elimination_summary all branches

#[test]
fn build_elimination_summary_waiting_for_players() {
    let state = blank_state();
    assert_eq!(
        build_elimination_summary(&state, TournamentPhase::WaitingForPlayers),
        "Waiting for the first real hand to start."
    );
}

#[test]
fn build_elimination_summary_ready_check() {
    let state = blank_state();
    assert_eq!(
        build_elimination_summary(&state, TournamentPhase::ReadyCheck),
        "Waiting for every seated player to be ready."
    );
}

#[test]
fn build_elimination_summary_running_no_hand_results() {
    let state = blank_state();
    assert_eq!(
        build_elimination_summary(&state, TournamentPhase::Running),
        "Table state is live."
    );
}

#[test]
fn build_elimination_summary_running_with_hand_result() {
    let mut state = blank_state();
    state
        .participants
        .insert("alice".to_string(), participant("alice", "Alice"));
    state.hand_results.push(hand_result(1, "alice", 200));
    let summary = build_elimination_summary(&state, TournamentPhase::Running);
    assert_eq!(summary, "Alice won 200 chip(s).");
}

// T5.11 — build_table_standings_for_state ranking and sorting

#[test]
fn build_table_standings_alice_ranks_above_bob_when_more_chips() {
    let mut state = blank_state();

    let mut alice_seat = blank_seat(0);
    alice_seat.participant_id = Some("alice".to_string());
    alice_seat.display_name = Some("Alice".to_string());
    alice_seat.chip_count = Some(1200);

    let mut bob_seat = blank_seat(1);
    bob_seat.participant_id = Some("bob".to_string());
    bob_seat.display_name = Some("Bob".to_string());
    bob_seat.chip_count = Some(800);

    state.seats = vec![alice_seat, bob_seat];

    let standings = build_table_standings_for_state(&state, "alice");
    assert_eq!(standings[0].rank, 1);
    assert_eq!(standings[0].display_name, "Alice");
    assert_eq!(standings[1].rank, 2);
    assert_eq!(standings[1].display_name, "Bob");
}

#[test]
fn build_table_standings_tie_broken_alphabetically() {
    let mut state = blank_state();

    let mut a_seat = blank_seat(0);
    a_seat.participant_id = Some("pa".to_string());
    a_seat.display_name = Some("Aaron".to_string());
    a_seat.chip_count = Some(1000);

    let mut b_seat = blank_seat(1);
    b_seat.participant_id = Some("pb".to_string());
    b_seat.display_name = Some("Zara".to_string());
    b_seat.chip_count = Some(1000);

    state.seats = vec![b_seat, a_seat];

    let standings = build_table_standings_for_state(&state, "pa");
    assert_eq!(standings[0].rank, 1);
    assert_eq!(standings[0].display_name, "Aaron");
}

#[test]
fn build_table_standings_excludes_reserved_player_seat() {
    let mut state = blank_state();

    let mut reserved_seat = blank_seat(0);
    reserved_seat.participant_id = Some("reserved-player".to_string());
    reserved_seat.display_name = Some("Reserved".to_string());
    reserved_seat.chip_count = Some(1000);

    let mut alice_seat = blank_seat(1);
    alice_seat.participant_id = Some("alice".to_string());
    alice_seat.display_name = Some("Alice".to_string());
    alice_seat.chip_count = Some(900);

    state.seats = vec![reserved_seat, alice_seat];

    let standings = build_table_standings_for_state(&state, "alice");
    assert_eq!(standings.len(), 1);
    assert_eq!(standings[0].display_name, "Alice");
}

#[test]
fn build_table_standings_is_local_flag_set_correctly() {
    let mut state = blank_state();

    let mut alice_seat = blank_seat(0);
    alice_seat.participant_id = Some("alice".to_string());
    alice_seat.display_name = Some("Alice".to_string());
    alice_seat.chip_count = Some(1000);

    let mut bob_seat = blank_seat(1);
    bob_seat.participant_id = Some("bob".to_string());
    bob_seat.display_name = Some("Bob".to_string());
    bob_seat.chip_count = Some(900);

    state.seats = vec![alice_seat, bob_seat];

    let standings = build_table_standings_for_state(&state, "alice");
    let alice = standings
        .iter()
        .find(|s| s.display_name == "Alice")
        .unwrap();
    let bob = standings.iter().find(|s| s.display_name == "Bob").unwrap();
    assert!(alice.is_local);
    assert!(!bob.is_local);
}

#[test]
fn build_table_standings_is_observer_flag_set_for_eliminated() {
    let mut state = blank_state();

    let mut alice_seat = blank_seat(0);
    alice_seat.participant_id = Some("alice".to_string());
    alice_seat.display_name = Some("Alice".to_string());
    alice_seat.chip_count = Some(0);
    alice_seat.tournament_state = TournamentSeatState::EliminatedObserver;

    state.seats = vec![alice_seat];

    let standings = build_table_standings_for_state(&state, "other");
    assert!(standings[0].is_observer);
}

// T5.12 — build_table_history_for_state order and structure

#[test]
fn build_table_history_empty_hand_results_returns_empty() {
    let state = blank_state();
    assert!(build_table_history_for_state(&state).is_empty());
}

#[test]
fn build_table_history_most_recent_first() {
    let mut state = blank_state();
    state
        .participants
        .insert("p1".to_string(), participant("p1", "Player 1"));
    state.hand_results.push(hand_result(1, "p1", 100));
    state.hand_results.push(hand_result(2, "p1", 200));

    let history = build_table_history_for_state(&state);
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].hand_number, 2); // most recent first
    assert_eq!(history[1].hand_number, 1);
}

#[test]
fn build_table_history_entry_fields_correct() {
    let mut state = blank_state();
    state
        .participants
        .insert("alice".to_string(), participant("alice", "Alice"));
    state.hand_results.push(hand_result(5, "alice", 300));

    let history = build_table_history_for_state(&state);
    assert_eq!(history[0].hand_number, 5);
    assert_eq!(history[0].pot_total, 300);
    assert_eq!(history[0].winning_players, vec!["Alice".to_string()]);
    assert!(history[0].board_cards.is_empty());
}

// T5.13 — display_name_for_state and display_names_for_state

#[test]
fn display_name_for_state_returns_display_name_for_known_player() {
    let mut state = blank_state();
    state
        .participants
        .insert("alice".to_string(), participant("alice", "Alice"));
    assert_eq!(
        display_name_for_state(&state, "alice"),
        Some("Alice".to_string())
    );
}

#[test]
fn display_name_for_state_returns_none_for_unknown_player() {
    let state = blank_state();
    assert_eq!(display_name_for_state(&state, "ghost"), None);
}

#[test]
fn display_names_for_state_falls_back_to_player_id_for_unknown() {
    let mut state = blank_state();
    state
        .participants
        .insert("alice".to_string(), participant("alice", "Alice"));
    let names = display_names_for_state(&state, &["alice".to_string(), "unknown-id".to_string()]);
    assert_eq!(names, vec!["Alice".to_string(), "unknown-id".to_string()]);
}
