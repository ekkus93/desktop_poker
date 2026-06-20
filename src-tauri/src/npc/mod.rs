pub mod api_key;
pub mod hand_log;
pub mod llm_action;
pub mod llm_client;
pub mod llm_strategy;
pub mod opponent_stats;
pub mod postflop;
pub mod preflop;
pub mod profile;
pub mod profile_store;
pub mod prompt;
pub mod provider;
pub mod provider_storage;
pub mod runner;
pub mod session_history;
pub mod strategy;
pub mod tilt;

pub use llm_strategy::LlmFallbackReason;
pub use provider::{LlmProviderConfig, LlmProviderType};

use serde::{Deserialize, Serialize};

pub use profile::NpcProfile;

/// Playing style for a rule-based NPC player.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NpcStyle {
    Aggressive,
    Conservative,
}

/// Internal configuration for a single NPC seat during a host session.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcConfig {
    /// Stable player ID assigned to this NPC (matches `NpcConfig::player_id(seat_index)`).
    pub player_id: String,
    pub display_name: String,
    pub style: NpcStyle,
    /// Optional LLM profile; when present the NPC uses LLM-based decisions.
    pub profile: Option<NpcProfile>,
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

/// Per-NPC entry in the `add_npc_players` request payload.
///
/// This is the frontend-facing type.  The backend resolves `profile_id` to a
/// loaded `NpcProfile` before creating the internal `NpcConfig`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcConfigRequest {
    pub display_name: String,
    pub style: NpcStyle,
    /// Optional profile ID; when set the backend loads the profile by this ID.
    pub profile_id: Option<String>,
}

/// Request payload for the `add_npc_players` Tauri command.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddNpcPlayersRequest {
    pub npcs: Vec<NpcConfigRequest>,
}
