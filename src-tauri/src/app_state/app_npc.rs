use std::sync::{Arc, Mutex};

use crate::domain;

use super::*;

impl DesktopAppState {
    pub fn add_npc_players(
        &self,
        request: crate::npc::AddNpcPlayersRequest,
    ) -> Result<HostSessionStatus, String> {
        use std::sync::atomic::AtomicBool;

        if request.npcs.is_empty() {
            return Err("npcs list must not be empty".to_string());
        }

        // Resolve profiles before locking the session.
        // If an explicit profile_id is specified, fail loudly if it cannot be loaded.
        let profiles_dir = crate::npc::profile_store::profiles_dir(&self.app_data_dir);

        struct NpcDraft {
            display_name: String,
            style: crate::npc::NpcStyle,
            profile: Option<crate::npc::NpcProfile>,
        }

        let drafts: Vec<NpcDraft> = request
            .npcs
            .iter()
            .map(|req| -> Result<NpcDraft, String> {
                let profile = if let Some(id) = req.profile_id.as_deref() {
                    let loaded = crate::npc::profile_store::load_profile(&profiles_dir, id)
                        .map_err(|e| format!("could not load NPC profile '{id}': {e}"))?;
                    Some(loaded)
                } else {
                    None
                };
                Ok(NpcDraft {
                    display_name: req.display_name.clone(),
                    style: req.style.clone(),
                    profile,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        let mut host_session = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?;
        let session = host_session
            .as_mut()
            .ok_or_else(|| "no active host session".to_string())?;

        let authoritative = session
            .host_server
            .authoritative_state()
            .map_err(|e| e.to_string())?;

        if !matches!(
            authoritative.phase,
            domain::TournamentPhase::WaitingForPlayers | domain::TournamentPhase::ReadyCheck
        ) {
            return Err("NPCs can only be added before the tournament starts".to_string());
        }

        let occupied = authoritative
            .seats
            .iter()
            .filter(|s| s.occupancy == domain::SeatOccupancyState::Occupied)
            .count() as u8;
        let capacity = session.config.max_players;
        let total_after = occupied.saturating_add(request.npcs.len() as u8);
        if total_after > capacity {
            return Err(format!(
                "not enough open seats: {occupied} occupied, {capacity} max, {} NPCs requested",
                request.npcs.len()
            ));
        }

        let open_seats: Vec<u8> = authoritative
            .seats
            .iter()
            .filter(|s| s.occupancy == domain::SeatOccupancyState::Empty)
            .map(|s| s.seat_index)
            .take(request.npcs.len())
            .collect();

        if open_seats.len() < request.npcs.len() {
            return Err("not enough open seats for the requested NPCs".to_string());
        }

        // Build final NpcConfigs with stable player_ids tied to the assigned seat indices.
        let npc_configs: Vec<crate::npc::NpcConfig> = drafts
            .into_iter()
            .zip(open_seats.iter())
            .map(|(draft, &seat_index)| crate::npc::NpcConfig {
                player_id: crate::npc::NpcConfig::player_id(seat_index),
                display_name: draft.display_name,
                style: draft.style,
                profile: draft.profile,
            })
            .collect();

        for (npc_config, &seat_index) in npc_configs.iter().zip(open_seats.iter()) {
            session
                .host_server
                .register_npc_participant(&npc_config.player_id, &npc_config.display_name)
                .map_err(|e| e.to_string())?;
            session
                .host_server
                .claim_seat(&npc_config.player_id, seat_index)
                .map_err(|e| e.to_string())?;
            session
                .host_server
                .set_ready_state(&npc_config.player_id, true)
                .map_err(|e| e.to_string())?;
        }

        let stop = Arc::new(AtomicBool::new(false));
        let tilt_levels = Arc::new(Mutex::new(std::collections::BTreeMap::new()));
        let runner_handle = crate::npc::runner::start_npc_runner(
            Arc::clone(&session.host_server),
            npc_configs,
            Arc::clone(&stop),
            Arc::clone(&self.llm_provider),
            Arc::clone(&tilt_levels),
        );
        session.npc_runner = Some(crate::npc::runner::NpcRunnerGuard::new(
            stop,
            runner_handle,
            tilt_levels,
        ));

        session.status()
    }

    /// Save a provider config to disk and update the in-memory holder.
    pub fn set_llm_provider_config(
        &self,
        config: crate::npc::LlmProviderConfig,
    ) -> Result<(), String> {
        crate::npc::provider_storage::save_provider_config(&self.app_data_dir, Some(&config))
            .map_err(|e| e.to_string())?;
        *self
            .llm_provider
            .lock()
            .map_err(|_| "llm provider lock poisoned".to_string())? = Some(config);
        Ok(())
    }

    /// Clear the provider config from disk and memory.
    pub fn clear_llm_provider_config(&self) -> Result<(), String> {
        crate::npc::provider_storage::save_provider_config(&self.app_data_dir, None)
            .map_err(|e| e.to_string())?;
        *self
            .llm_provider
            .lock()
            .map_err(|_| "llm provider lock poisoned".to_string())? = None;
        Ok(())
    }

    /// Return the current provider config (None if not set).
    pub fn get_llm_provider_config(&self) -> Result<Option<crate::npc::LlmProviderConfig>, String> {
        Ok(self
            .llm_provider
            .lock()
            .map_err(|_| "llm provider lock poisoned".to_string())?
            .clone())
    }

    /// Backward-compat: set the Anthropic API key (creates/overwrites an Anthropic provider).
    pub fn set_llm_api_key(&self, key: String) -> Result<(), String> {
        let trimmed = key.trim().to_string();
        if trimmed.is_empty() {
            return Err("API key must not be empty".to_string());
        }
        self.set_llm_provider_config(crate::npc::LlmProviderConfig::from_anthropic_key(trimmed))
    }

    /// Backward-compat: clear any configured provider.
    pub fn clear_llm_api_key(&self) -> Result<(), String> {
        self.clear_llm_provider_config()
    }

    /// Returns true when any usable LLM provider is configured.
    pub fn llm_api_key_configured(&self) -> bool {
        self.llm_provider
            .lock()
            .map(|g| g.as_ref().is_some_and(|c| c.is_usable()))
            .unwrap_or(false)
    }

    /// List all NPC profiles from the profiles directory.
    pub fn list_npc_profiles(&self) -> Result<Vec<crate::npc::NpcProfile>, String> {
        let dir = crate::npc::profile_store::profiles_dir(&self.app_data_dir);
        crate::npc::profile_store::list_profiles(&dir).map_err(|e| e.to_string())
    }

    /// Load a single NPC profile by ID.
    pub fn get_npc_profile(&self, id: &str) -> Result<crate::npc::NpcProfile, String> {
        let dir = crate::npc::profile_store::profiles_dir(&self.app_data_dir);
        crate::npc::profile_store::load_profile(&dir, id).map_err(|e| e.to_string())
    }

    /// Save an NPC profile (validates before writing).
    pub fn save_npc_profile(
        &self,
        id: &str,
        content: &str,
    ) -> Result<crate::npc::NpcProfile, String> {
        let dir = crate::npc::profile_store::profiles_dir(&self.app_data_dir);
        crate::npc::profile_store::save_profile(&dir, id, content).map_err(|e| e.to_string())
    }

    /// Delete an NPC profile by ID.  Refuses to delete built-in starter profiles.
    pub fn delete_npc_profile(&self, id: &str) -> Result<(), String> {
        if crate::npc::profile_store::BUILTIN_PROFILE_IDS.contains(&id) {
            return Err(format!("built-in profile '{id}' cannot be deleted"));
        }
        let dir = crate::npc::profile_store::profiles_dir(&self.app_data_dir);
        crate::npc::profile_store::delete_profile(&dir, id).map_err(|e| e.to_string())
    }
}
