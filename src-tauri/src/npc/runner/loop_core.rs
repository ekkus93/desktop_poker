use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use crate::networking::HostServer;

use crate::app_state::NpcActionErrorDebug;

use super::super::provider::LlmProviderConfig;
use super::super::NpcConfig;
use super::{process_completed_hands, try_npc_action, RunnerState, POLL_INTERVAL_MS};

pub(super) fn npc_runner_loop(
    host_server: &HostServer,
    npc_configs: &[NpcConfig],
    stop: &AtomicBool,
    api_key_holder: &Arc<Mutex<Option<LlmProviderConfig>>>,
    shared_tilt: Arc<Mutex<std::collections::BTreeMap<String, String>>>,
    shared_fallback: Arc<Mutex<Option<String>>>,
    shared_action_error: Arc<Mutex<Option<NpcActionErrorDebug>>>,
) {
    let mut consecutive_errors: u32 = 0;
    let mut runner_state = RunnerState::new(
        npc_configs,
        shared_tilt,
        shared_fallback,
        shared_action_error,
    );

    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }

        let state = match host_server.authoritative_state() {
            Ok(s) => s,
            Err(_) => {
                consecutive_errors += 1;
                if consecutive_errors > 10 {
                    break;
                }
                thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
                continue;
            }
        };

        consecutive_errors = 0;

        // Process any newly completed hands before deciding the next action.
        process_completed_hands(&state, npc_configs, &mut runner_state);

        let outcome = try_npc_action(
            host_server,
            &state,
            npc_configs,
            stop,
            api_key_holder,
            &mut runner_state,
        );

        if !outcome.acted() {
            thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
        }
    }
}
