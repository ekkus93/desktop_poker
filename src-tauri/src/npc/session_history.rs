/// A condensed record of a completed hand relevant to a single NPC.
#[derive(Clone, Debug)]
pub struct HandSummary {
    pub hand_number: u32,
    pub npc_won: bool,
    pub pot_size: u32,
    pub net_chips: i32,
    pub npc_went_to_showdown: bool,
    pub npc_bluffed: bool,
    pub npc_bluff_caught: bool,
    pub opponent_ids_in_hand: Vec<String>,
}

/// Accumulates summarised hand events for one NPC across the current session.
pub struct NpcSessionHistory {
    pub npc_player_id: String,
    summaries: Vec<HandSummary>,
}

impl NpcSessionHistory {
    pub fn new(npc_player_id: String) -> Self {
        Self {
            npc_player_id,
            summaries: Vec::new(),
        }
    }

    pub fn record_hand(&mut self, summary: HandSummary) {
        self.summaries.push(summary);
    }

    /// Number of consecutive losses ending at the most recent hand.
    pub fn consecutive_losses(&self) -> u32 {
        self.summaries
            .iter()
            .rev()
            .take_while(|s| !s.npc_won)
            .count() as u32
    }

    /// Number of consecutive wins ending at the most recent hand.
    pub fn consecutive_wins(&self) -> u32 {
        self.summaries
            .iter()
            .rev()
            .take_while(|s| s.npc_won)
            .count() as u32
    }

    pub fn hands_played(&self) -> usize {
        self.summaries.len()
    }

    pub fn total_net_chips(&self) -> i32 {
        self.summaries.iter().map(|s| s.net_chips).sum()
    }

    /// Render a concise session context block for LLM prompts (≤ ~400 tokens).
    pub fn render_context(&self) -> String {
        if self.summaries.is_empty() {
            return "No hands completed yet this session.".to_string();
        }

        let mut lines: Vec<String> = Vec::new();

        let total = self.hands_played();
        let net = self.total_net_chips();
        let direction = if net >= 0 { "up" } else { "down" };
        lines.push(format!(
            "Session: {total} hands played, {direction} {chips} chips overall.",
            chips = net.unsigned_abs()
        ));

        let losses = self.consecutive_losses();
        let wins = self.consecutive_wins();
        if losses >= 2 {
            lines.push(format!("Currently on a {losses}-hand losing streak."));
        } else if wins >= 2 {
            lines.push(format!("Currently on a {wins}-hand winning streak."));
        }

        // Most recent 5 key hands only.
        let recent: Vec<&HandSummary> = self.summaries.iter().rev().take(5).collect();
        for s in recent.into_iter().rev() {
            let outcome = if s.npc_won { "won" } else { "lost" };
            let mut detail = format!("Hand {}: {} (pot {})", s.hand_number, outcome, s.pot_size);
            if s.npc_bluff_caught {
                detail.push_str(", bluff caught");
            } else if s.npc_bluffed && s.npc_won {
                detail.push_str(", bluff succeeded");
            }
            if s.npc_went_to_showdown {
                detail.push_str(", went to showdown");
            }
            lines.push(detail);
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loss(hand_number: u32, net: i32) -> HandSummary {
        HandSummary {
            hand_number,
            npc_won: false,
            pot_size: 300,
            net_chips: net,
            npc_went_to_showdown: false,
            npc_bluffed: false,
            npc_bluff_caught: false,
            opponent_ids_in_hand: vec!["p1".to_string()],
        }
    }

    fn win(hand_number: u32, net: i32) -> HandSummary {
        HandSummary {
            hand_number,
            npc_won: true,
            pot_size: 300,
            net_chips: net,
            npc_went_to_showdown: false,
            npc_bluffed: false,
            npc_bluff_caught: false,
            opponent_ids_in_hand: vec!["p1".to_string()],
        }
    }

    #[test]
    fn consecutive_losses_after_losing_streak() {
        let mut h = NpcSessionHistory::new("npc-seat-0".into());
        h.record_hand(win(1, 200));
        h.record_hand(loss(2, -100));
        h.record_hand(loss(3, -150));
        h.record_hand(loss(4, -80));
        assert_eq!(h.consecutive_losses(), 3);
    }

    #[test]
    fn consecutive_wins_after_winning_streak() {
        let mut h = NpcSessionHistory::new("npc-seat-0".into());
        h.record_hand(loss(1, -100));
        h.record_hand(win(2, 200));
        h.record_hand(win(3, 150));
        assert_eq!(h.consecutive_wins(), 2);
    }

    #[test]
    fn consecutive_losses_resets_after_win() {
        let mut h = NpcSessionHistory::new("npc-seat-0".into());
        h.record_hand(loss(1, -100));
        h.record_hand(loss(2, -100));
        h.record_hand(loss(3, -100));
        h.record_hand(win(4, 400));
        assert_eq!(h.consecutive_losses(), 0);
    }

    #[test]
    fn render_context_contains_hand_count_and_chip_trajectory() {
        let mut h = NpcSessionHistory::new("npc-seat-0".into());
        h.record_hand(win(1, 300));
        h.record_hand(loss(2, -100));
        let ctx = h.render_context();
        assert!(ctx.contains("2 hands played"), "got: {ctx}");
        assert!(ctx.contains("up") || ctx.contains("down"), "got: {ctx}");
    }

    #[test]
    fn render_context_truncates_to_five_key_hands() {
        let mut h = NpcSessionHistory::new("npc-seat-0".into());
        for i in 1..=10 {
            h.record_hand(win(i, 50));
        }
        let ctx = h.render_context();
        // Should mention hands 6-10 (most recent 5) but not hand 1.
        assert!(ctx.contains("Hand 10"), "got: {ctx}");
        assert!(ctx.contains("Hand 6"), "got: {ctx}");
        assert!(!ctx.contains("Hand 1:"), "should not show hand 1: {ctx}");
    }

    #[test]
    fn render_context_mentions_streak_when_two_or_more_losses() {
        let mut h = NpcSessionHistory::new("npc-seat-0".into());
        h.record_hand(win(1, 200));
        h.record_hand(loss(2, -100));
        h.record_hand(loss(3, -100));
        let ctx = h.render_context();
        assert!(ctx.contains("losing streak"), "got: {ctx}");
    }
}
