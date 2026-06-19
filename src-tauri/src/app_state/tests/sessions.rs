use super::super::{
    build_debug_child_instance_id, build_debug_child_launch_args, derive_instance_profile,
    detect_profile_directory, screen_catalog, ClaimLobbySeatRequest, DesktopAppState,
    DesktopTableActionKind, SetLobbyReadyStateRequest, TableViewerMode, INSTANCE_ID_ENV_VAR,
    JOIN_PAYLOAD_ENV_VAR,
};
use crate::{networking::HostRuntimeMode, protocol::decode_join_payload};

use super::support::*;

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
