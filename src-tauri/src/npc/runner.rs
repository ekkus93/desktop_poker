use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use crate::domain::{ActionType, BlindLevel, StreetPhase, TournamentState};
use crate::networking::HostServer;

use super::llm_strategy::choose_llm_action;
use super::postflop::postflop_hand_category;
use super::preflop::preflop_hand_tier;
use super::prompt::GameStateSnapshot;
use super::strategy::{choose_postflop_action, choose_preflop_action, derive_position, NpcAction};
use super::{NpcConfig, NpcStyle};

/// Interval between polls when no NPC action is pending.
const POLL_INTERVAL_MS: u64 = 80;

/// Range for the simulated thinking delay: [MIN_DELAY_MS, MAX_DELAY_MS].
const MIN_DELAY_MS: u64 = 300;
const MAX_DELAY_MS: u64 = 1200;

/// Start the NPC auto-action background thread.
pub fn start_npc_runner(
    host_server: Arc<HostServer>,
    npc_configs: Vec<NpcConfig>,
    stop: Arc<AtomicBool>,
    api_key_holder: Arc<Mutex<Option<String>>>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("npc-runner".into())
        .spawn(move || {
            npc_runner_loop(&host_server, &npc_configs, &stop, &api_key_holder);
        })
        .expect("failed to spawn npc-runner thread")
}

pub fn run_npc_loop(
    host_server: &HostServer,
    npc_configs: &[NpcConfig],
    stop: &AtomicBool,
    api_key_holder: &Arc<Mutex<Option<String>>>,
) {
    npc_runner_loop(host_server, npc_configs, stop, api_key_holder);
}

fn npc_runner_loop(
    host_server: &HostServer,
    npc_configs: &[NpcConfig],
    stop: &AtomicBool,
    api_key_holder: &Arc<Mutex<Option<String>>>,
) {
    let mut consecutive_errors: u32 = 0;

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

        let acted = try_npc_action(host_server, &state, npc_configs, stop, api_key_holder);

        if !acted {
            thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
        }
    }
}

fn try_npc_action(
    host_server: &HostServer,
    state: &TournamentState,
    npc_configs: &[NpcConfig],
    stop: &AtomicBool,
    api_key_holder: &Arc<Mutex<Option<String>>>,
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

    let npc_seat: u8 = window
        .player_id
        .strip_prefix("npc-seat-")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let npc_config = npc_configs
        .get(npc_seat as usize)
        .or_else(|| npc_configs.first());

    let style = npc_config
        .map(|c| &c.style)
        .unwrap_or(&NpcStyle::Aggressive);

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

    // LLM path when the NPC has a profile and an API key is available.
    let (action_type, raise_to) = if let Some(profile) = npc_config.and_then(|c| c.profile.as_ref())
    {
        let api_key = api_key_holder.lock().ok().and_then(|g| g.clone());
        if let Some(key) = api_key {
            let blind_level = fresh_state
                .config
                .blind_schedule
                .levels
                .get(fresh_state.blind_level_index)
                .cloned()
                .unwrap_or_else(fallback_blind_level);

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
            };

            let client = crate::npc::llm_client::LlmClient::new(key);
            choose_llm_action(&client, profile, &snapshot)
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

    let _ = host_server.submit_action(
        &fresh_window.player_id,
        fresh_window.action_window_id.clone(),
        action_type,
        raise_to,
    );

    true
}

fn fallback_blind_level() -> BlindLevel {
    BlindLevel {
        level_index: 0,
        label: "L1".to_string(),
        small_blind: 10,
        big_blind: 20,
        ante: 0,
        duration_seconds: 600,
    }
}

#[allow(clippy::too_many_arguments)]
fn rule_based_decision(
    style: &NpcStyle,
    hole_cards: &[crate::domain::Card],
    board: &[crate::domain::Card],
    street: StreetPhase,
    pot_total: u32,
    call_amount: u32,
    min_raise_to: Option<u32>,
    max_raise_to: Option<u32>,
    facing_bet: bool,
    stack: u32,
    active_count: u8,
    dealer_seat: u8,
    npc_seat: u8,
    blind_level_index: usize,
    state: &TournamentState,
    legal_actions: &[ActionType],
    seed: u64,
) -> (ActionType, Option<u32>) {
    let position = derive_position(npc_seat, dealer_seat, active_count.max(2));

    let action = if street == StreetPhase::Preflop {
        let tier = preflop_hand_tier(hole_cards);
        let big_blind = state
            .config
            .blind_schedule
            .levels
            .get(blind_level_index)
            .map(|l| l.big_blind)
            .unwrap_or(20);
        let facing_raise = call_amount > big_blind;
        let raise_count = if state
            .current_hand
            .as_ref()
            .map(|h| h.betting_round.current_bet)
            .unwrap_or(0)
            > big_blind * 3
        {
            2
        } else if facing_raise {
            1
        } else {
            0
        };

        choose_preflop_action(
            style,
            tier,
            position,
            facing_raise,
            raise_count,
            min_raise_to,
            max_raise_to,
            call_amount,
            pot_total,
            stack,
            seed,
        )
    } else {
        let category = postflop_hand_category(hole_cards, board);
        let facing_bet_fraction = if pot_total > 0 {
            call_amount as f32 / pot_total as f32
        } else {
            0.0
        };
        choose_postflop_action(
            style,
            category,
            facing_bet,
            facing_bet_fraction,
            min_raise_to,
            max_raise_to,
            call_amount,
            pot_total,
            stack,
            seed,
        )
    };

    match action {
        NpcAction::Fold => {
            if legal_actions.contains(&ActionType::Fold) {
                (ActionType::Fold, None)
            } else {
                first_check_or_call(legal_actions)
            }
        }
        NpcAction::CheckOrCall => first_check_or_call(legal_actions),
        NpcAction::Raise(amount) => {
            if legal_actions.contains(&ActionType::Raise)
                || legal_actions.contains(&ActionType::Bet)
            {
                let at = if legal_actions.contains(&ActionType::Raise) {
                    ActionType::Raise
                } else {
                    ActionType::Bet
                };
                (at, Some(amount))
            } else if legal_actions.contains(&ActionType::AllIn) {
                (ActionType::AllIn, None)
            } else {
                first_check_or_call(legal_actions)
            }
        }
    }
}

pub(crate) fn first_check_or_call(legal: &[ActionType]) -> (ActionType, Option<u32>) {
    if legal.contains(&ActionType::Check) {
        (ActionType::Check, None)
    } else if legal.contains(&ActionType::Call) {
        (ActionType::Call, None)
    } else {
        (ActionType::Fold, None)
    }
}

fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 14_695_981_039_346_656_037;
    for byte in s.bytes() {
        h ^= u64::from(byte);
        h = h.wrapping_mul(1_099_511_628_211);
    }
    h
}

/// A guard that stops the NPC runner thread when dropped.
pub struct NpcRunnerGuard {
    stop: Arc<AtomicBool>,
    handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl NpcRunnerGuard {
    pub fn new(stop: Arc<AtomicBool>, handle: thread::JoinHandle<()>) -> Self {
        Self {
            stop,
            handle: Mutex::new(Some(handle)),
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
