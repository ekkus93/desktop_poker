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
