use std::{fmt::Display, net::IpAddr};

use tauri::{AppHandle, Emitter, State};

use crate::{
    app_state::{
        ClaimLobbySeatRequest, ClientSessionStatus, DebugInspectorState, DesktopAppState,
        DesktopBootstrapState, DesktopTableActionError, DesktopTableActionErrorCode,
        DesktopTableActionKind, HostSessionStatus, JoinHostSessionRequest, ScreenDescriptor,
        SetLobbyReadyStateRequest, StartHostSessionRequest, TableViewSnapshot, TableViewerMode,
    },
    domain::JoinPayload,
    networking::resolve_connectable_host_ip,
    protocol::decode_join_payload,
};

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCommandError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl DesktopCommandError {
    fn new(code: &'static str, message: String, recoverable: bool) -> Self {
        Self {
            code: code.to_string(),
            message,
            recoverable,
        }
    }

    fn from_table_action(error: DesktopTableActionError) -> Self {
        let (code, recoverable) = match error.code() {
            DesktopTableActionErrorCode::NoActiveSession => ("NO_ACTIVE_SESSION", true),
            DesktopTableActionErrorCode::ObserverReadOnly => ("OBSERVER_READ_ONLY", true),
            DesktopTableActionErrorCode::NotActingPlayer => ("NOT_ACTING_PLAYER", true),
            DesktopTableActionErrorCode::StaleActionWindow => ("STALE_ACTION_WINDOW", true),
            DesktopTableActionErrorCode::NetworkTimeout => ("NETWORK_TIMEOUT", true),
            DesktopTableActionErrorCode::ClientRuntimeDisconnected => {
                ("CLIENT_RUNTIME_DISCONNECTED", false)
            }
            DesktopTableActionErrorCode::HostRejectedAction => ("HOST_REJECTED_ACTION", true),
            DesktopTableActionErrorCode::CommandFailed => ("COMMAND_FAILED", false),
        };
        Self::new(code, error.into_message(), recoverable)
    }
}

#[tauri::command]
pub fn get_bootstrap_state(state: State<'_, DesktopAppState>) -> DesktopBootstrapState {
    get_bootstrap_state_inner(|| state.bootstrap())
}

#[tauri::command]
pub fn list_screen_catalog(state: State<'_, DesktopAppState>) -> Vec<ScreenDescriptor> {
    list_screen_catalog_inner(|| state.screen_catalog())
}

#[tauri::command]
pub fn validate_join_payload_input(payload: String) -> Result<JoinPayload, String> {
    validate_join_payload_input_inner(&payload, |trimmed_payload| {
        decode_join_payload(trimmed_payload).map_err(|error| error.to_string())
    })
}

fn emit_session_update(app: &AppHandle) {
    if let Err(e) = app.emit("desktop://session-update", ()) {
        eprintln!("[emit] session-update failed: {e}");
    }
}

fn emit_table_update(app: &AppHandle) {
    if let Err(e) = app.emit("desktop://table-update", ()) {
        eprintln!("[emit] table-update failed: {e}");
    }
}

fn emit_bootstrap_update(app: &AppHandle, bootstrap: DesktopBootstrapState) {
    if let Err(e) = app.emit("desktop://bootstrap", bootstrap) {
        eprintln!("[emit] bootstrap update failed: {e}");
    }
}

#[tauri::command]
pub fn start_host_session(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
    request: StartHostSessionRequest,
) -> Result<HostSessionStatus, String> {
    let result = state.start_host_session(request)?;
    emit_session_update(&app);
    Ok(result)
}

#[tauri::command]
pub fn get_host_session_status(
    state: State<'_, DesktopAppState>,
) -> Result<Option<HostSessionStatus>, String> {
    state.host_session_status()
}

#[tauri::command]
pub fn stop_host_session(app: AppHandle, state: State<'_, DesktopAppState>) -> Result<(), String> {
    state.stop_host_session()?;
    emit_session_update(&app);
    Ok(())
}

#[tauri::command]
pub fn host_claim_lobby_seat(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
    request: ClaimLobbySeatRequest,
) -> Result<HostSessionStatus, String> {
    let result = state.host_claim_lobby_seat(request)?;
    emit_session_update(&app);
    Ok(result)
}

#[tauri::command]
pub fn host_set_lobby_ready_state(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
    request: SetLobbyReadyStateRequest,
) -> Result<HostSessionStatus, String> {
    let result = state.host_set_lobby_ready_state(request)?;
    emit_session_update(&app);
    Ok(result)
}

#[tauri::command]
pub fn host_start_tournament(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
) -> Result<HostSessionStatus, DesktopCommandError> {
    let result = state
        .host_start_tournament()
        .map_err(|message| DesktopCommandError::new("HOST_REJECTED_ACTION", message, true))?;
    emit_session_update(&app);
    Ok(result)
}

#[tauri::command]
pub fn join_host_session(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
    request: JoinHostSessionRequest,
) -> Result<ClientSessionStatus, DesktopCommandError> {
    let result = state
        .join_host_session(request)
        .map_err(|message| DesktopCommandError::new("INVALID_JOIN_PAYLOAD", message, true))?;
    emit_session_update(&app);
    Ok(result)
}

#[tauri::command]
pub fn get_client_session_status(
    state: State<'_, DesktopAppState>,
) -> Result<Option<ClientSessionStatus>, String> {
    state.client_session_status()
}

#[tauri::command]
pub fn leave_client_session(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
) -> Result<(), String> {
    state.leave_client_session()?;
    emit_session_update(&app);
    Ok(())
}

#[tauri::command]
pub fn client_claim_lobby_seat(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
    request: ClaimLobbySeatRequest,
) -> Result<ClientSessionStatus, DesktopCommandError> {
    let result = state
        .client_claim_lobby_seat(request)
        .map_err(|message| DesktopCommandError::new("HOST_REJECTED_ACTION", message, true))?;
    emit_session_update(&app);
    Ok(result)
}

#[tauri::command]
pub fn client_set_lobby_ready_state(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
    request: SetLobbyReadyStateRequest,
) -> Result<ClientSessionStatus, DesktopCommandError> {
    let result = state
        .client_set_lobby_ready_state(request)
        .map_err(|message| DesktopCommandError::new("HOST_REJECTED_ACTION", message, true))?;
    emit_session_update(&app);
    Ok(result)
}

#[tauri::command]
pub fn add_npc_players(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
    request: crate::npc::AddNpcPlayersRequest,
) -> Result<HostSessionStatus, String> {
    let result = state.add_npc_players(request)?;
    emit_session_update(&app);
    Ok(result)
}

#[tauri::command]
pub fn set_llm_api_key(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
    key: String,
) -> Result<(), String> {
    state.set_llm_api_key(key)?;
    emit_bootstrap_update(&app, state.bootstrap());
    Ok(())
}

#[tauri::command]
pub fn clear_llm_api_key(app: AppHandle, state: State<'_, DesktopAppState>) -> Result<(), String> {
    state.clear_llm_api_key()?;
    emit_bootstrap_update(&app, state.bootstrap());
    Ok(())
}

#[tauri::command]
pub fn get_llm_provider_config(
    state: State<'_, DesktopAppState>,
) -> Result<Option<crate::npc::LlmProviderSettings>, String> {
    state.get_llm_provider_config()
}

#[tauri::command]
pub fn set_llm_provider_config(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
    config: crate::npc::LlmProviderConfig,
) -> Result<(), String> {
    state.set_llm_provider_config(config)?;
    emit_bootstrap_update(&app, state.bootstrap());
    Ok(())
}

#[tauri::command]
pub fn save_llm_provider_settings(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
    settings: crate::npc::LlmProviderSettings,
) -> Result<(), String> {
    state.save_llm_provider_settings(settings)?;
    emit_bootstrap_update(&app, state.bootstrap());
    Ok(())
}

#[tauri::command]
pub fn clear_llm_provider_config(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
) -> Result<(), String> {
    state.clear_llm_provider_config()?;
    emit_bootstrap_update(&app, state.bootstrap());
    Ok(())
}

#[tauri::command]
pub fn list_npc_profiles(
    state: State<'_, DesktopAppState>,
) -> Result<crate::npc::profile_store::NpcProfileListResult, String> {
    state.list_npc_profiles()
}

#[tauri::command]
pub fn get_npc_profile(
    state: State<'_, DesktopAppState>,
    id: String,
) -> Result<crate::npc::NpcProfile, String> {
    state.get_npc_profile(&id)
}

#[tauri::command]
pub fn save_npc_profile(
    state: State<'_, DesktopAppState>,
    id: String,
    content: String,
) -> Result<crate::npc::NpcProfile, String> {
    state.save_npc_profile(&id, &content)
}

#[tauri::command]
pub fn delete_npc_profile(state: State<'_, DesktopAppState>, id: String) -> Result<(), String> {
    state.delete_npc_profile(&id)
}

#[tauri::command]
pub fn resolve_host_lan_address() -> Result<String, String> {
    resolve_host_lan_address_inner(resolve_connectable_host_ip)
}

#[tauri::command]
pub fn get_table_view(
    state: State<'_, DesktopAppState>,
    viewer_mode: TableViewerMode,
) -> Result<TableViewSnapshot, String> {
    get_table_view_inner(viewer_mode, |next_viewer_mode| {
        state.table_view(next_viewer_mode)
    })
}

#[tauri::command]
pub fn submit_table_action(
    app: AppHandle,
    state: State<'_, DesktopAppState>,
    viewer_mode: TableViewerMode,
    action_kind: DesktopTableActionKind,
    raise_to_amount: Option<u32>,
) -> Result<TableViewSnapshot, DesktopCommandError> {
    let result = submit_table_action_inner(
        viewer_mode,
        action_kind,
        raise_to_amount,
        |next_viewer_mode, next_action_kind, next_raise_to_amount| {
            state.submit_table_action(next_viewer_mode, next_action_kind, next_raise_to_amount)
        },
    )
    .map_err(DesktopCommandError::from_table_action)?;
    emit_table_update(&app);
    Ok(result)
}

#[tauri::command]
pub fn get_debug_state(
    state: State<'_, DesktopAppState>,
    viewer_mode: TableViewerMode,
) -> Result<DebugInspectorState, String> {
    get_debug_state_inner(viewer_mode, |next_viewer_mode| {
        state.debug_state(next_viewer_mode)
    })
}

#[tauri::command]
pub fn launch_additional_client_instance(
    state: State<'_, DesktopAppState>,
    join_payload: Option<String>,
) -> Result<String, String> {
    launch_additional_client_instance_inner(join_payload, |next_join_payload| {
        state.launch_additional_client_instance(next_join_payload)
    })
}

fn get_bootstrap_state_inner(
    bootstrap: impl FnOnce() -> DesktopBootstrapState,
) -> DesktopBootstrapState {
    bootstrap()
}

fn list_screen_catalog_inner(
    list_catalog: impl FnOnce() -> Vec<ScreenDescriptor>,
) -> Vec<ScreenDescriptor> {
    list_catalog()
}

fn validate_join_payload_input_inner(
    payload: &str,
    decode: impl FnOnce(&str) -> Result<JoinPayload, String>,
) -> Result<JoinPayload, String> {
    decode(payload.trim())
}

fn resolve_host_lan_address_inner<E: Display>(
    resolve: impl FnOnce() -> Result<IpAddr, E>,
) -> Result<String, String> {
    resolve()
        .map(|ip| ip.to_string())
        .map_err(|error| error.to_string())
}

fn get_table_view_inner(
    viewer_mode: TableViewerMode,
    get_table_view: impl FnOnce(TableViewerMode) -> Result<TableViewSnapshot, String>,
) -> Result<TableViewSnapshot, String> {
    get_table_view(viewer_mode)
}

fn submit_table_action_inner(
    viewer_mode: TableViewerMode,
    action_kind: DesktopTableActionKind,
    raise_to_amount: Option<u32>,
    submit_table_action: impl FnOnce(
        TableViewerMode,
        DesktopTableActionKind,
        Option<u32>,
    ) -> Result<TableViewSnapshot, DesktopTableActionError>,
) -> Result<TableViewSnapshot, DesktopTableActionError> {
    submit_table_action(viewer_mode, action_kind, raise_to_amount)
}

fn get_debug_state_inner(
    viewer_mode: TableViewerMode,
    get_debug_state: impl FnOnce(TableViewerMode) -> Result<DebugInspectorState, String>,
) -> Result<DebugInspectorState, String> {
    get_debug_state(viewer_mode)
}

fn launch_additional_client_instance_inner(
    join_payload: Option<String>,
    launch_additional_client_instance: impl FnOnce(Option<String>) -> Result<String, String>,
) -> Result<String, String> {
    launch_additional_client_instance(join_payload)
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;
    use crate::{
        app_state::{DesktopBootstrapState, ModuleDescriptor, ScreenDescriptor},
        protocol::{encode_join_payload, PROTOCOL_VERSION},
    };

    fn sample_bootstrap() -> DesktopBootstrapState {
        DesktopBootstrapState {
            app_name: "Desktop Poker",
            protocol_version: PROTOCOL_VERSION,
            default_host_port: 43_818,
            frontend_stack: "React + TypeScript",
            serialization_strategy: "serde",
            framing_strategy: "length-prefixed JSON",
            join_payload_encoding: "pkr1_",
            runtime_transport: "raw TCP over LAN",
            crypto_stack: vec!["ed25519", "x25519"],
            instance_id: "instance-a".to_string(),
            instance_label: "Instance A".to_string(),
            storage_namespace: "desktop-poker:instance-a".to_string(),
            session_identity: "desktop-session:instance-a".to_string(),
            reconnect_namespace: "desktop-reconnect:instance-a".to_string(),
            profile_directory: "/tmp/instance-a".to_string(),
            launch_join_payload: None,
            parsed_launch_join_payload: None,
            launch_join_payload_error: None,
            debug_tools_enabled: true,
            llm_api_key_configured: false,
            llm_provider_type: None,
            provider_config_error: None,
            backend_modules: vec![ModuleDescriptor {
                name: "protocol",
                responsibility: "Owns protocol behavior.",
            }],
            screens: vec![ScreenDescriptor {
                id: "home",
                title: "Home",
                route: "/",
                surface: "home",
            }],
        }
    }

    fn sample_table_view() -> TableViewSnapshot {
        TableViewSnapshot {
            viewer_mode: TableViewerMode::Local,
            session_connection: "normal".to_string(),
            tournament_name: "Friday Night".to_string(),
            table_name: "Table 1".to_string(),
            table_id: "table-1".to_string(),
            tournament_phase: "running".to_string(),
            phase_label: "Running".to_string(),
            street_label: "Turn".to_string(),
            blind_level_label: "10 / 20".to_string(),
            current_hand_number: Some(12),
            board_cards: vec![],
            pot_total: 120,
            action_owner_label: "Alice".to_string(),
            elimination_summary: String::new(),
            observer_banner: None,
            seats: vec![],
            standings: vec![],
            hand_history: vec![],
            event_feed: vec![],
            action_tray: None,
        }
    }

    fn sample_debug_state() -> DebugInspectorState {
        DebugInspectorState {
            protocol_log: vec![],
            snapshot_json: "{}".to_string(),
            current_sequence: 7,
            current_hand_number: Some(4),
            action_window_summary: Some("Alice to act".to_string()),
            launch_hint: "hint".to_string(),
            npc_tilt_levels: std::collections::BTreeMap::new(),
            last_llm_fallback: None,
            last_npc_action_error: None,
            host_runtime_health: None,
        }
    }

    #[test]
    fn validate_join_payload_input_trims_surrounding_whitespace() {
        let payload = crate::domain::JoinPayload {
            payload_version: PROTOCOL_VERSION,
            host_address: "127.0.0.1".to_string(),
            host_port: 43_818,
            table_id: "table-join".to_string(),
            session_epoch: 3,
            host_signing_public_key: "pubkey".to_string(),
            join_token: "token".to_string(),
            generated_at_ms: 1,
            table_name: Some("Friday Night".to_string()),
        };
        let encoded = encode_join_payload(&payload).expect("payload should encode");

        assert_eq!(
            validate_join_payload_input(format!("  {encoded}  ")).expect("payload should decode"),
            payload
        );
    }

    #[test]
    fn validate_join_payload_input_returns_string_errors() {
        let result = validate_join_payload_input_inner("bad payload", |_payload| {
            Err("decode failed".to_string())
        });

        assert_eq!(result.expect_err("payload should fail"), "decode failed");
    }

    #[test]
    fn resolve_host_lan_address_inner_stringifies_resolved_ip() {
        let result = resolve_host_lan_address_inner(|| {
            Ok::<IpAddr, String>(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)))
        });

        assert_eq!(result.expect("ip should resolve"), "192.168.1.20");
    }

    #[test]
    fn get_bootstrap_state_inner_returns_the_bootstrap_contract() {
        let bootstrap = sample_bootstrap();

        assert_eq!(get_bootstrap_state_inner(|| bootstrap.clone()), bootstrap,);
    }

    #[test]
    fn list_screen_catalog_inner_returns_the_catalog_unchanged() {
        let screens = sample_bootstrap().screens;

        assert_eq!(list_screen_catalog_inner(|| screens.clone()), screens,);
    }

    #[test]
    fn get_table_view_inner_forwards_the_requested_viewer_mode() {
        let table_view = sample_table_view();

        let result = get_table_view_inner(TableViewerMode::Observer, |viewer_mode| {
            assert_eq!(viewer_mode, TableViewerMode::Observer);
            Ok(table_view.clone())
        });

        assert_eq!(result.expect("table view should succeed"), table_view);
    }

    #[test]
    fn submit_table_action_inner_forwards_all_arguments_unchanged() {
        let table_view = sample_table_view();

        let result = submit_table_action_inner(
            TableViewerMode::Observer,
            DesktopTableActionKind::BetOrRaise,
            Some(180),
            |viewer_mode, action_kind, raise_to_amount| {
                assert_eq!(viewer_mode, TableViewerMode::Observer);
                assert_eq!(action_kind, DesktopTableActionKind::BetOrRaise);
                assert_eq!(raise_to_amount, Some(180));
                Ok(table_view.clone())
            },
        );

        assert_eq!(result.expect("action should succeed"), table_view);
    }

    #[test]
    fn get_debug_state_inner_forwards_the_requested_viewer_mode() {
        let debug_state = sample_debug_state();

        let result = get_debug_state_inner(TableViewerMode::Local, |viewer_mode| {
            assert_eq!(viewer_mode, TableViewerMode::Local);
            Ok(debug_state.clone())
        });

        assert_eq!(result.expect("debug state should succeed"), debug_state);
    }

    #[test]
    fn launch_additional_client_instance_inner_forwards_optional_join_payload() {
        let result = launch_additional_client_instance_inner(
            Some("pkr1_join".to_string()),
            |join_payload| {
                assert_eq!(join_payload.as_deref(), Some("pkr1_join"));
                Ok("child-1".to_string())
            },
        );

        assert_eq!(result.expect("launch should succeed"), "child-1");
    }

    #[test]
    fn command_helpers_surface_downstream_errors_unchanged() {
        let table_view_error = get_table_view_inner(TableViewerMode::Observer, |_viewer_mode| {
            Err("observer mode is read-only".to_string())
        });
        let submit_error = submit_table_action_inner(
            TableViewerMode::Observer,
            DesktopTableActionKind::Fold,
            None,
            |_viewer_mode, _action_kind, raise_to_amount| {
                assert_eq!(raise_to_amount, None);
                Err(DesktopTableActionError::new(
                    DesktopTableActionErrorCode::HostRejectedAction,
                    "invalid action",
                ))
            },
        );

        assert_eq!(
            table_view_error.expect_err("table view should fail"),
            "observer mode is read-only"
        );
        assert_eq!(
            submit_error.expect_err("submit should fail").message(),
            "invalid action"
        );
    }
}

#[cfg(test)]
mod command_error_tests {
    use super::{DesktopCommandError, DesktopTableActionError, DesktopTableActionErrorCode};

    #[test]
    fn typed_timeout_code_is_recoverable_independent_of_wording() {
        for message in [
            "host acknowledgement deadline expired",
            "rewritten timeout copy",
        ] {
            let error = DesktopCommandError::from_table_action(DesktopTableActionError::new(
                DesktopTableActionErrorCode::NetworkTimeout,
                message,
            ));
            assert_eq!(error.code, "NETWORK_TIMEOUT");
            assert!(error.recoverable);
        }
    }

    #[test]
    fn typed_dead_runtime_code_is_fatal_independent_of_wording() {
        for message in ["runtime unavailable", "connection ended"] {
            let error = DesktopCommandError::from_table_action(DesktopTableActionError::new(
                DesktopTableActionErrorCode::ClientRuntimeDisconnected,
                message,
            ));
            assert_eq!(error.code, "CLIENT_RUNTIME_DISCONNECTED");
            assert!(!error.recoverable);
        }
    }

    #[test]
    fn command_error_serializes_stable_fields() {
        let error = DesktopCommandError::from_table_action(DesktopTableActionError::new(
            DesktopTableActionErrorCode::ObserverReadOnly,
            "spectators cannot act",
        ));
        let value = serde_json::to_value(error).expect("command error serializes");
        assert_eq!(value["code"], "OBSERVER_READ_ONLY");
        assert_eq!(value["recoverable"], true);
        assert_eq!(value["message"], "spectators cannot act");
    }
}
