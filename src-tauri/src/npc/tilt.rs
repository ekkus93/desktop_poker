use super::session_history::NpcSessionHistory;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TiltLevel {
    None,
    Mild,
    Full,
}

pub struct TiltState {
    pub level: TiltLevel,
    pub consecutive_losses: u32,
    pub consecutive_wins: u32,
}

impl TiltState {
    pub fn from_history(history: &NpcSessionHistory) -> Self {
        let consecutive_losses = history.consecutive_losses();
        let consecutive_wins = history.consecutive_wins();
        let level = if consecutive_losses >= 3 {
            TiltLevel::Full
        } else if consecutive_losses >= 2 {
            TiltLevel::Mild
        } else {
            TiltLevel::None
        };
        Self {
            level,
            consecutive_losses,
            consecutive_wins,
        }
    }

    pub fn is_tilted(&self) -> bool {
        self.level != TiltLevel::None
    }

    /// Short phrase for prompt injection, e.g. "on a 3-hand losing streak".
    pub fn description(&self) -> Option<String> {
        match self.level {
            TiltLevel::None => None,
            TiltLevel::Mild => Some(format!(
                "on a {}-hand losing streak (mild tilt)",
                self.consecutive_losses
            )),
            TiltLevel::Full => Some(format!(
                "on a {}-hand losing streak (full tilt)",
                self.consecutive_losses
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npc::session_history::{HandSummary, NpcSessionHistory};

    fn loss(n: u32) -> HandSummary {
        HandSummary {
            hand_number: n,
            npc_won: false,
            pot_size: 200,
            net_chips: -100,
            npc_went_to_showdown: false,
            npc_bluffed: false,
            npc_bluff_caught: false,
            opponent_ids_in_hand: vec![],
        }
    }

    fn win(n: u32) -> HandSummary {
        HandSummary {
            hand_number: n,
            npc_won: true,
            pot_size: 200,
            net_chips: 150,
            npc_went_to_showdown: false,
            npc_bluffed: false,
            npc_bluff_caught: false,
            opponent_ids_in_hand: vec![],
        }
    }

    fn history_with(hands: Vec<HandSummary>) -> NpcSessionHistory {
        let mut h = NpcSessionHistory::new("npc-seat-0".into());
        for hand in hands {
            h.record_hand(hand);
        }
        h
    }

    #[test]
    fn tilt_none_when_no_losses() {
        let h = history_with(vec![win(1), win(2)]);
        let t = TiltState::from_history(&h);
        assert_eq!(t.level, TiltLevel::None);
    }

    #[test]
    fn tilt_mild_at_exactly_two_consecutive_losses() {
        let h = history_with(vec![win(1), loss(2), loss(3)]);
        let t = TiltState::from_history(&h);
        assert_eq!(t.level, TiltLevel::Mild);
    }

    #[test]
    fn tilt_full_at_three_or_more_consecutive_losses() {
        let h = history_with(vec![win(1), loss(2), loss(3), loss(4)]);
        let t = TiltState::from_history(&h);
        assert_eq!(t.level, TiltLevel::Full);

        let h2 = history_with(vec![loss(1), loss(2), loss(3), loss(4), loss(5)]);
        assert_eq!(TiltState::from_history(&h2).level, TiltLevel::Full);
    }

    #[test]
    fn is_tilted_false_for_none_true_otherwise() {
        let none_h = history_with(vec![win(1)]);
        assert!(!TiltState::from_history(&none_h).is_tilted());

        let mild_h = history_with(vec![loss(1), loss(2)]);
        assert!(TiltState::from_history(&mild_h).is_tilted());
    }

    #[test]
    fn description_returns_none_when_not_tilted() {
        let h = history_with(vec![win(1), win(2)]);
        assert!(TiltState::from_history(&h).description().is_none());
    }

    #[test]
    fn description_returns_phrase_when_tilted() {
        let h = history_with(vec![loss(1), loss(2), loss(3)]);
        let desc = TiltState::from_history(&h).description();
        assert!(desc.is_some());
        let s = desc.unwrap();
        assert!(s.contains("3"), "got: {s}");
        assert!(s.contains("losing streak"), "got: {s}");
    }

    #[test]
    fn win_after_four_losses_resets_to_none() {
        let h = history_with(vec![loss(1), loss(2), loss(3), loss(4), win(5)]);
        let t = TiltState::from_history(&h);
        assert_eq!(t.level, TiltLevel::None);
    }
}
