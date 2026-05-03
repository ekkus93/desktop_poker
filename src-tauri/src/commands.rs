use tauri::State;

use crate::{
    app_state::{DesktopAppState, DesktopBootstrapState, ScreenDescriptor},
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
