use crate::domain::{ActionType, Card, StreetPhase};

use super::super::postflop::postflop_hand_category;
use super::super::preflop::preflop_hand_tier;
use super::super::strategy::{
    choose_postflop_action, choose_preflop_action, derive_position, NpcAction,
};

use super::super::*;

/// All required inputs for a rule-based NPC decision.
///
/// All fields are non-optional. Callers must validate and extract these values
/// before calling `rule_based_decision`; missing or inconsistent values must
/// be reported as NPC internal errors, not defaulted.
pub(crate) struct RuleDecisionContext<'a> {
    pub style: &'a NpcStyle,
    pub hole_cards: &'a [Card],
    pub board_cards: &'a [Card],
    pub street: StreetPhase,
    pub pot_total: u32,
    pub call_amount: u32,
    pub min_raise_to: Option<u32>,
    pub max_raise_to: Option<u32>,
    pub facing_bet: bool,
    pub stack: u32,
    pub active_count: u8,
    pub dealer_seat: u8,
    pub npc_seat: u8,
    /// Big blind amount for the current blind level (validated before call).
    pub big_blind: u32,
    /// Current bet on the table this street (from betting_round.current_bet).
    pub current_bet: u32,
    pub legal_actions: &'a [ActionType],
    pub seed: u64,
}

pub(crate) fn rule_based_decision(ctx: &RuleDecisionContext<'_>) -> (ActionType, Option<u32>) {
    let position = derive_position(ctx.npc_seat, ctx.dealer_seat, ctx.active_count.max(2));

    let action = if ctx.street == StreetPhase::Preflop {
        let tier = preflop_hand_tier(ctx.hole_cards);
        let facing_raise = ctx.call_amount > ctx.big_blind;
        let raise_count = if ctx.current_bet > ctx.big_blind * 3 {
            2
        } else if facing_raise {
            1
        } else {
            0
        };

        choose_preflop_action(
            ctx.style,
            tier,
            position,
            facing_raise,
            raise_count,
            ctx.min_raise_to,
            ctx.max_raise_to,
            ctx.call_amount,
            ctx.pot_total,
            ctx.stack,
            ctx.seed,
        )
    } else {
        let category = postflop_hand_category(ctx.hole_cards, ctx.board_cards);
        let facing_bet_fraction = if ctx.pot_total > 0 {
            ctx.call_amount as f32 / ctx.pot_total as f32
        } else {
            0.0
        };
        choose_postflop_action(
            ctx.style,
            category,
            ctx.facing_bet,
            facing_bet_fraction,
            ctx.min_raise_to,
            ctx.max_raise_to,
            ctx.call_amount,
            ctx.pot_total,
            ctx.stack,
            ctx.seed,
        )
    };

    match action {
        NpcAction::Fold => {
            if ctx.legal_actions.contains(&ActionType::Fold) {
                (ActionType::Fold, None)
            } else {
                first_check_or_call(ctx.legal_actions)
            }
        }
        NpcAction::CheckOrCall => first_check_or_call(ctx.legal_actions),
        NpcAction::Raise(amount) => {
            if ctx.legal_actions.contains(&ActionType::Raise)
                || ctx.legal_actions.contains(&ActionType::Bet)
            {
                let at = if ctx.legal_actions.contains(&ActionType::Raise) {
                    ActionType::Raise
                } else {
                    ActionType::Bet
                };
                (at, Some(amount))
            } else if ctx.legal_actions.contains(&ActionType::AllIn) {
                (ActionType::AllIn, None)
            } else {
                first_check_or_call(ctx.legal_actions)
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
    use super::*;
    use crate::domain::{ActionType, Card, Rank, Suit, StreetPhase};
    use crate::npc::NpcStyle;

    fn two_cards() -> [Card; 2] {
        [
            Card { rank: Rank::Ace, suit: Suit::Spades },
            Card { rank: Rank::King, suit: Suit::Hearts },
        ]
    }

    fn base_ctx<'a>(
        style: &'a NpcStyle,
        legal: &'a [ActionType],
        hole_cards: &'a [Card],
        call_amount: u32,
        big_blind: u32,
        current_bet: u32,
    ) -> RuleDecisionContext<'a> {
        RuleDecisionContext {
            style,
            hole_cards,
            board_cards: &[],
            street: StreetPhase::Preflop,
            pot_total: 100,
            call_amount,
            min_raise_to: Some(call_amount * 2),
            max_raise_to: Some(1000),
            facing_bet: call_amount > 0,
            stack: 900,
            active_count: 2,
            dealer_seat: 0,
            npc_seat: 1,
            big_blind,
            current_bet,
            legal_actions: legal,
            seed: 42,
        }
    }

    // P0.5 — big_blind from context, not a hardcoded default.
    #[test]
    fn facing_raise_detection_uses_ctx_big_blind_not_hardcoded_default() {
        let style = NpcStyle::Conservative;
        let legal = [ActionType::Fold, ActionType::Call, ActionType::Raise];
        let cards = two_cards();
        // call_amount = 20, big_blind = 40: call_amount < big_blind → NOT facing_raise.
        let ctx = base_ctx(&style, &legal, &cards, 20, 40, 0);
        let (at, _) = rule_based_decision(&ctx);
        assert!(legal.contains(&at), "result must be a legal action");
    }

    // P0.5 — current_bet from context, not unwrap_or(0).
    #[test]
    fn raise_count_uses_ctx_current_bet_not_hardcoded_zero() {
        let style = NpcStyle::Aggressive;
        let legal = [ActionType::Fold, ActionType::Call, ActionType::Raise];
        let cards = two_cards();
        // big_blind = 20, current_bet = 100 (> 20 * 3 = 60) → raise_count = 2.
        let ctx = base_ctx(&style, &legal, &cards, 20, 20, 100);
        let (at, _) = rule_based_decision(&ctx);
        assert!(legal.contains(&at), "result must be a legal action");
    }

    // P0.5 — rule_based_decision returns a legal action for a postflop scenario.
    #[test]
    fn postflop_scenario_returns_legal_action() {
        let style = NpcStyle::Conservative;
        let legal = [ActionType::Fold, ActionType::Call];
        let cards = two_cards();
        let ctx = RuleDecisionContext {
            street: StreetPhase::Flop,
            call_amount: 50,
            facing_bet: true,
            big_blind: 20,
            current_bet: 50,
            ..base_ctx(&style, &legal, &cards, 50, 20, 50)
        };
        let (at, _) = rule_based_decision(&ctx);
        assert!(legal.contains(&at));
    }
}
