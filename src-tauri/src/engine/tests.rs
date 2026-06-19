
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
