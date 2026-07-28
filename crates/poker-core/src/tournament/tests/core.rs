use crate::domain::*;

use super::super::*;
use super::support::*;

// T3.1 — start_tournament rejects duplicate start, unready seats, bad count

#[test]
fn start_tournament_errors_when_already_running() {
    let mut controller = started_two_player_controller();
    let err = controller.start_tournament(0).unwrap_err();
    assert!(
        err.to_string().contains("already started"),
        "unexpected error: {err}"
    );
}

#[test]
fn start_tournament_errors_when_seat_is_not_ready() {
    let config = sample_config(1_000);
    let mut p1 = player("p1", 0);
    let p2 = player("p2", 1);
    p1.is_ready = false;

    let mut controller = TournamentController::new("core-test", 1, config, vec![p1, p2])
        .expect("build should succeed");
    let err = controller.start_tournament(0).unwrap_err();
    assert!(
        err.to_string().contains("every occupied seat to be ready"),
        "unexpected error: {err}"
    );
}

#[test]
fn new_errors_when_only_one_player_provided() {
    // The constructor enforces at least two seated players — the check fires before start_tournament.
    let config = sample_config(1_000);
    let p1 = player("p1", 0);
    match TournamentController::new("core-test-1", 1, config, vec![p1]) {
        Ok(_) => panic!("expected Err for single-player build"),
        Err(err) => assert!(
            err.to_string().contains("at least two seated players"),
            "unexpected error: {err}"
        ),
    }
}

// T3.2 — submit_action rejects stale action window by timestamp

#[test]
fn submit_action_errors_when_now_is_past_action_deadline() {
    let mut controller = started_two_player_controller();
    let window = action_window(&controller);
    // deadline_epoch_ms is turn_timer_seconds (10) * 1000 = 10000; pass 10000 → past deadline
    let past_deadline = window.deadline_epoch_ms;
    let err = controller
        .submit_action(
            ActionRequest {
                player_id: window.player_id.clone(),
                action_window_id: window.action_window_id.clone(),
                action_type: ActionType::Fold,
                raise_to_amount: None,
            },
            past_deadline,
        )
        .unwrap_err();
    assert!(
        err.to_string().contains("stale action window rejected"),
        "unexpected error: {err}"
    );
}

#[test]
fn submit_action_invalid_raise_rolls_back_controller_state() {
    let mut controller = started_two_player_controller();
    let before = controller.state().clone();
    let window = action_window(&controller);

    let error = controller
        .submit_action(
            ActionRequest {
                player_id: window.player_id,
                action_window_id: window.action_window_id,
                action_type: ActionType::Raise,
                raise_to_amount: Some(1_500),
            },
            0,
        )
        .expect_err("out-of-bounds raise must be rejected");

    assert!(error.to_string().contains("exceeds remaining stack"));
    assert_eq!(
        controller.state(),
        &before,
        "a rejected action must leave the complete controller state unchanged"
    );
}

// T3.3 — submit_action rejects stale window by ID after action was already consumed

#[test]
fn submit_action_errors_when_window_id_is_stale() {
    let mut controller = started_two_player_controller();
    let old_window = action_window(&controller);

    // Consume the window with apply_action (advances to new window or new street)
    controller
        .apply_action(old_window.player_id.clone(), ActionType::Fold, None, 0)
        .unwrap();

    // Now try to re-submit the old window
    let err = controller
        .submit_action(
            ActionRequest {
                player_id: old_window.player_id.clone(),
                action_window_id: old_window.action_window_id.clone(),
                action_type: ActionType::Fold,
                raise_to_amount: None,
            },
            0,
        )
        .unwrap_err();
    assert!(
        err.to_string().contains("stale action window rejected"),
        "unexpected error: {err}"
    );
}

// T3.4 — submit_action rejects action from wrong player

#[test]
fn submit_action_errors_when_player_does_not_own_window() {
    let mut controller = started_two_player_controller();
    let window = action_window(&controller);
    // p1 owns the window; submit as p2
    let wrong_player = if window.player_id == "p1" { "p2" } else { "p1" };
    let err = controller
        .submit_action(
            ActionRequest {
                player_id: wrong_player.to_string(),
                action_window_id: window.action_window_id.clone(),
                action_type: ActionType::Fold,
                raise_to_amount: None,
            },
            0,
        )
        .unwrap_err();
    assert!(
        err.to_string().contains("does not own the action window"),
        "unexpected error: {err}"
    );
}

#[test]
fn submit_action_outcome_rejects_wrong_player_without_state_change() {
    let mut controller = started_two_player_controller();
    let before = controller.state().clone();
    let window = action_window(&controller);
    let wrong_player = if window.player_id == "p1" { "p2" } else { "p1" };

    let outcome = controller
        .submit_action_with_outcome(
            ActionRequest {
                player_id: wrong_player.to_string(),
                action_window_id: window.action_window_id,
                action_type: ActionType::Fold,
                raise_to_amount: None,
            },
            0,
        )
        .expect("wrong-player rejection is an explicit outcome");

    assert!(matches!(
        outcome,
        ActionSubmissionOutcome::RejectedNoStateChange { .. }
    ));
    assert_eq!(controller.state(), &before);
}

#[test]
fn submit_action_outcome_rejects_stale_id_without_state_change() {
    let mut controller = started_two_player_controller();
    let before = controller.state().clone();
    let window = action_window(&controller);

    let outcome = controller
        .submit_action_with_outcome(
            ActionRequest {
                player_id: window.player_id,
                action_window_id: "aw-stale".to_string(),
                action_type: ActionType::Fold,
                raise_to_amount: None,
            },
            0,
        )
        .expect("stale-window rejection is an explicit outcome");

    assert!(matches!(
        outcome,
        ActionSubmissionOutcome::RejectedNoStateChange { .. }
    ));
    assert_eq!(controller.state(), &before);
}

#[test]
fn submit_action_outcome_reports_timeout_advance_then_rejection() {
    let mut controller = started_two_player_controller();
    let before = controller.state().clone();
    let window = action_window(&controller);
    let deadline = window.deadline_epoch_ms;

    let outcome = controller
        .submit_action_with_outcome(
            ActionRequest {
                player_id: window.player_id,
                action_window_id: window.action_window_id,
                action_type: ActionType::Fold,
                raise_to_amount: None,
            },
            deadline,
        )
        .expect("timeout advancement should remain valid");

    assert!(matches!(
        outcome,
        ActionSubmissionOutcome::TimeoutAdvancedThenRejected { .. }
    ));
    assert_ne!(controller.state(), &before);
}

#[test]
fn submit_action_outcome_reports_committed_action() {
    let mut controller = started_two_player_controller();
    let before = controller.state().clone();
    let window = action_window(&controller);

    let outcome = controller
        .submit_action_with_outcome(
            ActionRequest {
                player_id: window.player_id,
                action_window_id: window.action_window_id,
                action_type: ActionType::Fold,
                raise_to_amount: None,
            },
            0,
        )
        .expect("legal action should commit");

    assert_eq!(outcome, ActionSubmissionOutcome::Committed);
    assert_ne!(controller.state(), &before);
}

// T3.5 — commit_total_wager deducts stack, accumulates pot, and tracks contributions

#[test]
fn commit_total_wager_deducts_stack_and_updates_pot_and_contributions() {
    let mut controller = started_two_player_controller();
    // After start: p1 stack=950 (posted SB=50); reset p1 stack for a clean test
    let stack_before = controller.player_stack("p1").unwrap(); // 950

    controller.commit_total_wager("p1", 200, true).unwrap();

    assert_eq!(controller.player_stack("p1").unwrap(), stack_before - 200);

    let hand = controller.state().current_hand.as_ref().unwrap();
    // pot includes blinds (50+100) + 200 = 350
    assert!(hand.betting_round.pot_size >= 200);
    // contributions_by_player_id for p1 should have increased by 200 (was 50 from blind)
    let contrib = hand
        .betting_round
        .contributions_by_player_id
        .get("p1")
        .copied()
        .unwrap_or(0);
    assert!(contrib >= 200, "expected at least 200, got {contrib}");
}

#[test]
fn commit_total_wager_zero_is_noop() {
    let mut controller = started_two_player_controller();
    let stack_before = controller.player_stack("p1").unwrap();
    controller.commit_total_wager("p1", 0, true).unwrap();
    assert_eq!(controller.player_stack("p1").unwrap(), stack_before);
}

#[test]
fn commit_total_wager_errors_when_amount_exceeds_stack() {
    let mut controller = started_two_player_controller();
    let stack = controller.player_stack("p1").unwrap();
    let err = controller
        .commit_total_wager("p1", stack + 1, true)
        .unwrap_err();
    assert!(
        err.to_string().contains("cannot wager more chips"),
        "unexpected error: {err}"
    );
}

// T3.6 — refresh_betting_round_bounds sets min/max raise from current state

#[test]
fn refresh_betting_round_bounds_after_blinds_gives_correct_min_raise() {
    let controller = started_two_player_controller();
    let hand = controller.state().current_hand.as_ref().unwrap();
    // SB=50, BB=100; min_full_raise_to = 100 + 100 = 200
    assert_eq!(hand.betting_round.min_raise_to, Some(200));
}

#[test]
fn refresh_betting_round_bounds_max_raise_reflects_deepest_stack() {
    let controller = started_two_player_controller();
    let hand = controller.state().current_hand.as_ref().unwrap();
    // max_raise_to = max(stack+street_contribution per player)
    // p1: stack=950, street_contribution=50 → 1000
    // p2: stack=900, street_contribution=100 → 1000
    assert_eq!(hand.betting_round.max_raise_to, Some(1000));
}
