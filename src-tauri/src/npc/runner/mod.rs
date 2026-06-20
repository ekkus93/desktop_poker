use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use crate::domain::{ActionType, HandResult, StreetPhase, TournamentState};
use crate::networking::HostServer;

use super::hand_log::{HandActionRecord, HandLog};
use super::llm_client::LlmClient;
use super::llm_strategy::{choose_llm_action, LlmFallbackReason};
use super::opponent_stats::OpponentStatsTable;
use super::prompt::GameStateSnapshot;
use super::provider::LlmProviderConfig;
use super::session_history::{HandSummary, NpcSessionHistory};
use super::strategy::derive_position;
use super::tilt::TiltState;
use super::NpcConfig;

mod decision;
pub(crate) use decision::*;

/// Interval between polls when no NPC action is pending.
const POLL_INTERVAL_MS: u64 = 80;

/// Range for the simulated thinking delay: [MIN_DELAY_MS, MAX_DELAY_MS].
const MIN_DELAY_MS: u64 = 300;
const MAX_DELAY_MS: u64 = 1200;

/// Per-loop mutable state tracking hand history across the runner lifecycle.
pub(crate) struct RunnerState {
    /// Log of actions for the hand currently in progress.
    hand_log: Option<HandLog>,
    /// Per-NPC session history; indexed parallel to `npc_configs`.
    session_histories: Vec<NpcSessionHistory>,
    /// Accumulated opponent stats for every human player seen this session.
    opponent_stats: OpponentStatsTable,
    /// Number of HandResults seen so far (used to detect hand completion).
    last_hand_result_count: usize,
    /// Stack size of each NPC at the start of the most recent hand.
    pre_hand_stacks: BTreeMap<String, u32>,
    /// Shared tilt level snapshot for the debug inspector (player_id → "none"/"mild"/"full").
    shared_tilt: Arc<Mutex<BTreeMap<String, String>>>,
    /// Most recent LLM fallback event; written by the runner, read by the debug inspector.
    shared_fallback: Arc<Mutex<Option<String>>>,
}

impl RunnerState {
    pub(crate) fn new(
        npc_configs: &[NpcConfig],
        shared_tilt: Arc<Mutex<BTreeMap<String, String>>>,
        shared_fallback: Arc<Mutex<Option<String>>>,
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
        }
    }
}

/// Start the NPC auto-action background thread.
///
/// Returns the join handle. The caller supplies `shared_tilt` and `shared_fallback` and may
/// read from them at any time (e.g. for the debug inspector).
pub fn start_npc_runner(
    host_server: Arc<HostServer>,
    npc_configs: Vec<NpcConfig>,
    stop: Arc<AtomicBool>,
    api_key_holder: Arc<Mutex<Option<LlmProviderConfig>>>,
    shared_tilt: Arc<Mutex<BTreeMap<String, String>>>,
    shared_fallback: Arc<Mutex<Option<String>>>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("npc-runner".into())
        .spawn(move || {
            npc_runner_loop(
                &host_server,
                &npc_configs,
                &stop,
                &api_key_holder,
                shared_tilt,
                shared_fallback,
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
    let shared_fallback = Arc::new(Mutex::new(None));
    npc_runner_loop(
        host_server,
        npc_configs,
        stop,
        api_key_holder,
        shared_tilt,
        shared_fallback,
    );
}

fn npc_runner_loop(
    host_server: &HostServer,
    npc_configs: &[NpcConfig],
    stop: &AtomicBool,
    api_key_holder: &Arc<Mutex<Option<LlmProviderConfig>>>,
    shared_tilt: Arc<Mutex<BTreeMap<String, String>>>,
    shared_fallback: Arc<Mutex<Option<String>>>,
) {
    let mut consecutive_errors: u32 = 0;
    let mut runner_state = RunnerState::new(npc_configs, shared_tilt, shared_fallback);

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

        let acted = try_npc_action(
            host_server,
            &state,
            npc_configs,
            stop,
            api_key_holder,
            &mut runner_state,
        );

        if !acted {
            thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
        }
    }
}

/// Build a `player_id → display_name` map from the current tournament state.
fn build_display_names(state: &TournamentState) -> BTreeMap<String, String> {
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
fn current_stacks(state: &TournamentState) -> BTreeMap<String, u32> {
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

/// Detect newly completed hands and update session histories and opponent stats.
fn process_completed_hands(
    state: &TournamentState,
    npc_configs: &[NpcConfig],
    runner_state: &mut RunnerState,
) {
    let result_count = state.hand_results.len();
    if result_count <= runner_state.last_hand_result_count {
        return;
    }

    // Process each newly completed hand.
    let new_results: Vec<&HandResult> = state
        .hand_results
        .iter()
        .skip(runner_state.last_hand_result_count)
        .collect();

    let display_names = build_display_names(state);
    let empty_log = HandLog::new(0);
    let hand_log = runner_state.hand_log.as_ref().unwrap_or(&empty_log);

    for result in &new_results {
        runner_state
            .opponent_stats
            .update_from_hand(hand_log, result, &display_names);

        for (npc_config, history) in npc_configs
            .iter()
            .zip(runner_state.session_histories.iter_mut())
        {
            let player_id = &npc_config.player_id;
            let npc_won = result.winning_player_ids.contains(player_id);
            let pot_size: u32 = result.pot_summaries.iter().map(|p| p.amount).sum();
            let went_to_showdown = result
                .revealed_hands_by_player_id
                .contains_key(player_id.as_str());

            // Determine net chips from pre-hand stack vs. post-hand stack.
            let pre_stack = runner_state
                .pre_hand_stacks
                .get(player_id.as_str())
                .copied()
                .unwrap_or(0);
            let post_stack = result
                .final_stack_by_player_id
                .get(player_id.as_str())
                .copied()
                .unwrap_or(0);
            let net_chips = post_stack as i32 - pre_stack as i32;

            // Determine bluff caught: NPC lost at showdown and had post-flop aggression.
            let npc_bluff_caught = if went_to_showdown && !npc_won {
                hand_log.actions_by(player_id).iter().any(|r| {
                    matches!(
                        r.street,
                        StreetPhase::Flop | StreetPhase::Turn | StreetPhase::River
                    ) && matches!(r.action_type, ActionType::Bet | ActionType::Raise)
                })
            } else {
                false
            };

            // Determine bluffed: NPC bet/raised post-flop but did NOT go to showdown or
            // went to showdown and lost with post-flop aggression.
            let had_postflop_bet = hand_log.actions_by(player_id).iter().any(|r| {
                matches!(
                    r.street,
                    StreetPhase::Flop | StreetPhase::Turn | StreetPhase::River
                ) && matches!(r.action_type, ActionType::Bet | ActionType::Raise)
            });
            let npc_bluffed = had_postflop_bet && (!went_to_showdown || npc_bluff_caught);

            let opponent_ids: Vec<String> = result
                .final_stack_by_player_id
                .keys()
                .filter(|id| id.as_str() != player_id.as_str())
                .cloned()
                .collect();

            let summary = HandSummary {
                hand_number: result.hand_number,
                npc_won,
                pot_size,
                net_chips,
                npc_went_to_showdown: went_to_showdown,
                npc_bluffed,
                npc_bluff_caught,
                opponent_ids_in_hand: opponent_ids,
            };

            history.record_hand(summary);
        }
    }

    runner_state.last_hand_result_count = result_count;

    // Publish updated tilt levels for the debug inspector.
    if let Ok(mut tilt_map) = runner_state.shared_tilt.lock() {
        tilt_map.clear();
        for (npc_config, history) in npc_configs
            .iter()
            .zip(runner_state.session_histories.iter())
        {
            let tilt = TiltState::from_history(history);
            let level_str = match tilt.level {
                super::tilt::TiltLevel::None => "none",
                super::tilt::TiltLevel::Mild => "mild",
                super::tilt::TiltLevel::Full => "full",
            };
            tilt_map.insert(npc_config.player_id.clone(), level_str.to_string());
        }
    }

    // Snapshot stacks for the next hand.
    runner_state.pre_hand_stacks = current_stacks(state);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn try_npc_action(
    host_server: &HostServer,
    state: &TournamentState,
    npc_configs: &[NpcConfig],
    stop: &AtomicBool,
    api_key_holder: &Arc<Mutex<Option<LlmProviderConfig>>>,
    runner_state: &mut RunnerState,
) -> bool {
    let hand = match &state.current_hand {
        Some(h) => h,
        None => return false,
    };

    let window = match &hand.action_window {
        Some(w) => w,
        None => return false,
    };

    if !NpcConfig::is_npc_player_id(&window.player_id) {
        return false;
    }

    let config_entry = npc_configs
        .iter()
        .enumerate()
        .find(|(_, c)| c.player_id == window.player_id);

    let (npc_config_idx, npc_config) = match config_entry {
        Some(pair) => pair,
        None => {
            eprintln!(
                "[npc-runner] no config found for player {}; skipping action",
                window.player_id
            );
            return false;
        }
    };

    let style = &npc_config.style;

    let seed = hash_str(&window.player_id) ^ hash_str(&window.action_window_id);

    let delay_ms = MIN_DELAY_MS + (seed % (MAX_DELAY_MS - MIN_DELAY_MS + 1));
    thread::sleep(Duration::from_millis(delay_ms));

    if stop.load(Ordering::SeqCst) {
        return false;
    }

    let fresh_state = match host_server.authoritative_state() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let fresh_window = match fresh_state
        .current_hand
        .as_ref()
        .and_then(|h| h.action_window.as_ref())
    {
        Some(w) if w.action_window_id == window.action_window_id => w.clone(),
        _ => return false,
    };
    let fresh_hand = match &fresh_state.current_hand {
        Some(h) => h,
        None => return false,
    };

    // Initialize or reset the hand log when we detect a new hand.
    if runner_state
        .hand_log
        .as_ref()
        .map(|l| l.hand_number != fresh_hand.hand_number)
        .unwrap_or(true)
    {
        runner_state.hand_log = Some(HandLog::new(fresh_hand.hand_number));
        // Snapshot pre-hand stacks if not already done for this hand.
        if runner_state.pre_hand_stacks.is_empty() {
            runner_state.pre_hand_stacks = current_stacks(&fresh_state);
        }
    }

    let hole_cards = fresh_hand
        .hole_cards_by_player_id
        .get(&fresh_window.player_id)
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    let board = &fresh_hand.board_cards;
    let street = fresh_hand.betting_round.street;
    let pot_total = fresh_hand.betting_round.pot_size;
    let call_amount = fresh_window.call_amount;
    let min_raise_to = fresh_window.min_raise_to;
    let max_raise_to = fresh_window.max_raise_to;
    let facing_bet = call_amount > 0;
    let legal_actions = &fresh_window.legal_actions;

    let stack = fresh_state
        .seats
        .iter()
        .find(|s| s.participant_id.as_deref() == Some(fresh_window.player_id.as_str()))
        .and_then(|s| s.chip_count)
        .unwrap_or(1);

    let active_count = fresh_state
        .seats
        .iter()
        .filter(|s| {
            matches!(
                s.tournament_state,
                crate::domain::TournamentSeatState::Active
            )
        })
        .count() as u8;

    let dealer_seat = fresh_hand.dealer_seat_index;
    let position = derive_position(fresh_window.seat_index, dealer_seat, active_count.max(2));

    // LLM path when the NPC has a profile and a usable provider config is available.
    let (action_type, raise_to) = if let Some(profile) = npc_config.profile.as_ref() {
        let provider_cfg = api_key_holder.lock().ok().and_then(|g| g.clone());
        let usable_cfg = provider_cfg.filter(|c| c.is_usable());
        if let Some(cfg) = usable_cfg {
            let blind_level = fresh_state
                .config
                .blind_schedule
                .levels
                .get(fresh_state.blind_level_index)
                .cloned()
                .unwrap_or_else(fallback_blind_level);

            // Build session context.
            let (session_ctx, tilt_desc) =
                if let Some(history) = runner_state.session_histories.get(npc_config_idx) {
                    let ctx = if history.hands_played() > 0 {
                        Some(history.render_context())
                    } else {
                        None
                    };
                    let tilt = TiltState::from_history(history);
                    let desc = tilt.description();
                    (ctx, desc)
                } else {
                    (None, None)
                };

            let opp_ctx = {
                let ctx = runner_state.opponent_stats.render_context();
                if ctx.is_empty() {
                    None
                } else {
                    Some(ctx)
                }
            };

            let snapshot = GameStateSnapshot {
                hand_number: fresh_hand.hand_number,
                street,
                board_cards: board.clone(),
                hole_cards: hole_cards.to_vec(),
                pot_total,
                call_amount,
                min_raise_to,
                max_raise_to,
                stack,
                position,
                active_player_count: active_count,
                legal_actions: legal_actions.clone(),
                blind_level,
                street_history: vec![],
                session_context: session_ctx,
                opponent_context: opp_ctx,
                tilt_description: tilt_desc,
            };

            let provider_label = format!("{:?}", cfg.provider);
            let llm_client = LlmClient::new(cfg);
            if let Err(ref e) = llm_client {
                eprintln!("[npc-runner] failed to build LLM client: {e}");
            }
            let (llm_action, llm_raise, fallback_reason) = match llm_client {
                Ok(client) => choose_llm_action(&client, profile, &snapshot),
                Err(_) => {
                    let rb = rule_based_decision(
                        style,
                        hole_cards,
                        board,
                        street,
                        pot_total,
                        call_amount,
                        min_raise_to,
                        max_raise_to,
                        facing_bet,
                        stack,
                        active_count,
                        dealer_seat,
                        fresh_window.seat_index,
                        fresh_state.blind_level_index,
                        &fresh_state,
                        legal_actions,
                        seed,
                    );
                    (rb.0, rb.1, Some(LlmFallbackReason::RequestFailed))
                }
            };

            if let Some(reason) = &fallback_reason {
                let msg = format!(
                    "{}: {reason} (profile={}, provider={provider_label})",
                    fresh_window.player_id, profile.id,
                );
                eprintln!("[npc-runner] LLM fallback — {msg}");
                if let Ok(mut g) = runner_state.shared_fallback.lock() {
                    *g = Some(msg);
                }
            }

            // Record the action into the hand log.
            if let Some(log) = &mut runner_state.hand_log {
                log.push(HandActionRecord {
                    hand_number: fresh_hand.hand_number,
                    street,
                    player_id: fresh_window.player_id.clone(),
                    action_type: llm_action,
                    amount: llm_raise,
                    is_voluntary: true,
                });
            }

            (llm_action, llm_raise)
        } else {
            // Profile is set but no usable provider config is available.
            let msg = format!(
                "{}: ProviderNotConfigured (profile={})",
                fresh_window.player_id, profile.id,
            );
            eprintln!("[npc-runner] LLM fallback — {msg}");
            if let Ok(mut g) = runner_state.shared_fallback.lock() {
                *g = Some(msg);
            }
            rule_based_decision(
                style,
                hole_cards,
                board,
                street,
                pot_total,
                call_amount,
                min_raise_to,
                max_raise_to,
                facing_bet,
                stack,
                active_count,
                dealer_seat,
                fresh_window.seat_index,
                fresh_state.blind_level_index,
                &fresh_state,
                legal_actions,
                seed,
            )
        }
    } else {
        rule_based_decision(
            style,
            hole_cards,
            board,
            street,
            pot_total,
            call_amount,
            min_raise_to,
            max_raise_to,
            facing_bet,
            stack,
            active_count,
            dealer_seat,
            fresh_window.seat_index,
            fresh_state.blind_level_index,
            &fresh_state,
            legal_actions,
            seed,
        )
    };

    // Record the action into the hand log for rule-based path too (when LLM not used).
    if let Some(log) = &mut runner_state.hand_log {
        // Avoid double-logging: the LLM path already records when it executes.
        // The rule-based path always falls through here, so record it.
        if npc_config.profile.is_none() {
            log.push(HandActionRecord {
                hand_number: fresh_hand.hand_number,
                street,
                player_id: fresh_window.player_id.clone(),
                action_type,
                amount: raise_to,
                is_voluntary: !matches!(action_type, ActionType::AllIn) || call_amount == 0,
            });
        }
    }

    match host_server.submit_action(
        &fresh_window.player_id,
        fresh_window.action_window_id.clone(),
        action_type,
        raise_to,
    ) {
        Ok(()) => true,
        Err(e) => {
            eprintln!(
                "[npc-runner] submit_action failed for {} (window={}, action={:?}): {e}",
                fresh_window.player_id, fresh_window.action_window_id, action_type
            );
            false
        }
    }
}

/// A guard that stops the NPC runner thread when dropped.
pub struct NpcRunnerGuard {
    stop: Arc<AtomicBool>,
    handle: Mutex<Option<thread::JoinHandle<()>>>,
    /// Shared tilt level snapshot; written by the runner, readable by the host session.
    pub tilt_levels: Arc<Mutex<BTreeMap<String, String>>>,
    /// Most recent LLM fallback event; written by the runner, readable by the debug inspector.
    pub last_llm_fallback: Arc<Mutex<Option<String>>>,
}

impl NpcRunnerGuard {
    pub fn new(
        stop: Arc<AtomicBool>,
        handle: thread::JoinHandle<()>,
        tilt_levels: Arc<Mutex<BTreeMap<String, String>>>,
        last_llm_fallback: Arc<Mutex<Option<String>>>,
    ) -> Self {
        Self {
            stop,
            handle: Mutex::new(Some(handle)),
            tilt_levels,
            last_llm_fallback,
        }
    }
}

impl Drop for NpcRunnerGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(mut guard) = self.handle.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }
    }
}

#[cfg(test)]
mod tests;
