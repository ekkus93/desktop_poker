use std::collections::BTreeMap;

use crate::domain::{ActionType, HandResult, StreetPhase};

use super::hand_log::HandLog;

/// Aggregate stats observed for a single opponent over the current session.
#[derive(Clone, Debug, Default)]
pub struct OpponentStats {
    pub player_id: String,
    pub display_name: String,
    pub hands_observed: u32,
    /// Hands where the player voluntarily put chips in pre-flop (call or raise, not blind).
    pub vpip_count: u32,
    /// Hands where the player raised pre-flop.
    pub preflop_raise_count: u32,
    /// Post-flop bets and raises.
    pub aggression_bets: u32,
    /// Post-flop calls.
    pub aggression_calls: u32,
    pub showdowns_seen: u32,
    pub showdowns_won: u32,
    pub times_bluffed_caught: u32,
}

impl OpponentStats {
    /// VPIP as a percentage (0–100).
    pub fn vpip_pct(&self) -> f32 {
        if self.hands_observed == 0 {
            return 0.0;
        }
        (self.vpip_count as f32 / self.hands_observed as f32) * 100.0
    }

    /// Preflop raise frequency as a percentage (0–100).
    pub fn pfr_pct(&self) -> f32 {
        if self.hands_observed == 0 {
            return 0.0;
        }
        (self.preflop_raise_count as f32 / self.hands_observed as f32) * 100.0
    }

    /// Aggression factor: (bets + raises) / calls. Returns f32::INFINITY when calls == 0.
    pub fn aggression_factor(&self) -> f32 {
        if self.aggression_calls == 0 {
            if self.aggression_bets == 0 {
                return 0.0;
            }
            return f32::INFINITY;
        }
        self.aggression_bets as f32 / self.aggression_calls as f32
    }

    /// Showdown win rate as a percentage (0–100).
    pub fn wtsd_win_pct(&self) -> f32 {
        if self.showdowns_seen == 0 {
            return 0.0;
        }
        (self.showdowns_won as f32 / self.showdowns_seen as f32) * 100.0
    }

    /// One-line summary for prompt injection.
    pub fn render_line(&self) -> String {
        let name = if self.display_name.is_empty() {
            &self.player_id
        } else {
            &self.display_name
        };
        let af = self.aggression_factor();
        let af_str = if af.is_infinite() {
            "∞".to_string()
        } else {
            format!("{af:.1}")
        };
        format!(
            "{name}: VPIP {vpip:.0}%, PFR {pfr:.0}%, AF {af}, showdown win {wtsd:.0}% ({n} hands)",
            vpip = self.vpip_pct(),
            pfr = self.pfr_pct(),
            af = af_str,
            wtsd = self.wtsd_win_pct(),
            n = self.hands_observed,
        )
    }
}

/// Tracks opponent stats for every non-NPC player seen this session.
pub struct OpponentStatsTable {
    stats: BTreeMap<String, OpponentStats>,
}

impl Default for OpponentStatsTable {
    fn default() -> Self {
        Self::new()
    }
}

impl OpponentStatsTable {
    pub fn new() -> Self {
        Self {
            stats: BTreeMap::new(),
        }
    }

    pub fn get(&self, player_id: &str) -> Option<&OpponentStats> {
        self.stats.get(player_id)
    }

    /// Update stats from a completed hand.
    ///
    /// `hand_log` contains all recorded actions for the hand.
    /// `result` is the authoritative `HandResult` from the tournament engine.
    /// `display_names` maps player_id → display name.
    pub fn update_from_hand(
        &mut self,
        hand_log: &HandLog,
        result: &HandResult,
        display_names: &BTreeMap<String, String>,
    ) {
        // Collect all player IDs that participated in this hand.
        let participants: Vec<String> = result.final_stack_by_player_id.keys().cloned().collect();

        // Track which players had at least one preflop voluntary action.
        let preflop_actions = hand_log.actions_on_street(StreetPhase::Preflop);

        let preflop_vpip: std::collections::HashSet<&str> = preflop_actions
            .iter()
            .filter(|r| {
                r.is_voluntary
                    && matches!(
                        r.action_type,
                        ActionType::Call | ActionType::Raise | ActionType::Bet
                    )
            })
            .map(|r| r.player_id.as_str())
            .collect();

        let preflop_raisers: std::collections::HashSet<&str> = preflop_actions
            .iter()
            .filter(|r| {
                r.is_voluntary && matches!(r.action_type, ActionType::Raise | ActionType::Bet)
            })
            .map(|r| r.player_id.as_str())
            .collect();

        // Post-flop actions.
        let postflop_actions: Vec<_> = hand_log
            .actions
            .iter()
            .filter(|r| {
                matches!(
                    r.street,
                    StreetPhase::Flop | StreetPhase::Turn | StreetPhase::River
                )
            })
            .collect();

        // Which players went to showdown (revealed hands).
        let showdown_players: std::collections::HashSet<&str> = result
            .revealed_hands_by_player_id
            .keys()
            .map(|s| s.as_str())
            .collect();

        for player_id in &participants {
            let entry = self
                .stats
                .entry(player_id.clone())
                .or_insert_with(|| OpponentStats {
                    player_id: player_id.clone(),
                    display_name: display_names
                        .get(player_id.as_str())
                        .cloned()
                        .unwrap_or_default(),
                    ..Default::default()
                });

            // Update display name if now available.
            if entry.display_name.is_empty() {
                if let Some(name) = display_names.get(player_id.as_str()) {
                    entry.display_name = name.clone();
                }
            }

            entry.hands_observed += 1;

            if preflop_vpip.contains(player_id.as_str()) {
                entry.vpip_count += 1;
            }
            if preflop_raisers.contains(player_id.as_str()) {
                entry.preflop_raise_count += 1;
            }

            // Post-flop aggression.
            for action in &postflop_actions {
                if action.player_id != *player_id {
                    continue;
                }
                match action.action_type {
                    ActionType::Bet | ActionType::Raise => entry.aggression_bets += 1,
                    ActionType::Call => entry.aggression_calls += 1,
                    _ => {}
                }
            }

            // Showdown tracking.
            if showdown_players.contains(player_id.as_str()) {
                entry.showdowns_seen += 1;
                if result.winning_player_ids.contains(player_id) {
                    entry.showdowns_won += 1;
                } else {
                    // Lost at showdown with revealed hand — count as bluff caught if
                    // they were betting (aggression_bets incremented this hand).
                    let had_postflop_aggression = postflop_actions.iter().any(|r| {
                        r.player_id == *player_id
                            && matches!(r.action_type, ActionType::Bet | ActionType::Raise)
                    });
                    if had_postflop_aggression {
                        entry.times_bluffed_caught += 1;
                    }
                }
            }
        }
    }

    /// Render opponent context for LLM prompt (≤ ~300 tokens).
    ///
    /// Only includes opponents with ≥ 3 hands observed.
    pub fn render_context(&self) -> String {
        let eligible: Vec<&OpponentStats> = self
            .stats
            .values()
            .filter(|s| s.hands_observed >= 3)
            .collect();

        if eligible.is_empty() {
            return String::new();
        }

        // Cap at 5 opponents to stay within token budget.
        let lines: Vec<String> = eligible.iter().take(5).map(|s| s.render_line()).collect();
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::domain::{ActionType, HandResult, StreetPhase};

    use super::super::hand_log::{HandActionRecord, HandLog};
    use super::*;

    fn make_result(winners: Vec<&str>, participants: Vec<&str>) -> HandResult {
        let mut final_stacks = BTreeMap::new();
        for p in &participants {
            final_stacks.insert(p.to_string(), 1000u32);
        }
        HandResult {
            hand_number: 1,
            winning_player_ids: winners.iter().map(|s| s.to_string()).collect(),
            pot_summaries: vec![],
            board_cards: vec![],
            revealed_hands_by_player_id: BTreeMap::new(),
            eliminated_player_ids: vec![],
            final_stack_by_player_id: final_stacks,
        }
    }

    fn action(
        player: &str,
        street: StreetPhase,
        at: ActionType,
        voluntary: bool,
    ) -> HandActionRecord {
        HandActionRecord {
            hand_number: 1,
            street,
            player_id: player.to_string(),
            action_type: at,
            amount: Some(50),
            is_voluntary: voluntary,
        }
    }

    #[test]
    fn vpip_pct_rounds_correctly() {
        let s = OpponentStats {
            hands_observed: 10,
            vpip_count: 4,
            ..Default::default()
        };
        assert!((s.vpip_pct() - 40.0).abs() < 0.01);
    }

    #[test]
    fn aggression_factor_handles_divide_by_zero() {
        let infinite = OpponentStats {
            aggression_bets: 5,
            aggression_calls: 0,
            ..Default::default()
        };
        assert!(infinite.aggression_factor().is_infinite());

        let zero = OpponentStats::default();
        assert_eq!(zero.aggression_factor(), 0.0);
    }

    #[test]
    fn update_from_hand_increments_counters_correctly() {
        let mut table = OpponentStatsTable::new();
        let mut log = HandLog::new(1);
        log.push(action(
            "alice",
            StreetPhase::Preflop,
            ActionType::Raise,
            true,
        ));
        log.push(action("bob", StreetPhase::Preflop, ActionType::Call, true));
        log.push(action("alice", StreetPhase::Flop, ActionType::Bet, true));
        log.push(action("bob", StreetPhase::Flop, ActionType::Call, true));

        let result = make_result(vec!["alice"], vec!["alice", "bob"]);
        let names: BTreeMap<String, String> = [
            ("alice".to_string(), "Alice".to_string()),
            ("bob".to_string(), "Bob".to_string()),
        ]
        .into();

        table.update_from_hand(&log, &result, &names);

        let alice = table.get("alice").unwrap();
        assert_eq!(alice.hands_observed, 1);
        assert_eq!(alice.vpip_count, 1);
        assert_eq!(alice.preflop_raise_count, 1);
        assert_eq!(alice.aggression_bets, 1);
        assert_eq!(alice.display_name, "Alice");

        let bob = table.get("bob").unwrap();
        assert_eq!(bob.vpip_count, 1);
        assert_eq!(bob.preflop_raise_count, 0);
        assert_eq!(bob.aggression_calls, 1);
    }

    #[test]
    fn render_context_omits_opponents_with_fewer_than_three_hands() {
        let mut table = OpponentStatsTable::new();
        let mut log = HandLog::new(1);
        log.push(action(
            "alice",
            StreetPhase::Preflop,
            ActionType::Call,
            true,
        ));
        let result = make_result(vec!["alice"], vec!["alice"]);
        let names: BTreeMap<String, String> = [("alice".to_string(), "Alice".to_string())].into();

        // Only 2 hands — below threshold.
        table.update_from_hand(&log, &result, &names);
        table.update_from_hand(&log, &result, &names);
        assert!(table.render_context().is_empty());
    }

    #[test]
    fn render_context_includes_stats_for_qualified_opponent() {
        let mut table = OpponentStatsTable::new();
        let mut log = HandLog::new(1);
        log.push(action(
            "alice",
            StreetPhase::Preflop,
            ActionType::Call,
            true,
        ));
        let result = make_result(vec!["alice"], vec!["alice"]);
        let names: BTreeMap<String, String> = [("alice".to_string(), "Alice".to_string())].into();

        for _ in 0..3 {
            table.update_from_hand(&log, &result, &names);
        }
        let ctx = table.render_context();
        assert!(ctx.contains("Alice"), "got: {ctx}");
        assert!(ctx.contains("VPIP"), "got: {ctx}");
    }
}
