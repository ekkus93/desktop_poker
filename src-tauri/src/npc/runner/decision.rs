use crate::domain::{ActionType, BlindLevel, StreetPhase, TournamentState};

use super::super::postflop::postflop_hand_category;
use super::super::preflop::preflop_hand_tier;
use super::super::strategy::{
    choose_postflop_action, choose_preflop_action, derive_position, NpcAction,
};

use super::super::*;

pub(crate) fn fallback_blind_level() -> BlindLevel {
    BlindLevel {
        level_index: 0,
        label: "L1".to_string(),
        small_blind: 10,
        big_blind: 20,
        ante: 0,
        duration_seconds: 600,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn rule_based_decision(
    style: &NpcStyle,
    hole_cards: &[crate::domain::Card],
    board: &[crate::domain::Card],
    street: StreetPhase,
    pot_total: u32,
    call_amount: u32,
    min_raise_to: Option<u32>,
    max_raise_to: Option<u32>,
    facing_bet: bool,
    stack: u32,
    active_count: u8,
    dealer_seat: u8,
    npc_seat: u8,
    blind_level_index: usize,
    state: &TournamentState,
    legal_actions: &[ActionType],
    seed: u64,
) -> (ActionType, Option<u32>) {
    let position = derive_position(npc_seat, dealer_seat, active_count.max(2));

    let action = if street == StreetPhase::Preflop {
        let tier = preflop_hand_tier(hole_cards);
        let big_blind = state
            .config
            .blind_schedule
            .levels
            .get(blind_level_index)
            .map(|l| l.big_blind)
            .unwrap_or(20);
        let facing_raise = call_amount > big_blind;
        let raise_count = if state
            .current_hand
            .as_ref()
            .map(|h| h.betting_round.current_bet)
            .unwrap_or(0)
            > big_blind * 3
        {
            2
        } else if facing_raise {
            1
        } else {
            0
        };

        choose_preflop_action(
            style,
            tier,
            position,
            facing_raise,
            raise_count,
            min_raise_to,
            max_raise_to,
            call_amount,
            pot_total,
            stack,
            seed,
        )
    } else {
        let category = postflop_hand_category(hole_cards, board);
        let facing_bet_fraction = if pot_total > 0 {
            call_amount as f32 / pot_total as f32
        } else {
            0.0
        };
        choose_postflop_action(
            style,
            category,
            facing_bet,
            facing_bet_fraction,
            min_raise_to,
            max_raise_to,
            call_amount,
            pot_total,
            stack,
            seed,
        )
    };

    match action {
        NpcAction::Fold => {
            if legal_actions.contains(&ActionType::Fold) {
                (ActionType::Fold, None)
            } else {
                first_check_or_call(legal_actions)
            }
        }
        NpcAction::CheckOrCall => first_check_or_call(legal_actions),
        NpcAction::Raise(amount) => {
            if legal_actions.contains(&ActionType::Raise)
                || legal_actions.contains(&ActionType::Bet)
            {
                let at = if legal_actions.contains(&ActionType::Raise) {
                    ActionType::Raise
                } else {
                    ActionType::Bet
                };
                (at, Some(amount))
            } else if legal_actions.contains(&ActionType::AllIn) {
                (ActionType::AllIn, None)
            } else {
                first_check_or_call(legal_actions)
            }
        }
    }
}

pub(crate) fn first_check_or_call(legal: &[ActionType]) -> (ActionType, Option<u32>) {
    if legal.contains(&ActionType::Check) {
        (ActionType::Check, None)
    } else if legal.contains(&ActionType::Call) {
        (ActionType::Call, None)
    } else {
        (ActionType::Fold, None)
    }
}

pub(crate) fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 14_695_981_039_346_656_037;
    for byte in s.bytes() {
        h ^= u64::from(byte);
        h = h.wrapping_mul(1_099_511_628_211);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::rule_based_decision;
    use crate::domain::{
        ActionType, BlindLevel, BlindSchedule, Card, Rank, Rank::*, StreetPhase, Suit, Suit::*,
        TournamentConfig, TournamentPhase, TournamentState,
    };
    use crate::npc::NpcStyle;
    use std::collections::BTreeMap;

    fn card(rank: Rank, suit: Suit) -> Card {
        Card { rank, suit }
    }

    fn aces() -> Vec<Card> {
        vec![card(Ace, Spades), card(Ace, Hearts)]
    }

    fn seventy_two_offsuit() -> Vec<Card> {
        vec![card(Seven, Spades), card(Two, Diamonds)]
    }

    fn pair_of_eights() -> Vec<Card> {
        vec![card(Eight, Spades), card(Eight, Hearts)]
    }

    /// `rule_based_decision` only reads `config.blind_schedule` and
    /// `current_hand.betting_round.current_bet` from the state; everything else
    /// is filled with empty defaults.
    fn fixture_state(big_blind: u32) -> TournamentState {
        let levels = vec![BlindLevel {
            level_index: 0,
            label: "L1".to_string(),
            small_blind: big_blind / 2,
            big_blind,
            ante: 0,
            duration_seconds: 600,
        }];
        TournamentState {
            table_id: "table".to_string(),
            session_epoch: 1,
            phase: TournamentPhase::Running,
            config: TournamentConfig {
                tournament_name: "T".to_string(),
                table_name: None,
                max_players: 6,
                starting_stack: 1500,
                turn_timer_seconds: 20,
                blind_schedule: BlindSchedule {
                    levels: levels.clone(),
                },
            },
            blind_schedule: BlindSchedule { levels },
            blind_level_index: 0,
            participants: BTreeMap::new(),
            seats: Vec::new(),
            current_hand: None,
            hand_results: Vec::new(),
            placements: Vec::new(),
        }
    }

    /// A decision spot with sensible pre-flop-open defaults; tests override only
    /// the inputs they care about.
    struct Spot {
        style: NpcStyle,
        hole: Vec<Card>,
        board: Vec<Card>,
        street: StreetPhase,
        pot_total: u32,
        call_amount: u32,
        min_raise_to: Option<u32>,
        max_raise_to: Option<u32>,
        facing_bet: bool,
        stack: u32,
        active_count: u8,
        dealer_seat: u8,
        npc_seat: u8,
        legal: Vec<ActionType>,
        seed: u64,
    }

    impl Spot {
        fn preflop_open() -> Self {
            Spot {
                style: NpcStyle::Conservative,
                hole: aces(),
                board: Vec::new(),
                street: StreetPhase::Preflop,
                pot_total: 30,
                call_amount: 20,
                min_raise_to: Some(60),
                max_raise_to: Some(1000),
                facing_bet: false,
                stack: 1000,
                active_count: 6,
                dealer_seat: 0,
                npc_seat: 3,
                legal: vec![
                    ActionType::Fold,
                    ActionType::Call,
                    ActionType::Raise,
                    ActionType::AllIn,
                ],
                seed: 42,
            }
        }

        fn run(&self) -> (ActionType, Option<u32>) {
            let state = fixture_state(20);
            rule_based_decision(
                &self.style,
                &self.hole,
                &self.board,
                self.street,
                self.pot_total,
                self.call_amount,
                self.min_raise_to,
                self.max_raise_to,
                self.facing_bet,
                self.stack,
                self.active_count,
                self.dealer_seat,
                self.npc_seat,
                0,
                &state,
                &self.legal,
                self.seed,
            )
        }
    }

    // ----- 4.2 Legality invariants -----

    #[test]
    fn never_returns_an_action_outside_legal_actions() {
        let legal_sets = [
            vec![
                ActionType::Fold,
                ActionType::Call,
                ActionType::Raise,
                ActionType::AllIn,
            ],
            vec![ActionType::Check, ActionType::Bet, ActionType::AllIn],
            vec![ActionType::Fold, ActionType::Call],
            vec![ActionType::Check],
        ];
        let hands = [aces(), pair_of_eights(), seventy_two_offsuit()];
        let board = vec![card(King, Clubs), card(Seven, Hearts), card(Two, Spades)];

        for style in [NpcStyle::Conservative, NpcStyle::Aggressive] {
            for street in [StreetPhase::Preflop, StreetPhase::Flop] {
                for facing_bet in [false, true] {
                    for hole in &hands {
                        for legal in &legal_sets {
                            for seed in [1_u64, 7, 99, 1000] {
                                let mut spot = Spot::preflop_open();
                                spot.style = style.clone();
                                spot.hole = hole.clone();
                                spot.board = if street == StreetPhase::Preflop {
                                    Vec::new()
                                } else {
                                    board.clone()
                                };
                                spot.street = street;
                                spot.facing_bet = facing_bet;
                                spot.call_amount = if facing_bet { 60 } else { 0 };
                                spot.legal = legal.clone();
                                spot.seed = seed;

                                let (action, _) = spot.run();
                                assert!(
                                    legal.contains(&action),
                                    "{action:?} not in legal set {legal:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn raise_amount_within_min_and_max_when_raising() {
        for (min, max) in [(60_u32, 300_u32), (100, 500), (200, 1500)] {
            let mut spot = Spot::preflop_open();
            spot.style = NpcStyle::Aggressive;
            spot.min_raise_to = Some(min);
            spot.max_raise_to = Some(max);
            spot.stack = max;

            if let (ActionType::Raise | ActionType::Bet, Some(amount)) = spot.run() {
                assert!(
                    amount >= min && amount <= max,
                    "raise {amount} out of bounds [{min}, {max}]"
                );
            }
        }
    }

    #[test]
    fn no_raise_amount_when_not_raising() {
        // A fold-tier hand facing a bet folds with no amount.
        let mut spot = Spot::preflop_open();
        spot.hole = seventy_two_offsuit();
        spot.facing_bet = true;
        spot.call_amount = 60;
        let (action, amount) = spot.run();
        assert!(matches!(
            action,
            ActionType::Fold | ActionType::Check | ActionType::Call | ActionType::AllIn
        ));
        assert_eq!(amount, None);
    }

    #[test]
    fn does_not_raise_when_raise_is_not_legal() {
        // Premium hand, but only fold/call are offered: it cannot raise.
        let mut spot = Spot::preflop_open();
        spot.legal = vec![ActionType::Fold, ActionType::Call];
        let (action, _) = spot.run();
        assert!(!matches!(action, ActionType::Raise | ActionType::Bet));
    }

    // ----- 4.3 Hand-strength behavior -----

    #[test]
    fn folds_trash_facing_a_bet() {
        let mut spot = Spot::preflop_open();
        spot.hole = seventy_two_offsuit();
        spot.facing_bet = true;
        spot.call_amount = 60;
        assert_eq!(spot.run().0, ActionType::Fold);
    }

    #[test]
    fn does_not_fold_when_checking_is_free() {
        // When checking is free the engine does not offer Fold, so even trash
        // checks rather than folding.
        let mut spot = Spot::preflop_open();
        spot.hole = seventy_two_offsuit();
        spot.call_amount = 0;
        spot.facing_bet = false;
        spot.legal = vec![ActionType::Check, ActionType::Bet, ActionType::Raise];
        assert_ne!(spot.run().0, ActionType::Fold);
    }

    #[test]
    fn raises_or_bets_premium_hand_when_allowed() {
        let spot = Spot::preflop_open(); // pocket aces, raise legal
        assert!(matches!(spot.run().0, ActionType::Raise | ActionType::Bet));
    }

    #[test]
    fn calls_a_medium_strong_hand_from_early_position() {
        // Conservative pocket eights (Strong tier) in early position with no
        // raise calls the big blind rather than raising or folding.
        let mut spot = Spot::preflop_open();
        spot.hole = pair_of_eights();
        spot.npc_seat = 1; // small blind => early position
        spot.call_amount = 20;
        assert_eq!(spot.run().0, ActionType::Call);
    }

    // ----- 4.4 Stack / position behavior -----

    #[test]
    fn short_stack_shoves_when_only_all_in_is_offered() {
        // Premium hand, but the stack is too short for a full raise so the engine
        // only offers all-in; the desired raise becomes an all-in shove.
        let mut spot = Spot::preflop_open();
        spot.legal = vec![ActionType::Fold, ActionType::Call, ActionType::AllIn];
        assert_eq!(spot.run().0, ActionType::AllIn);
    }

    #[test]
    fn position_influences_a_strong_hand() {
        // Conservative pocket eights, no raise: it opens from the button (late)
        // but only calls from the small blind (early).
        let mut late = Spot::preflop_open();
        late.hole = pair_of_eights();
        late.npc_seat = 0; // button => late
        late.call_amount = 20;

        let mut early = Spot::preflop_open();
        early.hole = pair_of_eights();
        early.npc_seat = 1; // small blind => early
        early.call_amount = 20;

        assert!(matches!(late.run().0, ActionType::Raise | ActionType::Bet));
        assert_eq!(early.run().0, ActionType::Call);
    }

    // ----- 4.5 Style differences -----

    #[test]
    fn aggressive_raises_a_strong_hand_a_conservative_only_calls() {
        // Pocket eights in middle position, no raise: aggressive opens, while
        // conservative merely limps/calls.
        let mut conservative = Spot::preflop_open();
        conservative.style = NpcStyle::Conservative;
        conservative.hole = pair_of_eights();
        conservative.npc_seat = 1; // early position: conservative calls
        conservative.call_amount = 20;

        let mut aggressive = Spot::preflop_open();
        aggressive.style = NpcStyle::Aggressive;
        aggressive.hole = pair_of_eights();
        aggressive.npc_seat = 1;
        aggressive.call_amount = 20;

        assert_eq!(conservative.run().0, ActionType::Call);
        assert!(matches!(
            aggressive.run().0,
            ActionType::Raise | ActionType::Bet
        ));
    }

    // ----- 4.6 Determinism -----

    #[test]
    fn same_inputs_and_seed_produce_same_decision() {
        let spot = Spot::preflop_open();
        assert_eq!(spot.run(), spot.run());
    }

    #[test]
    fn different_seeds_can_diverge_on_a_borderline_hand() {
        // Aggressive pocket deuces (Playable) in middle position is a seeded
        // mixed-strategy spot; some seeds open and some fold via the deviation.
        let mut spot = Spot::preflop_open();
        spot.style = NpcStyle::Aggressive;
        spot.hole = vec![card(Two, Spades), card(Two, Hearts)];
        spot.npc_seat = 3; // middle
        spot.call_amount = 20;

        let mut seen = std::collections::BTreeSet::new();
        for seed in 0_u64..300 {
            spot.seed = seed;
            seen.insert(format!("{:?}", spot.run().0));
        }
        assert!(
            seen.len() > 1,
            "a seeded mixed-strategy spot should not be constant: {seen:?}"
        );
    }
}
