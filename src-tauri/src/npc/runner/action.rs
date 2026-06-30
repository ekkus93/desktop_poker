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
    llm_strategy::{choose_llm_action, resolve_fallback_style},
    prompt::GameStateSnapshot,
    provider::LlmProviderConfig,
    tilt::TiltState,
    NpcConfig,
};

use super::decision::RuleDecisionContext;
use super::{
    current_stacks, hash_str, rule_based_decision, NpcActionOutcome, RunnerState, MAX_DELAY_MS,
    MIN_DELAY_MS,
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

/// Record a structured NPC internal error and return `RuntimeUnavailable`.
///
/// Use for failures where the NPC cannot act and no rule-based fallback is
/// appropriate: poisoned provider state, invalid stored config, or LLM failure
/// when `allow_rule_based_llm_fallback` is false.
pub(super) fn record_npc_internal_error(
    runner_state: &mut RunnerState,
    player_id: String,
    hand_number: Option<u32>,
    reason: NpcActionErrorReason,
    message: String,
) -> NpcActionOutcome {
    eprintln!("[npc-runner] {message}");
    runner_state.error_sequence += 1;
    let seq = runner_state.error_sequence;
    if let Ok(mut g) = runner_state.shared_action_error.lock() {
        *g = Some(NpcActionErrorDebug {
            player_id: Some(player_id),
            action: None,
            reason,
            message,
            hand_number,
            sequence: seq,
            submitted: false,
            occurred_at_ms: now_epoch_ms(),
        });
    }
    NpcActionOutcome::RuntimeUnavailable
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
    let fallback_style =
        resolve_fallback_style(npc_config.profile.as_ref(), npc_config.style.clone());

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

    let hole_cards = match fresh_hand
        .hole_cards_by_player_id
        .get(&fresh_window.player_id)
    {
        Some(cards) if cards.len() == 2 => cards.as_slice(),
        Some(cards) => {
            let msg = format!(
                "[npc-runner] NPC {} has invalid hole-card count {}; expected 2; no action submitted",
                fresh_window.player_id,
                cards.len()
            );
            return record_npc_internal_error(
                runner_state,
                fresh_window.player_id.clone(),
                Some(fresh_hand.hand_number),
                NpcActionErrorReason::InternalError,
                msg,
            );
        }
        None => {
            let msg = format!(
                "[npc-runner] NPC {} is missing hole cards; no action submitted",
                fresh_window.player_id
            );
            return record_npc_internal_error(
                runner_state,
                fresh_window.player_id.clone(),
                Some(fresh_hand.hand_number),
                NpcActionErrorReason::InternalError,
                msg,
            );
        }
    };

    let board = &fresh_hand.board_cards;
    let street = fresh_hand.betting_round.street;
    let pot_total = fresh_hand.betting_round.pot_size;
    let call_amount = fresh_window.call_amount;
    let min_raise_to = fresh_window.min_raise_to;
    let max_raise_to = fresh_window.max_raise_to;
    let facing_bet = call_amount > 0;
    let legal_actions = &fresh_window.legal_actions;

    // P0.5: checked stack extraction — missing stack is an internal error, not a default.
    let stack = match fresh_state
        .seats
        .iter()
        .find(|s| s.participant_id.as_deref() == Some(fresh_window.player_id.as_str()))
        .and_then(|s| s.chip_count)
    {
        Some(s) => s,
        None => {
            let msg = format!(
                "[npc-runner] cannot determine stack for player {}; no action submitted",
                fresh_window.player_id
            );
            return record_npc_internal_error(
                runner_state,
                fresh_window.player_id.clone(),
                Some(fresh_hand.hand_number),
                NpcActionErrorReason::InternalError,
                msg,
            );
        }
    };

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

    // P0.5: checked blind level extraction — missing blind is an internal error, not a default.
    let blind_level = match fresh_state
        .config
        .blind_schedule
        .levels
        .get(fresh_state.blind_level_index)
    {
        Some(l) => l.clone(),
        None => {
            let msg = format!(
                "[npc-runner] blind schedule has no entry for index {} (player={}); no action submitted",
                fresh_state.blind_level_index, fresh_window.player_id
            );
            return record_npc_internal_error(
                runner_state,
                fresh_window.player_id.clone(),
                Some(fresh_hand.hand_number),
                NpcActionErrorReason::InternalError,
                msg,
            );
        }
    };
    let big_blind = blind_level.big_blind;
    // current_bet is non-optional: we have a validated current_hand (fresh_hand).
    let current_bet = fresh_hand.betting_round.current_bet;

    // Resolve provider config with explicit lock-failure detection.
    let provider_state = if npc_config.profile.is_some() {
        match api_key_holder.lock() {
            Ok(g) => {
                let usable = g.clone().filter(|c| c.is_usable());
                match usable {
                    Some(cfg) => ProviderState::Usable(cfg),
                    None => ProviderState::NotConfigured,
                }
            }
            // Mutex poisoning is an internal failure, not "provider not configured".
            Err(_) => ProviderState::StateUnavailable,
        }
    } else {
        ProviderState::NotConfigured
    };

    // Build the rule-based decision context once; all required fields are validated above.
    let rule_ctx = RuleDecisionContext {
        style: &fallback_style,
        hole_cards,
        board_cards: board,
        street,
        pot_total,
        call_amount,
        min_raise_to,
        max_raise_to,
        facing_bet,
        stack,
        active_count,
        dealer_seat,
        npc_seat: fresh_window.seat_index,
        big_blind,
        current_bet,
        legal_actions,
        seed,
    };
    let do_rule_based = || rule_based_decision(&rule_ctx);

    // Choose an action. Hand-log write happens AFTER submit_action returns Ok(()).
    let (action_type, raise_to) = if let Some(profile) = npc_config.profile.as_ref() {
        match provider_state {
            ProviderState::StateUnavailable => {
                // Mutex poisoning is an internal failure — never use rule-based fallback.
                let msg = format!(
                    "{}: provider state lock unavailable for profile-backed NPC {}; no action submitted",
                    fresh_window.player_id, profile.id,
                );
                if let Ok(mut g) = runner_state.shared_fallback.lock() {
                    *g = Some(msg.clone());
                }
                return record_npc_internal_error(
                    runner_state,
                    fresh_window.player_id.clone(),
                    Some(fresh_hand.hand_number),
                    NpcActionErrorReason::ProviderStateUnavailable,
                    msg,
                );
            }
            ProviderState::NotConfigured => {
                // Profile-backed NPC has no usable provider config.
                let msg = format!(
                    "{}: profile-backed NPC {} cannot act — no usable LLM provider configured",
                    fresh_window.player_id, profile.id,
                );
                if npc_config.allow_rule_based_llm_fallback {
                    let rb = do_rule_based();
                    let log_msg = format!("{msg} → rule-based:{:?} at ts={}", rb.0, now_epoch_ms());
                    eprintln!("[npc-runner] LLM fallback — {log_msg}");
                    if let Ok(mut g) = runner_state.shared_fallback.lock() {
                        *g = Some(log_msg);
                    }
                    rb
                } else {
                    if let Ok(mut g) = runner_state.shared_fallback.lock() {
                        *g = Some(msg.clone());
                    }
                    return record_npc_internal_error(
                        runner_state,
                        fresh_window.player_id.clone(),
                        Some(fresh_hand.hand_number),
                        NpcActionErrorReason::InternalError,
                        msg,
                    );
                }
            }
            ProviderState::Usable(cfg) => {
                // blind_level already validated and extracted above.

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
                // LLM client construction failure is an internal error (bad stored config).
                let client = match LlmClient::new(cfg) {
                    Ok(c) => c,
                    Err(e) => {
                        let msg = format!(
                            "{}: failed to build LLM client for profile {} (provider={}): {e}; no action submitted",
                            fresh_window.player_id, profile.id, provider_label
                        );
                        if let Ok(mut g) = runner_state.shared_fallback.lock() {
                            *g = Some(msg.clone());
                        }
                        return record_npc_internal_error(
                            runner_state,
                            fresh_window.player_id.clone(),
                            Some(fresh_hand.hand_number),
                            NpcActionErrorReason::InternalError,
                            msg,
                        );
                    }
                };

                match choose_llm_action(&client, profile, &snapshot) {
                    Ok((at, amt)) => (at, amt),
                    Err(reason) => {
                        let failure_msg = format!(
                            "{}: {reason} (profile={}, provider={provider_label})",
                            fresh_window.player_id, profile.id,
                        );
                        if npc_config.allow_rule_based_llm_fallback {
                            let rb = do_rule_based();
                            let log_msg = format!(
                                "{failure_msg} → rule-based:{:?} at ts={}",
                                rb.0,
                                now_epoch_ms()
                            );
                            eprintln!("[npc-runner] LLM fallback — {log_msg}");
                            if let Ok(mut g) = runner_state.shared_fallback.lock() {
                                *g = Some(log_msg);
                            }
                            rb
                        } else {
                            if let Ok(mut g) = runner_state.shared_fallback.lock() {
                                *g = Some(failure_msg.clone());
                            }
                            return record_npc_internal_error(
                                runner_state,
                                fresh_window.player_id.clone(),
                                Some(fresh_hand.hand_number),
                                NpcActionErrorReason::InternalError,
                                failure_msg,
                            );
                        }
                    }
                }
            }
        }
    } else {
        do_rule_based()
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
