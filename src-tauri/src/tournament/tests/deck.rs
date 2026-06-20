use super::super::*;
use crate::domain::*;

use super::support::*;

// ---------------------------------------------------------------------------
// Section 6 — stacked-deck hole-card invariants
// ---------------------------------------------------------------------------

#[test]
fn stacked_deck_deals_hole_cards_in_deal_order() {
    // In a heads-up game seat 0 is the dealer / small blind (SB) and receives
    // the first card, so p1 picks up cards at deck positions 0 and 2, while
    // p2 (big blind) picks up positions 1 and 3.
    //
    // Deck layout:
    //   0 As → p1 hole 1    1 Kh → p2 hole 1
    //   2 Qc → p1 hole 2    3 Jd → p2 hole 2
    //   4 Ts → burn          5 9h → flop 1
    //   6 8c → flop 2        7 7d → flop 3
    //   8 6s → burn          9 5h → turn
    //  10 4c → burn         11 3d → river
    let mut controller = TournamentController::new(
        "table-deck-order",
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
        card(Rank::Five, Suit::Hearts),
        card(Rank::Four, Suit::Clubs),
        card(Rank::Three, Suit::Diamonds),
    ]));
    controller
        .start_tournament(0)
        .expect("tournament should start");

    let hand = controller
        .state()
        .current_hand
        .as_ref()
        .expect("current hand should exist after tournament start");

    assert_eq!(
        hand.hole_cards_by_player_id.get("p1"),
        Some(&vec![
            card(Rank::Ace, Suit::Spades),
            card(Rank::Queen, Suit::Clubs),
        ]),
        "p1 (small blind) should receive As and Qc"
    );
    assert_eq!(
        hand.hole_cards_by_player_id.get("p2"),
        Some(&vec![
            card(Rank::King, Suit::Hearts),
            card(Rank::Jack, Suit::Diamonds),
        ]),
        "p2 (big blind) should receive Kh and Jd"
    );

    // Drive to the river by always checking or calling.
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
            .expect("scripted check/call should succeed");
        if controller
            .state()
            .current_hand
            .as_ref()
            .is_some_and(|h| h.cycle_phase == HandCyclePhase::BetweenHands)
        {
            break;
        }
    }

    let board = &controller
        .state()
        .current_hand
        .as_ref()
        .expect("hand should remain visible during intermission")
        .board_cards;
    assert_eq!(
        board,
        &vec![
            card(Rank::Nine, Suit::Hearts),
            card(Rank::Eight, Suit::Clubs),
            card(Rank::Seven, Suit::Diamonds),
            card(Rank::Five, Suit::Hearts),
            card(Rank::Three, Suit::Diamonds),
        ],
        "board should be [9h, 8c, 7d, 5h, 3d] — burn cards are excluded"
    );
}

#[test]
fn no_card_dealt_twice_in_a_real_shuffled_hand() {
    use std::collections::BTreeSet;

    let mut controller = TournamentController::new(
        "table-shuffle-unique",
        1,
        sample_config(1_000),
        vec![player("p1", 0), player("p2", 1), player("p3", 2)],
    )
    .expect("controller should build");
    controller
        .start_tournament(0)
        .expect("tournament should start");

    // Drive to showdown.
    for now_ms in 1..=20 {
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
            .expect("scripted check/call should succeed");
        if controller
            .state()
            .current_hand
            .as_ref()
            .is_some_and(|h| h.cycle_phase == HandCyclePhase::BetweenHands)
        {
            break;
        }
    }

    let hand = controller
        .state()
        .current_hand
        .as_ref()
        .expect("hand must be visible during intermission");

    let mut all_cards: Vec<Card> = hand
        .hole_cards_by_player_id
        .values()
        .flat_map(|cards| cards.iter().copied())
        .collect();
    all_cards.extend_from_slice(&hand.board_cards);

    // 3 players × 2 hole cards + 5 board cards = 11 distinct cards expected.
    assert_eq!(all_cards.len(), 11, "3 hole-card pairs plus 5 board cards");

    let unique: BTreeSet<Card> = all_cards.iter().copied().collect();
    assert_eq!(
        unique.len(),
        all_cards.len(),
        "no card should appear twice in a shuffled hand"
    );
}
