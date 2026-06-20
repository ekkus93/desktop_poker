use crate::domain::{ActionType, HandResult, StreetPhase, TournamentState};
use crate::npc::{
    hand_log::HandLog,
    session_history::HandSummary,
    tilt::{TiltLevel, TiltState},
    NpcConfig,
};

use super::{build_display_names, current_stacks, RunnerState};

/// Detect newly completed hands and update session histories and opponent stats.
pub(crate) fn process_completed_hands(
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
                TiltLevel::None => "none",
                TiltLevel::Mild => "mild",
                TiltLevel::Full => "full",
            };
            tilt_map.insert(npc_config.player_id.clone(), level_str.to_string());
        }
    }

    // Snapshot stacks for the next hand.
    runner_state.pre_hand_stacks = current_stacks(state);
}
