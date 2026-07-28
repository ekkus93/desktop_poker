use tauri::State;

use crate::app_state::DesktopAppState;

#[tauri::command]
pub fn get_runtime_warnings(
    state: State<'_, DesktopAppState>,
) -> Result<Vec<String>, String> {
    state.runtime_warnings()
}
