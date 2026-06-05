use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use rand_core::{OsRng, RngCore};

use crate::{
    app_state::ModuleDescriptor,
    domain::{ActionType, Card, PotSummary, Rank, Suit},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineError {
    message: String,
}

impl EngineError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for EngineError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deck {
    cards: Vec<Card>,
}

impl Deck {
    #[must_use]
    pub fn standard_52() -> Self {
        let mut cards = Vec::with_capacity(52);
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            for rank in [
                Rank::Two,
                Rank::Three,
                Rank::Four,
                Rank::Five,
                Rank::Six,
                Rank::Seven,
                Rank::Eight,
                Rank::Nine,
                Rank::Ten,
                Rank::Jack,
                Rank::Queen,
                Rank::King,
                Rank::Ace,
            ] {
                cards.push(Card { rank, suit });
            }
        }

        Self { cards }
    }

    pub fn from_cards(cards: Vec<Card>) -> Result<Self, EngineError> {
        let unique_cards = cards.iter().copied().collect::<BTreeSet<_>>();
        if unique_cards.len() != cards.len() {
            return Err(EngineError::new("deck cannot contain duplicate cards"));
        }

        Ok(Self { cards })
    }

    #[must_use]
    pub fn shuffled() -> Self {
        let mut deck = Self::standard_52();
        let mut rng = OsRng;
        deck.shuffle(&mut rng);
        deck
    }

    pub fn shuffle(&mut self, rng: &mut impl RngCore) {
        for index in (1..self.cards.len()).rev() {
            let swap_with = (rng.next_u64() as usize) % (index + 1);
            self.cards.swap(index, swap_with);
        }
    }

    #[must_use]
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.cards.len()
    }

    pub fn deal_one(&mut self) -> Result<Card, EngineError> {
        if self.cards.is_empty() {
            return Err(EngineError::new("deck is exhausted"));
        }

        Ok(self.cards.remove(0))
    }

    pub fn burn_one(&mut self) -> Result<Card, EngineError> {
        self.deal_one()
    }

    pub fn deal_hole_cards(
        &mut self,
        player_ids_in_order: &[String],
    ) -> Result<BTreeMap<String, Vec<Card>>, EngineError> {
        let mut hole_cards_by_player_id = player_ids_in_order
            .iter()
            .cloned()
            .map(|player_id| (player_id, Vec::with_capacity(2)))
            .collect::<BTreeMap<_, _>>();

        for _ in 0..2 {
            for player_id in player_ids_in_order {
                let card = self.deal_one()?;
                hole_cards_by_player_id
                    .get_mut(player_id)
                    .expect("player hole card bucket should exist")
                    .push(card);
            }
        }

        Ok(hole_cards_by_player_id)
    }

    pub fn reveal_flop(&mut self) -> Result<Vec<Card>, EngineError> {
        self.burn_one()?;
        self.deal_many(3)
    }

    pub fn reveal_turn(&mut self) -> Result<Card, EngineError> {
        self.burn_one()?;
        self.deal_one()
    }

    pub fn reveal_river(&mut self) -> Result<Card, EngineError> {
        self.burn_one()?;
        self.deal_one()
    }

    fn deal_many(&mut self, count: usize) -> Result<Vec<Card>, EngineError> {
        (0..count).map(|_| self.deal_one()).collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandCategory {
    HighCard,
    OnePair,
    TwoPair,
    ThreeOfAKind,
    Straight,
    Flush,
    FullHouse,
    FourOfAKind,
    StraightFlush,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandStrength {
    pub category: HandCategory,
    pub key_ranks: [u8; 5],
}

impl HandStrength {
    #[must_use]
    pub fn key_ranks(&self) -> [u8; 5] {
        self.key_ranks
    }
}

impl Ord for HandStrength {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.category, self.key_ranks).cmp(&(other.category, other.key_ranks))
    }
}

impl PartialOrd for HandStrength {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettlementResult {
    pub winning_player_ids: Vec<String>,
    pub payouts_by_player_id: BTreeMap<String, u32>,
    pub pot_summaries: Vec<PotSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegalActionContext {
    pub player_stack: u32,
    pub player_commit: u32,
    pub current_bet: u32,
    pub min_full_raise_to: u32,
    pub may_raise: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegalActionSet {
    pub legal_actions: Vec<ActionType>,
    pub call_amount: u32,
    pub min_raise_to: Option<u32>,
    pub max_raise_to: Option<u32>,
}

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "engine",
        responsibility:
            "Evaluates poker hands, betting legality, and hand settlement without letting the frontend become the source of truth.",
    }
}

pub fn evaluate_best_holdem_hand(
    hole_cards: &[Card],
    board_cards: &[Card],
) -> Result<HandStrength, EngineError> {
    if hole_cards.len() != 2 {
        return Err(EngineError::new(
            "hold'em hole cards must contain exactly two cards",
        ));
    }

    if board_cards.len() != 5 {
        return Err(EngineError::new(
            "hold'em board must contain exactly five cards",
        ));
    }

    let mut combined = Vec::with_capacity(7);
    combined.extend_from_slice(hole_cards);
    combined.extend_from_slice(board_cards);

    let unique_cards = combined.iter().copied().collect::<BTreeSet<_>>();
    if unique_cards.len() != combined.len() {
        return Err(EngineError::new(
            "duplicate cards detected while evaluating hand",
        ));
    }

    let mut best_strength = None;
    for first in 0..combined.len() - 4 {
        for second in first + 1..combined.len() - 3 {
            for third in second + 1..combined.len() - 2 {
                for fourth in third + 1..combined.len() - 1 {
                    for fifth in fourth + 1..combined.len() {
                        let strength = evaluate_five_card_hand([
                            combined[first],
                            combined[second],
                            combined[third],
                            combined[fourth],
                            combined[fifth],
                        ]);

                        if best_strength.is_none_or(|best| strength > best) {
                            best_strength = Some(strength);
                        }
                    }
                }
            }
        }
    }

    best_strength.ok_or_else(|| EngineError::new("could not evaluate best hold'em hand"))
}

#[must_use]
pub fn legal_actions(context: &LegalActionContext) -> LegalActionSet {
    let to_call = context.current_bet.saturating_sub(context.player_commit);
    let max_raise_to = context.player_commit + context.player_stack;
    let mut legal_actions = BTreeSet::new();

    if to_call == 0 {
        legal_actions.insert(ActionType::Check);
    } else {
        legal_actions.insert(ActionType::Fold);
        if context.player_stack > to_call {
            legal_actions.insert(ActionType::Call);
        } else {
            legal_actions.insert(ActionType::AllIn);
        }
    }

    if context.may_raise && context.player_stack > to_call {
        if max_raise_to >= context.min_full_raise_to {
            legal_actions.insert(if context.current_bet == 0 {
                ActionType::Bet
            } else {
                ActionType::Raise
            });
            legal_actions.insert(ActionType::AllIn);
        } else if max_raise_to > context.current_bet {
            legal_actions.insert(ActionType::AllIn);
        }
    }

    let has_full_raise =
        legal_actions.contains(&ActionType::Bet) || legal_actions.contains(&ActionType::Raise);

    LegalActionSet {
        legal_actions: legal_actions.into_iter().collect(),
        call_amount: to_call.min(context.player_stack),
        min_raise_to: has_full_raise.then_some(context.min_full_raise_to),
        max_raise_to: has_full_raise.then_some(max_raise_to),
    }
}

pub fn settle_showdown(
    contributions_by_player_id: &BTreeMap<String, u32>,
    hand_strengths_by_player_id: &BTreeMap<String, HandStrength>,
    odd_chip_order: &[String],
) -> Result<SettlementResult, EngineError> {
    if contributions_by_player_id.is_empty() {
        return Err(EngineError::new(
            "cannot settle a hand without contributions",
        ));
    }

    if hand_strengths_by_player_id.is_empty() {
        return Err(EngineError::new(
            "cannot settle a hand without at least one contender",
        ));
    }

    let thresholds = contributions_by_player_id
        .values()
        .copied()
        .filter(|amount| *amount > 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let mut previous_threshold = 0_u32;
    let mut payouts_by_player_id = BTreeMap::new();
    let mut pot_summaries = Vec::new();
    let mut winning_player_ids = BTreeSet::new();

    for (pot_index, threshold) in thresholds.into_iter().enumerate() {
        let contributors = contributions_by_player_id
            .iter()
            .filter(|(_, contribution)| **contribution >= threshold)
            .map(|(player_id, _)| player_id.clone())
            .collect::<Vec<_>>();

        let eligible_player_ids = contributors
            .iter()
            .filter(|player_id| hand_strengths_by_player_id.contains_key(player_id.as_str()))
            .cloned()
            .collect::<Vec<_>>();

        if eligible_player_ids.is_empty() {
            return Err(EngineError::new(
                "a side pot was created without an eligible contender",
            ));
        }

        let pot_amount = (threshold - previous_threshold) * contributors.len() as u32;
        previous_threshold = threshold;

        let best_strength = eligible_player_ids
            .iter()
            .filter_map(|player_id| hand_strengths_by_player_id.get(player_id))
            .max()
            .copied()
            .ok_or_else(|| EngineError::new("missing winning hand strength"))?;

        let winner_player_ids = eligible_player_ids
            .iter()
            .filter(|player_id| {
                hand_strengths_by_player_id
                    .get(player_id.as_str())
                    .is_some_and(|strength| *strength == best_strength)
            })
            .cloned()
            .collect::<Vec<_>>();

        let split_amount = pot_amount / winner_player_ids.len() as u32;
        let odd_chip_count = pot_amount % winner_player_ids.len() as u32;
        let odd_chip_awarded_to = if odd_chip_count > 0 {
            Some(select_odd_chip_recipient(
                odd_chip_order,
                &winner_player_ids,
            ))
        } else {
            None
        };

        for winner_player_id in &winner_player_ids {
            let payout = payouts_by_player_id
                .entry(winner_player_id.clone())
                .or_insert(0);
            *payout += split_amount;
            winning_player_ids.insert(winner_player_id.clone());
        }

        if let Some(odd_chip_player_id) = odd_chip_awarded_to.as_ref() {
            let payout = payouts_by_player_id
                .entry(odd_chip_player_id.clone())
                .or_insert(0);
            *payout += odd_chip_count;
        }

        pot_summaries.push(PotSummary {
            pot_index: pot_index as u8,
            amount: pot_amount,
            eligible_player_ids,
            winner_player_ids,
            odd_chip_count,
            odd_chip_awarded_to,
        });
    }

    Ok(SettlementResult {
        winning_player_ids: winning_player_ids.into_iter().collect(),
        payouts_by_player_id,
        pot_summaries,
    })
}

fn select_odd_chip_recipient(odd_chip_order: &[String], winners: &[String]) -> String {
    odd_chip_order
        .iter()
        .find(|player_id| winners.contains(player_id))
        .cloned()
        .unwrap_or_else(|| winners[0].clone())
}

pub(crate) fn evaluate_five_card_hand(cards: [Card; 5]) -> HandStrength {
    let flush = cards.iter().all(|card| card.suit == cards[0].suit);
    let rank_values = cards
        .iter()
        .map(|card| rank_value(card.rank))
        .collect::<Vec<_>>();
    let straight_high = straight_high_card(&rank_values);
    let mut counts_by_rank = BTreeMap::new();
    for rank_value in rank_values {
        *counts_by_rank.entry(rank_value).or_insert(0_u8) += 1;
    }

    let mut groups = counts_by_rank
        .into_iter()
        .map(|(rank, count)| (count, rank))
        .collect::<Vec<_>>();
    groups.sort_unstable_by(|left, right| right.cmp(left));

    if flush && straight_high.is_some() {
        return HandStrength {
            category: HandCategory::StraightFlush,
            key_ranks: [straight_high.unwrap_or_default(), 0, 0, 0, 0],
        };
    }

    if let Some((_, quad_rank)) = groups.iter().find(|(count, _)| *count == 4) {
        let kicker = groups
            .iter()
            .find(|(count, _)| *count == 1)
            .map(|(_, rank)| *rank)
            .unwrap_or_default();
        return HandStrength {
            category: HandCategory::FourOfAKind,
            key_ranks: [*quad_rank, kicker, 0, 0, 0],
        };
    }

    if groups[0].0 == 3 && groups.get(1).is_some_and(|group| group.0 == 2) {
        return HandStrength {
            category: HandCategory::FullHouse,
            key_ranks: [groups[0].1, groups[1].1, 0, 0, 0],
        };
    }

    if flush {
        let mut sorted_ranks = groups.iter().map(|(_, rank)| *rank).collect::<Vec<_>>();
        sorted_ranks.sort_unstable_by(|left, right| right.cmp(left));
        return HandStrength {
            category: HandCategory::Flush,
            key_ranks: [
                sorted_ranks[0],
                sorted_ranks[1],
                sorted_ranks[2],
                sorted_ranks[3],
                sorted_ranks[4],
            ],
        };
    }

    if let Some(straight_high) = straight_high {
        return HandStrength {
            category: HandCategory::Straight,
            key_ranks: [straight_high, 0, 0, 0, 0],
        };
    }

    if groups[0].0 == 3 {
        let kickers = groups
            .iter()
            .filter(|(count, _)| *count == 1)
            .map(|(_, rank)| *rank)
            .collect::<Vec<_>>();
        return HandStrength {
            category: HandCategory::ThreeOfAKind,
            key_ranks: [groups[0].1, kickers[0], kickers[1], 0, 0],
        };
    }

    let pair_ranks = groups
        .iter()
        .filter(|(count, _)| *count == 2)
        .map(|(_, rank)| *rank)
        .collect::<Vec<_>>();
    if pair_ranks.len() == 2 {
        let kicker = groups
            .iter()
            .find(|(count, _)| *count == 1)
            .map(|(_, rank)| *rank)
            .unwrap_or_default();
        let high_pair = pair_ranks[0].max(pair_ranks[1]);
        let low_pair = pair_ranks[0].min(pair_ranks[1]);
        return HandStrength {
            category: HandCategory::TwoPair,
            key_ranks: [high_pair, low_pair, kicker, 0, 0],
        };
    }

    if pair_ranks.len() == 1 {
        let kickers = groups
            .iter()
            .filter(|(count, _)| *count == 1)
            .map(|(_, rank)| *rank)
            .collect::<Vec<_>>();
        return HandStrength {
            category: HandCategory::OnePair,
            key_ranks: [pair_ranks[0], kickers[0], kickers[1], kickers[2], 0],
        };
    }

    let high_cards = groups.iter().map(|(_, rank)| *rank).collect::<Vec<_>>();
    HandStrength {
        category: HandCategory::HighCard,
        key_ranks: [
            high_cards[0],
            high_cards[1],
            high_cards[2],
            high_cards[3],
            high_cards[4],
        ],
    }
}

fn straight_high_card(rank_values: &[u8]) -> Option<u8> {
    let mut unique_ranks = rank_values
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    unique_ranks.sort_unstable();

    if unique_ranks == [2, 3, 4, 5, 14] {
        return Some(5);
    }

    if unique_ranks.len() != 5 {
        return None;
    }

    unique_ranks
        .windows(2)
        .all(|window| window[1] == window[0] + 1)
        .then_some(*unique_ranks.last().unwrap_or(&0))
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

    #[derive(Clone)]
    struct TestRng(u64);

    impl RngCore for TestRng {
        fn next_u32(&mut self) -> u32 {
            self.next_u64() as u32
        }

        fn next_u64(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            self.try_fill_bytes(dest)
                .expect("test rng should fill bytes")
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
            for chunk in dest.chunks_mut(8) {
                let bytes = self.next_u64().to_le_bytes();
                for (index, byte) in chunk.iter_mut().enumerate() {
                    *byte = bytes[index];
                }
            }
            Ok(())
        }
    }

    fn card(rank: Rank, suit: Suit) -> Card {
        Card { rank, suit }
    }

    #[test]
    fn standard_deck_contains_52_unique_cards() {
        let deck = Deck::standard_52();

        assert_eq!(deck.remaining(), 52);
        assert_eq!(
            deck.cards().iter().copied().collect::<BTreeSet<_>>().len(),
            52
        );
    }

    #[test]
    fn shuffle_reorders_cards_without_losing_any() {
        let mut deck = Deck::standard_52();
        let original = deck.cards().to_vec();

        deck.shuffle(&mut TestRng(0xC0FFEE));

        assert_eq!(deck.remaining(), 52);
        assert_eq!(
            deck.cards().iter().copied().collect::<BTreeSet<_>>().len(),
            52
        );
        assert_ne!(deck.cards(), original.as_slice());
    }

    #[test]
    fn dealing_and_street_reveals_follow_holdem_shape() {
        let mut deck = Deck::from_cards(vec![
            card(Rank::Ace, Suit::Spades),
            card(Rank::King, Suit::Spades),
            card(Rank::Queen, Suit::Spades),
            card(Rank::Jack, Suit::Spades),
            card(Rank::Ten, Suit::Spades),
            card(Rank::Nine, Suit::Spades),
            card(Rank::Eight, Suit::Spades),
            card(Rank::Seven, Suit::Spades),
            card(Rank::Six, Suit::Spades),
            card(Rank::Five, Suit::Spades),
            card(Rank::Four, Suit::Spades),
            card(Rank::Three, Suit::Spades),
        ])
        .expect("test deck should be valid");

        let hole_cards = deck
            .deal_hole_cards(&["a".to_string(), "b".to_string()])
            .expect("hole cards should deal");
        let flop = deck.reveal_flop().expect("flop should reveal");
        let turn = deck.reveal_turn().expect("turn should reveal");
        let river = deck.reveal_river().expect("river should reveal");

        assert_eq!(hole_cards["a"].len(), 2);
        assert_eq!(hole_cards["b"].len(), 2);
        assert_eq!(flop.len(), 3);
        assert_eq!(turn, card(Rank::Five, Suit::Spades));
        assert_eq!(river, card(Rank::Three, Suit::Spades));
        assert_eq!(deck.remaining(), 0);
    }

    #[test]
    fn evaluator_picks_best_five_card_hand() {
        let strength = evaluate_best_holdem_hand(
            &[
                card(Rank::Ace, Suit::Spades),
                card(Rank::King, Suit::Spades),
            ],
            &[
                card(Rank::Queen, Suit::Spades),
                card(Rank::Jack, Suit::Spades),
                card(Rank::Ten, Suit::Spades),
                card(Rank::Two, Suit::Clubs),
                card(Rank::Two, Suit::Hearts),
            ],
        )
        .expect("hand should evaluate");

        assert_eq!(strength.category, HandCategory::StraightFlush);
        assert_eq!(strength.key_ranks(), [14, 0, 0, 0, 0]);
    }

    #[test]
    fn side_pots_and_odd_chip_are_settled_deterministically() {
        let result = settle_showdown(
            &BTreeMap::from([
                ("alice".to_string(), 101),
                ("bob".to_string(), 101),
                ("carol".to_string(), 40),
            ]),
            &BTreeMap::from([
                (
                    "alice".to_string(),
                    HandStrength {
                        category: HandCategory::Straight,
                        key_ranks: [9, 0, 0, 0, 0],
                    },
                ),
                (
                    "bob".to_string(),
                    HandStrength {
                        category: HandCategory::Straight,
                        key_ranks: [9, 0, 0, 0, 0],
                    },
                ),
                (
                    "carol".to_string(),
                    HandStrength {
                        category: HandCategory::TwoPair,
                        key_ranks: [14, 13, 12, 0, 0],
                    },
                ),
            ]),
            &["bob".to_string(), "alice".to_string(), "carol".to_string()],
        )
        .expect("settlement should succeed");

        assert_eq!(result.pot_summaries.len(), 2);
        assert_eq!(result.pot_summaries[0].amount, 120);
        assert_eq!(
            result.pot_summaries[0].winner_player_ids,
            vec!["alice", "bob"]
        );
        assert_eq!(result.pot_summaries[1].amount, 122);
        assert_eq!(result.pot_summaries[1].odd_chip_count, 0);
        assert_eq!(result.payouts_by_player_id["alice"], 121);
        assert_eq!(result.payouts_by_player_id["bob"], 121);

        let odd_chip_result = settle_showdown(
            &BTreeMap::from([("alice".to_string(), 5), ("bob".to_string(), 5)]),
            &BTreeMap::from([
                (
                    "alice".to_string(),
                    HandStrength {
                        category: HandCategory::OnePair,
                        key_ranks: [14, 13, 12, 11, 0],
                    },
                ),
                (
                    "bob".to_string(),
                    HandStrength {
                        category: HandCategory::OnePair,
                        key_ranks: [14, 13, 12, 11, 0],
                    },
                ),
            ]),
            &["bob".to_string(), "alice".to_string()],
        )
        .expect("odd chip settlement should succeed");

        assert_eq!(odd_chip_result.pot_summaries[0].odd_chip_count, 0);
        assert_eq!(odd_chip_result.payouts_by_player_id["alice"], 5);
        assert_eq!(odd_chip_result.payouts_by_player_id["bob"], 5);
    }

    #[test]
    fn explicit_all_in_is_emitted_when_no_full_raise_exists() {
        let short_stack = legal_actions(&LegalActionContext {
            player_stack: 30,
            player_commit: 20,
            current_bet: 50,
            min_full_raise_to: 80,
            may_raise: true,
        });
        assert_eq!(
            short_stack.legal_actions,
            vec![ActionType::Fold, ActionType::AllIn]
        );
        assert_eq!(short_stack.call_amount, 30);
        assert_eq!(short_stack.min_raise_to, None);
        assert_eq!(short_stack.max_raise_to, None);

        let can_raise = legal_actions(&LegalActionContext {
            player_stack: 150,
            player_commit: 20,
            current_bet: 50,
            min_full_raise_to: 80,
            may_raise: true,
        });
        assert!(can_raise.legal_actions.contains(&ActionType::Raise));
        assert!(can_raise.legal_actions.contains(&ActionType::AllIn));
        assert_eq!(can_raise.min_raise_to, Some(80));
        assert_eq!(can_raise.max_raise_to, Some(170));
    }

    #[test]
    fn full_raise_requires_meeting_the_minimum_raise_to() {
        let short_stack = legal_actions(&LegalActionContext {
            player_stack: 120,
            player_commit: 0,
            current_bet: 100,
            min_full_raise_to: 200,
            may_raise: true,
        });
        assert_eq!(
            short_stack.legal_actions,
            vec![ActionType::Fold, ActionType::Call, ActionType::AllIn]
        );
        assert_eq!(short_stack.min_raise_to, None);
        assert_eq!(short_stack.max_raise_to, None);

        let full_stack = legal_actions(&LegalActionContext {
            player_stack: 400,
            player_commit: 0,
            current_bet: 100,
            min_full_raise_to: 200,
            may_raise: true,
        });
        assert!(full_stack.legal_actions.contains(&ActionType::Call));
        assert!(full_stack.legal_actions.contains(&ActionType::Raise));
        assert_eq!(full_stack.min_raise_to, Some(200));
        assert_eq!(full_stack.max_raise_to, Some(400));
    }
}
