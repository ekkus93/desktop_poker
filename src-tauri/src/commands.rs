use tauri::State;

use crate::app_state::{DesktopAppState, DesktopBootstrapState, ScreenDescriptor};

#[tauri::command]
pub fn get_bootstrap_state(state: State<'_, DesktopAppState>) -> DesktopBootstrapState {
    state.bootstrap()
}

#[tauri::command]
pub fn list_screen_catalog(state: State<'_, DesktopAppState>) -> Vec<ScreenDescriptor> {
    state.screen_catalog()
}
