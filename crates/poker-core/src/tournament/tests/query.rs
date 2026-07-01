use super::super::*;
use crate::domain::*;

use super::support::*;

// T1.1
#[test]
fn player_order_starting_with_rotates_correctly_from_given_seat() {
    let controller = started_two_player_controller();
    let players = vec![
        (0u8, "p1".to_string()),
        (2u8, "p3".to_string()),
        (4u8, "px".to_string()),
    ];

    // Start from seat 2 → p3, px, p1
    let order = controller.player_order_starting_with(2, &players);
    assert_eq!(order, vec!["p3", "px", "p1"]);

    // Start from seat 0 → p1, p3, px
    let order = controller.player_order_starting_with(0, &players);
    assert_eq!(order, vec!["p1", "p3", "px"]);

    // Seat not in list → falls back to first sorted entry (seat 0)
    let order = controller.player_order_starting_with(3, &players);
    assert_eq!(order, vec!["p1", "p3", "px"]);

    // Empty list → empty vec
    let order = controller.player_order_starting_with(0, &[]);
    assert!(order.is_empty());
}

// T1.2
#[test]
fn next_active_seat_after_advances_and_wraps_correctly() {
    let controller = started_two_player_controller();
    let players = vec![
        (0u8, "p1".to_string()),
        (2u8, "p3".to_string()),
        (4u8, "px".to_string()),
    ];

    assert_eq!(controller.next_active_seat_after(0, &players).unwrap(), 2);
    assert_eq!(controller.next_active_seat_after(2, &players).unwrap(), 4);
    // Wrap: highest seat wraps to first
    assert_eq!(controller.next_active_seat_after(4, &players).unwrap(), 0);

    // Single seat always returns itself
    let single = vec![(5u8, "px".to_string())];
    assert_eq!(controller.next_active_seat_after(5, &single).unwrap(), 5);

    // Empty → error
    assert!(controller.next_active_seat_after(0, &[]).is_err());
}

// T1.3
#[test]
fn next_dealer_seat_index_returns_first_on_first_hand_then_advances() {
    // Fresh unstarted controller has dealer_button = None → first active seat returned
    let fresh = TournamentController::new(
        "q-dealer",
        1,
        sample_config(1_000),
        vec![player("p1", 0), player("p2", 1)],
    )
    .expect("build");
    let players = vec![(0u8, "p1".to_string()), (1u8, "p2".to_string())];
    assert_eq!(fresh.next_dealer_seat_index(&players).unwrap(), 0);

    // After start, dealer is at seat 0; next dealer is seat 1
    let started = started_two_player_controller();
    assert_eq!(started.next_dealer_seat_index(&players).unwrap(), 1);
}

// T1.4
#[test]
fn advance_blind_levels_if_due_does_not_advance_before_deadline() {
    let mut controller = started_two_player_controller();
    // sample_config level 0 deadline = 5000 ms
    controller.advance_blind_levels_if_due(4999);
    assert_eq!(controller.state().blind_level_index, 0);
}

#[test]
fn advance_blind_levels_if_due_advances_at_deadline() {
    let mut controller = started_two_player_controller();
    controller.advance_blind_levels_if_due(5000);
    assert_eq!(controller.state().blind_level_index, 1);
}

#[test]
fn advance_blind_levels_if_due_advances_multiple_expired_levels_in_one_call() {
    let mut controller = started_two_player_controller();
    // Both level deadlines have passed: 5000 and 10000
    controller.advance_blind_levels_if_due(10001);
    // sample_config has 2 levels; index stops at 1 (the last)
    assert_eq!(controller.state().blind_level_index, 1);
}

#[test]
fn advance_blind_levels_if_due_does_not_advance_past_the_last_level() {
    let mut controller = started_two_player_controller();
    controller.advance_blind_levels_if_due(5000); // → level 1 (last)
    assert_eq!(controller.state().blind_level_index, 1);
    controller.advance_blind_levels_if_due(99_999); // already at last level
    assert_eq!(controller.state().blind_level_index, 1);
}

// T1.5
#[test]
fn player_needs_action_true_when_street_contribution_is_behind_current_bet() {
    let controller = started_two_player_controller();
    // p1 is SB (posted 50); current_bet is 100 (BB) → behind → needs action
    assert!(controller.player_needs_action("p1").unwrap());
}

#[test]
fn player_needs_action_true_when_contribution_equal_but_not_yet_acted() {
    let controller = started_two_player_controller();
    // p2 is BB (posted 100 == current_bet) but hasn't acted → needs action
    assert!(controller.player_needs_action("p2").unwrap());
}

#[test]
fn player_needs_action_false_for_non_active_participation_states() {
    let mut controller = started_two_player_controller();

    controller
        .set_participation("p2", HandParticipationState::Folded)
        .unwrap();
    assert!(!controller.player_needs_action("p2").unwrap());

    controller
        .set_participation("p2", HandParticipationState::AllIn)
        .unwrap();
    assert!(!controller.player_needs_action("p2").unwrap());
}

#[test]
fn player_needs_action_false_after_player_has_acted_and_matched_bet() {
    let mut controller = started_two_player_controller();
    // p1 calls to match BB (100); p1 is then in acted_since_last_full_raise with
    // street_contribution == current_bet
    let window = action_window(&controller);
    controller
        .submit_action(
            ActionRequest {
                player_id: window.player_id,
                action_window_id: window.action_window_id,
                action_type: ActionType::Call,
                raise_to_amount: None,
            },
            0,
        )
        .unwrap();
    // After p1 calls, p1 has acted and contribution == current_bet
    assert!(!controller.player_needs_action("p1").unwrap());
}

// T1.6
#[test]
fn player_may_raise_true_before_acting_false_after_non_reopening_action() {
    let mut controller = started_two_player_controller();
    // p1 has not acted yet
    assert!(controller.player_may_raise("p1"));

    // p1 calls (non-reopening) → added to acted set
    let window = action_window(&controller);
    controller
        .submit_action(
            ActionRequest {
                player_id: window.player_id,
                action_window_id: window.action_window_id,
                action_type: ActionType::Call,
                raise_to_amount: None,
            },
            0,
        )
        .unwrap();
    assert!(!controller.player_may_raise("p1"));
}

#[test]
fn player_may_raise_false_for_folded_participant() {
    let mut controller = started_two_player_controller();
    controller
        .set_participation("p2", HandParticipationState::Folded)
        .unwrap();
    assert!(!controller.player_may_raise("p2"));
}

// T1.7
#[test]
fn players_who_can_act_excludes_all_in_participants() {
    let mut controller = started_two_player_controller();
    // Initially at least one player can act (there is an open action window)
    assert!(!controller.players_who_can_act().is_empty());

    // Mark p2 AllIn → p2 can no longer act
    controller
        .set_participation("p2", HandParticipationState::AllIn)
        .unwrap();
    assert!(!controller.players_who_can_act().contains(&"p2".to_string()));
}

// T1.8
#[test]
fn remaining_contenders_includes_active_and_all_in_excludes_folded_and_out() {
    let mut controller = started_three_player_controller();

    // All three start active
    assert_eq!(controller.remaining_contenders().len(), 3);

    // Fold p1 → no longer a contender
    controller
        .set_participation("p1", HandParticipationState::Folded)
        .unwrap();
    let contenders = controller.remaining_contenders();
    assert!(!contenders.contains(&"p1".to_string()));
    assert_eq!(contenders.len(), 2);

    // AllIn p2 → still a contender
    controller
        .set_participation("p2", HandParticipationState::AllIn)
        .unwrap();
    assert!(controller
        .remaining_contenders()
        .contains(&"p2".to_string()));
}

// T1.9
#[test]
fn active_player_seats_excludes_zero_stack_and_empty_seats() {
    let mut controller = started_two_player_controller();
    // Both players have stacks; a third seat is empty
    assert_eq!(controller.active_player_seats().len(), 2);

    // Set p1 to zero stack → excluded
    controller.set_player_stack("p1", 0).unwrap();
    let seats = controller.active_player_seats();
    assert_eq!(seats.len(), 1);
    assert_eq!(seats[0].1, "p2");
}

// T1.10
#[test]
fn competitor_count_counts_seated_participants_only() {
    let controller = TournamentController::new(
        "q-count",
        1,
        sample_config(1_000),
        vec![player("p1", 0), player("p2", 1)],
    )
    .expect("build");
    // Both p1 and p2 have seat_index assigned
    assert_eq!(controller.competitor_count(), 2);
}

// T1.11
#[test]
fn assign_markers_places_correct_markers_and_clear_markers_removes_them() {
    let mut controller = started_three_player_controller();

    controller.assign_markers(0, 1, 2);
    let seats = controller.state().seats.clone();
    assert_eq!(seats[0].marker, Some(SeatMarker::Dealer));
    assert_eq!(seats[1].marker, Some(SeatMarker::SmallBlind));
    assert_eq!(seats[2].marker, Some(SeatMarker::BigBlind));

    controller.clear_markers();
    let seats = controller.state().seats.clone();
    assert!(seats.iter().all(|s| s.marker.is_none()));
}

#[test]
fn assign_markers_overwrites_previous_marker_on_same_seat() {
    let mut controller = started_three_player_controller();

    // In heads-up dealer == small_blind; seat gets SmallBlind (last write wins)
    controller.assign_markers(0, 0, 1);
    assert_eq!(
        controller.state().seats[0].marker,
        Some(SeatMarker::SmallBlind)
    );
    assert_eq!(
        controller.state().seats[1].marker,
        Some(SeatMarker::BigBlind)
    );
}

// T1.12
#[test]
fn sort_placements_orders_entries_by_place_ascending() {
    let mut controller = started_two_player_controller();

    // Eliminate p2 first (places are assigned by competitor_count - placed_count)
    // competitor_count = 2, placed = 0 → next_place = 2 for first elimination
    controller.set_player_stack("p2", 0).unwrap();
    controller.process_eliminations(1);

    // After p2 eliminated with place=2, call complete_tournament to add p1 as place=1
    controller.set_player_stack("p1", 0).unwrap();
    controller.complete_tournament();

    let placements = controller.state().placements.clone();
    let places: Vec<u8> = placements.iter().map(|e| e.place).collect();
    // sort_placements ensures ascending order
    assert!(places.windows(2).all(|w| w[0] <= w[1]));
}

// T1.13
#[test]
fn complete_tournament_sets_phase_adds_winner_and_assigns_dealer_marker() {
    let mut controller = started_two_player_controller();
    // Remove p2 from active consideration by zeroing their stack
    controller.set_player_stack("p2", 0).unwrap();

    controller.complete_tournament();

    assert_eq!(controller.state().phase, TournamentPhase::Complete);
    assert!(controller.state().current_hand.is_none());

    // p1 (seat 0) should be the winner
    let placements = controller.state().placements.clone();
    let p1_entry = placements.iter().find(|e| e.player_id == "p1");
    assert!(p1_entry.is_some());
    assert_eq!(p1_entry.unwrap().place, 1);

    // Seat 0 (p1) should have the Dealer marker
    assert_eq!(controller.state().seats[0].marker, Some(SeatMarker::Dealer));
}

#[test]
fn complete_tournament_does_not_double_add_winner_already_in_placements() {
    let mut controller = started_two_player_controller();
    controller.set_player_stack("p2", 0).unwrap();

    controller.complete_tournament();
    controller.complete_tournament(); // second call: p1 already placed

    let p1_entries: Vec<_> = controller
        .state()
        .placements
        .iter()
        .filter(|e| e.player_id == "p1")
        .collect();
    assert_eq!(p1_entries.len(), 1);
}
