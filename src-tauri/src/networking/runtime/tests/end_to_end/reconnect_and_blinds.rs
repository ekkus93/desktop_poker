use std::{
    thread,
    time::{Duration, Instant},
};

use crate::{
    domain::{ActionType, BlindLevel, BlindSchedule},
    networking::ClientRuntimeEvent,
    protocol::ProtocolMessageType,
};

use super::super::support::*;

// ── Section 1: Reconnect mid-hand ───────────────────────────────────────────

/// The idle player (not currently on the clock) drops and reconnects mid-hand.
/// The reconnect snapshot must carry the in-progress hand state, and the
/// reconnected client must still receive subsequent action broadcasts.
#[test]
fn d_client_reconnects_mid_hand_and_resumes_acting() {
    let provider = crate::crypto::DefaultCryptoProvider;
    let mut state = sample_tournament_state("table-mid-hand-reconnect", 90);
    state.config.starting_stack = 500;
    let levels = vec![BlindLevel {
        level_index: 1,
        label: "L1".to_string(),
        small_blind: 50,
        big_blind: 100,
        ante: 0,
        duration_seconds: 600,
    }];
    state.config.blind_schedule = BlindSchedule {
        levels: levels.clone(),
    };
    state.blind_schedule = BlindSchedule { levels };
    let host = bind_test_host_with_state(&provider, "table-mid-hand-reconnect", 90, state);

    let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
    let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
    let _ = expect_snapshot_event(&alice);
    let _ = expect_snapshot_event(&bob);

    host.claim_seat("player-alice", 0).expect("alice seat");
    host.claim_seat("player-bob", 1).expect("bob seat");
    host.set_ready_state("player-alice", true)
        .expect("alice ready");
    host.set_ready_state("player-bob", true).expect("bob ready");
    host.start_tournament().expect("start");

    // Wait for the first action window.
    let deadline = Instant::now() + Duration::from_secs(5);
    let window = loop {
        if let Some(w) = host
            .authoritative_state()
            .expect("state")
            .current_hand
            .as_ref()
            .and_then(|h| h.action_window.clone())
        {
            break w;
        }
        assert!(Instant::now() < deadline, "action window should open");
        thread::sleep(Duration::from_millis(20));
    };

    let (actor, idle, idle_id) = if window.player_id == "player-alice" {
        (&alice, &bob, "player-bob")
    } else {
        (&bob, &alice, "player-alice")
    };
    wait_for_client_command_connection(actor);
    wait_for_client_command_connection(idle);

    // Forcibly disconnect the idle player.
    disconnect_client(&host, idle_id);

    // Drain the idle client's buffered events until Reconnecting is observed,
    // confirming the TCP drop was detected. Only then wait for the reconnect
    // Snapshot — this avoids confusing a pre-buffered mid-session snapshot with
    // the actual reconnect snapshot.
    let reconnect_deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match idle.next_event(Duration::from_millis(200)) {
            Ok(ClientRuntimeEvent::Reconnecting { .. }) => break,
            Ok(_) => {}
            Err(_) => {}
        }
        assert!(
            Instant::now() < reconnect_deadline,
            "idle should emit Reconnecting"
        );
    }

    let snapshot = expect_snapshot_where(idle, |snap| snap.state.current_hand.is_some());
    assert!(
        snapshot.state.current_hand.is_some(),
        "reconnect snapshot must carry in-progress hand"
    );

    // Actor advances the hand; the reconnected idle client must receive the event.
    let action = if window.legal_actions.contains(&ActionType::Check) {
        ActionType::Check
    } else {
        ActionType::Call
    };
    actor
        .submit_action(
            window.action_window_id.clone(),
            window.seat_index,
            action,
            None,
        )
        .expect("actor action should succeed");
    let _ = wait_for_public_event(idle, ProtocolMessageType::PlayerActionCommittedEvent);
}

/// The acting player (currently on the clock) drops. With a 1-second timer the
/// host auto-commits their action. The reconnected client must see the
/// committed-action event on its event stream.
#[test]
fn e_reconnected_client_receives_auto_committed_event_after_reconnect() {
    let provider = crate::crypto::DefaultCryptoProvider;
    let mut state = sample_tournament_state("table-actor-reconnect", 91);
    state.config.turn_timer_seconds = 1;
    let host = bind_test_host_with_state(&provider, "table-actor-reconnect", 91, state);

    let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
    let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
    let _ = expect_snapshot_event(&alice);
    let _ = expect_snapshot_event(&bob);

    host.claim_seat("player-alice", 0).expect("alice seat");
    host.claim_seat("player-bob", 1).expect("bob seat");
    host.set_ready_state("player-alice", true)
        .expect("alice ready");
    host.set_ready_state("player-bob", true).expect("bob ready");
    host.start_tournament().expect("start");

    let deadline = Instant::now() + Duration::from_secs(5);
    let window = loop {
        if let Some(w) = host
            .authoritative_state()
            .expect("state")
            .current_hand
            .as_ref()
            .and_then(|h| h.action_window.clone())
        {
            break w;
        }
        assert!(Instant::now() < deadline, "action window should open");
        thread::sleep(Duration::from_millis(20));
    };

    // Disconnect the acting player without submitting.
    let (actor, actor_id) = if window.player_id == "player-alice" {
        (&alice, "player-alice")
    } else {
        (&bob, "player-bob")
    };
    wait_for_client_command_connection(actor);
    disconnect_client(&host, actor_id);

    // The host times the player out (~1s) and auto-commits. The ClientRuntime
    // reconnects quickly, then receives the broadcast committed-action event.
    let _ = wait_for_public_event(actor, ProtocolMessageType::PlayerActionCommittedEvent);

    // Poll until the host confirms the hand has advanced past the original window.
    let advance_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let advanced = host
            .authoritative_state()
            .expect("state")
            .current_hand
            .as_ref()
            .and_then(|h| h.action_window.as_ref())
            .map(|w| w.action_window_id != window.action_window_id)
            .unwrap_or(true);
        if advanced {
            break;
        }
        assert!(
            Instant::now() < advance_deadline,
            "hand must advance after timeout"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

// ── Section 2: Multi-hand blind escalation ──────────────────────────────────

/// After the first hand completes and the between-hands delay elapses, the
/// host's tick loop must increment the blind level and start hand 2 at the
/// higher blind. The new action window reflects the updated current_bet.
#[test]
fn f_blind_level_increments_are_reflected_in_the_second_hand() {
    let provider = crate::crypto::DefaultCryptoProvider;
    let mut state = sample_tournament_state("table-blind-escalation", 92);
    state.config.starting_stack = 500;
    let levels = vec![
        BlindLevel {
            level_index: 0,
            label: "L0".to_string(),
            small_blind: 10,
            big_blind: 20,
            ante: 0,
            duration_seconds: 1, // expires quickly
        },
        BlindLevel {
            level_index: 1,
            label: "L1".to_string(),
            small_blind: 50,
            big_blind: 100,
            ante: 0,
            duration_seconds: 600,
        },
    ];
    state.config.blind_schedule = BlindSchedule {
        levels: levels.clone(),
    };
    state.blind_schedule = BlindSchedule { levels };
    let host = bind_test_host_with_state(&provider, "table-blind-escalation", 92, state);

    let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
    let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
    let _ = expect_snapshot_event(&alice);
    let _ = expect_snapshot_event(&bob);

    host.claim_seat("player-alice", 0).expect("alice seat");
    host.claim_seat("player-bob", 1).expect("bob seat");
    host.set_ready_state("player-alice", true)
        .expect("alice ready");
    host.set_ready_state("player-bob", true).expect("bob ready");
    host.start_tournament().expect("start");

    for (_, client) in [("player-alice", &alice), ("player-bob", &bob)] {
        wait_for_client_command_connection(client);
    }

    // Drive hand 1 to completion using check/call (no all-in so both survive).
    let mut last_submitted: Option<String> = None;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let s = host.authoritative_state().expect("state");
        if !s.hand_results.is_empty() {
            break;
        }
        if let Some(window) = s
            .current_hand
            .as_ref()
            .and_then(|h| h.action_window.clone())
        {
            if last_submitted.as_deref() != Some(window.action_window_id.as_str()) {
                let client = if window.player_id == "player-alice" {
                    &alice
                } else {
                    &bob
                };
                let action = if window.legal_actions.contains(&ActionType::Check) {
                    ActionType::Check
                } else {
                    ActionType::Call
                };
                let _ = client.submit_action(
                    window.action_window_id.clone(),
                    window.seat_index,
                    action,
                    None,
                );
                last_submitted = Some(window.action_window_id.clone());
            }
        }
        assert!(Instant::now() < deadline, "hand 1 should complete");
        thread::sleep(Duration::from_millis(20));
    }

    // Wait for hand 2 to start. By this time ≥1s of wall clock has elapsed so
    // the blind level should have incremented during between-hands advance_time.
    let hand2_deadline = Instant::now() + Duration::from_secs(10);
    let state2 = wait_for_hand_number(&host, 2, hand2_deadline);
    assert_eq!(
        state2.blind_level_index, 1,
        "blind level must be 1 by hand 2"
    );

    // The action window in hand 2 must reflect the new big blind (current_bet=100).
    let window2_deadline = Instant::now() + Duration::from_secs(5);
    let window2 = loop {
        if let Some(w) = host
            .authoritative_state()
            .expect("state")
            .current_hand
            .as_ref()
            .and_then(|h| h.action_window.clone())
        {
            break w;
        }
        assert!(
            Instant::now() < window2_deadline,
            "hand 2 action window should open"
        );
        thread::sleep(Duration::from_millis(20));
    };
    assert!(
        window2.call_amount >= 50,
        "hand-2 action window should reflect the new big blind (call_amount={})",
        window2.call_amount,
    );
}
