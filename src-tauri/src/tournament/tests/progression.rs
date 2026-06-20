use super::super::*;
use crate::domain::*;

use super::support::*;

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

// ---------------------------------------------------------------------------
// Section 7 — fold-to-winner without showdown
// ---------------------------------------------------------------------------

#[test]
fn preflop_fold_awards_pot_immediately_without_board() {
    let mut controller = TournamentController::new(
        "table-fold-preflop",
        1,
        sample_config(1_000),
        vec![player("p1", 0), player("p2", 1)],
    )
    .expect("controller should build");
    controller
        .start_tournament(0)
        .expect("tournament should start");

    let window = action_window(&controller);
    let folder_id = window.player_id.clone();
    let winner_id = if folder_id == "p1" { "p2" } else { "p1" };

    controller
        .submit_action(
            ActionRequest {
                player_id: window.player_id,
                action_window_id: window.action_window_id,
                action_type: ActionType::Fold,
                raise_to_amount: None,
            },
            1,
        )
        .expect("preflop fold should succeed");

    let state = controller.state();
    let hand = state
        .current_hand
        .as_ref()
        .expect("hand should still be visible after fold");

    assert_eq!(
        hand.board_cards.len(),
        0,
        "no community cards should be dealt on a preflop fold"
    );
    assert!(
        !state.hand_results.is_empty(),
        "hand result should be recorded immediately after preflop fold"
    );

    let result = &state.hand_results[0];
    let total: u32 = result.final_stack_by_player_id.values().sum();
    assert_eq!(total, 2_000, "chips must be conserved across the fold");
    assert!(
        result.final_stack_by_player_id.get(winner_id)
            > result.final_stack_by_player_id.get(&folder_id),
        "the non-folding player should have more chips than the folder"
    );
}

#[test]
fn postflop_fold_awards_pot_to_remaining_player() {
    // The engine only includes Fold in the legal-action set when `to_call > 0`
    // (i.e. when the actor is facing an outstanding bet).  To exercise a
    // postflop fold we therefore need two postflop actions: first the initial
    // actor bets the minimum, then the second actor folds against that bet.
    let mut controller = TournamentController::new(
        "table-fold-postflop",
        1,
        sample_config(1_000),
        vec![player("p1", 0), player("p2", 1)],
    )
    .expect("controller should build");
    controller
        .start_tournament(0)
        .expect("tournament should start");

    // Drive through preflop (SB calls, BB checks) so we reach the flop.
    for now_ms in 1..=2 {
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
            .expect("preflop check/call should succeed");

        if controller
            .state()
            .current_hand
            .as_ref()
            .is_some_and(|h| !h.board_cards.is_empty())
        {
            break;
        }
    }

    assert_eq!(
        controller
            .state()
            .current_hand
            .as_ref()
            .map(|h| h.board_cards.len()),
        Some(3),
        "flop should be dealt after preflop"
    );

    // First postflop actor: bet the minimum amount so that the second actor
    // faces an outstanding bet and can legally fold.
    let bet_window = action_window(&controller);
    assert!(
        bet_window.legal_actions.contains(&ActionType::Bet),
        "first postflop actor should be able to bet into an empty pot"
    );
    let bet_amount = bet_window
        .min_raise_to
        .expect("min bet amount should be present for a bet window");
    controller
        .submit_action(
            ActionRequest {
                player_id: bet_window.player_id.clone(),
                action_window_id: bet_window.action_window_id,
                action_type: ActionType::Bet,
                raise_to_amount: Some(bet_amount),
            },
            3,
        )
        .expect("postflop bet should succeed");

    // Second postflop actor faces the bet → Fold is now legal.
    let fold_window = action_window(&controller);
    let folder_id = fold_window.player_id.clone();
    let winner_id = if folder_id == "p1" { "p2" } else { "p1" };
    assert!(
        fold_window.legal_actions.contains(&ActionType::Fold),
        "the actor facing a postflop bet must have Fold as a legal action"
    );
    controller
        .submit_action(
            ActionRequest {
                player_id: fold_window.player_id,
                action_window_id: fold_window.action_window_id,
                action_type: ActionType::Fold,
                raise_to_amount: None,
            },
            4,
        )
        .expect("postflop fold should succeed");

    let state = controller.state();
    let hand = state
        .current_hand
        .as_ref()
        .expect("hand should be visible after postflop fold");

    assert_eq!(
        hand.board_cards.len(),
        3,
        "only the flop was dealt — turn and river must not have been added"
    );
    assert!(
        !state.hand_results.is_empty(),
        "hand result should be recorded after the postflop fold"
    );

    let result = &state.hand_results[0];
    let total: u32 = result.final_stack_by_player_id.values().sum();
    assert_eq!(
        total, 2_000,
        "chips must be conserved across the postflop fold"
    );
    assert!(
        result.final_stack_by_player_id.get(winner_id)
            > result.final_stack_by_player_id.get(&folder_id),
        "the non-folding player should have more chips than the folder"
    );
}
