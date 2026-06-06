use crate::domain::{ActionType, StreetPhase};

/// A single action taken during a hand (across any street).
#[derive(Clone, Debug)]
pub struct HandActionRecord {
    pub hand_number: u32,
    pub street: StreetPhase,
    pub player_id: String,
    pub action_type: ActionType,
    /// Chips committed by this action (None for Fold and Check).
    pub amount: Option<u32>,
    /// False only for forced blind/ante postings.
    pub is_voluntary: bool,
}

/// Running log of all actions in the hand currently in progress.
pub struct HandLog {
    pub actions: Vec<HandActionRecord>,
    pub hand_number: u32,
}

impl HandLog {
    pub fn new(hand_number: u32) -> Self {
        Self {
            actions: Vec::new(),
            hand_number,
        }
    }

    /// Append one action record.
    pub fn push(&mut self, record: HandActionRecord) {
        self.actions.push(record);
    }

    /// All actions on a given street.
    pub fn actions_on_street(&self, street: StreetPhase) -> Vec<&HandActionRecord> {
        self.actions.iter().filter(|r| r.street == street).collect()
    }

    /// All actions taken by a given player.
    pub fn actions_by(&self, player_id: &str) -> Vec<&HandActionRecord> {
        self.actions
            .iter()
            .filter(|r| r.player_id == player_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(
        player_id: &str,
        street: StreetPhase,
        action_type: ActionType,
        amount: Option<u32>,
        is_voluntary: bool,
    ) -> HandActionRecord {
        HandActionRecord {
            hand_number: 1,
            street,
            player_id: player_id.to_string(),
            action_type,
            amount,
            is_voluntary,
        }
    }

    #[test]
    fn actions_on_street_filters_correctly() {
        let mut log = HandLog::new(1);
        log.push(make_record(
            "p1",
            StreetPhase::Preflop,
            ActionType::Call,
            Some(50),
            true,
        ));
        log.push(make_record(
            "p2",
            StreetPhase::Flop,
            ActionType::Bet,
            Some(100),
            true,
        ));
        log.push(make_record(
            "p1",
            StreetPhase::Flop,
            ActionType::Call,
            Some(100),
            true,
        ));
        log.push(make_record(
            "p2",
            StreetPhase::River,
            ActionType::Check,
            None,
            true,
        ));

        let preflop = log.actions_on_street(StreetPhase::Preflop);
        assert_eq!(preflop.len(), 1);
        assert_eq!(preflop[0].player_id, "p1");

        let flop = log.actions_on_street(StreetPhase::Flop);
        assert_eq!(flop.len(), 2);

        let turn = log.actions_on_street(StreetPhase::Turn);
        assert_eq!(turn.len(), 0);
    }

    #[test]
    fn actions_by_filters_correctly() {
        let mut log = HandLog::new(1);
        log.push(make_record(
            "alice",
            StreetPhase::Preflop,
            ActionType::Raise,
            Some(150),
            true,
        ));
        log.push(make_record(
            "bob",
            StreetPhase::Preflop,
            ActionType::Call,
            Some(150),
            true,
        ));
        log.push(make_record(
            "alice",
            StreetPhase::Flop,
            ActionType::Bet,
            Some(200),
            true,
        ));

        let alice = log.actions_by("alice");
        assert_eq!(alice.len(), 2);
        assert!(alice.iter().all(|r| r.player_id == "alice"));

        let bob = log.actions_by("bob");
        assert_eq!(bob.len(), 1);
    }

    #[test]
    fn actions_appended_in_order() {
        let mut log = HandLog::new(3);
        log.push(make_record(
            "p1",
            StreetPhase::Preflop,
            ActionType::Fold,
            None,
            true,
        ));
        log.push(make_record(
            "p2",
            StreetPhase::Preflop,
            ActionType::Check,
            None,
            true,
        ));
        log.push(make_record(
            "p3",
            StreetPhase::Flop,
            ActionType::Bet,
            Some(80),
            true,
        ));

        assert_eq!(log.actions.len(), 3);
        assert_eq!(log.actions[0].player_id, "p1");
        assert_eq!(log.actions[1].player_id, "p2");
        assert_eq!(log.actions[2].player_id, "p3");
        assert_eq!(log.hand_number, 3);
    }
}
