
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
