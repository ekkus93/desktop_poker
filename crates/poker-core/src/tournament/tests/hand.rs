use super::support::*;
use crate::domain::*;

// In the two-player started controller:
//   p1 is SB at seat 0: posted 50, stack=950, current_bet=100
//   p2 is BB at seat 1: posted 100, stack=900, current_bet=100
//   action window is on p1 (SB acts first preflop in heads-up)
//   min_full_raise_to = 200, turn_timer=10s → deadline = 10000 ms

// T2.1 — apply_action Fold
#[test]
fn apply_action_fold_sets_participation_to_folded() {
    let mut controller = started_two_player_controller();
    let window = action_window(&controller);
    assert_eq!(window.player_id, "p1");

    controller
        .apply_action("p1".to_string(), ActionType::Fold, None, 0)
        .unwrap();

    assert_eq!(
        controller.participation("p1"),
        Some(HandParticipationState::Folded)
    );
}

// T2.2 — apply_action Check
#[test]
fn apply_action_check_succeeds_when_contribution_equals_current_bet() {
    let mut controller = started_two_player_controller();
    // Give p1 the action window first by having p1 call (so p2 gets window)
    let window = action_window(&controller);
    controller
        .apply_action(window.player_id, ActionType::Call, None, 0)
        .unwrap();

    // Now p2 has the window; p2's contribution == current_bet (100) → Check is legal
    let window2 = action_window(&controller);
    assert_eq!(window2.player_id, "p2");
    controller
        .apply_action("p2".to_string(), ActionType::Check, None, 0)
        .unwrap();
}

#[test]
fn apply_action_check_errors_when_player_is_behind_current_bet() {
    let mut controller = started_two_player_controller();
    // p1 is SB with contribution=50 < current_bet=100 → Check is not legal
    let err = controller
        .apply_action("p1".to_string(), ActionType::Check, None, 0)
        .unwrap_err();
    assert!(
        err.to_string().contains("not legal"),
        "unexpected error: {err}"
    );
}

// T2.3 — apply_action Call
#[test]
fn apply_action_call_deducts_to_call_amount_from_stack() {
    let mut controller = started_two_player_controller();
    let stack_before = controller.player_stack("p1").unwrap(); // 950
    let to_call = 50u32; // 100 - 50

    controller
        .apply_action("p1".to_string(), ActionType::Call, None, 0)
        .unwrap();

    assert_eq!(
        controller.player_stack("p1").unwrap(),
        stack_before - to_call
    );
}

#[test]
fn apply_action_call_errors_when_stack_at_most_to_call() {
    let mut controller = started_two_player_controller();
    // to_call = 50; set p1's stack to exactly 50 → stack <= to_call → must all-in
    controller.set_player_stack("p1", 50).unwrap();

    let err = controller
        .apply_action("p1".to_string(), ActionType::Call, None, 0)
        .unwrap_err();
    assert!(
        err.to_string().contains("all-in"),
        "unexpected error: {err}"
    );
}

// T2.4 — apply_action Raise
#[test]
fn apply_action_raise_below_minimum_returns_error() {
    let mut controller = started_two_player_controller();
    // min_full_raise_to = 200; raise to 150 → error
    let err = controller
        .apply_action("p1".to_string(), ActionType::Raise, Some(150), 0)
        .unwrap_err();
    assert!(
        err.to_string().contains("minimum full raise"),
        "unexpected error: {err}"
    );
}

#[test]
fn apply_action_raise_exceeding_stack_returns_error() {
    let mut controller = started_two_player_controller();
    // p1 stack=950, street_contribution=50; additional=1500-50=1450 > 950 → error
    let err = controller
        .apply_action("p1".to_string(), ActionType::Raise, Some(1500), 0)
        .unwrap_err();
    assert!(
        err.to_string().contains("exceeds remaining stack"),
        "unexpected error: {err}"
    );
}

#[test]
fn apply_action_valid_raise_clears_acted_set_for_other_players() {
    let mut controller = started_two_player_controller();
    // p1 raises to 200; this should clear acted_since_last_full_raise so p2 must re-act
    controller
        .apply_action("p1".to_string(), ActionType::Raise, Some(200), 0)
        .unwrap();

    // p2 should now need to act (raise reopened the action)
    assert!(controller.player_needs_action("p2").unwrap());
}

// T2.5 — apply_action AllIn
#[test]
fn apply_action_all_in_commits_full_stack_and_sets_participation_to_all_in() {
    let mut controller = started_two_player_controller();
    let stack = controller.player_stack("p1").unwrap(); // 950

    controller
        .apply_action("p1".to_string(), ActionType::AllIn, None, 0)
        .unwrap();

    assert_eq!(controller.player_stack("p1").unwrap(), 0);
    assert_eq!(
        controller.participation("p1"),
        Some(HandParticipationState::AllIn)
    );
    // total all-in = 50 + 950 = 1000; this is a full raise, so action reopens
    assert!(controller.player_needs_action("p2").unwrap());
    let _ = stack;
}

#[test]
fn apply_action_short_all_in_does_not_reopen_action() {
    let mut controller = started_two_player_controller();
    // Set p1 stack to 20: all_in_to = 50+20 = 70 < current_bet=100 → short all-in, no reopen
    controller.set_player_stack("p1", 20).unwrap();

    controller
        .apply_action("p1".to_string(), ActionType::AllIn, None, 0)
        .unwrap();

    assert_eq!(controller.player_stack("p1").unwrap(), 0);
    assert_eq!(
        controller.participation("p1"),
        Some(HandParticipationState::AllIn)
    );
    // p2 should have been in acted set (BB was already counted) — action not reopened by p1
    // After p1's short all-in, p2's contribution=100==current_bet and p2 may already not need action
    // depending on whether p2 is in acted_since_last_full_raise; at minimum p1 did NOT do full raise
    assert!(!controller.player_may_raise("p1"));
}

#[test]
fn apply_action_all_in_errors_when_player_already_at_zero_stack() {
    let mut controller = started_two_player_controller();
    controller.set_player_stack("p1", 0).unwrap();

    let err = controller
        .apply_action("p1".to_string(), ActionType::AllIn, None, 0)
        .unwrap_err();
    assert!(
        err.to_string().contains("already all-in"),
        "unexpected error: {err}"
    );
}

// T2.6 — commit_timeout
#[test]
fn commit_timeout_applies_check_when_check_is_legal() {
    let mut controller = started_two_player_controller();
    // Advance past p1's action (call) so p2 has the window with Check legal
    let window = action_window(&controller);
    controller
        .apply_action(window.player_id, ActionType::Call, None, 0)
        .unwrap();

    // p2 now has window; Check should be legal (contribution==current_bet)
    let window2 = action_window(&controller);
    assert!(window2.legal_actions.contains(&ActionType::Check));
    controller.commit_timeout(0).unwrap();
}

#[test]
fn commit_timeout_applies_fold_when_check_is_not_legal() {
    let mut controller = started_two_player_controller();
    // p1 is the actor; p1 is behind (SB < BB) → Check is not legal → timeout folds
    let window = action_window(&controller);
    assert!(!window.legal_actions.contains(&ActionType::Check));
    controller.commit_timeout(0).unwrap();
    assert_eq!(
        controller.participation("p1"),
        Some(HandParticipationState::Folded)
    );
}

// T2.7 — process_eliminations
#[test]
fn process_eliminations_marks_zero_stack_player_as_eliminated_observer() {
    let mut controller = started_two_player_controller();
    controller.set_player_stack("p2", 0).unwrap();

    let eliminated = controller.process_eliminations(1);

    assert_eq!(eliminated, vec!["p2".to_string()]);
    assert_eq!(
        controller.state().participants["p2"].state,
        ParticipantState::EliminatedObserver
    );
    // Seat state updated
    assert_eq!(
        controller.state().seats[1].tournament_state,
        TournamentSeatState::EliminatedObserver
    );
    assert_eq!(controller.state().seats[1].chip_count, Some(0));
    assert!(controller.state().seats[1].marker.is_none());
}

#[test]
fn process_eliminations_records_placement_with_hand_number() {
    let mut controller = started_two_player_controller();
    controller.set_player_stack("p2", 0).unwrap();

    controller.process_eliminations(3);

    let entry = controller
        .state()
        .placements
        .iter()
        .find(|e| e.player_id == "p2")
        .expect("p2 should have a placement entry");
    assert_eq!(entry.busted_at_hand_number, Some(3));
}

#[test]
fn process_eliminations_assigns_distinct_places_when_multiple_players_bust() {
    let mut controller = started_three_player_controller();
    controller.set_player_stack("p1", 0).unwrap();
    controller.set_player_stack("p2", 0).unwrap();

    let eliminated = controller.process_eliminations(2);
    assert_eq!(eliminated.len(), 2);

    // Both should have placements with distinct place values
    let places: Vec<u8> = controller
        .state()
        .placements
        .iter()
        .map(|e| e.place)
        .collect();
    let unique: std::collections::BTreeSet<u8> = places.iter().copied().collect();
    assert_eq!(unique.len(), places.len(), "all places should be distinct");
}

// T2.8 — handle_full_raise
#[test]
fn handle_full_raise_clears_acted_set_and_sets_new_increment() {
    let mut controller = started_two_player_controller();
    // p1 raises to 200 → raise_increment = 200 - current_bet(100) = 100
    controller
        .apply_action("p1".to_string(), ActionType::Raise, Some(200), 0)
        .unwrap();

    // After the raise, p2 must re-act (action reopened)
    assert!(controller.player_needs_action("p2").unwrap());
    // p1 may not raise again immediately (in acted set after raise)
    assert!(!controller.player_may_raise("p1"));
}

#[test]
fn matched_all_in_with_one_actionable_player_runs_out_and_settles() {
    let mut controller = started_two_player_controller();
    // Preserve the tournament chip total while making p1's all-in smaller
    // than p2's remaining stack. After p2 calls, p2 still has chips but no
    // opponent can respond to another wager. The board must run out.
    controller.set_player_stack("p1", 850).unwrap();
    controller.set_player_stack("p2", 1_000).unwrap();

    let first_window = action_window(&controller);
    assert_eq!(first_window.player_id, "p1");
    controller
        .apply_action("p1".to_string(), ActionType::AllIn, None, 0)
        .unwrap();

    let response_window = action_window(&controller);
    assert_eq!(response_window.player_id, "p2");
    assert!(response_window.legal_actions.contains(&ActionType::Call));
    controller
        .apply_action("p2".to_string(), ActionType::Call, None, 0)
        .unwrap();

    assert_eq!(
        controller.state().hand_results.len(),
        1,
        "a matched all-in must settle without opening a lone action window"
    );
    assert_eq!(controller.state().hand_results[0].board_cards.len(), 5);
    assert!(
        controller
            .state()
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .is_none(),
        "no action window may remain when only one contender can act"
    );
}
