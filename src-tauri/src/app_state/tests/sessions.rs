use super::super::{
    build_debug_child_instance_id, build_debug_child_launch_args, derive_instance_profile,
    detect_profile_directory, screen_catalog, DesktopAppState, DesktopTableActionErrorCode,
    DesktopTableActionKind, TableViewerMode, INSTANCE_ID_ENV_VAR, JOIN_PAYLOAD_ENV_VAR,
};

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
    let error = DesktopAppState::detect()
        .submit_table_action(TableViewerMode::Local, DesktopTableActionKind::Fold, None)
        .expect_err("table actions should require an active live session");

    assert_eq!(error.code(), DesktopTableActionErrorCode::NoActiveSession);
    assert_eq!(
        error.message(),
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
