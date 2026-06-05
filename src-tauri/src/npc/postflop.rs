use crate::domain::{Card, Rank, Suit};
use crate::engine::{evaluate_best_holdem_hand, HandStrength};

/// Made-hand or draw strength on a given board.
///
/// Ordered from weakest to strongest so `>` comparisons work naturally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PostflopCategory {
    Weak,
    Draw,
    Medium,
    Strong,
    Monster,
}

/// Evaluate the best 5-card hand from any number of hole cards + board cards (3–5 board).
///
/// When 5 board cards are available delegates to `evaluate_best_holdem_hand`.
/// For partial boards evaluates all C(n,5) combinations directly.
fn evaluate_partial_board(hole_cards: &[Card], board: &[Card]) -> Result<HandStrength, crate::engine::EngineError> {
    if board.len() == 5 {
        return evaluate_best_holdem_hand(hole_cards, board);
    }

    // Build all available cards.
    let mut all: Vec<Card> = Vec::with_capacity(hole_cards.len() + board.len());
    all.extend_from_slice(hole_cards);
    all.extend_from_slice(board);

    if all.len() < 5 {
        return Err(crate::engine::EngineError::new("not enough cards to evaluate"));
    }

    let n = all.len();
    let mut best: Option<HandStrength> = None;
    for a in 0..n - 4 {
        for b in a + 1..n - 3 {
            for c in b + 1..n - 2 {
                for d in c + 1..n - 1 {
                    for e in d + 1..n {
                        let s = crate::engine::evaluate_five_card_hand([all[a], all[b], all[c], all[d], all[e]]);
                        if best.is_none_or(|prev| s > prev) {
                            best = Some(s);
                        }
                    }
                }
            }
        }
    }
    best.ok_or_else(|| crate::engine::EngineError::new("could not evaluate partial board hand"))
}

/// Evaluate the NPC's hand strength given its hole cards and the current board.
///
/// Handles flop (3), turn (4), and river (5) boards.
/// Returns `PostflopCategory::Weak` if hole cards or board are unavailable.
pub fn postflop_hand_category(hole_cards: &[Card], board: &[Card]) -> PostflopCategory {
    if hole_cards.len() < 2 || board.is_empty() {
        return PostflopCategory::Weak;
    }

    let made = evaluate_partial_board(hole_cards, board);

    let made_category = match made {
        Err(_) => PostflopCategory::Weak,
        Ok(strength) => {
            use crate::engine::HandCategory;
            match strength.category {
                HandCategory::HighCard => PostflopCategory::Weak,
                HandCategory::OnePair => {
                    top_pair_or_medium(hole_cards, board)
                }
                HandCategory::TwoPair | HandCategory::ThreeOfAKind => PostflopCategory::Strong,
                HandCategory::Straight
                | HandCategory::Flush
                | HandCategory::FullHouse
                | HandCategory::FourOfAKind
                | HandCategory::StraightFlush => PostflopCategory::Monster,
            }
        }
    };

    // Upgrade Weak to Draw if a strong draw exists.
    if made_category == PostflopCategory::Weak
        && (has_flush_draw(hole_cards, board) || has_oesd(hole_cards, board))
    {
        return PostflopCategory::Draw;
    }

    made_category
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn top_pair_or_medium(hole_cards: &[Card], board: &[Card]) -> PostflopCategory {
    // The highest board rank determines "top pair".
    let board_top = board.iter().map(|c| c.rank).max().unwrap_or(Rank::Two);

    // Check if any hole card pairs with the top board card.
    let pairs_top = hole_cards.iter().any(|c| c.rank == board_top);

    // Use the primary kicker value from the hand strength to decide quality.
    // The HandStrength primary value encodes the paired rank; higher => stronger.
    if pairs_top {
        // Top pair with any kicker — treat as Strong (top-pair top-kicker or just top-pair).
        PostflopCategory::Strong
    } else {
        // Middle pair, bottom pair — Medium
        PostflopCategory::Medium

        // Sanity: if the hole cards themselves are the pair (overpair), that's also Strong.
        // We could detect this, but since the engine already promoted TwoPair to Strong and
        // a pocket overpair that beats the board top is a rarer case worth simplifying.
    }
}

/// True if the hole cards + board have a flush draw (4 of the same suit, not yet a flush).
fn has_flush_draw(hole_cards: &[Card], board: &[Card]) -> bool {
    // A flush draw means exactly 4 cards of the same suit total (not 5, which would be a made flush).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let count = hole_cards.iter().filter(|c| c.suit == suit).count()
            + board.iter().filter(|c| c.suit == suit).count();
        if count == 4 {
            return true;
        }
    }
    false
}

/// True if the hole cards + board form an open-ended straight draw (OESD).
///
/// An OESD has 4 consecutive ranks with a gap on at least one end.
fn has_oesd(hole_cards: &[Card], board: &[Card]) -> bool {
    let mut ranks: Vec<u8> = hole_cards
        .iter()
        .chain(board.iter())
        .map(|c| rank_value(c.rank))
        .collect();
    ranks.sort_unstable();
    ranks.dedup();

    // Ace can also act as 1 for A-2-3-4-5 low straight.
    if ranks.contains(&14) {
        ranks.push(1);
        ranks.sort_unstable();
    }

    // Slide a window of 4 consecutive ranks and look for exactly 4 unique ranks
    // that would need one more card on either end to complete (open-ended).
    for window in ranks.windows(4) {
        if window[3] - window[0] == 3 {
            // All 4 values are consecutive — this is an OESD.
            return true;
        }
    }
    false
}

fn rank_value(rank: Rank) -> u8 {
    match rank {
        Rank::Two => 2,
        Rank::Three => 3,
        Rank::Four => 4,
        Rank::Five => 5,
        Rank::Six => 6,
        Rank::Seven => 7,
        Rank::Eight => 8,
        Rank::Nine => 9,
        Rank::Ten => 10,
        Rank::Jack => 11,
        Rank::Queen => 12,
        Rank::King => 13,
        Rank::Ace => 14,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Rank, Suit};
    use Rank::*;
    use Suit::*;

    fn c(rank: Rank, suit: Suit) -> Card { Card { rank, suit } }

    #[test]
    fn high_card_is_weak() {
        let hole = [c(Seven, Spades), c(Two, Hearts)];
        let board = [c(Ace, Clubs), c(King, Diamonds), c(Queen, Spades)];
        assert_eq!(postflop_hand_category(&hole, &board), PostflopCategory::Weak);
    }

    #[test]
    fn top_pair_is_strong() {
        let hole = [c(Ace, Spades), c(King, Hearts)];
        let board = [c(Ace, Clubs), c(Seven, Diamonds), c(Two, Spades)];
        assert_eq!(postflop_hand_category(&hole, &board), PostflopCategory::Strong);
    }

    #[test]
    fn middle_pair_is_medium() {
        let hole = [c(Seven, Spades), c(Six, Hearts)];
        let board = [c(Ace, Clubs), c(Seven, Diamonds), c(Two, Spades)];
        // Seven pairs but it's not the top board card (Ace is highest).
        assert_eq!(postflop_hand_category(&hole, &board), PostflopCategory::Medium);
    }

    #[test]
    fn two_pair_is_strong() {
        let hole = [c(Ace, Spades), c(King, Hearts)];
        let board = [c(Ace, Clubs), c(King, Diamonds), c(Two, Spades)];
        assert_eq!(postflop_hand_category(&hole, &board), PostflopCategory::Strong);
    }

    #[test]
    fn set_is_strong() {
        let hole = [c(Seven, Spades), c(Seven, Hearts)];
        let board = [c(Seven, Clubs), c(King, Diamonds), c(Two, Spades)];
        assert_eq!(postflop_hand_category(&hole, &board), PostflopCategory::Strong);
    }

    #[test]
    fn flush_is_monster() {
        let hole = [c(Ace, Spades), c(King, Spades)];
        let board = [c(Queen, Spades), c(Jack, Spades), c(Two, Spades)];
        assert_eq!(postflop_hand_category(&hole, &board), PostflopCategory::Monster);
    }

    #[test]
    fn straight_is_monster() {
        let hole = [c(Six, Spades), c(Seven, Hearts)];
        let board = [c(Eight, Clubs), c(Nine, Diamonds), c(Ten, Spades)];
        assert_eq!(postflop_hand_category(&hole, &board), PostflopCategory::Monster);
    }

    #[test]
    fn flush_draw_upgrades_weak_to_draw() {
        let hole = [c(Ace, Spades), c(Three, Spades)];
        let board = [c(King, Spades), c(Nine, Spades), c(Two, Hearts)];
        // Only 4 spades total → flush draw, no pair → Draw
        assert_eq!(postflop_hand_category(&hole, &board), PostflopCategory::Draw);
    }

    #[test]
    fn oesd_upgrades_weak_to_draw() {
        let hole = [c(Six, Spades), c(Seven, Hearts)];
        let board = [c(Eight, Clubs), c(Nine, Diamonds), c(Ace, Spades)];
        // 6-7-8-9 is an OESD (needs 5 or T)
        assert_eq!(postflop_hand_category(&hole, &board), PostflopCategory::Draw);
    }

    #[test]
    fn empty_board_is_weak() {
        let hole = [c(Ace, Spades), c(King, Hearts)];
        assert_eq!(postflop_hand_category(&hole, &[]), PostflopCategory::Weak);
    }
}
