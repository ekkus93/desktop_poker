use super::super::{evaluate_best_holdem_hand, HandCategory, HandStrength};
use super::card;
use crate::domain::{Card, Rank::*, Suit::*};

/// Terse wrapper that unwraps the `Result` for the common positive case.
/// `evaluate_best_holdem_hand` requires exactly two hole cards and five board
/// cards, so every hand below is a full seven-card pool.
fn eval(holes: &[Card], board: &[Card]) -> HandStrength {
    evaluate_best_holdem_hand(holes, board).expect("hand should evaluate")
}

// ----- 2.2 Category detection -----

#[test]
fn detects_high_card() {
    let strength = eval(
        &[card(Ace, Spades), card(King, Diamonds)],
        &[
            card(Queen, Clubs),
            card(Jack, Hearts),
            card(Nine, Spades),
            card(Seven, Diamonds),
            card(Two, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::HighCard);
    assert_eq!(strength.key_ranks[0], 14);
}

#[test]
fn detects_one_pair() {
    let strength = eval(
        &[card(Ace, Spades), card(Ace, Diamonds)],
        &[
            card(King, Clubs),
            card(Queen, Hearts),
            card(Jack, Spades),
            card(Nine, Diamonds),
            card(Two, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::OnePair);
    assert_eq!(strength.key_ranks[0], 14);
}

#[test]
fn detects_two_pair() {
    let strength = eval(
        &[card(Ace, Spades), card(Ace, Diamonds)],
        &[
            card(King, Clubs),
            card(King, Hearts),
            card(Queen, Spades),
            card(Jack, Diamonds),
            card(Two, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::TwoPair);
    assert_eq!(strength.key_ranks[0], 14);
    assert_eq!(strength.key_ranks[1], 13);
}

#[test]
fn detects_three_of_a_kind() {
    let strength = eval(
        &[card(Ace, Spades), card(Ace, Diamonds)],
        &[
            card(Ace, Clubs),
            card(King, Hearts),
            card(Queen, Spades),
            card(Nine, Diamonds),
            card(Two, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::ThreeOfAKind);
    assert_eq!(strength.key_ranks[0], 14);
}

#[test]
fn detects_straight() {
    let strength = eval(
        &[card(Nine, Spades), card(Eight, Diamonds)],
        &[
            card(Seven, Clubs),
            card(Six, Hearts),
            card(Five, Spades),
            card(King, Diamonds),
            card(Two, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::Straight);
    assert_eq!(strength.key_ranks[0], 9);
}

#[test]
fn detects_flush() {
    let strength = eval(
        &[card(Ace, Spades), card(Jack, Spades)],
        &[
            card(Nine, Spades),
            card(Six, Spades),
            card(Three, Spades),
            card(King, Diamonds),
            card(Two, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::Flush);
    assert_eq!(strength.key_ranks[0], 14);
}

#[test]
fn detects_full_house() {
    let strength = eval(
        &[card(Ace, Spades), card(Ace, Diamonds)],
        &[
            card(Ace, Clubs),
            card(King, Hearts),
            card(King, Spades),
            card(Queen, Diamonds),
            card(Two, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::FullHouse);
    assert_eq!(strength.key_ranks[0], 14);
}

#[test]
fn detects_four_of_a_kind() {
    let strength = eval(
        &[card(Ace, Spades), card(Ace, Diamonds)],
        &[
            card(Ace, Clubs),
            card(Ace, Hearts),
            card(King, Spades),
            card(Queen, Diamonds),
            card(Two, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::FourOfAKind);
    assert_eq!(strength.key_ranks[0], 14);
}

#[test]
fn detects_straight_flush() {
    let strength = eval(
        &[card(Nine, Spades), card(Eight, Spades)],
        &[
            card(Seven, Spades),
            card(Six, Spades),
            card(Five, Spades),
            card(King, Diamonds),
            card(Two, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::StraightFlush);
    assert_eq!(strength.key_ranks[0], 9);
}

// ----- 2.3 Straight edge cases -----

#[test]
fn wheel_straight_ace_plays_low() {
    let strength = eval(
        &[card(Ace, Spades), card(Two, Diamonds)],
        &[
            card(Three, Clubs),
            card(Four, Hearts),
            card(Five, Spades),
            card(King, Diamonds),
            card(Nine, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::Straight);
    // The wheel is a 5-high straight, not Ace-high.
    assert_eq!(strength.key_ranks[0], 5);
}

#[test]
fn broadway_straight_ace_high() {
    let strength = eval(
        &[card(Ace, Spades), card(King, Diamonds)],
        &[
            card(Queen, Clubs),
            card(Jack, Hearts),
            card(Ten, Spades),
            card(Six, Diamonds),
            card(Two, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::Straight);
    assert_eq!(strength.key_ranks[0], 14);
}

#[test]
fn royal_flush_is_straight_flush_ace_high() {
    let strength = eval(
        &[card(Ace, Spades), card(King, Spades)],
        &[
            card(Queen, Spades),
            card(Jack, Spades),
            card(Ten, Spades),
            card(Six, Diamonds),
            card(Two, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::StraightFlush);
    assert_eq!(strength.key_ranks[0], 14);
}

#[test]
fn steel_wheel_straight_flush_ace_low() {
    let strength = eval(
        &[card(Ace, Spades), card(Two, Spades)],
        &[
            card(Three, Spades),
            card(Four, Spades),
            card(Five, Spades),
            card(King, Diamonds),
            card(Nine, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::StraightFlush);
    assert_eq!(strength.key_ranks[0], 5);
}

#[test]
fn no_false_straight_across_suits_only() {
    // A flush whose ranks are not sequential (A-K-Q-J-9, missing the Ten) must
    // be a Flush, never a Straight.
    let strength = eval(
        &[card(Ace, Spades), card(King, Spades)],
        &[
            card(Queen, Spades),
            card(Jack, Spades),
            card(Nine, Spades),
            card(Six, Diamonds),
            card(Two, Clubs),
        ],
    );
    assert_eq!(strength.category, HandCategory::Flush);
}

// ----- 2.4 Best-5-of-7 selection -----

#[test]
fn picks_best_five_from_seven() {
    // The board makes trip twos, but the hole + board spades make a flush, which
    // is the better five-card hand.
    let strength = eval(
        &[card(Ace, Spades), card(Five, Spades)],
        &[
            card(King, Spades),
            card(Nine, Spades),
            card(Two, Spades),
            card(Two, Diamonds),
            card(Two, Hearts),
        ],
    );
    assert_eq!(strength.category, HandCategory::Flush);
}

#[test]
fn board_plays_when_hole_cards_are_irrelevant() {
    // Quad aces sit entirely on the board; weak and strong hole cards both
    // evaluate to the same board hand.
    let board = [
        card(Ace, Spades),
        card(Ace, Diamonds),
        card(Ace, Clubs),
        card(Ace, Hearts),
        card(King, Spades),
    ];
    let weak = eval(&[card(Two, Clubs), card(Three, Diamonds)], &board);
    let strong = eval(&[card(Queen, Clubs), card(Jack, Diamonds)], &board);

    assert_eq!(weak.category, HandCategory::FourOfAKind);
    assert_eq!(weak, strong);
}

#[test]
fn uses_hole_cards_to_break_into_a_better_category() {
    // Board shows trip aces; a hole ace upgrades the hand all the way to quads.
    let board = [
        card(Ace, Spades),
        card(Ace, Diamonds),
        card(Ace, Clubs),
        card(Five, Hearts),
        card(Nine, Spades),
    ];
    let with_ace = eval(&[card(Ace, Hearts), card(Two, Diamonds)], &board);
    let without_ace = eval(&[card(Two, Diamonds), card(Three, Clubs)], &board);

    assert_eq!(with_ace.category, HandCategory::FourOfAKind);
    assert_eq!(without_ace.category, HandCategory::ThreeOfAKind);
}

// ----- 2.5 Tie-breaks and ordering (`HandStrength: Ord`) -----

fn strength(category: HandCategory, key_ranks: [u8; 5]) -> HandStrength {
    HandStrength {
        category,
        key_ranks,
    }
}

#[test]
fn category_ordering_is_correct() {
    use HandCategory::*;
    let ascending = [
        HighCard,
        OnePair,
        TwoPair,
        ThreeOfAKind,
        Straight,
        Flush,
        FullHouse,
        FourOfAKind,
        StraightFlush,
    ];
    for pair in ascending.windows(2) {
        let lower = strength(pair[0], [0; 5]);
        let higher = strength(pair[1], [0; 5]);
        assert!(higher > lower, "{:?} should outrank {:?}", pair[1], pair[0]);
    }
}

#[test]
fn flush_tiebreak_by_high_cards() {
    let ace_high = strength(HandCategory::Flush, [14, 13, 9, 6, 3]);
    let king_high = strength(HandCategory::Flush, [13, 12, 9, 6, 3]);
    assert!(ace_high > king_high);
}

#[test]
fn full_house_tiebreak_trips_then_pair() {
    let kings_full = strength(HandCategory::FullHouse, [13, 2, 0, 0, 0]);
    let queens_full = strength(HandCategory::FullHouse, [12, 14, 0, 0, 0]);
    // Trips rank decides first: KKK22 beats QQQAA.
    assert!(kings_full > queens_full);

    // Same trips, the higher pair wins: KKKQQ beats KKK22.
    let kings_full_queens = strength(HandCategory::FullHouse, [13, 12, 0, 0, 0]);
    assert!(kings_full_queens > kings_full);
}

#[test]
fn two_pair_tiebreak_high_low_kicker() {
    let aces_nines = strength(HandCategory::TwoPair, [14, 9, 13, 0, 0]);
    let aces_eights = strength(HandCategory::TwoPair, [14, 8, 13, 0, 0]);
    assert!(aces_nines > aces_eights);
}

#[test]
fn one_pair_tiebreak_by_kickers() {
    let better_kicker = strength(HandCategory::OnePair, [14, 13, 12, 11, 0]);
    let worse_kicker = strength(HandCategory::OnePair, [14, 13, 12, 10, 0]);
    assert!(better_kicker > worse_kicker);
}

#[test]
fn high_card_tiebreak_by_full_key_ranks() {
    let better = strength(HandCategory::HighCard, [14, 13, 12, 11, 9]);
    let worse = strength(HandCategory::HighCard, [14, 13, 12, 11, 8]);
    assert!(better > worse);
}

#[test]
fn identical_hands_compare_equal() {
    let a = strength(HandCategory::Straight, [9, 0, 0, 0, 0]);
    let b = strength(HandCategory::Straight, [9, 0, 0, 0, 0]);
    assert_eq!(a, b);
    assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
}

// ----- 2.6 Error / boundary cases -----

#[test]
fn errors_on_too_few_cards() {
    // The board must hold exactly five cards; four total cards cannot evaluate.
    let result = evaluate_best_holdem_hand(
        &[card(Ace, Spades), card(King, Spades)],
        &[card(Queen, Spades), card(Jack, Spades)],
    );
    assert!(result.is_err());
}

#[test]
fn errors_on_wrong_hole_card_count() {
    // The contract requires exactly two hole cards.
    let board = [
        card(Queen, Spades),
        card(Jack, Spades),
        card(Ten, Spades),
        card(Six, Diamonds),
        card(Two, Clubs),
    ];
    assert!(evaluate_best_holdem_hand(&[card(Ace, Spades)], &board).is_err());
    assert!(evaluate_best_holdem_hand(
        &[card(Ace, Spades), card(King, Spades), card(Queen, Diamonds)],
        &board,
    )
    .is_err());
}
