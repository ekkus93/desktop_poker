use super::super::{DesktopAppState, JoinHostSessionRequest, StartHostSessionRequest};
use crate::domain::{ActionType, ActionWindow};

/// Serializes tests that read and write the shared provider config on disk.
/// `DesktopAppState::detect()` uses a fixed path; concurrent writers race.
pub(super) static PROVIDER_CFG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(super) fn sample_host_session_request(host_address: &str) -> StartHostSessionRequest {
    StartHostSessionRequest {
        host_address: host_address.to_string(),
        host_port: 0,
        tournament_name: "Friday Finals".to_string(),
        max_players: 6,
        starting_stack: 1_500,
        blind_preset_id: "normal".to_string(),
        turn_timer_seconds: 30,
        display_name: "Host Alpha".to_string(),
    }
}

pub(super) fn sample_join_host_session_request(join_payload: &str) -> JoinHostSessionRequest {
    JoinHostSessionRequest {
        join_payload: join_payload.to_string(),
        display_name: "Client Bravo".to_string(),
    }
}

pub(super) fn sample_action_window(legal_actions: Vec<ActionType>) -> ActionWindow {
    ActionWindow {
        action_window_id: "window-1".to_string(),
        player_id: super::super::LOCAL_PLAYER_ID.to_string(),
        seat_index: 0,
        legal_actions,
        call_amount: 40,
        min_raise_to: Some(80),
        max_raise_to: Some(200),
        deadline_epoch_ms: 123_456,
    }
}

pub(super) fn start_live_table_state() -> DesktopAppState {
    let state = DesktopAppState::detect();
    state
        .debug_table_runtime
        .lock()
        .expect("debug table runtime")
        .get_or_insert_with(|| {
            super::super::DebugTableRuntime::new().expect("debug table runtime should initialize")
        })
        .controller
        .start_tournament(1)
        .expect("start tournament");
    state
}
