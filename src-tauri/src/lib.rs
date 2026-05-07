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
    client_claim_lobby_seat, client_set_lobby_ready_state, create_host_invite, get_bootstrap_state,
    get_client_session_status, get_debug_state, get_host_session_status, get_table_view,
    host_claim_lobby_seat, host_set_lobby_ready_state, host_start_tournament, join_host_session,
    launch_additional_client_instance, leave_client_session, list_screen_catalog,
    resolve_host_lan_address, start_host_session, stop_host_session, submit_table_action,
    validate_join_payload_input,
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
            create_host_invite,
            start_host_session,
            get_host_session_status,
            stop_host_session,
            host_claim_lobby_seat,
            host_set_lobby_ready_state,
            host_start_tournament,
            join_host_session,
            get_client_session_status,
            leave_client_session,
            client_claim_lobby_seat,
            client_set_lobby_ready_state,
            validate_join_payload_input,
            resolve_host_lan_address,
            get_table_view,
            submit_table_action,
            get_debug_state,
            launch_additional_client_instance
        ])
        .run(tauri::generate_context!())
        .expect("error while running desktop poker application");
}
