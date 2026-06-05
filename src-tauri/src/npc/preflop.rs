use crate::domain::{Card, Rank};

/// Relative strength tier for a two-card starting hand.
///
/// Ordered from weakest to strongest so `>` comparisons work naturally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PreflopTier {
    Fold,
    Marginal,
    Playable,
    Strong,
    Premium,
}

/// Classify a two-card starting hand into a `PreflopTier`.
///
/// Panics in debug if `hole_cards` does not have exactly 2 elements.
pub fn preflop_hand_tier(hole_cards: &[Card]) -> PreflopTier {
    debug_assert_eq!(
        hole_cards.len(),
        2,
        "preflop_hand_tier requires exactly 2 hole cards"
    );
    if hole_cards.len() < 2 {
        return PreflopTier::Fold;
    }

    let c1 = hole_cards[0];
    let c2 = hole_cards[1];

    // Normalise so high_rank >= low_rank
    let (high_rank, low_rank) = if c1.rank >= c2.rank {
        (c1.rank, c2.rank)
    } else {
        (c2.rank, c1.rank)
    };
    let suited = c1.suit == c2.suit;
    let is_pair = high_rank == low_rank;

    if is_pair {
        return pair_tier(high_rank);
    }

    unpaired_tier(high_rank, low_rank, suited)
}

fn pair_tier(rank: Rank) -> PreflopTier {
    match rank {
        Rank::Ace | Rank::King | Rank::Queen | Rank::Jack | Rank::Ten => PreflopTier::Premium,
        Rank::Nine | Rank::Eight | Rank::Seven => PreflopTier::Strong,
        Rank::Six | Rank::Five | Rank::Four | Rank::Three | Rank::Two => PreflopTier::Playable,
    }
}

fn unpaired_tier(high: Rank, low: Rank, suited: bool) -> PreflopTier {
    use Rank::*;

    match (high, low, suited) {
        // ── Premium ──────────────────────────────────────────────────
        // AK always premium
        (Ace, King, _) => PreflopTier::Premium,
        // AQ both suited and offsuit
        (Ace, Queen, _) => PreflopTier::Premium,
        // AJs
        (Ace, Jack, true) => PreflopTier::Premium,

        // ── Strong ───────────────────────────────────────────────────
        // AJ offsuit
        (Ace, Jack, false) => PreflopTier::Strong,
        // ATs
        (Ace, Ten, true) => PreflopTier::Strong,
        // KQ both
        (King, Queen, _) => PreflopTier::Strong,
        // KJs
        (King, Jack, true) => PreflopTier::Strong,

        // ── Playable ─────────────────────────────────────────────────
        // A9s–A2s (any ace suited below AT)
        (Ace, low_rank, true) if low_rank <= Nine => PreflopTier::Playable,
        // Suited broadway connectors / one-gappers
        (King, Jack, false) => PreflopTier::Playable,
        (King, Ten, true) => PreflopTier::Playable,
        (Queen, Jack, true) => PreflopTier::Playable,
        (Jack, Ten, true) => PreflopTier::Playable,
        (Ten, Nine, true) => PreflopTier::Playable,
        (Nine, Eight, true) => PreflopTier::Playable,
        (Eight, Seven, true) => PreflopTier::Playable,
        (Seven, Six, true) => PreflopTier::Playable,
        (Six, Five, true) => PreflopTier::Playable,
        (Five, Four, true) => PreflopTier::Playable,

        // ── Marginal ─────────────────────────────────────────────────
        // Any two broadway cards offsuit not already categorised
        (high_rank, low_rank, false) if high_rank >= Ten && low_rank >= Ten => {
            PreflopTier::Marginal
        }
        // Weak aces offsuit
        (Ace, _, false) => PreflopTier::Marginal,

        // ── Fold ─────────────────────────────────────────────────────
        _ => PreflopTier::Fold,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Card, Rank, Suit};
    use Rank::*;
    use Suit::*;

    fn card(rank: Rank, suit: Suit) -> Card {
        Card { rank, suit }
    }

    #[test]
    fn pairs() {
        assert_eq!(
            preflop_hand_tier(&[card(Ace, Spades), card(Ace, Hearts)]),
            PreflopTier::Premium
        );
        assert_eq!(
            preflop_hand_tier(&[card(King, Spades), card(King, Hearts)]),
            PreflopTier::Premium
        );
        assert_eq!(
            preflop_hand_tier(&[card(Ten, Spades), card(Ten, Hearts)]),
            PreflopTier::Premium
        );
        assert_eq!(
            preflop_hand_tier(&[card(Nine, Spades), card(Nine, Hearts)]),
            PreflopTier::Strong
        );
        assert_eq!(
            preflop_hand_tier(&[card(Seven, Spades), card(Seven, Hearts)]),
            PreflopTier::Strong
        );
        assert_eq!(
            preflop_hand_tier(&[card(Six, Spades), card(Six, Hearts)]),
            PreflopTier::Playable
        );
        assert_eq!(
            preflop_hand_tier(&[card(Two, Spades), card(Two, Hearts)]),
            PreflopTier::Playable
        );
    }

    #[test]
    fn premium_unpaired() {
        assert_eq!(
            preflop_hand_tier(&[card(Ace, Spades), card(King, Hearts)]),
            PreflopTier::Premium
        );
        assert_eq!(
            preflop_hand_tier(&[card(Ace, Spades), card(King, Spades)]),
            PreflopTier::Premium
        );
        assert_eq!(
            preflop_hand_tier(&[card(Ace, Spades), card(Queen, Hearts)]),
            PreflopTier::Premium
        );
        assert_eq!(
            preflop_hand_tier(&[card(Ace, Spades), card(Jack, Spades)]),
            PreflopTier::Premium
        );
    }

    #[test]
    fn strong_unpaired() {
        assert_eq!(
            preflop_hand_tier(&[card(Ace, Spades), card(Jack, Hearts)]),
            PreflopTier::Strong
        );
        assert_eq!(
            preflop_hand_tier(&[card(Ace, Spades), card(Ten, Spades)]),
            PreflopTier::Strong
        );
        assert_eq!(
            preflop_hand_tier(&[card(King, Spades), card(Queen, Hearts)]),
            PreflopTier::Strong
        );
        assert_eq!(
            preflop_hand_tier(&[card(King, Spades), card(Jack, Spades)]),
            PreflopTier::Strong
        );
    }

    #[test]
    fn playable_suited_connectors() {
        assert_eq!(
            preflop_hand_tier(&[card(Jack, Spades), card(Ten, Spades)]),
            PreflopTier::Playable
        );
        assert_eq!(
            preflop_hand_tier(&[card(Ten, Spades), card(Nine, Spades)]),
            PreflopTier::Playable
        );
        assert_eq!(
            preflop_hand_tier(&[card(Seven, Spades), card(Six, Spades)]),
            PreflopTier::Playable
        );
        assert_eq!(
            preflop_hand_tier(&[card(Ace, Spades), card(Five, Spades)]),
            PreflopTier::Playable
        );
        assert_eq!(
            preflop_hand_tier(&[card(Ace, Spades), card(Two, Spades)]),
            PreflopTier::Playable
        );
    }

    #[test]
    fn marginal_and_fold() {
        // Offsuit broadway
        assert_eq!(
            preflop_hand_tier(&[card(Jack, Spades), card(Ten, Hearts)]),
            PreflopTier::Marginal
        );
        // Weak ace offsuit
        assert_eq!(
            preflop_hand_tier(&[card(Ace, Spades), card(Five, Hearts)]),
            PreflopTier::Marginal
        );
        // Random junk
        assert_eq!(
            preflop_hand_tier(&[card(Seven, Spades), card(Two, Hearts)]),
            PreflopTier::Fold
        );
        assert_eq!(
            preflop_hand_tier(&[card(Nine, Spades), card(Four, Hearts)]),
            PreflopTier::Fold
        );
    }
}
