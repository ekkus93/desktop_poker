pub mod app_state;
pub mod commands;
pub mod crypto;
pub mod domain;
pub mod engine;
pub mod interop;
pub mod networking;
pub mod protocol;
pub mod storage;
pub mod tournament;

use app_state::DesktopAppState;
use commands::{
    get_bootstrap_state, list_screen_catalog, resolve_host_lan_address, validate_join_payload_input,
};
use tauri::Emitter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = DesktopAppState::detect();
    let bootstrap_event = app_state.bootstrap();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .setup(move |app| {
            app.emit("desktop://bootstrap", bootstrap_event.clone())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_bootstrap_state,
            list_screen_catalog,
            validate_join_payload_input,
            resolve_host_lan_address
        ])
        .run(tauri::generate_context!())
        .expect("error while running desktop poker application");
}
