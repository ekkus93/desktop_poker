use std::collections::{BTreeMap, BTreeSet};

use crate::{
    app_state::ModuleDescriptor,
    domain::{ActionType, PlayerIdentity, TournamentState},
    engine::{Deck, EngineError},
};

mod controller_core;
mod controller_hand;
mod controller_query;

pub const BETWEEN_HANDS_DELAY_MS: u64 = 1_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TournamentError {
    message: String,
}

impl TournamentError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TournamentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TournamentError {}

impl From<crate::domain::DomainError> for TournamentError {
    fn from(value: crate::domain::DomainError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<EngineError> for TournamentError {
    fn from(value: EngineError) -> Self {
        Self::new(value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredPlayer {
    pub identity: PlayerIdentity,
    pub seat_index: u8,
    pub is_host: bool,
    pub is_ready: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionRequest {
    pub player_id: String,
    pub action_window_id: String,
    pub action_type: ActionType,
    pub raise_to_amount: Option<u32>,
}

#[derive(Clone, Debug)]
struct ActiveHandRuntime {
    deck: Deck,
    street_contributions_by_player_id: BTreeMap<String, u32>,
    acted_since_last_full_raise: BTreeSet<String>,
    full_raise_increment: u32,
    last_actor_id: Option<String>,
}

pub struct TournamentController {
    state: TournamentState,
    roster_frozen: bool,
    started_at_ms: Option<u64>,
    next_blind_deadline_ms: Option<u64>,
    intermission_deadline_ms: Option<u64>,
    dealer_button_seat_index: Option<u8>,
    next_action_window_id: u64,
    pending_deck: Option<Deck>,
    active_hand: Option<ActiveHandRuntime>,
}

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "tournament",
        responsibility:
            "Coordinates roster freeze, blind scheduling, hand loops, eliminations, and tournament completion.",
    }
}

#[cfg(test)]
mod tests;
