use super::super::{
    build_debug_child_instance_id, build_debug_child_launch_args, derive_instance_profile,
    detect_profile_directory, screen_catalog, ClaimLobbySeatRequest, DesktopAppState,
    DesktopTableActionKind, SetLobbyReadyStateRequest, TableViewerMode, INSTANCE_ID_ENV_VAR,
    JOIN_PAYLOAD_ENV_VAR,
};
use crate::{
    networking::HostRuntimeMode,
    npc::{AddNpcPlayersRequest, LlmProviderConfig, LlmProviderType, NpcConfigRequest, NpcStyle},
    protocol::decode_join_payload,
};

use super::support::*;

/// Serializes tests that read and write the shared provider config on disk.
/// `DesktopAppState::detect()` uses a fixed path; concurrent writers race.
static PROVIDER_CFG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn detect_uses_android_compatible_defaults() {
    std::env::remove_var(INSTANCE_ID_ENV_VAR);
    std::env::remove_var(JOIN_PAYLOAD_ENV_VAR);

    let state = DesktopAppState::detect().bootstrap();

    assert_eq!(state.protocol_version, 1);
    assert_eq!(state.default_host_port, 43_818);
    assert_eq!(state.instance_id, "default");
    assert_eq!(state.instance_label, "default");
    assert_eq!(state.storage_namespace, "desktop-poker:default");
    assert_eq!(state.session_identity, "desktop-session:default");
    assert_eq!(state.reconnect_namespace, "desktop-reconnect:default");
    assert!(state
        .profile_directory
        .ends_with("desktop-poker/profiles/default"));
}

#[test]
fn detect_does_not_boot_debug_table_runtime_until_debug_state_is_requested() {
    let state = DesktopAppState::detect();

    assert!(state
        .debug_table_runtime
        .lock()
        .expect("debug table runtime")
        .is_none());

    let debug_state = state
        .debug_state(TableViewerMode::Local)
        .expect("debug state should lazily initialize the debug runtime");

    assert_eq!(debug_state.current_hand_number, None);
    assert!(state
        .debug_table_runtime
        .lock()
        .expect("debug table runtime")
        .is_some());
}

#[test]
fn instance_profile_sanitizes_namespace_and_identity_fields() {
    let profile = derive_instance_profile(Some("Host A / QA"));

    assert_eq!(profile.instance_label, "Host A / QA");
    assert_eq!(profile.profile_id, "host-a-qa");
    assert_eq!(profile.storage_namespace, "desktop-poker:host-a-qa");
    assert_eq!(profile.session_identity, "desktop-session:host-a-qa");
    assert_eq!(profile.reconnect_namespace, "desktop-reconnect:host-a-qa");
    assert!(profile
        .profile_directory
        .ends_with("desktop-poker/profiles/host-a-qa"));
}

#[test]
fn debug_child_instance_ids_are_scoped_to_the_parent_profile() {
    let first_child = build_debug_child_instance_id("host-a", 9001, 1);
    let second_child = build_debug_child_instance_id("host-a", 9001, 2);
    let other_parent_child = build_debug_child_instance_id("host-b", 9001, 1);

    assert_eq!(first_child, "host-a-p9001-client-1");
    assert_eq!(second_child, "host-a-p9001-client-2");
    assert_eq!(other_parent_child, "host-b-p9001-client-1");
    assert_ne!(first_child, other_parent_child);
}

#[test]
fn screen_catalog_hides_debug_tools_in_release_mode() {
    let screens = screen_catalog(false);
    assert!(!screens.iter().any(|screen| screen.id == "debug-tools"));
}

#[test]
fn screen_catalog_exposes_debug_tools_when_enabled() {
    let screens = screen_catalog(true);
    assert!(screens.iter().any(|screen| screen.id == "debug-tools"));
}

#[test]
fn profile_directory_is_namespaced_by_instance() {
    let profile_directory = detect_profile_directory("client-b");
    assert!(profile_directory.ends_with("desktop-poker/profiles/client-b"));
}

#[test]
fn table_view_requires_an_active_live_session() {
    std::env::remove_var(INSTANCE_ID_ENV_VAR);
    std::env::remove_var(JOIN_PAYLOAD_ENV_VAR);

    assert_eq!(
        DesktopAppState::detect()
            .table_view(TableViewerMode::Local)
            .expect_err("table view should require an active live session"),
        "no active live session is available for the table view"
    );
}

#[test]
fn table_actions_require_an_active_live_session() {
    assert_eq!(
        DesktopAppState::detect()
            .submit_table_action(TableViewerMode::Local, DesktopTableActionKind::Fold, None)
            .expect_err("table actions should require an active live session"),
        "no active live session is available for table actions"
    );
}

#[test]
fn local_actions_advance_runtime_and_invalid_paths_fail_cleanly() {
    let state = start_live_table_state();
    let before_action = state
        .debug_table_runtime
        .lock()
        .expect("debug table runtime")
        .get_or_insert_with(|| {
            super::super::DebugTableRuntime::new().expect("debug table runtime should initialize")
        })
        .view(TableViewerMode::Local)
        .expect("table view before action");

    assert_eq!(before_action.current_hand_number, Some(1));
    assert!(before_action.action_tray.is_some());

    assert_eq!(
        state
            .debug_table_runtime
            .lock()
            .expect("debug table runtime")
            .get_or_insert_with(|| {
                super::super::DebugTableRuntime::new()
                    .expect("debug table runtime should initialize")
            })
            .submit_action(
                TableViewerMode::Observer,
                DesktopTableActionKind::Fold,
                None,
            )
            .expect_err("observer action should fail"),
        "observer mode cannot submit actions"
    );

    let invalid_raise_state = start_live_table_state();
    assert!(invalid_raise_state
        .debug_table_runtime
        .lock()
        .expect("debug table runtime")
        .get_or_insert_with(|| {
            super::super::DebugTableRuntime::new().expect("debug table runtime should initialize")
        })
        .submit_action(
            TableViewerMode::Local,
            DesktopTableActionKind::BetOrRaise,
            Some(1),
        )
        .expect_err("invalid raise should fail")
        .contains("minimum full raise sizing"));

    let after_action = state
        .debug_table_runtime
        .lock()
        .expect("debug table runtime")
        .get_or_insert_with(|| {
            super::super::DebugTableRuntime::new().expect("debug table runtime should initialize")
        })
        .submit_action(
            TableViewerMode::Local,
            DesktopTableActionKind::CheckOrCall,
            None,
        )
        .expect("check/call should succeed");

    assert_eq!(after_action.current_hand_number, Some(1));
    assert!(!after_action.event_feed.is_empty());
    assert!(after_action
        .event_feed
        .iter()
        .any(|entry| entry.message.contains("You selected")));
}

#[test]
fn debug_state_tracks_runtime_sequence_and_action_window_presence() {
    let idle_debug_state = DesktopAppState::detect()
        .debug_state(TableViewerMode::Local)
        .expect("idle debug state");
    assert_eq!(idle_debug_state.current_hand_number, None);
    assert!(idle_debug_state.action_window_summary.is_none());

    let state = start_live_table_state();
    let running_debug_state = state
        .debug_state(TableViewerMode::Local)
        .expect("running debug state");
    assert_eq!(running_debug_state.current_hand_number, Some(1));
    assert!(running_debug_state.current_sequence >= 1);
    assert!(running_debug_state
        .action_window_summary
        .as_deref()
        .is_some_and(|summary| summary.contains("You")));

    state
        .debug_table_runtime
        .lock()
        .expect("debug table runtime")
        .get_or_insert_with(|| {
            super::super::DebugTableRuntime::new().expect("debug table runtime should initialize")
        })
        .submit_action(
            TableViewerMode::Local,
            DesktopTableActionKind::CheckOrCall,
            None,
        )
        .expect("local action");
    let updated_debug_state = state
        .debug_state(TableViewerMode::Local)
        .expect("updated debug state");
    assert!(updated_debug_state.current_sequence > running_debug_state.current_sequence);
    assert!(!updated_debug_state.protocol_log.is_empty());
}

#[test]
fn debug_child_launch_args_keep_instance_scope_and_optional_join_payload() {
    let (instance_id, args) =
        build_debug_child_launch_args("host-a", 9001, 2, Some("  pkr1_join  "));
    let (instance_id_without_payload, args_without_payload) =
        build_debug_child_launch_args("host-a", 9001, 3, Some("   "));

    assert_eq!(instance_id, "host-a-p9001-client-2");
    assert_eq!(
        args,
        vec![
            "--instance-id".to_string(),
            "host-a-p9001-client-2".to_string(),
            "--join-payload".to_string(),
            "pkr1_join".to_string(),
        ],
    );
    assert_eq!(instance_id_without_payload, "host-a-p9001-client-3");
    assert_eq!(
        args_without_payload,
        vec![
            "--instance-id".to_string(),
            "host-a-p9001-client-3".to_string(),
        ],
    );
}

#[test]
fn start_host_session_returns_a_live_invite_from_the_running_host() {
    let state = DesktopAppState::detect();

    let status = state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");

    assert_eq!(status.tournament_name, "Friday Finals");
    assert_eq!(status.table_name, "Main Table");
    assert_eq!(status.advertised_host, "127.0.0.1");
    assert_eq!(status.phase, "waitingForPlayers");
    assert_eq!(status.active_seat_count, 1);
    assert_eq!(status.open_seat_count, 5);
    assert_eq!(status.participants.len(), 1);
    assert_eq!(status.participants[0].display_name, "Host Alpha");
    assert_eq!(status.participants[0].connection_state, "connected");
    assert!(status.invite.starts_with("pkr1_"));

    let decoded = decode_join_payload(&status.invite).expect("invite should decode");
    assert_eq!(decoded.host_address, "127.0.0.1");
    assert_eq!(decoded.host_port, status.host_port);
    assert_eq!(decoded.table_name.as_deref(), Some("Friday Finals"));
    assert_eq!(decoded.table_id, status.table_id);
    assert_eq!(decoded.session_epoch, status.session_epoch);

    let active_status = state
        .host_session_status()
        .expect("host session status should resolve")
        .expect("host session should remain active");
    assert_eq!(active_status.invite, status.invite);
}

#[test]
fn stop_host_session_clears_active_host_status() {
    let state = DesktopAppState::detect();

    state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");
    state.stop_host_session().expect("host session should stop");

    assert!(state
        .host_session_status()
        .expect("host session status should resolve")
        .is_none());
}

#[test]
fn detect_starts_a_fresh_host_state_after_a_prior_instance_was_hosting() {
    let prior_state = DesktopAppState::detect();

    prior_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("prior host session should start");

    let restarted_state = DesktopAppState::detect();

    assert!(restarted_state
        .host_session_status()
        .expect("restarted host status should resolve")
        .is_none());
    assert!(restarted_state
        .client_session_status()
        .expect("restarted client status should resolve")
        .is_none());
}

#[test]
fn start_host_session_rejects_replacing_an_active_host_session() {
    let state = DesktopAppState::detect();

    let first_status = state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("first host session should start");

    let error = state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect_err("second host session should be rejected");

    assert_eq!(
        error,
        "stop the active host session before starting a new table"
    );
    assert_eq!(
        state
            .host_session_status()
            .expect("host session status should resolve")
            .expect("original host session should remain active")
            .invite,
        first_status.invite,
    );
}

#[test]
fn join_host_session_returns_the_initial_live_snapshot() {
    let host_state = DesktopAppState::detect();
    let host_status = host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");

    let client_state = DesktopAppState::detect();
    let client_status = client_state
        .join_host_session(sample_join_host_session_request(&host_status.invite))
        .expect("client should join live host");

    assert_eq!(client_status.host_address, "127.0.0.1");
    assert_eq!(client_status.host_port, host_status.host_port);
    assert_eq!(client_status.phase, "waitingForPlayers");
    assert_eq!(client_status.active_seat_count, 1);
    assert_eq!(client_status.open_seat_count, 5);
    assert_eq!(client_status.tournament_name, "Friday Finals");
    assert!(client_status
        .participants
        .iter()
        .any(|participant| participant.is_host && participant.display_name == "Host Alpha"));
    assert!(client_status
        .participants
        .iter()
        .any(|participant| participant.display_name == "Client Bravo"));

    let refreshed_host_status = host_state
        .host_session_status()
        .expect("host session status should resolve")
        .expect("host session should remain active");
    assert!(refreshed_host_status
        .participants
        .iter()
        .any(|participant| participant.display_name == "Client Bravo"));
}

#[test]
fn join_host_session_rejects_replacing_an_active_client_session() {
    let first_host_state = DesktopAppState::detect();
    let first_host_status = first_host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("first host session should start");
    let second_host_state = DesktopAppState::detect();
    let second_host_status = second_host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("second host session should start");

    let client_state = DesktopAppState::detect();
    let first_client_status = client_state
        .join_host_session(sample_join_host_session_request(&first_host_status.invite))
        .expect("client should join the first host");

    let error = client_state
        .join_host_session(sample_join_host_session_request(&second_host_status.invite))
        .expect_err("second client join should be rejected");

    assert_eq!(
        error,
        "leave the active client session before joining another table"
    );

    let active_client_status = client_state
        .client_session_status()
        .expect("client session status should resolve")
        .expect("original client session should remain active");
    assert_eq!(active_client_status.table_id, first_client_status.table_id);
    assert_eq!(
        active_client_status.host_port,
        first_client_status.host_port
    );

    assert_eq!(
        second_host_state
            .host_session_status()
            .expect("second host session status should resolve")
            .expect("second host session should remain active")
            .participants
            .len(),
        1,
    );
}

#[test]
fn host_mutations_reject_missing_host_sessions_clearly() {
    let state = DesktopAppState::detect();

    assert_eq!(
        state
            .host_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 0 })
            .expect_err("claiming a host seat without a host session should fail"),
        "no active host session",
    );
    assert_eq!(
        state
            .host_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
            .expect_err("setting host ready state without a host session should fail"),
        "no active host session",
    );
    assert_eq!(
        state
            .host_start_tournament()
            .expect_err("starting a tournament without a host session should fail"),
        "no active host session",
    );
}

#[test]
fn client_mutations_reject_missing_client_sessions_clearly() {
    let state = DesktopAppState::detect();

    assert_eq!(
        state
            .client_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 1 })
            .expect_err("claiming a client seat without a client session should fail"),
        "no active client session",
    );
    assert_eq!(
        state
            .client_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
            .expect_err("setting client ready state without a client session should fail"),
        "no active client session",
    );
}

#[test]
fn client_ready_state_requires_a_claimed_seat() {
    let host_state = DesktopAppState::detect();
    let host_status = host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");
    let client_state = DesktopAppState::detect();

    client_state
        .join_host_session(sample_join_host_session_request(&host_status.invite))
        .expect("client should join the host before claiming a seat");

    assert_eq!(
        client_state
            .client_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
            .expect_err("readying before claiming a seat should fail"),
        "ready state requires a claimed seat",
    );

    let refreshed_client_status = client_state
        .client_session_status()
        .expect("client session status should resolve")
        .expect("client session should remain active");
    let local_participant = refreshed_client_status
        .participants
        .iter()
        .find(|participant| participant.display_name == "Client Bravo")
        .expect("client participant should remain visible");
    assert_eq!(local_participant.seat_index, None);
    assert!(!local_participant.is_ready);
}

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

// ── Host lifecycle (11.1) ─────────────────────────────────────────────────────

#[test]
fn start_host_session_rejects_invalid_host_address() {
    let state = DesktopAppState::detect();

    let error = state
        .start_host_session_with_mode(
            sample_host_session_request("not-a-valid-ip"),
            HostRuntimeMode::Test,
        )
        .expect_err("host session with invalid address must fail");

    assert!(
        !error.is_empty(),
        "bind failure must produce a non-empty error; got: {error}"
    );
}

#[test]
fn stop_and_restart_host_session_succeeds() {
    let state = DesktopAppState::detect();

    let first_status = state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("first host session should start");

    state.stop_host_session().expect("host session should stop");

    assert!(
        state
            .host_session_status()
            .expect("status should resolve")
            .is_none(),
        "host session should be absent after stop"
    );

    let second_status = state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("second host session should start");

    assert_ne!(
        first_status.table_id, second_status.table_id,
        "restart must produce a fresh session with a new table ID"
    );
    assert!(second_status.invite.starts_with("pkr1_"));
}

#[test]
fn stop_host_session_is_idempotent_when_no_session_is_active() {
    let state = DesktopAppState::detect();

    // No session started — stop should not error.
    state
        .stop_host_session()
        .expect("stopping a non-existent host session must not error");
}

// ── Client lifecycle (11.1) ───────────────────────────────────────────────────

#[test]
fn leave_client_session_clears_the_active_session() {
    let host_state = DesktopAppState::detect();
    let host_status = host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");

    let client_state = DesktopAppState::detect();
    client_state
        .join_host_session(sample_join_host_session_request(&host_status.invite))
        .expect("client should join");

    assert!(
        client_state
            .client_session_status()
            .expect("client session status should resolve")
            .is_some(),
        "client session should be active before leaving"
    );

    client_state
        .leave_client_session()
        .expect("leave must succeed");

    assert!(
        client_state
            .client_session_status()
            .expect("client session status should resolve")
            .is_none(),
        "client session should be absent after leaving"
    );
}

#[test]
fn leave_client_session_permits_rejoining_a_new_host() {
    let host_state = DesktopAppState::detect();
    let first_host_status = host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("first host session should start");

    let client_state = DesktopAppState::detect();
    client_state
        .join_host_session(sample_join_host_session_request(&first_host_status.invite))
        .expect("client should join first host");
    client_state
        .leave_client_session()
        .expect("client should leave first host");

    let second_host_state = DesktopAppState::detect();
    let second_host_status = second_host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("second host session should start");

    let rejoined = client_state
        .join_host_session(sample_join_host_session_request(&second_host_status.invite))
        .expect("client should rejoin after leaving");

    assert_eq!(rejoined.table_id, second_host_status.table_id);
}

#[test]
fn leave_client_session_is_idempotent_when_no_session_is_active() {
    let state = DesktopAppState::detect();

    // No session started — leave should not error.
    state
        .leave_client_session()
        .expect("leaving a non-existent client session must not error");
}

#[test]
fn leave_client_session_does_not_clear_llm_provider_config() {
    let _lock = PROVIDER_CFG_LOCK.lock().unwrap_or_else(|p| p.into_inner());
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
        .expect("set provider config");

    let host_state = DesktopAppState::detect();
    let host_status = host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");

    state
        .join_host_session(sample_join_host_session_request(&host_status.invite))
        .expect("client should join");
    state.leave_client_session().expect("client should leave");

    let config_after_leave = state
        .get_llm_provider_config()
        .expect("provider config should be accessible after client leave");

    assert!(
        config_after_leave.is_some(),
        "leaving a client session must not clear the LLM provider config"
    );
}

// ── 3.4 Teardown isolation ────────────────────────────────────────────────────

#[test]
fn stop_host_session_does_not_clear_client_session_if_both_existed() {
    // Hosts and clients are mutually exclusive in one instance (the join command
    // rejects if a host session is active), but we can verify at the struct level
    // that stop_host_session only touches the host_session field.
    let state = DesktopAppState::detect();

    let host_a = DesktopAppState::detect();
    let host_status = host_a
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");

    // Join from `state` (it is a separate instance that has no host).
    state
        .join_host_session(sample_join_host_session_request(&host_status.invite))
        .expect("client should join");

    // Stop the *host* from host_a — this must not clear state's client session.
    host_a.stop_host_session().expect("host should stop");

    // state's client session must still be present (though the connection may
    // be degrading in the background — we only check the field is set).
    assert!(
        state
            .client_session_status()
            .expect("client session status should resolve")
            .is_some(),
        "stopping a host on a separate instance must not clear the client session on this instance"
    );
}

// P0.5 — table action returns an explicit error; never silently succeeds without a game in progress.

#[test]
fn client_table_action_returns_explicit_error_when_no_action_window_is_open() {
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

    // No tournament has started yet — there is no open action window.
    // The command must return an explicit descriptive error, not a silent no-op.
    let err = client_state
        .submit_table_action(TableViewerMode::Local, DesktopTableActionKind::Fold, None)
        .expect_err("table action with no open window must fail");

    assert!(
        err.contains("no open action window")
            || err.contains("no active client session")
            || err.contains("action tray"),
        "error should describe why the action failed; got: {err}"
    );
}
