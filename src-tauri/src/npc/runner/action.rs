use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use crate::app_state::{NpcActionErrorDebug, NpcActionErrorReason};
use crate::domain::{ActionType, TournamentSeatState};
use crate::networking::HostServer;
use crate::npc::{
    hand_log::{HandActionRecord, HandLog},
    llm_client::LlmClient,
    llm_strategy::{choose_llm_action, resolve_fallback_style, LlmFallbackReason},
    prompt::GameStateSnapshot,
    provider::LlmProviderConfig,
    tilt::TiltState,
    NpcConfig,
};

use super::{
    current_stacks, fallback_blind_level, hash_str, rule_based_decision, NpcActionOutcome,
    RunnerState, MAX_DELAY_MS, MIN_DELAY_MS,
};

/// Resolved state of the LLM provider for a single action decision.
enum ProviderState {
    Usable(LlmProviderConfig),
    NotConfigured,
    StateUnavailable,
}

fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn try_npc_action(
    host_server: &HostServer,
    state: &crate::domain::TournamentState,
    npc_configs: &[NpcConfig],
    stop: &AtomicBool,
    api_key_holder: &Arc<Mutex<Option<LlmProviderConfig>>>,
    runner_state: &mut RunnerState,
) -> NpcActionOutcome {
    let hand = match &state.current_hand {
        Some(h) => h,
        None => return NpcActionOutcome::NoOpportunity,
    };

    let window = match &hand.action_window {
        Some(w) => w,
        None => return NpcActionOutcome::NoOpportunity,
    };

    if !NpcConfig::is_npc_player_id(&window.player_id) {
        return NpcActionOutcome::NotNpcTurn;
    }

    // Capture early for error records — available for all subsequent returns.
    let window_player_id = window.player_id.clone();
    let initial_hand_number = hand.hand_number;

    let config_entry = npc_configs
        .iter()
        .enumerate()
        .find(|(_, c)| c.player_id == window_player_id);

    let (npc_config_idx, npc_config) = match config_entry {
        Some(pair) => pair,
        None => {
            let msg = format!(
                "[npc-runner] no config found for player {}; skipping action",
                window_player_id
            );
            eprintln!("{msg}");
            runner_state.error_sequence += 1;
            let seq = runner_state.error_sequence;
            if let Ok(mut g) = runner_state.shared_action_error.lock() {
                *g = Some(NpcActionErrorDebug {
                    player_id: Some(window_player_id),
                    action: None,
                    reason: NpcActionErrorReason::NoConfig,
                    message: msg,
                    hand_number: Some(initial_hand_number),
                    sequence: seq,
                    submitted: false,
                    occurred_at_ms: now_epoch_ms(),
                });
            }
            return NpcActionOutcome::NoConfig;
        }
    };

    // Profile present → use profile style; profile absent → use config style.
    let fallback_style = resolve_fallback_style(npc_config.profile.as_ref(), npc_config.style.clone());

    let seed = hash_str(&window_player_id) ^ hash_str(&window.action_window_id);

    let delay_ms = MIN_DELAY_MS + (seed % (MAX_DELAY_MS - MIN_DELAY_MS + 1));
    thread::sleep(Duration::from_millis(delay_ms));

    if stop.load(Ordering::SeqCst) {
        return NpcActionOutcome::Stopped;
    }

    let fresh_state = match host_server.authoritative_state() {
        Ok(s) => s,
        Err(_) => {
            let msg = format!(
                "[npc-runner] runtime unavailable for player {}",
                window_player_id
            );
            eprintln!("{msg}");
            runner_state.error_sequence += 1;
            let seq = runner_state.error_sequence;
            if let Ok(mut g) = runner_state.shared_action_error.lock() {
                *g = Some(NpcActionErrorDebug {
                    player_id: Some(window_player_id),
                    action: None,
                    reason: NpcActionErrorReason::RuntimeUnavailable,
                    message: msg,
                    hand_number: Some(initial_hand_number),
                    sequence: seq,
                    submitted: false,
                    occurred_at_ms: now_epoch_ms(),
                });
            }
            return NpcActionOutcome::RuntimeUnavailable;
        }
    };
    let fresh_window = match fresh_state
        .current_hand
        .as_ref()
        .and_then(|h| h.action_window.as_ref())
    {
        Some(w) if w.action_window_id == window.action_window_id => w.clone(),
        _ => {
            let msg = format!(
                "[npc-runner] action window expired for player {} (window={})",
                window_player_id, window.action_window_id
            );
            eprintln!("{msg}");
            runner_state.error_sequence += 1;
            let seq = runner_state.error_sequence;
            if let Ok(mut g) = runner_state.shared_action_error.lock() {
                *g = Some(NpcActionErrorDebug {
                    player_id: Some(window_player_id),
                    action: None,
                    reason: NpcActionErrorReason::StaleWindow,
                    message: msg,
                    hand_number: Some(initial_hand_number),
                    sequence: seq,
                    submitted: false,
                    occurred_at_ms: now_epoch_ms(),
                });
            }
            return NpcActionOutcome::StaleWindow;
        }
    };
    let fresh_hand = match &fresh_state.current_hand {
        Some(h) => h,
        None => {
            let msg = format!(
                "[npc-runner] no current hand after window check for player {}",
                fresh_window.player_id
            );
            eprintln!("{msg}");
            runner_state.error_sequence += 1;
            let seq = runner_state.error_sequence;
            if let Ok(mut g) = runner_state.shared_action_error.lock() {
                *g = Some(NpcActionErrorDebug {
                    player_id: Some(fresh_window.player_id.clone()),
                    action: None,
                    reason: NpcActionErrorReason::StaleWindow,
                    message: msg,
                    hand_number: Some(initial_hand_number),
                    sequence: seq,
                    submitted: false,
                    occurred_at_ms: now_epoch_ms(),
                });
            }
            return NpcActionOutcome::StaleWindow;
        }
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
        .filter(|s| matches!(s.tournament_state, TournamentSeatState::Active))
        .count() as u8;

    let dealer_seat = fresh_hand.dealer_seat_index;
    let position = crate::npc::strategy::derive_position(
        fresh_window.seat_index,
        dealer_seat,
        active_count.max(2),
    );

    // Resolve provider config with explicit lock-failure detection (P1.2).
    let provider_state = if npc_config.profile.is_some() {
        match api_key_holder.lock() {
            Ok(g) => {
                let usable = g.clone().filter(|c| c.is_usable());
                match usable {
                    Some(cfg) => ProviderState::Usable(cfg),
                    None => ProviderState::NotConfigured,
                }
            }
            // Mutex poisoning is an internal failure, not "provider not configured". (P1.2)
            Err(_) => ProviderState::StateUnavailable,
        }
    } else {
        ProviderState::NotConfigured
    };

    // Choose an action. Hand-log write happens AFTER submit_action returns Ok(()).
    let (action_type, raise_to) = if let Some(profile) = npc_config.profile.as_ref() {
        match provider_state {
            ProviderState::Usable(cfg) => {
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

                let provider_label = format!("{:?}", cfg.settings.provider);
                let llm_client = LlmClient::new(cfg);
                if let Err(ref e) = llm_client {
                    eprintln!("[npc-runner] failed to build LLM client: {e}");
                }
                let (llm_action, llm_raise, fallback_reason) = match llm_client {
                    Ok(client) => choose_llm_action(&client, profile, &snapshot),
                    Err(_) => {
                        // Client construction failed — fall back to rule-based with profile style (P1.3).
                        let rb = rule_based_decision(
                            &fallback_style,
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

                (llm_action, llm_raise)
            }
            ProviderState::NotConfigured => {
                // Profile is set but no usable provider config is available.
                let fallback_reason = LlmFallbackReason::ProviderNotConfigured;
                let msg = format!(
                    "{}: {fallback_reason} (profile={})",
                    fresh_window.player_id, profile.id,
                );
                eprintln!("[npc-runner] LLM fallback — {msg}");
                if let Ok(mut g) = runner_state.shared_fallback.lock() {
                    *g = Some(msg);
                }
                rule_based_decision(
                    &fallback_style,
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
            ProviderState::StateUnavailable => {
                // Mutex poisoning — distinct from provider not configured (P1.2).
                let msg = format!(
                    "{}: ProviderStateUnavailable (profile={})",
                    fresh_window.player_id, profile.id,
                );
                eprintln!("[npc-runner] LLM fallback — {msg}");
                if let Ok(mut g) = runner_state.shared_fallback.lock() {
                    *g = Some(msg);
                }
                rule_based_decision(
                    &fallback_style,
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
        }
    } else {
        rule_based_decision(
            &fallback_style,
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

    let is_voluntary = !matches!(action_type, ActionType::AllIn) || call_amount == 0;

    // Submit first; only write to the hand log on acceptance (P0.1).
    match host_server.submit_action(
        &fresh_window.player_id,
        fresh_window.action_window_id.clone(),
        action_type,
        raise_to,
    ) {
        Ok(()) => {
            if let Some(log) = &mut runner_state.hand_log {
                log.push(HandActionRecord {
                    hand_number: fresh_hand.hand_number,
                    street,
                    player_id: fresh_window.player_id.clone(),
                    action_type,
                    amount: raise_to,
                    is_voluntary,
                });
            }
            NpcActionOutcome::Success
        }
        Err(e) => {
            let msg = format!(
                "{}: submit_action rejected (window={}, action={action_type:?}): {e}",
                fresh_window.player_id, fresh_window.action_window_id
            );
            eprintln!("[npc-runner] {msg}");
            runner_state.error_sequence += 1;
            let seq = runner_state.error_sequence;
            if let Ok(mut g) = runner_state.shared_action_error.lock() {
                *g = Some(NpcActionErrorDebug {
                    player_id: Some(fresh_window.player_id.clone()),
                    action: Some(format!("{action_type:?}")),
                    reason: NpcActionErrorReason::Rejected,
                    message: msg,
                    hand_number: Some(fresh_hand.hand_number),
                    sequence: seq,
                    submitted: true,
                    occurred_at_ms: now_epoch_ms(),
                });
            }
            NpcActionOutcome::Rejected(e.to_string())
        }
    }
}
