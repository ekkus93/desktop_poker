use std::{
    collections::BTreeMap,
    sync::{atomic::Ordering, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use serde_json::json;

use crate::{
    crypto::DefaultCryptoProvider,
    domain::{ActionType, TournamentPhase},
    protocol::ProtocolMessageType,
};

use super::support::*;

#[test]
fn host_start_tournament_emits_running_public_and_private_events_to_connected_clients() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-host-start", 61);
    let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
    let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
    let _ = expect_snapshot_event(&alice);
    let _ = expect_snapshot_event(&bob);

    host.claim_seat("player-alice", 0)
        .expect("alice seat claim should succeed");
    let _ = expect_snapshot_where(&alice, |snapshot| {
        snapshot
            .state
            .participants
            .get("player-alice")
            .and_then(|participant| participant.seat_index)
            == Some(0)
    });
    let _ = expect_snapshot_where(&bob, |snapshot| {
        snapshot
            .state
            .participants
            .get("player-alice")
            .and_then(|participant| participant.seat_index)
            == Some(0)
    });

    host.claim_seat("player-bob", 1)
        .expect("bob seat claim should succeed");
    let _ = expect_snapshot_where(&alice, |snapshot| {
        snapshot
            .state
            .participants
            .get("player-bob")
            .and_then(|participant| participant.seat_index)
            == Some(1)
    });
    let _ = expect_snapshot_where(&bob, |snapshot| {
        snapshot
            .state
            .participants
            .get("player-bob")
            .and_then(|participant| participant.seat_index)
            == Some(1)
    });

    host.set_ready_state("player-alice", true)
        .expect("alice ready should succeed");
    let _ = expect_snapshot_where(&alice, |snapshot| {
        snapshot
            .state
            .seats
            .iter()
            .find(|seat| seat.seat_index == 0)
            .is_some_and(|seat| seat.is_ready)
    });
    let _ = expect_snapshot_where(&bob, |snapshot| {
        snapshot
            .state
            .seats
            .iter()
            .find(|seat| seat.seat_index == 0)
            .is_some_and(|seat| seat.is_ready)
    });

    host.set_ready_state("player-bob", true)
        .expect("bob ready should succeed");
    let _ = expect_snapshot_where(&alice, |snapshot| {
        snapshot.state.phase == TournamentPhase::ReadyCheck
    });
    let _ = expect_snapshot_where(&bob, |snapshot| {
        snapshot.state.phase == TournamentPhase::ReadyCheck
    });

    host.start_tournament()
        .expect("host should start the tournament");
    for client in [&alice, &bob] {
        let _ = wait_for_public_event(client, ProtocolMessageType::TournamentStartedEvent);
        let hand_start_payload =
            wait_for_public_event(client, ProtocolMessageType::HandStartingEvent);
        assert_eq!(hand_start_payload.get("handNumber"), Some(&json!(1)));

        assert_eq!(wait_for_private_hole_cards(client).hole_cards.len(), 2);

        let action_window_payload =
            wait_for_public_event(client, ProtocolMessageType::ActionWindowOpenedEvent);
        assert_eq!(action_window_payload.get("handNumber"), Some(&json!(1)));
    }

    let host_state = host.authoritative_state().expect("host state");
    assert_eq!(host_state.phase, TournamentPhase::Running);
    assert!(host_state.current_hand.is_some());
}

/// Regression test for the duplicate-signing-key bug: every NPC used to be
/// registered with the shared placeholder key `"npc"`, so seating two or
/// more NPCs failed `validate_tournament_state` ("duplicate signing key
/// binding detected") the moment the tournament tried to start. NPC keys are
/// now derived from the unique player_id, so a multi-NPC table starts.
#[test]
fn two_npcs_can_be_seated_with_distinct_keys_and_start_a_tournament() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-two-npcs", 72);

    for seat in 0u8..2 {
        let npc_id = crate::npc::NpcConfig::player_id(seat);
        host.register_npc_participant(&npc_id, &format!("NPC {seat}"))
            .expect("npc participant registers");
        host.claim_seat(&npc_id, seat).expect("npc claims its seat");
        host.set_ready_state(&npc_id, true)
            .expect("npc marks ready");
    }

    host.start_tournament()
        .expect("a tournament with two NPCs should start");

    let state = host.authoritative_state().expect("running state");
    assert_eq!(state.phase, TournamentPhase::Running);

    // Each NPC must carry a distinct signing key for the binding check to pass.
    let distinct_keys: std::collections::BTreeSet<_> = state
        .participants
        .values()
        .map(|p| p.identity.signing_public_key.clone())
        .collect();
    assert_eq!(
        distinct_keys.len(),
        state.participants.len(),
        "each NPC must have a unique signing key binding"
    );
}

/// End-to-end "1 human + 1 NPC" tournament played through the real host
/// runtime: the human is a genuine TCP client (its own signing key, actions
/// sent over the wire exactly as in production), and the NPC seat is driven
/// by the production `start_npc_runner` loop. With `profile: None` the runner
/// uses the deterministic rule-based strategy (`rule_based_decision`) — no
/// LLM, no provider config. The test reads the *host's* authoritative state
/// to decide whose turn it is, so it does not depend on a client snapshot
/// catching up under load. This is the Rust home of the test the frontend
/// `NpcGame.integration.test.tsx` could not express (there is no
/// `TournamentController` on the frontend).
#[test]
fn npc_opponent_plays_a_full_hand_against_a_human_through_the_real_runtime() {
    use std::sync::atomic::AtomicBool;

    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-human-vs-npc", 71);

    // The human joins as a real client and claims seat 0.
    let human_id = "player-human";
    let human = connect_test_client(&provider, &host, human_id, "Human");
    let _ = expect_snapshot_event(&human);
    host.claim_seat(human_id, 0).expect("human claims seat 0");
    host.set_ready_state(human_id, true)
        .expect("human marks ready");

    // A single rule-based NPC takes seat 1, registered exactly the way
    // `app_state::add_npc_players` does: register -> claim seat -> ready.
    let npc_seat: u8 = 1;
    let npc_id = crate::npc::NpcConfig::player_id(npc_seat); // "npc-seat-1"
    host.register_npc_participant(&npc_id, "Rule NPC")
        .expect("npc participant registers");
    host.claim_seat(&npc_id, npc_seat)
        .expect("npc claims seat 1");
    host.set_ready_state(&npc_id, true)
        .expect("npc marks ready");

    host.start_tournament()
        .expect("tournament starts with 2 ready players");
    wait_for_client_command_connection(&human);

    let host = Arc::new(host);
    let starting_stack = host
        .authoritative_state()
        .expect("running state")
        .config
        .starting_stack;

    // Drive the NPC with the production runner. `profile: None` forces the
    // rule-based path, so no provider config is ever consulted.
    let npc_configs = vec![crate::npc::NpcConfig {
        display_name: "Rule NPC".to_string(),
        style: crate::npc::NpcStyle::Conservative,
        profile: None,
    }];
    let stop = Arc::new(AtomicBool::new(false));
    let api_key_holder = Arc::new(Mutex::new(None));
    let tilt = Arc::new(Mutex::new(BTreeMap::new()));
    let runner = crate::npc::runner::start_npc_runner(
        Arc::clone(&host),
        npc_configs,
        Arc::clone(&stop),
        api_key_holder,
        tilt,
    );

    // Play the hand: the human always checks/calls (never folds) so the hand
    // reaches a natural conclusion regardless of how the NPC plays, while the
    // runner acts for the NPC. We require positive evidence that the runner
    // actually moved the NPC's window — we never submit for the NPC seat.
    let mut npc_acted = false;
    let mut last_human_window: Option<String> = None;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let state = host.authoritative_state().expect("host state during hand");
        if !state.hand_results.is_empty() {
            break;
        }

        if let Some(window) = state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.clone())
        {
            if window.player_id == human_id {
                // Submit once per window id; the host processes the TCP
                // request asynchronously, so the same window may still be
                // open on the next poll.
                if last_human_window.as_deref() != Some(window.action_window_id.as_str()) {
                    let action = if window.legal_actions.contains(&ActionType::Check) {
                        ActionType::Check
                    } else {
                        ActionType::Call
                    };
                    let _ = human.submit_action(
                        window.action_window_id.clone(),
                        window.seat_index,
                        action,
                        None,
                    );
                    last_human_window = Some(window.action_window_id.clone());
                }
            } else if window.player_id == npc_id {
                // Wait for the runner to act for the NPC: the open window must
                // advance (or the hand must end) without our intervention.
                let npc_window_id = window.action_window_id.clone();
                let npc_deadline = Instant::now() + Duration::from_secs(10);
                loop {
                    let now_state = host
                        .authoritative_state()
                        .expect("host state while NPC acts");
                    let current_window_id = now_state
                        .current_hand
                        .as_ref()
                        .and_then(|h| h.action_window.as_ref())
                        .map(|w| w.action_window_id.clone());
                    if current_window_id.as_ref() != Some(&npc_window_id)
                        || !now_state.hand_results.is_empty()
                    {
                        npc_acted = true;
                        break;
                    }
                    assert!(
                        Instant::now() < npc_deadline,
                        "the NPC runner should submit an action for its open window"
                    );
                    thread::sleep(Duration::from_millis(20));
                }
            }
        }

        assert!(
            Instant::now() < deadline,
            "a full human-vs-NPC hand should complete within the deadline"
        );
        thread::sleep(Duration::from_millis(20));
    }

    stop.store(true, Ordering::SeqCst);
    let _ = runner.join();

    let final_state = host.authoritative_state().expect("host state after hand");
    assert!(
        npc_acted,
        "the NPC must take at least one runner-driven action"
    );

    let result = final_state
        .hand_results
        .first()
        .expect("at least one hand should reach a result");
    // Settlement is recorded against the completed hand, so this is immune to
    // a follow-on hand posting new blinds. Blinds/antes only move chips
    // between seats, so the post-hand stacks must still sum to the combined
    // starting stacks.
    let total_chips: u32 = result.final_stack_by_player_id.values().sum();
    assert_eq!(
        total_chips,
        starting_stack * 2,
        "chips must be conserved across the human and NPC after the hand"
    );
}

#[test]
fn client_action_submission_syncs_running_state_across_the_live_runtime() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-action-sync", 62);
    let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
    let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
    let _ = expect_snapshot_event(&alice);
    let _ = expect_snapshot_event(&bob);

    host.claim_seat("player-alice", 0)
        .expect("alice seat claim should succeed");
    let _ = expect_snapshot_where(&alice, |snapshot| {
        snapshot
            .state
            .participants
            .get("player-alice")
            .and_then(|participant| participant.seat_index)
            == Some(0)
    });
    let _ = expect_snapshot_where(&bob, |snapshot| {
        snapshot
            .state
            .participants
            .get("player-alice")
            .and_then(|participant| participant.seat_index)
            == Some(0)
    });

    host.claim_seat("player-bob", 1)
        .expect("bob seat claim should succeed");
    let _ = expect_snapshot_where(&alice, |snapshot| {
        snapshot
            .state
            .participants
            .get("player-bob")
            .and_then(|participant| participant.seat_index)
            == Some(1)
    });
    let _ = expect_snapshot_where(&bob, |snapshot| {
        snapshot
            .state
            .participants
            .get("player-bob")
            .and_then(|participant| participant.seat_index)
            == Some(1)
    });

    host.set_ready_state("player-alice", true)
        .expect("alice ready should succeed");
    let _ = expect_snapshot_where(&alice, |snapshot| {
        snapshot
            .state
            .seats
            .iter()
            .find(|seat| seat.seat_index == 0)
            .is_some_and(|seat| seat.is_ready)
    });
    let _ = expect_snapshot_where(&bob, |snapshot| {
        snapshot
            .state
            .seats
            .iter()
            .find(|seat| seat.seat_index == 0)
            .is_some_and(|seat| seat.is_ready)
    });

    host.set_ready_state("player-bob", true)
        .expect("bob ready should succeed");
    let _ = expect_snapshot_where(&alice, |snapshot| {
        snapshot.state.phase == TournamentPhase::ReadyCheck
    });
    let _ = expect_snapshot_where(&bob, |snapshot| {
        snapshot.state.phase == TournamentPhase::ReadyCheck
    });

    host.start_tournament()
        .expect("host should start the tournament");
    let deadline = Instant::now() + Duration::from_secs(2);
    let action_window = loop {
        if let Some(window) = host
            .authoritative_state()
            .expect("host state after start")
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.clone())
        {
            break window;
        }

        assert!(
            Instant::now() < deadline,
            "running hand should expose an action window"
        );
        thread::sleep(Duration::from_millis(20));
    };
    let acting_client = if action_window.player_id == "player-alice" {
        &alice
    } else {
        &bob
    };
    wait_for_client_command_connection(acting_client);
    let action_type = if action_window
        .legal_actions
        .contains(&crate::domain::ActionType::Check)
    {
        crate::domain::ActionType::Check
    } else {
        crate::domain::ActionType::Call
    };

    acting_client
        .submit_action(
            action_window.action_window_id.clone(),
            action_window.seat_index,
            action_type,
            None,
        )
        .expect("client action should succeed");

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let next_action_window_id = host
            .authoritative_state()
            .expect("host state")
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .map(|window| window.action_window_id.clone());
        if next_action_window_id != Some(action_window.action_window_id.clone()) {
            break;
        }

        assert!(
            Instant::now() < deadline,
            "host should advance beyond the submitted action window"
        );
        thread::sleep(Duration::from_millis(20));
    }
}
