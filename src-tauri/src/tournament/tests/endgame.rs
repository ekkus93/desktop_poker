use super::super::*;
use crate::domain::*;

use super::support::*;

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
fn action_window_authority_advances_without_leaking_to_the_prior_actor() {
    let mut controller = TournamentController::new(
        "table-action-authority",
        1,
        sample_config(1_000),
        vec![player("p1", 0), player("p2", 1), player("p3", 2)],
    )
    .expect("controller should build");
    controller
        .start_tournament(0)
        .expect("tournament should start");

    let first_window = action_window(&controller);
    let first_action_type = if first_window.legal_actions.contains(&ActionType::Check) {
        ActionType::Check
    } else {
        ActionType::Call
    };
    controller
        .submit_action(
            ActionRequest {
                player_id: first_window.player_id.clone(),
                action_window_id: first_window.action_window_id.clone(),
                action_type: first_action_type,
                raise_to_amount: None,
            },
            1,
        )
        .expect("first actor should be able to act");

    let second_window = action_window(&controller);
    assert_ne!(second_window.player_id, first_window.player_id);

    let stale_attempt = controller.submit_action(
        ActionRequest {
            player_id: first_window.player_id.clone(),
            action_window_id: second_window.action_window_id.clone(),
            action_type: if second_window.legal_actions.contains(&ActionType::Check) {
                ActionType::Check
            } else {
                ActionType::Call
            },
            raise_to_amount: None,
        },
        2,
    );

    assert!(stale_attempt.is_err());
    assert!(stale_attempt
        .expect_err("prior actor should lose the action window")
        .to_string()
        .contains("does not own the action window"));
    assert_eq!(
        action_window(&controller).player_id,
        second_window.player_id
    );
    assert_eq!(
        action_window(&controller).action_window_id,
        second_window.action_window_id
    );
}

#[test]
fn eliminated_observer_cannot_submit_actions_or_mutate_authoritative_state() {
    let mut controller = TournamentController::new(
        "table-observer-rejection",
        1,
        sample_config(1_000),
        vec![player("p1", 0), player("p2", 1), player("p3", 2)],
    )
    .expect("controller should build");
    controller
        .start_tournament(0)
        .expect("tournament should start");
    controller
        .set_player_stack("p3", 0)
        .expect("stack override should apply");

    let eliminated = controller.process_eliminations(7);
    assert_eq!(eliminated, vec!["p3".to_string()]);

    let live_window = action_window(&controller);
    let state_before = controller.state().clone();
    let observer_attempt = controller.submit_action(
        ActionRequest {
            player_id: "p3".to_string(),
            action_window_id: live_window.action_window_id.clone(),
            action_type: if live_window.legal_actions.contains(&ActionType::Check) {
                ActionType::Check
            } else {
                ActionType::Call
            },
            raise_to_amount: None,
        },
        8,
    );

    assert!(observer_attempt.is_err());
    assert!(observer_attempt
        .expect_err("observer action should be rejected")
        .to_string()
        .contains("does not own the action window"));
    let participant = controller
        .state()
        .participants
        .get("p3")
        .expect("participant");
    assert_eq!(participant.state, ParticipantState::EliminatedObserver);
    assert_eq!(participant.connection_state, ConnectionState::Connected);
    assert_eq!(controller.state().current_hand, state_before.current_hand);
    assert_eq!(controller.state().hand_results, state_before.hand_results);
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
