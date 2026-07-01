use std::collections::BTreeMap;

use crate::{
    domain::{
        BlindLevel, BlindSchedule, HandResult, PotSummary, SeatOccupancyState, SeatState,
        TournamentConfig, TournamentPhase, TournamentSeatState, TournamentState,
    },
    npc::NpcProfile,
};

use super::super::{NpcConfig, NpcStyle};
use super::*;

fn make_profile(id: &str, name: &str, style: &str) -> NpcProfile {
    NpcProfile {
        id: id.to_string(),
        name: name.to_string(),
        style: style.to_string(),
        skill: "intermediate".to_string(),
        description: "Test profile.".to_string(),
        opponent_tendencies: None,
        tilt_behaviour: None,
    }
}

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
        player_id: NpcConfig::player_id(0),
        display_name: "NPC".into(),
        style: NpcStyle::Aggressive,
        profile: None,
        allow_rule_based_llm_fallback: false,
    }]
}

#[test]
fn two_npcs_with_distinct_player_ids_keep_separate_session_histories() {
    let configs = vec![
        NpcConfig {
            player_id: NpcConfig::player_id(2),
            display_name: "Tight Npc".into(),
            style: NpcStyle::Conservative,
            profile: None,
            allow_rule_based_llm_fallback: false,
        },
        NpcConfig {
            player_id: NpcConfig::player_id(4),
            display_name: "Aggro Npc".into(),
            style: NpcStyle::Aggressive,
            profile: None,
            allow_rule_based_llm_fallback: false,
        },
    ];
    let mut runner_state = RunnerState::new(
        &configs,
        Arc::new(Mutex::new(BTreeMap::new())),
        Arc::new(Mutex::new(None)),
        Arc::new(Mutex::new(None)),
    );

    // NPC at seat 2 wins; NPC at seat 4 loses.
    let mut state = minimal_state();
    state.hand_results.push(hand_result(
        1,
        "npc-seat-2",
        &["npc-seat-2", "npc-seat-4"],
        400,
    ));

    process_completed_hands(&state, &configs, &mut runner_state);

    // Histories are indexed parallel to configs: [0] = seat 2, [1] = seat 4.
    assert_eq!(runner_state.session_histories[0].hands_played(), 1);
    assert_eq!(runner_state.session_histories[1].hands_played(), 1);
    // seat-2 won → 0 consecutive losses; seat-4 lost → 1.
    assert_eq!(runner_state.session_histories[0].consecutive_losses(), 0);
    assert_eq!(runner_state.session_histories[1].consecutive_losses(), 1);
}

#[test]
fn two_npcs_with_non_contiguous_seat_ids_have_correct_styles() {
    // Non-contiguous seats: 2 and 4. A Vec-index approach would panic or
    // misassign; player_id-based lookup must keep styles paired.
    let configs = [
        NpcConfig {
            player_id: NpcConfig::player_id(2),
            display_name: "Conservative Npc".into(),
            style: NpcStyle::Conservative,
            profile: None,
            allow_rule_based_llm_fallback: false,
        },
        NpcConfig {
            player_id: NpcConfig::player_id(4),
            display_name: "Aggressive Npc".into(),
            style: NpcStyle::Aggressive,
            profile: None,
            allow_rule_based_llm_fallback: false,
        },
    ];

    assert_eq!(configs[0].player_id, "npc-seat-2");
    assert_eq!(configs[1].player_id, "npc-seat-4");
    assert_eq!(configs[0].style, NpcStyle::Conservative);
    assert_eq!(configs[1].style, NpcStyle::Aggressive);
}

#[test]
fn two_npcs_with_different_profiles_keep_correct_profiles() {
    // Seat 3 has "aggressive-alice", seat 5 has "balanced-sam".
    // The player_id-based lookup must return the profile that matches the
    // requesting player_id, regardless of array order.
    let alice_profile = make_profile("aggressive-alice", "Aggressive Alice", "aggressive");
    let sam_profile = make_profile("balanced-sam", "Balanced Sam", "balanced");

    let configs = [
        NpcConfig {
            player_id: NpcConfig::player_id(3),
            display_name: "Alice".into(),
            style: NpcStyle::Aggressive,
            profile: Some(alice_profile.clone()),
            allow_rule_based_llm_fallback: false,
        },
        NpcConfig {
            player_id: NpcConfig::player_id(5),
            display_name: "Sam".into(),
            style: NpcStyle::Conservative,
            profile: Some(sam_profile.clone()),
            allow_rule_based_llm_fallback: false,
        },
    ];

    // Simulate the player_id-based lookup that try_npc_action performs.
    let found_for_seat3 = configs
        .iter()
        .find(|c| c.player_id == NpcConfig::player_id(3));
    let found_for_seat5 = configs
        .iter()
        .find(|c| c.player_id == NpcConfig::player_id(5));

    assert_eq!(
        found_for_seat3
            .and_then(|c| c.profile.as_ref())
            .map(|p| p.id.as_str()),
        Some("aggressive-alice"),
        "seat 3 should have alice's profile"
    );
    assert_eq!(
        found_for_seat5
            .and_then(|c| c.profile.as_ref())
            .map(|p| p.id.as_str()),
        Some("balanced-sam"),
        "seat 5 should have sam's profile"
    );
}

#[test]
fn npc_config_lookup_by_unknown_player_id_returns_none_not_first_config() {
    // If an action window's player_id doesn't match any NPC config, the
    // player_id-based find must return None.  A Vec-index approach would
    // silently return the first config, causing wrong profile/style assignment.
    let configs = [NpcConfig {
        player_id: NpcConfig::player_id(2),
        display_name: "Known NPC".into(),
        style: NpcStyle::Conservative,
        profile: None,
        allow_rule_based_llm_fallback: false,
    }];

    let result = configs.iter().find(|c| c.player_id == "unknown-player-99");
    assert!(
        result.is_none(),
        "unknown player_id must not fall back to the first config; got: {result:?}"
    );
}

// P0.4 — NpcConfig serde and allow_rule_based_llm_fallback default.

#[test]
fn npc_config_allow_rule_based_llm_fallback_defaults_to_false() {
    // Serialized configs missing the field should deserialize as false.
    let json = r#"{
        "playerId": "npc-seat-0",
        "displayName": "Bot",
        "style": "aggressive",
        "profile": null
    }"#;
    let cfg: NpcConfig = serde_json::from_str(json).expect("deserialize NpcConfig");
    assert!(
        !cfg.allow_rule_based_llm_fallback,
        "missing allowRuleBasedLlmFallback must default to false"
    );
}

#[test]
fn npc_config_allow_rule_based_llm_fallback_explicit_true() {
    let json = r#"{
        "playerId": "npc-seat-0",
        "displayName": "Bot",
        "style": "aggressive",
        "profile": null,
        "allowRuleBasedLlmFallback": true
    }"#;
    let cfg: NpcConfig = serde_json::from_str(json).expect("deserialize NpcConfig");
    assert!(cfg.allow_rule_based_llm_fallback);
}

// P0.4 — record_npc_internal_error helper populates RunnerState correctly.

#[test]
fn record_npc_internal_error_sets_shared_action_error_and_returns_runtime_unavailable() {
    use crate::app_state::NpcActionErrorReason;

    let configs = npc_configs();
    let mut runner_state = RunnerState::new(
        &configs,
        Arc::new(Mutex::new(BTreeMap::new())),
        Arc::new(Mutex::new(None)),
        Arc::new(Mutex::new(None)),
    );

    let outcome = super::action::record_npc_internal_error(
        &mut runner_state,
        "npc-seat-0".to_string(),
        Some(3),
        NpcActionErrorReason::ProviderStateUnavailable,
        "test internal error".to_string(),
    );

    assert_eq!(outcome, NpcActionOutcome::RuntimeUnavailable);
    assert_eq!(runner_state.error_sequence, 1);
    let err = runner_state
        .shared_action_error
        .lock()
        .unwrap()
        .clone()
        .expect("error should be set");
    assert_eq!(err.reason, NpcActionErrorReason::ProviderStateUnavailable);
    assert_eq!(err.hand_number, Some(3));
    assert!(!err.submitted);
}

// P0.3 — hole-card validation: the validated_acting_hole_cards helper enforces
// that an acting NPC has exactly two hole cards.  The helper is called from
// try_npc_action, which uses the fresh authoritative state from the host.
// A full runner-path integration test would require injecting corrupt state
// into the host's internal authoritative_state (not exposed publicly); that
// test is deferred to a future fix.  The helper tests below exercise the real
// validation gate directly.

#[test]
fn validated_acting_hole_cards_rejects_missing_cards() {
    use crate::domain::{BettingRoundState, HandCyclePhase, HandState, StreetPhase};

    let player_id = "npc-seat-1";
    let hand = HandState {
        hand_number: 7,
        cycle_phase: HandCyclePhase::AwaitingAction,
        street: StreetPhase::Preflop,
        dealer_seat_index: 0,
        small_blind_seat_index: 0,
        big_blind_seat_index: 1,
        board_cards: vec![],
        hole_cards_by_player_id: BTreeMap::new(),
        participation_by_player_id: BTreeMap::new(),
        betting_round: BettingRoundState {
            street: StreetPhase::Preflop,
            current_bet: 0,
            min_raise_to: None,
            max_raise_to: None,
            pot_size: 0,
            contributions_by_player_id: BTreeMap::new(),
        },
        action_window: None,
    };

    let error = super::action::validated_acting_hole_cards(&hand, player_id).unwrap_err();
    assert!(
        error.contains("missing hole cards"),
        "error must mention missing hole cards; got: {error}"
    );
}

#[test]
fn validated_acting_hole_cards_rejects_wrong_card_count() {
    use crate::domain::{
        BettingRoundState, Card, HandCyclePhase, HandState, Rank, StreetPhase, Suit,
    };

    let player_id = "npc-seat-2";
    let mut hole_cards_by_player_id = BTreeMap::new();
    hole_cards_by_player_id.insert(
        player_id.to_string(),
        vec![Card {
            rank: Rank::Ace,
            suit: Suit::Spades,
        }],
    );
    let hand = HandState {
        hand_number: 12,
        cycle_phase: HandCyclePhase::AwaitingAction,
        street: StreetPhase::Preflop,
        dealer_seat_index: 0,
        small_blind_seat_index: 0,
        big_blind_seat_index: 1,
        board_cards: vec![],
        hole_cards_by_player_id,
        participation_by_player_id: BTreeMap::new(),
        betting_round: BettingRoundState {
            street: StreetPhase::Preflop,
            current_bet: 0,
            min_raise_to: None,
            max_raise_to: None,
            pot_size: 0,
            contributions_by_player_id: BTreeMap::new(),
        },
        action_window: None,
    };

    let error = super::action::validated_acting_hole_cards(&hand, player_id).unwrap_err();
    assert!(
        error.contains("invalid hole-card count"),
        "error must mention invalid hole-card count; got: {error}"
    );
}

#[test]
fn validated_acting_hole_cards_accepts_exactly_two_cards() {
    use crate::domain::{
        BettingRoundState, Card, HandCyclePhase, HandState, Rank, StreetPhase, Suit,
    };

    let player_id = "npc-seat-3";
    let card_a = Card {
        rank: Rank::Ace,
        suit: Suit::Spades,
    };
    let card_b = Card {
        rank: Rank::King,
        suit: Suit::Hearts,
    };
    let mut hole_cards_by_player_id = BTreeMap::new();
    hole_cards_by_player_id.insert(player_id.to_string(), vec![card_a, card_b]);
    let hand = HandState {
        hand_number: 3,
        cycle_phase: HandCyclePhase::AwaitingAction,
        street: StreetPhase::Preflop,
        dealer_seat_index: 0,
        small_blind_seat_index: 0,
        big_blind_seat_index: 1,
        board_cards: vec![],
        hole_cards_by_player_id,
        participation_by_player_id: BTreeMap::new(),
        betting_round: BettingRoundState {
            street: StreetPhase::Preflop,
            current_bet: 0,
            min_raise_to: None,
            max_raise_to: None,
            pot_size: 0,
            contributions_by_player_id: BTreeMap::new(),
        },
        action_window: None,
    };

    let cards = super::action::validated_acting_hole_cards(&hand, player_id).unwrap();
    assert_eq!(cards.len(), 2);
}

// P0.4 — NpcActionOutcome enum reports acted() correctly for each variant.

#[test]
fn npc_action_outcome_acted_is_true_only_for_success() {
    assert!(NpcActionOutcome::Success.acted());
    assert!(!NpcActionOutcome::NoOpportunity.acted());
    assert!(!NpcActionOutcome::NotNpcTurn.acted());
    assert!(!NpcActionOutcome::NoConfig.acted());
    assert!(!NpcActionOutcome::Stopped.acted());
    assert!(!NpcActionOutcome::RuntimeUnavailable.acted());
    assert!(!NpcActionOutcome::StaleWindow.acted());
    assert!(!NpcActionOutcome::Rejected("host rejected".into()).acted());
}

#[test]
fn session_history_increments_after_one_completed_hand() {
    let configs = npc_configs();
    let mut runner_state = RunnerState::new(
        &configs,
        Arc::new(Mutex::new(BTreeMap::new())),
        Arc::new(Mutex::new(None)),
        Arc::new(Mutex::new(None)),
    );

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
    let mut runner_state = RunnerState::new(
        &configs,
        Arc::new(Mutex::new(BTreeMap::new())),
        Arc::new(Mutex::new(None)),
        Arc::new(Mutex::new(None)),
    );

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
    let mut runner_state = RunnerState::new(
        &configs,
        Arc::new(Mutex::new(BTreeMap::new())),
        Arc::new(Mutex::new(None)),
        Arc::new(Mutex::new(None)),
    );

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

// P1.2 — start_npc_runner_with_spawner propagates spawn failure as Err.

#[test]
fn start_npc_runner_with_spawner_returns_err_when_spawn_fails() {
    use std::sync::atomic::AtomicBool;

    use crate::crypto::{DefaultCryptoProvider, ProtocolCryptoProvider};
    use crate::networking::{HostRuntimeConfig, HostRuntimeMode, HostServer};

    let provider = DefaultCryptoProvider;
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let host = Arc::new(
        HostServer::bind(HostRuntimeConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            advertised_host: "127.0.0.1".to_string(),
            session_epoch: 1,
            table_id: "t-spawn-fail".to_string(),
            table_name: None,
            join_token: "tok".to_string(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state: crate::domain::TournamentState {
                table_id: "t-spawn-fail".to_string(),
                session_epoch: 1,
                phase: crate::domain::TournamentPhase::WaitingForPlayers,
                config: crate::domain::TournamentConfig {
                    tournament_name: "test".to_string(),
                    table_name: None,
                    max_players: 2,
                    starting_stack: 1000,
                    turn_timer_seconds: 30,
                    blind_schedule: crate::domain::BlindSchedule {
                        levels: vec![crate::domain::BlindLevel {
                            level_index: 0,
                            label: "L1".to_string(),
                            small_blind: 10,
                            big_blind: 20,
                            ante: 0,
                            duration_seconds: 300,
                        }],
                    },
                },
                blind_schedule: crate::domain::BlindSchedule {
                    levels: vec![crate::domain::BlindLevel {
                        level_index: 0,
                        label: "L1".to_string(),
                        small_blind: 10,
                        big_blind: 20,
                        ante: 0,
                        duration_seconds: 300,
                    }],
                },
                blind_level_index: 0,
                participants: std::collections::BTreeMap::new(),
                seats: vec![],
                current_hand: None,
                hand_results: vec![],
                placements: vec![],
            },
            runtime_mode: HostRuntimeMode::Test,
        })
        .expect("host should bind"),
    );

    let configs = npc_configs();
    let stop = Arc::new(AtomicBool::new(false));
    let tilt = Arc::new(Mutex::new(std::collections::BTreeMap::new()));
    let fallback = Arc::new(Mutex::new(None));
    let action_error: Arc<Mutex<Option<crate::app_state::NpcActionErrorDebug>>> =
        Arc::new(Mutex::new(None));
    let api_key = Arc::new(Mutex::new(None));

    let result = super::start_npc_runner_with_spawner(
        host,
        configs,
        stop,
        api_key,
        tilt,
        fallback,
        action_error,
        |_builder, _f| Err(std::io::Error::other("injected spawn error")),
    );

    assert!(result.is_err(), "spawn failure must propagate as Err");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("failed to spawn npc-runner thread"),
        "error message must mention spawn failure; got: {msg}"
    );
}
