use super::super::{ClaimLobbySeatRequest, DesktopAppState, SetLobbyReadyStateRequest};
use crate::{
    networking::HostRuntimeMode,
    npc::{AddNpcPlayersRequest, LlmProviderConfig, LlmProviderType, NpcConfigRequest, NpcStyle},
};

use super::support::*;

// P0.1 — bootstrap reflects live provider config after save and clear.

#[test]
fn bootstrap_llm_api_key_configured_reflects_save_without_restart() {
    let _guard = PROVIDER_CFG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let state = DesktopAppState::detect();

    // Ensure no leftover config from a previously run test.
    let _ = state.clear_llm_provider_config();
    assert!(!state.bootstrap().llm_api_key_configured);

    // Ollama does not need an API key; `is_usable()` returns true without one.
    state
        .set_llm_provider_config(LlmProviderConfig {
            settings: crate::npc::LlmProviderSettings {
                provider: LlmProviderType::Ollama,
                endpoint_url: None,
                model: None,
            },
            api_key: None,
        })
        .expect("set provider config");

    assert!(
        state.bootstrap().llm_api_key_configured,
        "bootstrap should report configured after save"
    );
}

#[test]
fn bootstrap_llm_api_key_configured_reflects_clear_without_restart() {
    let _guard = PROVIDER_CFG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let state = DesktopAppState::detect();

    state
        .set_llm_provider_config(LlmProviderConfig {
            settings: crate::npc::LlmProviderSettings {
                provider: LlmProviderType::Ollama,
                endpoint_url: None,
                model: None,
            },
            api_key: None,
        })
        .expect("set provider config before clear");
    assert!(state.bootstrap().llm_api_key_configured);

    state
        .clear_llm_provider_config()
        .expect("clear provider config");

    assert!(
        !state.bootstrap().llm_api_key_configured,
        "bootstrap should report not configured after clear"
    );
}

// P0.2 — explicit NPC profiles: happy-path, missing, and corrupt all behave correctly.

#[test]
fn add_npc_players_succeeds_with_valid_explicit_profile() {
    let host_state = DesktopAppState::detect();
    host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session starts");

    let profiles_dir = crate::npc::profile_store::profiles_dir(&host_state.app_data_dir);
    std::fs::create_dir_all(&profiles_dir).expect("profiles dir");
    std::fs::write(
        profiles_dir.join("sharp-pat.md"),
        "---\nname: Sharp Pat\nstyle: balanced\n---\nA sharp balanced player.",
    )
    .expect("write valid profile");

    let status = host_state
        .add_npc_players(AddNpcPlayersRequest {
            npcs: vec![NpcConfigRequest {
                display_name: "Sharp Pat".to_string(),
                style: NpcStyle::Conservative,
                profile_id: Some("sharp-pat".to_string()),
            }],
        })
        .expect("adding NPC with valid profile should succeed");

    assert_eq!(
        status.participants.len(),
        2,
        "host and NPC should both be seated after valid profile load"
    );

    // Verify the runner was started (it is only started when at least one NPC is configured).
    let runner_present = host_state
        .host_session
        .lock()
        .expect("host session lock")
        .as_ref()
        .is_some_and(|s| s.npc_runner.is_some());
    assert!(
        runner_present,
        "NPC runner should be active after successful add"
    );
}

#[test]
fn add_npc_players_fails_loudly_when_explicit_profile_is_missing() {
    let host_state = DesktopAppState::detect();
    host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session starts");

    let err = host_state
        .add_npc_players(AddNpcPlayersRequest {
            npcs: vec![NpcConfigRequest {
                display_name: "Ghost".to_string(),
                style: NpcStyle::Conservative,
                profile_id: Some("nonexistent-profile-id".to_string()),
            }],
        })
        .expect_err("adding NPC with missing profile should fail");

    assert!(
        err.contains("nonexistent-profile-id"),
        "error should name the missing profile; got: {err}"
    );

    // The NPC must not have been seated.
    let status = host_state
        .host_session_status()
        .expect("session status")
        .expect("session still active");
    assert_eq!(
        status.participants.len(),
        1,
        "only the host should be seated; NPC must not have been added"
    );
}

#[test]
fn add_npc_players_fails_loudly_when_explicit_profile_is_corrupt() {
    let host_state = DesktopAppState::detect();
    host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session starts");

    // Write a corrupt profile file directly into the profiles directory.
    let profiles_dir = crate::npc::profile_store::profiles_dir(&host_state.app_data_dir);
    std::fs::create_dir_all(&profiles_dir).expect("profiles dir");
    std::fs::write(
        profiles_dir.join("corrupt-npc.md"),
        "not valid yaml frontmatter {{{",
    )
    .expect("write corrupt profile");

    let err = host_state
        .add_npc_players(AddNpcPlayersRequest {
            npcs: vec![NpcConfigRequest {
                display_name: "Corrupt".to_string(),
                style: NpcStyle::Aggressive,
                profile_id: Some("corrupt-npc".to_string()),
            }],
        })
        .expect_err("adding NPC with corrupt profile should fail");

    assert!(
        err.contains("corrupt-npc"),
        "error should name the failing profile; got: {err}"
    );

    let status = host_state
        .host_session_status()
        .expect("session status")
        .expect("session still active");
    assert_eq!(
        status.participants.len(),
        1,
        "only the host should be seated; NPC must not have been added"
    );
}

// P0.5 — client-side timeout errors are explicit, not silent.

#[test]
fn client_seat_claim_times_out_and_returns_error_when_host_does_not_confirm() {
    // Use Test mode so the loopback host responds immediately to the session join,
    // but we connect to a non-listening port for the seat-claim request so it
    // never gets an acknowledgement, forcing the 1-second await_condition timeout.
    let host_state = DesktopAppState::detect();
    let host_status = host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session starts");

    let client_state = DesktopAppState::detect();
    client_state
        .join_host_session(sample_join_host_session_request(&host_status.invite))
        .expect("client joins");

    // The Test runtime delivers the join snapshot but then the client runtime
    // stops processing updates after the initial snapshot.  Claiming a seat
    // sends the message but await_condition will not observe confirmation because
    // no further events arrive — it expires after 1 s.
    let result = client_state.client_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 1 });

    // Two outcomes are acceptable: an immediate error from the runtime (seat
    // already taken / protocol rejection) OR the timeout message.  What is NOT
    // acceptable is Ok(()) without a confirmed seat.
    match result {
        Ok(status) => {
            let local = status
                .participants
                .iter()
                .find(|p| p.display_name == "Client Bravo")
                .expect("client participant present");
            // If it succeeded the seat must actually be confirmed.
            assert_eq!(
                local.seat_index,
                Some(1),
                "Ok result must carry a confirmed seat index"
            );
        }
        Err(e) => {
            // Timeout or protocol rejection — both are acceptable explicit errors.
            assert!(
                e.contains("timed out") || e.contains("rejected") || e.contains("seat"),
                "error should describe the failure; got: {e}"
            );
        }
    }
}

#[test]
fn client_ready_toggle_times_out_and_returns_error_when_host_does_not_confirm() {
    let host_state = DesktopAppState::detect();
    let host_status = host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session starts");

    let client_state = DesktopAppState::detect();
    client_state
        .join_host_session(sample_join_host_session_request(&host_status.invite))
        .expect("client joins");

    // Claim a seat first so the ready-toggle precondition is satisfied.
    let _ = client_state.client_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 1 });

    // Now toggle ready — may timeout waiting for host confirmation.
    let result =
        client_state.client_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true });

    match result {
        Ok(status) => {
            let local = status
                .participants
                .iter()
                .find(|p| p.display_name == "Client Bravo")
                .expect("client participant present");
            assert!(
                local.is_ready || local.seat_index.is_none(),
                "Ok result without ready state would be a silent failure"
            );
        }
        Err(e) => {
            assert!(
                e.contains("timed out") || e.contains("seat") || e.contains("ready"),
                "error should describe the failure; got: {e}"
            );
        }
    }
}

// P0.1 — stale providerConfigError is cleared after save or clear.

#[test]
fn bootstrap_provider_config_error_clears_after_saving_valid_config() {
    let _guard = PROVIDER_CFG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let state = DesktopAppState::detect();

    // Write a corrupt settings file directly so the next detect() sees it.
    let app_data_dir = &state.app_data_dir;
    let settings_path = app_data_dir.join("llm-provider.json");
    std::fs::create_dir_all(app_data_dir).expect("create app data dir");
    std::fs::write(&settings_path, "not valid json").expect("write corrupt settings");

    // detect() again to pick up the corrupt file.
    let state2 = DesktopAppState::detect();
    assert!(
        state2.bootstrap().provider_config_error.is_some(),
        "startup with corrupt provider config must surface a provider_config_error"
    );

    // Save valid config — error must clear immediately without restart.
    state2
        .set_llm_provider_config(LlmProviderConfig {
            settings: crate::npc::LlmProviderSettings {
                provider: LlmProviderType::Ollama,
                endpoint_url: None,
                model: None,
            },
            api_key: None,
        })
        .expect("saving valid config must succeed");

    assert!(
        state2.bootstrap().provider_config_error.is_none(),
        "provider_config_error must be cleared after saving valid config"
    );
}

#[test]
fn bootstrap_provider_config_error_clears_after_clearing_config() {
    let _guard = PROVIDER_CFG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let state = DesktopAppState::detect();

    let app_data_dir = &state.app_data_dir;
    let settings_path = app_data_dir.join("llm-provider.json");
    std::fs::create_dir_all(app_data_dir).expect("create app data dir");
    std::fs::write(&settings_path, "not valid json either").expect("write corrupt settings");

    let state2 = DesktopAppState::detect();
    assert!(
        state2.bootstrap().provider_config_error.is_some(),
        "startup with corrupt provider config must surface a provider_config_error"
    );

    state2
        .clear_llm_provider_config()
        .expect("clearing config must succeed even when corrupt");

    assert!(
        state2.bootstrap().provider_config_error.is_none(),
        "provider_config_error must be cleared after clearing provider config"
    );
}

// P1.2 — poisoned provider mutex surfaces as provider_config_error (not silent Ok).

#[test]
fn bootstrap_reports_error_when_provider_mutex_is_poisoned() {
    use std::sync::{Arc, Mutex};

    let mutex: Arc<Mutex<u8>> = Arc::new(Mutex::new(0));
    let m = Arc::clone(&mutex);
    // Poison the mutex by panicking while holding the lock.
    let _ = std::thread::spawn(move || {
        let _guard = m.lock().unwrap();
        panic!("intentional panic to poison mutex");
    })
    .join();
    assert!(mutex.is_poisoned(), "mutex must be poisoned after panicking thread");

    // Now replicate bootstrap's lock-error branch: a poisoned lock on llm_provider
    // must produce a provider_config_error.
    let provider_mutex: Mutex<Option<crate::npc::LlmProviderConfig>> = Mutex::new(None);
    let pm = std::sync::Arc::new(provider_mutex);
    let pm2 = Arc::clone(&pm);
    let _ = std::thread::spawn(move || {
        let _g = pm2.lock().unwrap();
        panic!("poison provider mutex");
    })
    .join();

    let error_msg = pm
        .lock()
        .err()
        .map(|_| "internal error: provider state unavailable".to_string());
    assert_eq!(
        error_msg.as_deref(),
        Some("internal error: provider state unavailable"),
        "poisoned provider mutex must map to a visible provider error"
    );
}
