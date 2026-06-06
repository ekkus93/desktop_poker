use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use crate::domain::{ActionType, BlindLevel, HandResult, StreetPhase, TournamentState};
use crate::networking::HostServer;

use super::hand_log::{HandActionRecord, HandLog};
use super::llm_strategy::choose_llm_action;
use super::opponent_stats::OpponentStatsTable;
use super::postflop::postflop_hand_category;
use super::preflop::preflop_hand_tier;
use super::prompt::GameStateSnapshot;
use super::session_history::{HandSummary, NpcSessionHistory};
use super::strategy::{choose_postflop_action, choose_preflop_action, derive_position, NpcAction};
use super::tilt::TiltState;
use super::{NpcConfig, NpcStyle};

/// Interval between polls when no NPC action is pending.
const POLL_INTERVAL_MS: u64 = 80;

/// Range for the simulated thinking delay: [MIN_DELAY_MS, MAX_DELAY_MS].
const MIN_DELAY_MS: u64 = 300;
const MAX_DELAY_MS: u64 = 1200;

/// Per-loop mutable state tracking hand history across the runner lifecycle.
struct RunnerState {
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
}

impl RunnerState {
    fn new(npc_configs: &[NpcConfig], shared_tilt: Arc<Mutex<BTreeMap<String, String>>>) -> Self {
        let session_histories = npc_configs
            .iter()
            .enumerate()
            .map(|(i, _)| NpcSessionHistory::new(NpcConfig::player_id(i as u8)))
            .collect();
        Self {
            hand_log: None,
            session_histories,
            opponent_stats: OpponentStatsTable::new(),
            last_hand_result_count: 0,
            pre_hand_stacks: BTreeMap::new(),
            shared_tilt,
        }
    }
}

/// Start the NPC auto-action background thread.
///
/// Returns the join handle. The caller supplies `shared_tilt` and may read from it
/// at any time to observe current tilt levels (e.g. for the debug inspector).
pub fn start_npc_runner(
    host_server: Arc<HostServer>,
    npc_configs: Vec<NpcConfig>,
    stop: Arc<AtomicBool>,
    api_key_holder: Arc<Mutex<Option<String>>>,
    shared_tilt: Arc<Mutex<BTreeMap<String, String>>>,
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
            );
        })
        .expect("failed to spawn npc-runner thread")
}

pub fn run_npc_loop(
    host_server: &HostServer,
    npc_configs: &[NpcConfig],
    stop: &AtomicBool,
    api_key_holder: &Arc<Mutex<Option<String>>>,
) {
    let shared_tilt = Arc::new(Mutex::new(BTreeMap::new()));
    npc_runner_loop(host_server, npc_configs, stop, api_key_holder, shared_tilt);
}

fn npc_runner_loop(
    host_server: &HostServer,
    npc_configs: &[NpcConfig],
    stop: &AtomicBool,
    api_key_holder: &Arc<Mutex<Option<String>>>,
    shared_tilt: Arc<Mutex<BTreeMap<String, String>>>,
) {
    let mut consecutive_errors: u32 = 0;
    let mut runner_state = RunnerState::new(npc_configs, shared_tilt);

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

        for (seat_index, history) in runner_state.session_histories.iter_mut().enumerate() {
            let player_id = NpcConfig::player_id(seat_index as u8);
            let npc_won = result.winning_player_ids.contains(&player_id);
            let pot_size: u32 = result.pot_summaries.iter().map(|p| p.amount).sum();
            let went_to_showdown = result.revealed_hands_by_player_id.contains_key(&player_id);

            // Determine net chips from pre-hand stack vs. post-hand stack.
            let pre_stack = runner_state
                .pre_hand_stacks
                .get(&player_id)
                .copied()
                .unwrap_or(0);
            let post_stack = result
                .final_stack_by_player_id
                .get(&player_id)
                .copied()
                .unwrap_or(0);
            let net_chips = post_stack as i32 - pre_stack as i32;

            // Determine bluff caught: NPC lost at showdown and had post-flop aggression.
            let npc_bluff_caught = if went_to_showdown && !npc_won {
                hand_log.actions_by(&player_id).iter().any(|r| {
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
            let had_postflop_bet = hand_log.actions_by(&player_id).iter().any(|r| {
                matches!(
                    r.street,
                    StreetPhase::Flop | StreetPhase::Turn | StreetPhase::River
                ) && matches!(r.action_type, ActionType::Bet | ActionType::Raise)
            });
            let npc_bluffed = had_postflop_bet && (!went_to_showdown || npc_bluff_caught);

            let opponent_ids: Vec<String> = result
                .final_stack_by_player_id
                .keys()
                .filter(|id| *id != &player_id)
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

            // Only record if the NPC was in this hand (had a pre-hand stack).
            if npc_configs.get(seat_index).is_some() {
                history.record_hand(summary);
            }
        }
    }

    runner_state.last_hand_result_count = result_count;

    // Publish updated tilt levels for the debug inspector.
    if let Ok(mut tilt_map) = runner_state.shared_tilt.lock() {
        tilt_map.clear();
        for (seat_index, history) in runner_state.session_histories.iter().enumerate() {
            let player_id = NpcConfig::player_id(seat_index as u8);
            if npc_configs.get(seat_index).is_some() {
                let tilt = TiltState::from_history(history);
                let level_str = match tilt.level {
                    super::tilt::TiltLevel::None => "none",
                    super::tilt::TiltLevel::Mild => "mild",
                    super::tilt::TiltLevel::Full => "full",
                };
                tilt_map.insert(player_id, level_str.to_string());
            }
        }
    }

    // Snapshot stacks for the next hand.
    runner_state.pre_hand_stacks = current_stacks(state);
}

#[allow(clippy::too_many_arguments)]
fn try_npc_action(
    host_server: &HostServer,
    state: &TournamentState,
    npc_configs: &[NpcConfig],
    stop: &AtomicBool,
    api_key_holder: &Arc<Mutex<Option<String>>>,
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

            // Build session context.
            let npc_seat_usize = npc_seat as usize;
            let (session_ctx, tilt_desc) =
                if let Some(history) = runner_state.session_histories.get(npc_seat_usize) {
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

            let result = choose_llm_action(&client_for(&key), profile, &snapshot);

            // Record the action into the hand log.
            if let Some(log) = &mut runner_state.hand_log {
                log.push(HandActionRecord {
                    hand_number: fresh_hand.hand_number,
                    street,
                    player_id: fresh_window.player_id.clone(),
                    action_type: result.0,
                    amount: result.1,
                    is_voluntary: true,
                });
            }

            result
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

    // Record the action into the hand log for rule-based path too (when LLM not used).
    if let Some(log) = &mut runner_state.hand_log {
        // Avoid double-logging: the LLM path already records when it executes.
        // The rule-based path always falls through here, so record it.
        if npc_config.and_then(|c| c.profile.as_ref()).is_none() {
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

    let _ = host_server.submit_action(
        &fresh_window.player_id,
        fresh_window.action_window_id.clone(),
        action_type,
        raise_to,
    );

    true
}

fn client_for(key: &str) -> crate::npc::llm_client::LlmClient {
    crate::npc::llm_client::LlmClient::new(key.to_string())
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
    /// Shared tilt level snapshot; written by the runner, readable by the host session.
    pub tilt_levels: Arc<Mutex<BTreeMap<String, String>>>,
}

impl NpcRunnerGuard {
    pub fn new(
        stop: Arc<AtomicBool>,
        handle: thread::JoinHandle<()>,
        tilt_levels: Arc<Mutex<BTreeMap<String, String>>>,
    ) -> Self {
        Self {
            stop,
            handle: Mutex::new(Some(handle)),
            tilt_levels,
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
mod tests {
    use std::collections::BTreeMap;

    use crate::domain::{
        BlindLevel, BlindSchedule, HandResult, PotSummary, SeatOccupancyState, SeatState,
        TournamentConfig, TournamentPhase, TournamentSeatState, TournamentState,
    };

    use super::super::NpcConfig;
    use super::*;

    fn minimal_state() -> TournamentState {
        TournamentState {
            table_id: "t1".into(),
            session_epoch: 1,
            phase: TournamentPhase::Running,
            config: TournamentConfig {
                tournament_name: "test".into(),
                table_name: None,
                max_players: 4,
                starting_stack: 1000,
                turn_timer_seconds: 30,
                blind_schedule: BlindSchedule {
                    levels: vec![BlindLevel {
                        level_index: 0,
                        label: "L1".into(),
                        small_blind: 10,
                        big_blind: 20,
                        ante: 0,
                        duration_seconds: 300,
                    }],
                },
            },
            blind_schedule: BlindSchedule {
                levels: vec![BlindLevel {
                    level_index: 0,
                    label: "L1".into(),
                    small_blind: 10,
                    big_blind: 20,
                    ante: 0,
                    duration_seconds: 300,
                }],
            },
            blind_level_index: 0,
            participants: BTreeMap::new(),
            seats: vec![
                SeatState {
                    seat_index: 0,
                    occupancy: SeatOccupancyState::Occupied,
                    tournament_state: TournamentSeatState::Active,
                    participant_id: Some("npc-seat-0".into()),
                    display_name: Some("NPC".into()),
                    chip_count: Some(1000),
                    is_ready: true,
                    marker: None,
                },
                SeatState {
                    seat_index: 1,
                    occupancy: SeatOccupancyState::Occupied,
                    tournament_state: TournamentSeatState::Active,
                    participant_id: Some("human-1".into()),
                    display_name: Some("Human".into()),
                    chip_count: Some(1000),
                    is_ready: true,
                    marker: None,
                },
            ],
            current_hand: None,
            hand_results: vec![],
            placements: vec![],
        }
    }

    fn hand_result(hand_number: u32, winner: &str, players: &[&str], pot: u32) -> HandResult {
        let mut stacks = BTreeMap::new();
        for p in players {
            stacks.insert(p.to_string(), if *p == winner { 1500u32 } else { 500u32 });
        }
        HandResult {
            hand_number,
            winning_player_ids: vec![winner.to_string()],
            pot_summaries: vec![PotSummary {
                pot_index: 0,
                amount: pot,
                eligible_player_ids: players.iter().map(|s| s.to_string()).collect(),
                winner_player_ids: vec![winner.to_string()],
                odd_chip_count: 0,
                odd_chip_awarded_to: None,
            }],
            board_cards: vec![],
            revealed_hands_by_player_id: BTreeMap::new(),
            eliminated_player_ids: vec![],
            final_stack_by_player_id: stacks,
        }
    }

    fn npc_configs() -> Vec<NpcConfig> {
        vec![NpcConfig {
            display_name: "NPC".into(),
            style: NpcStyle::Aggressive,
            profile: None,
        }]
    }

    #[test]
    fn session_history_increments_after_one_completed_hand() {
        let configs = npc_configs();
        let mut runner_state = RunnerState::new(&configs, Arc::new(Mutex::new(BTreeMap::new())));

        let mut state = minimal_state();
        state.hand_results.push(hand_result(
            1,
            "npc-seat-0",
            &["npc-seat-0", "human-1"],
            200,
        ));

        process_completed_hands(&state, &configs, &mut runner_state);

        assert_eq!(runner_state.session_histories[0].hands_played(), 1);
    }

    #[test]
    fn consecutive_losses_increments_across_multiple_hands() {
        let configs = npc_configs();
        let mut runner_state = RunnerState::new(&configs, Arc::new(Mutex::new(BTreeMap::new())));

        let mut state = minimal_state();
        // NPC loses hands 1, 2, 3.
        for i in 1..=3 {
            state
                .hand_results
                .push(hand_result(i, "human-1", &["npc-seat-0", "human-1"], 200));
        }

        process_completed_hands(&state, &configs, &mut runner_state);

        assert_eq!(runner_state.session_histories[0].consecutive_losses(), 3);
    }

    #[test]
    fn opponent_stats_has_entries_for_human_player_after_hand() {
        let configs = npc_configs();
        let mut runner_state = RunnerState::new(&configs, Arc::new(Mutex::new(BTreeMap::new())));

        let mut state = minimal_state();
        state
            .hand_results
            .push(hand_result(1, "human-1", &["npc-seat-0", "human-1"], 200));

        process_completed_hands(&state, &configs, &mut runner_state);

        // "human-1" should have been tracked.
        assert!(runner_state.opponent_stats.get("human-1").is_some());
        assert_eq!(
            runner_state
                .opponent_stats
                .get("human-1")
                .unwrap()
                .hands_observed,
            1
        );
    }
}
