use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

use crate::app_state::NpcActionErrorDebug;
use crate::domain::TournamentState;
use crate::networking::HostServer;

use super::hand_log::HandLog;
use super::opponent_stats::OpponentStatsTable;
use super::provider::LlmProviderConfig;
use super::session_history::NpcSessionHistory;
use super::NpcConfig;

mod action;
mod decision;
mod hand_tracker;
mod loop_core;

pub(crate) use action::try_npc_action;
pub(crate) use decision::*;
pub(crate) use hand_tracker::process_completed_hands;

/// Outcome of a single `try_npc_action` call.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NpcActionOutcome {
    /// No hand or no action window exists right now.
    NoOpportunity,
    /// The window was for a human player; the NPC runner has nothing to do.
    NotNpcTurn,
    /// No NPC config was found for the player whose window is open.
    NoConfig,
    /// The stop signal was set before a decision could be taken.
    Stopped,
    /// The runtime could not be reached to read authoritative state.
    RuntimeUnavailable,
    /// The action window expired between the initial snapshot and submission.
    StaleWindow,
    /// The action was submitted and the host accepted it.
    Success,
    /// The host rejected the submitted action.
    Rejected(String),
}

impl NpcActionOutcome {
    /// Returns `true` only when an action was successfully submitted.
    pub(crate) fn acted(&self) -> bool {
        matches!(self, NpcActionOutcome::Success)
    }
}

/// Interval between polls when no NPC action is pending.
pub(super) const POLL_INTERVAL_MS: u64 = 80;

/// Range for the simulated thinking delay: [MIN_DELAY_MS, MAX_DELAY_MS].
pub(super) const MIN_DELAY_MS: u64 = 300;
pub(super) const MAX_DELAY_MS: u64 = 1200;

/// Per-loop mutable state tracking hand history across the runner lifecycle.
pub(crate) struct RunnerState {
    /// Log of actions for the hand currently in progress.
    pub(super) hand_log: Option<HandLog>,
    /// Per-NPC session history; indexed parallel to `npc_configs`.
    pub(super) session_histories: Vec<NpcSessionHistory>,
    /// Accumulated opponent stats for every human player seen this session.
    pub(super) opponent_stats: OpponentStatsTable,
    /// Number of HandResults seen so far (used to detect hand completion).
    pub(super) last_hand_result_count: usize,
    /// Stack size of each NPC at the start of the most recent hand.
    pub(super) pre_hand_stacks: BTreeMap<String, u32>,
    /// Shared tilt level snapshot for the debug inspector (player_id → "none"/"mild"/"full").
    pub(super) shared_tilt: Arc<Mutex<BTreeMap<String, String>>>,
    /// Most recent LLM fallback event; written by the runner, read by the debug inspector.
    pub(super) shared_fallback: Arc<Mutex<Option<String>>>,
    /// Most recent NPC action failure; written by runner, read by debug inspector.
    pub(super) shared_action_error: Arc<Mutex<Option<NpcActionErrorDebug>>>,
    /// Monotonically increasing counter, incremented each time an error is recorded.
    pub(super) error_sequence: u64,
}

impl RunnerState {
    pub(crate) fn new(
        npc_configs: &[NpcConfig],
        shared_tilt: Arc<Mutex<BTreeMap<String, String>>>,
        shared_fallback: Arc<Mutex<Option<String>>>,
        shared_action_error: Arc<Mutex<Option<NpcActionErrorDebug>>>,
    ) -> Self {
        let session_histories = npc_configs
            .iter()
            .map(|c| NpcSessionHistory::new(c.player_id.clone()))
            .collect();
        Self {
            hand_log: None,
            session_histories,
            opponent_stats: OpponentStatsTable::new(),
            last_hand_result_count: 0,
            pre_hand_stacks: BTreeMap::new(),
            shared_tilt,
            shared_fallback,
            shared_action_error,
            error_sequence: 0,
        }
    }

    /// Number of actions recorded in the current hand log. Returns 0 when no log exists.
    #[cfg(test)]
    pub(crate) fn hand_log_action_count(&self) -> usize {
        self.hand_log.as_ref().map(|l| l.actions.len()).unwrap_or(0)
    }
}

/// Build a `player_id → display_name` map from the current tournament state.
pub(super) fn build_display_names(state: &TournamentState) -> BTreeMap<String, String> {
    state
        .seats
        .iter()
        .filter_map(|s| {
            let pid = s.participant_id.as_deref()?;
            let name = s.display_name.as_deref().unwrap_or(pid);
            Some((pid.to_string(), name.to_string()))
        })
        .collect()
}

/// Build a `player_id → chip_count` snapshot from the current state.
pub(super) fn current_stacks(state: &TournamentState) -> BTreeMap<String, u32> {
    state
        .seats
        .iter()
        .filter_map(|s| {
            let pid = s.participant_id.as_deref()?;
            let chips = s.chip_count?;
            Some((pid.to_string(), chips))
        })
        .collect()
}

/// Start the NPC auto-action background thread.
///
/// Returns the join handle. The caller supplies `shared_tilt`, `shared_fallback`, and
/// `shared_action_error` and may read from them at any time (e.g. for the debug inspector).
pub fn start_npc_runner(
    host_server: Arc<HostServer>,
    npc_configs: Vec<NpcConfig>,
    stop: Arc<AtomicBool>,
    api_key_holder: Arc<Mutex<Option<LlmProviderConfig>>>,
    shared_tilt: Arc<Mutex<BTreeMap<String, String>>>,
    shared_fallback: Arc<Mutex<Option<String>>>,
    shared_action_error: Arc<Mutex<Option<NpcActionErrorDebug>>>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("npc-runner".into())
        .spawn(move || {
            loop_core::npc_runner_loop(
                &host_server,
                &npc_configs,
                &stop,
                &api_key_holder,
                shared_tilt,
                shared_fallback,
                shared_action_error,
            );
        })
        .expect("failed to spawn npc-runner thread")
}

pub fn run_npc_loop(
    host_server: &HostServer,
    npc_configs: &[NpcConfig],
    stop: &AtomicBool,
    api_key_holder: &Arc<Mutex<Option<LlmProviderConfig>>>,
) {
    let shared_tilt = Arc::new(Mutex::new(BTreeMap::new()));
    let shared_fallback: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let shared_action_error: Arc<Mutex<Option<NpcActionErrorDebug>>> = Arc::new(Mutex::new(None));
    loop_core::npc_runner_loop(
        host_server,
        npc_configs,
        stop,
        api_key_holder,
        shared_tilt,
        shared_fallback,
        shared_action_error,
    );
}

/// A guard that stops the NPC runner thread when dropped.
pub struct NpcRunnerGuard {
    stop: Arc<AtomicBool>,
    handle: Mutex<Option<thread::JoinHandle<()>>>,
    /// Shared tilt level snapshot; written by the runner, readable by the host session.
    pub tilt_levels: Arc<Mutex<BTreeMap<String, String>>>,
    /// Most recent LLM fallback event; written by the runner, readable by the debug inspector.
    pub last_llm_fallback: Arc<Mutex<Option<String>>>,
    /// Most recent NPC action failure; written by runner, read by debug inspector.
    pub last_npc_action_error: Arc<Mutex<Option<NpcActionErrorDebug>>>,
}

impl NpcRunnerGuard {
    pub fn new(
        stop: Arc<AtomicBool>,
        handle: thread::JoinHandle<()>,
        tilt_levels: Arc<Mutex<BTreeMap<String, String>>>,
        last_llm_fallback: Arc<Mutex<Option<String>>>,
        last_npc_action_error: Arc<Mutex<Option<NpcActionErrorDebug>>>,
    ) -> Self {
        Self {
            stop,
            handle: Mutex::new(Some(handle)),
            tilt_levels,
            last_llm_fallback,
            last_npc_action_error,
        }
    }
}

impl Drop for NpcRunnerGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(mut guard) = self.handle.lock() {
            if let Some(handle) = guard.take() {
                if let Err(e) = handle.join() {
                    // The runner thread panicked. Log what we can — the panic payload
                    // is typically a &str or String.
                    let msg = e
                        .downcast_ref::<&str>()
                        .map(|s| s.to_string())
                        .or_else(|| e.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "unknown panic payload".to_string());
                    eprintln!("[npc-runner] runner thread panicked: {msg}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
