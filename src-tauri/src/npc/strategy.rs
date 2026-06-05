use super::postflop::PostflopCategory;
use super::preflop::PreflopTier;
use super::NpcStyle;

/// Table position relative to the dealer button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Early,
    Middle,
    Late,
}

/// The action chosen by the NPC engine.
#[derive(Clone, Debug)]
pub enum NpcAction {
    Fold,
    CheckOrCall,
    /// Raise-to amount (already clamped to legal bounds).
    Raise(u32),
}

/// Derive the NPC's table position from seat indices.
pub fn derive_position(npc_seat: u8, dealer_seat: u8, total_players: u8) -> Position {
    if total_players <= 2 {
        return Position::Late;
    }
    let offset = seat_offset(npc_seat, dealer_seat, total_players);
    // offset 0 = dealer/button (late), 1 = small blind (early), 2 = big blind (early)
    match (total_players, offset) {
        (_, 0) => Position::Late,
        (_, 1) | (_, 2) => Position::Early,
        (n, o) if o >= n.saturating_sub(2) => Position::Late,
        _ => Position::Middle,
    }
}

fn seat_offset(npc_seat: u8, dealer_seat: u8, total_players: u8) -> u8 {
    if total_players == 0 {
        return 0;
    }
    (npc_seat + total_players - dealer_seat) % total_players
}

/// Simple seeded hash for deterministic deviation rolls.
/// Returns true with probability `threshold / 100`.
fn seeded_bool(seed: u64, salt: u64, threshold: u64) -> bool {
    let h = seed
        .wrapping_add(salt)
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (h % 100) < threshold
}

// ── Pre-flop ─────────────────────────────────────────────────────────────────

/// Choose a pre-flop action.
///
/// # Arguments
/// * `facing_raise` – true if there is a bet/raise to call beyond the big blind
/// * `raise_count`  – how many raises have already occurred this street
/// * `seed`         – deterministic seed (from the action window ID hash)
#[allow(clippy::too_many_arguments)]
pub fn choose_preflop_action(
    style: &NpcStyle,
    tier: PreflopTier,
    position: Position,
    facing_raise: bool,
    raise_count: u8,
    min_raise_to: Option<u32>,
    max_raise_to: Option<u32>,
    call_amount: u32,
    pot_total: u32,
    stack: u32,
    seed: u64,
) -> NpcAction {
    let _ = pot_total; // may be used for sizing in future

    let effective_tier = apply_preflop_deviation(style, tier, seed);

    match style {
        NpcStyle::Conservative => conservative_preflop(
            effective_tier,
            position,
            facing_raise,
            raise_count,
            min_raise_to,
            max_raise_to,
            call_amount,
            stack,
            seed,
        ),
        NpcStyle::Aggressive => aggressive_preflop(
            effective_tier,
            position,
            facing_raise,
            raise_count,
            min_raise_to,
            max_raise_to,
            call_amount,
            stack,
            seed,
        ),
    }
}

fn apply_preflop_deviation(style: &NpcStyle, tier: PreflopTier, seed: u64) -> PreflopTier {
    match style {
        // Conservative: occasionally plays a Strong hand as Premium
        NpcStyle::Conservative => {
            if tier == PreflopTier::Strong && seeded_bool(seed, 1, 8) {
                PreflopTier::Premium
            } else {
                tier
            }
        }
        // Aggressive: occasionally folds one tier higher than normal
        NpcStyle::Aggressive => {
            if seeded_bool(seed, 2, 15) {
                // Fold one tier: Playable → fold it, but Premium/Strong keep going
                match tier {
                    PreflopTier::Playable => PreflopTier::Marginal,
                    PreflopTier::Marginal => PreflopTier::Fold,
                    other => other,
                }
            } else {
                tier
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn conservative_preflop(
    tier: PreflopTier,
    position: Position,
    facing_raise: bool,
    raise_count: u8,
    min_raise_to: Option<u32>,
    max_raise_to: Option<u32>,
    call_amount: u32,
    stack: u32,
    seed: u64,
) -> NpcAction {
    let _ = seed;
    match tier {
        PreflopTier::Premium => {
            if facing_raise && raise_count >= 2 {
                // 4-bet or all-in with AA/KK-equivalent
                try_raise(max_raise_to, max_raise_to, call_amount, stack)
            } else {
                // Open-raise or 3-bet
                let target = scale_raise(call_amount, 3, min_raise_to, max_raise_to, stack);
                try_raise(Some(target), max_raise_to, call_amount, stack)
            }
        }
        PreflopTier::Strong => {
            if position == Position::Late && !facing_raise {
                let target = scale_raise(call_amount, 3, min_raise_to, max_raise_to, stack);
                try_raise(Some(target), max_raise_to, call_amount, stack)
            } else if !facing_raise {
                // Call from early/middle (limp or call BB)
                NpcAction::CheckOrCall
            } else {
                // Fold to a raise
                NpcAction::Fold
            }
        }
        _ => NpcAction::Fold,
    }
}

#[allow(clippy::too_many_arguments)]
fn aggressive_preflop(
    tier: PreflopTier,
    position: Position,
    facing_raise: bool,
    raise_count: u8,
    min_raise_to: Option<u32>,
    max_raise_to: Option<u32>,
    call_amount: u32,
    stack: u32,
    seed: u64,
) -> NpcAction {
    match tier {
        PreflopTier::Premium => {
            if facing_raise {
                // 3-bet or 4-bet
                let target = scale_raise(call_amount, 3, min_raise_to, max_raise_to, stack);
                try_raise(Some(target), max_raise_to, call_amount, stack)
            } else {
                let target = scale_raise(call_amount, 3, min_raise_to, max_raise_to, stack);
                try_raise(Some(target), max_raise_to, call_amount, stack)
            }
        }
        PreflopTier::Strong => {
            if facing_raise && raise_count >= 2 {
                // Fold to heavy 4-bet pressure unless we got lucky with deviation
                NpcAction::Fold
            } else {
                let target = scale_raise(call_amount, 3, min_raise_to, max_raise_to, stack);
                try_raise(Some(target), max_raise_to, call_amount, stack)
            }
        }
        PreflopTier::Playable => {
            if facing_raise {
                if raise_count >= 1 {
                    NpcAction::Fold
                } else {
                    NpcAction::CheckOrCall
                }
            } else {
                // Open-raise or limp
                if position == Position::Late || position == Position::Middle {
                    let target = scale_raise(call_amount, 3, min_raise_to, max_raise_to, stack);
                    try_raise(Some(target), max_raise_to, call_amount, stack)
                } else {
                    NpcAction::CheckOrCall
                }
            }
        }
        PreflopTier::Marginal => {
            if facing_raise {
                NpcAction::Fold
            } else if position == Position::Late && seeded_bool(seed, 3, 30) {
                // Occasional steal from the button
                let target = scale_raise(call_amount, 3, min_raise_to, max_raise_to, stack);
                try_raise(Some(target), max_raise_to, call_amount, stack)
            } else {
                NpcAction::Fold
            }
        }
        PreflopTier::Fold => NpcAction::Fold,
    }
}

// ── Post-flop ─────────────────────────────────────────────────────────────────

/// Choose a post-flop action.
///
/// # Arguments
/// * `facing_bet`          – true if there is a bet to call
/// * `facing_bet_fraction` – the facing bet as a fraction of the pot (0.0–∞)
#[allow(clippy::too_many_arguments)]
pub fn choose_postflop_action(
    style: &NpcStyle,
    category: PostflopCategory,
    facing_bet: bool,
    facing_bet_fraction: f32,
    min_raise_to: Option<u32>,
    max_raise_to: Option<u32>,
    call_amount: u32,
    pot_total: u32,
    stack: u32,
    seed: u64,
) -> NpcAction {
    match style {
        NpcStyle::Conservative => conservative_postflop(
            category,
            facing_bet,
            facing_bet_fraction,
            min_raise_to,
            max_raise_to,
            call_amount,
            pot_total,
            stack,
            seed,
        ),
        NpcStyle::Aggressive => aggressive_postflop(
            category,
            facing_bet,
            facing_bet_fraction,
            min_raise_to,
            max_raise_to,
            call_amount,
            pot_total,
            stack,
            seed,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn conservative_postflop(
    category: PostflopCategory,
    facing_bet: bool,
    facing_bet_fraction: f32,
    min_raise_to: Option<u32>,
    max_raise_to: Option<u32>,
    call_amount: u32,
    pot_total: u32,
    stack: u32,
    _seed: u64,
) -> NpcAction {
    if facing_bet {
        match category {
            PostflopCategory::Monster => {
                // Raise for value
                let target = scale_raise(call_amount, 3, min_raise_to, max_raise_to, stack);
                try_raise(Some(target), max_raise_to, call_amount, stack)
            }
            PostflopCategory::Strong => NpcAction::CheckOrCall,
            PostflopCategory::Medium => {
                if facing_bet_fraction <= 0.25 {
                    NpcAction::CheckOrCall
                } else {
                    NpcAction::Fold
                }
            }
            PostflopCategory::Draw => {
                if facing_bet_fraction <= 0.30 {
                    NpcAction::CheckOrCall
                } else {
                    NpcAction::Fold
                }
            }
            PostflopCategory::Weak => NpcAction::Fold,
        }
    } else {
        match category {
            PostflopCategory::Monster | PostflopCategory::Strong => {
                let amount = pot_fraction(pot_total, 0.60, min_raise_to, max_raise_to, stack);
                try_raise(amount, max_raise_to, call_amount, stack)
            }
            PostflopCategory::Medium => NpcAction::CheckOrCall, // check
            PostflopCategory::Draw | PostflopCategory::Weak => NpcAction::CheckOrCall, // check
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn aggressive_postflop(
    category: PostflopCategory,
    facing_bet: bool,
    facing_bet_fraction: f32,
    min_raise_to: Option<u32>,
    max_raise_to: Option<u32>,
    call_amount: u32,
    pot_total: u32,
    stack: u32,
    seed: u64,
) -> NpcAction {
    if facing_bet {
        match category {
            PostflopCategory::Monster => {
                let target = scale_raise(call_amount, 3, min_raise_to, max_raise_to, stack);
                try_raise(Some(target), max_raise_to, call_amount, stack)
            }
            PostflopCategory::Strong => {
                if seeded_bool(seed, 10, 40) {
                    let target = scale_raise(call_amount, 3, min_raise_to, max_raise_to, stack);
                    try_raise(Some(target), max_raise_to, call_amount, stack)
                } else {
                    NpcAction::CheckOrCall
                }
            }
            PostflopCategory::Medium => {
                if facing_bet_fraction <= 0.50 {
                    NpcAction::CheckOrCall
                } else {
                    NpcAction::Fold
                }
            }
            PostflopCategory::Draw => {
                if facing_bet_fraction <= 0.35 {
                    NpcAction::CheckOrCall
                } else {
                    NpcAction::Fold
                }
            }
            PostflopCategory::Weak => {
                if facing_bet_fraction > 0.10 {
                    NpcAction::Fold
                } else {
                    NpcAction::CheckOrCall
                }
            }
        }
    } else {
        match category {
            PostflopCategory::Monster => {
                let amount = pot_fraction(pot_total, 0.80, min_raise_to, max_raise_to, stack);
                try_raise(amount, max_raise_to, call_amount, stack)
            }
            PostflopCategory::Strong => {
                let amount = pot_fraction(pot_total, 0.65, min_raise_to, max_raise_to, stack);
                try_raise(amount, max_raise_to, call_amount, stack)
            }
            PostflopCategory::Medium => {
                // Bet 40% of pot as a probe
                let amount = pot_fraction(pot_total, 0.40, min_raise_to, max_raise_to, stack);
                try_raise(amount, max_raise_to, call_amount, stack)
            }
            PostflopCategory::Draw => {
                // Semi-bluff 50% of pot
                let amount = pot_fraction(pot_total, 0.50, min_raise_to, max_raise_to, stack);
                try_raise(amount, max_raise_to, call_amount, stack)
            }
            PostflopCategory::Weak => NpcAction::CheckOrCall, // check behind
        }
    }
}

// ── Sizing helpers ────────────────────────────────────────────────────────────

/// Compute a pot-fraction bet target clamped to legal raise bounds.
fn pot_fraction(
    pot: u32,
    fraction: f32,
    min_raise_to: Option<u32>,
    max_raise_to: Option<u32>,
    stack: u32,
) -> Option<u32> {
    max_raise_to?;
    let target = (pot as f32 * fraction).round() as u32;
    let clamped = clamp_raise(target, min_raise_to, max_raise_to, stack);
    Some(clamped)
}

/// Compute a raise-to amount as `multiplier × call_amount` clamped to bounds.
fn scale_raise(
    call_amount: u32,
    multiplier: u32,
    min_raise_to: Option<u32>,
    max_raise_to: Option<u32>,
    stack: u32,
) -> u32 {
    let target = call_amount
        .saturating_mul(multiplier)
        .max(call_amount.saturating_add(1));
    clamp_raise(target, min_raise_to, max_raise_to, stack)
}

fn clamp_raise(
    target: u32,
    min_raise_to: Option<u32>,
    max_raise_to: Option<u32>,
    stack: u32,
) -> u32 {
    let lo = min_raise_to.unwrap_or(target);
    let hi = max_raise_to.unwrap_or(stack).min(stack);
    target.max(lo).min(hi)
}

/// Try to return a `Raise` action; fall back to `CheckOrCall` if raising is not possible.
fn try_raise(
    target: Option<u32>,
    max_raise_to: Option<u32>,
    call_amount: u32,
    stack: u32,
) -> NpcAction {
    match (target, max_raise_to) {
        (Some(t), Some(max)) => {
            let amount = t.min(max).min(stack);
            if amount > call_amount {
                NpcAction::Raise(amount)
            } else {
                NpcAction::CheckOrCall
            }
        }
        _ => NpcAction::CheckOrCall,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> u64 {
        12345
    }

    #[test]
    fn conservative_premium_opens() {
        let action = choose_preflop_action(
            &NpcStyle::Conservative,
            PreflopTier::Premium,
            Position::Early,
            false,
            0,
            Some(60),
            Some(1500),
            20,
            100,
            1500,
            seed(),
        );
        assert!(matches!(action, NpcAction::Raise(_)));
    }

    #[test]
    fn conservative_folds_playable() {
        let action = choose_preflop_action(
            &NpcStyle::Conservative,
            PreflopTier::Playable,
            Position::Middle,
            false,
            0,
            Some(60),
            Some(1500),
            20,
            100,
            1500,
            seed(),
        );
        assert!(matches!(action, NpcAction::Fold));
    }

    #[test]
    fn aggressive_playable_opens_late() {
        let action = choose_preflop_action(
            &NpcStyle::Aggressive,
            PreflopTier::Playable,
            Position::Late,
            false,
            0,
            Some(60),
            Some(1500),
            20,
            100,
            1500,
            seed(),
        );
        assert!(matches!(action, NpcAction::Raise(_)));
    }

    #[test]
    fn aggressive_marginal_folds_to_raise() {
        let action = choose_preflop_action(
            &NpcStyle::Aggressive,
            PreflopTier::Marginal,
            Position::Middle,
            true,
            1,
            Some(120),
            Some(1500),
            80,
            200,
            1500,
            seed(),
        );
        assert!(matches!(action, NpcAction::Fold));
    }

    #[test]
    fn raise_amounts_respect_bounds() {
        for &min in &[60u32, 100, 200] {
            for &max in &[300u32, 500, 1500] {
                let action = choose_preflop_action(
                    &NpcStyle::Aggressive,
                    PreflopTier::Premium,
                    Position::Late,
                    false,
                    0,
                    Some(min),
                    Some(max),
                    20,
                    100,
                    max,
                    seed(),
                );
                if let NpcAction::Raise(amount) = action {
                    assert!(
                        amount >= min && amount <= max,
                        "raise {amount} out of bounds [{min}, {max}]"
                    );
                }
            }
        }
    }

    #[test]
    fn no_raise_available_falls_back_to_check_or_call() {
        let action = choose_preflop_action(
            &NpcStyle::Aggressive,
            PreflopTier::Premium,
            Position::Late,
            false,
            0,
            None,
            None,
            0,
            100,
            1500,
            seed(),
        );
        assert!(matches!(action, NpcAction::CheckOrCall));
    }

    #[test]
    fn postflop_monster_bets_facing_bet_conservative() {
        let action = choose_postflop_action(
            &NpcStyle::Conservative,
            PostflopCategory::Monster,
            true,
            0.5,
            Some(200),
            Some(1500),
            150,
            300,
            1500,
            seed(),
        );
        assert!(matches!(action, NpcAction::Raise(_)));
    }

    #[test]
    fn postflop_weak_folds_to_bet() {
        let action = choose_postflop_action(
            &NpcStyle::Conservative,
            PostflopCategory::Weak,
            true,
            0.5,
            Some(200),
            Some(1500),
            150,
            300,
            1500,
            seed(),
        );
        assert!(matches!(action, NpcAction::Fold));
    }

    #[test]
    fn postflop_weak_checks_behind() {
        let action = choose_postflop_action(
            &NpcStyle::Aggressive,
            PostflopCategory::Weak,
            false,
            0.0,
            Some(200),
            Some(1500),
            0,
            300,
            1500,
            seed(),
        );
        assert!(matches!(action, NpcAction::CheckOrCall));
    }

    #[test]
    fn derive_position_heads_up_is_always_late() {
        assert_eq!(derive_position(0, 0, 2), Position::Late);
        assert_eq!(derive_position(1, 0, 2), Position::Late);
    }

    #[test]
    fn derive_position_button_is_late() {
        assert_eq!(derive_position(0, 0, 6), Position::Late);
    }

    #[test]
    fn derive_position_blinds_are_early() {
        // seat 1 is small blind (offset 1 from dealer 0)
        assert_eq!(derive_position(1, 0, 6), Position::Early);
        // seat 2 is big blind (offset 2 from dealer 0)
        assert_eq!(derive_position(2, 0, 6), Position::Early);
    }
}
