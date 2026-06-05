pub mod postflop;
pub mod preflop;
pub mod runner;
pub mod strategy;

use serde::{Deserialize, Serialize};

/// Playing style for a rule-based NPC player.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NpcStyle {
    Aggressive,
    Conservative,
}

/// Configuration for a single NPC seat at the host's table.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcConfig {
    pub display_name: String,
    pub style: NpcStyle,
}

impl NpcConfig {
    /// Stable player ID for an NPC assigned to a given seat index.
    pub fn player_id(seat_index: u8) -> String {
        format!("npc-seat-{seat_index}")
    }

    /// Returns true if the given player ID was issued by `NpcConfig::player_id`.
    pub fn is_npc_player_id(player_id: &str) -> bool {
        player_id.starts_with("npc-seat-")
    }
}

/// Request payload for the `add_npc_players` Tauri command.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddNpcPlayersRequest {
    pub npcs: Vec<NpcConfig>,
}
