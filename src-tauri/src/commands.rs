use tauri::State;

use crate::{
    app_state::{
        DebugInspectorState, DesktopAppState, DesktopBootstrapState, DesktopTableActionKind,
        ScreenDescriptor, TableViewSnapshot, TableViewerMode,
    },
    domain::JoinPayload,
    networking::resolve_connectable_host_ip,
    protocol::decode_join_payload,
};

#[tauri::command]
pub fn get_bootstrap_state(state: State<'_, DesktopAppState>) -> DesktopBootstrapState {
    state.bootstrap()
}

#[tauri::command]
pub fn list_screen_catalog(state: State<'_, DesktopAppState>) -> Vec<ScreenDescriptor> {
    state.screen_catalog()
}

#[tauri::command]
pub fn validate_join_payload_input(payload: String) -> Result<JoinPayload, String> {
    decode_join_payload(payload.trim()).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn resolve_host_lan_address() -> Result<String, String> {
    resolve_connectable_host_ip()
        .map(|ip| ip.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_table_view(
    state: State<'_, DesktopAppState>,
    viewer_mode: TableViewerMode,
) -> Result<TableViewSnapshot, String> {
    state.table_view(viewer_mode)
}

#[tauri::command]
pub fn submit_table_action(
    state: State<'_, DesktopAppState>,
    viewer_mode: TableViewerMode,
    action_kind: DesktopTableActionKind,
    raise_to_amount: Option<u32>,
) -> Result<TableViewSnapshot, String> {
    state.submit_table_action(viewer_mode, action_kind, raise_to_amount)
}

#[tauri::command]
pub fn get_debug_state(
    state: State<'_, DesktopAppState>,
    viewer_mode: TableViewerMode,
) -> Result<DebugInspectorState, String> {
    state.debug_state(viewer_mode)
}

#[tauri::command]
pub fn launch_additional_client_instance(
    state: State<'_, DesktopAppState>,
) -> Result<String, String> {
    state.launch_additional_client_instance()
}
