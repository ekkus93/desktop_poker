use std::{
    thread,
    time::{Duration, Instant},
};

use crate::{
    crypto::{DefaultCryptoProvider, ProtocolCryptoProvider},
    domain::{ActionType, BlindLevel, BlindSchedule, TournamentPhase},
    networking::ClientRuntimeEvent,
    protocol::{ProtocolMessageType, ReplayProtector},
};

use super::support::*;

/// A whole Sit 'n Go played end-to-end through the live host runtime until one
/// player holds every chip, with the completion event reaching a connected
/// client over TCP.
///
/// Tiny stacks and large blinds make the all-in line resolve the tournament
/// quickly and deterministically *in outcome* (everyone commits, players bust,
/// one winner remains). Deterministic multi-hand / blind-increase mechanics are
/// covered at the controller level in `tournament/tests`.
#[test]
fn a_full_sit_n_go_runs_to_a_single_winner_and_broadcasts_completion() {
    let provider = DefaultCryptoProvider;

    let mut state = sample_tournament_state("table-full-sng", 81);
    state.config.starting_stack = 100;
    let levels = vec![BlindLevel {
        level_index: 1,
        label: "Level 1".to_string(),
        small_blind: 30,
        big_blind: 60,
        ante: 0,
        duration_seconds: 600,
    }];
    state.config.blind_schedule = BlindSchedule {
        levels: levels.clone(),
    };
    state.blind_schedule = BlindSchedule { levels };
    let host = bind_test_host_with_state(&provider, "table-full-sng", 81, state);

    let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
    let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
    let carol = connect_test_client(&provider, &host, "player-carol", "Carol");
    let _ = expect_snapshot_event(&alice);
    let _ = expect_snapshot_event(&bob);
    let _ = expect_snapshot_event(&carol);

    for (player_id, seat) in [
        ("player-alice", 0u8),
        ("player-bob", 1),
        ("player-carol", 2),
    ] {
        host.claim_seat(player_id, seat).expect("seat claim");
        host.set_ready_state(player_id, true).expect("ready");
    }
    host.start_tournament().expect("tournament should start");

    let seated: [(&str, &_); 3] = [
        ("player-alice", &alice),
        ("player-bob", &bob),
        ("player-carol", &carol),
    ];
    for (_, client) in seated {
        wait_for_client_command_connection(client);
    }

    // Drive every open action through whoever is on the clock, preferring all-in
    // so chips concentrate fast. Never fold, so the hand always reaches showdown.
    // Submit each window exactly once: the host processes over TCP asynchronously,
    // so re-submitting a still-open window would be a stale (rejected) action.
    let mut last_submitted: Option<String> = None;
    let deadline = Instant::now() + Duration::from_secs(40);
    loop {
        let state = host.authoritative_state().expect("host state");
        if state.phase == TournamentPhase::Complete {
            break;
        }
        if let Some(window) = state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.clone())
        {
            if last_submitted.as_deref() != Some(window.action_window_id.as_str()) {
                if let Some((_, client)) = seated.iter().find(|(id, _)| *id == window.player_id) {
                    let action = if window.legal_actions.contains(&ActionType::AllIn) {
                        ActionType::AllIn
                    } else if window.legal_actions.contains(&ActionType::Call) {
                        ActionType::Call
                    } else {
                        ActionType::Check
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
        }
        assert!(
            Instant::now() < deadline,
            "the tournament should reach a single winner"
        );
        thread::sleep(Duration::from_millis(20));
    }

    let final_state = host.authoritative_state().expect("final state");
    assert_eq!(final_state.phase, TournamentPhase::Complete);
    let survivors = final_state
        .seats
        .iter()
        .filter(|seat| seat.chip_count.unwrap_or(0) > 0)
        .count();
    assert_eq!(survivors, 1, "exactly one player should hold all the chips");
    assert!(
        !final_state.placements.is_empty(),
        "a completed tournament should record placements"
    );

    // The completion event was broadcast to a connected client over the wire.
    let _ = wait_for_public_event(&alice, ProtocolMessageType::TournamentCompleteEvent);
}

/// The host is the sole authority: it rejects an out-of-turn action and a stale
/// action-window id without mutating authoritative state, and a legitimate
/// action from the real actor still advances the hand afterward.
#[test]
fn b_the_host_rejects_out_of_turn_and_stale_actions_without_corrupting_state() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-adversarial", 82);
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

    let deadline = Instant::now() + Duration::from_secs(2);
    let window = loop {
        if let Some(window) = host
            .authoritative_state()
            .expect("state")
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.clone())
        {
            break window;
        }
        assert!(
            Instant::now() < deadline,
            "a hand should open an action window"
        );
        thread::sleep(Duration::from_millis(20));
    };

    let (actor, offender) = if window.player_id == "player-alice" {
        (&alice, &bob)
    } else {
        (&bob, &alice)
    };
    wait_for_client_command_connection(actor);
    wait_for_client_command_connection(offender);

    // (1) Out-of-turn: the non-acting client submits for the open window. The host
    // attributes the action to the sender's own id (it never trusts a claimed
    // player), so the controller rejects it — it is not that player's turn.
    let _ = offender.submit_action(
        window.action_window_id.clone(),
        window.seat_index,
        ActionType::Call,
        None,
    );
    // (2) Stale/unknown window: the real actor submits with a bogus window id.
    let _ = actor.submit_action(
        "not-a-real-window".to_string(),
        window.seat_index,
        ActionType::Call,
        None,
    );

    // Give the host time to process and reject both. The open window must be
    // unchanged — neither bad submission advanced authoritative state.
    thread::sleep(Duration::from_millis(400));
    let after = host
        .authoritative_state()
        .expect("state")
        .current_hand
        .and_then(|hand| hand.action_window)
        .expect("the action window must still be open");
    assert_eq!(
        after.action_window_id, window.action_window_id,
        "rejected actions must not advance the action window"
    );
    assert_eq!(
        after.player_id, window.player_id,
        "the same player must still be on the clock"
    );

    // A legitimate in-turn action is still accepted and advances the hand,
    // proving the host's authority survived the adversarial input.
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
        .expect("a valid in-turn action should be accepted");

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let current = host
            .authoritative_state()
            .expect("state")
            .current_hand
            .as_ref()
            .and_then(|hand| {
                hand.action_window
                    .as_ref()
                    .map(|w| w.action_window_id.clone())
            });
        if current.as_deref() != Some(window.action_window_id.as_str()) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "a valid action should advance the window"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

/// With a short turn timer and no client action at all, the host's tick loop must
/// time the idle player out on its own authority and broadcast the committed
/// action to connected clients.
#[test]
fn c_the_host_times_out_an_idle_player_and_broadcasts_the_committed_action() {
    let provider = DefaultCryptoProvider;

    let mut state = sample_tournament_state("table-timeout", 83);
    state.config.turn_timer_seconds = 1;
    let host = bind_test_host_with_state(&provider, "table-timeout", 83, state);

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

    let deadline = Instant::now() + Duration::from_secs(3);
    let window = loop {
        if let Some(window) = host
            .authoritative_state()
            .expect("state")
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.clone())
        {
            break window;
        }
        assert!(
            Instant::now() < deadline,
            "a hand should open an action window"
        );
        thread::sleep(Duration::from_millis(20));
    };

    // Submit nothing. The host's tick loop must time the idle player out (~1s) and
    // advance the hand without any client action.
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let state = host.authoritative_state().expect("state");
        let advanced = state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .map(|w| w.action_window_id != window.action_window_id)
            // The window being gone (hand ended) also counts as advanced.
            .unwrap_or(true);
        if advanced || !state.hand_results.is_empty() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the host should time the idle player out with no client action"
        );
        thread::sleep(Duration::from_millis(50));
    }

    // No client ever acted, so the first committed-action event a client sees is
    // the host's timeout commit — proving the timeout was broadcast over TCP.
    let _ = wait_for_public_event(&alice, ProtocolMessageType::PlayerActionCommittedEvent);
}

// ── Section 1: Reconnect mid-hand ───────────────────────────────────────────

/// The idle player (not currently on the clock) drops and reconnects mid-hand.
/// The reconnect snapshot must carry the in-progress hand state, and the
/// reconnected client must still receive subsequent action broadcasts.
#[test]
fn d_client_reconnects_mid_hand_and_resumes_acting() {
    let provider = DefaultCryptoProvider;
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
    let provider = DefaultCryptoProvider;
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
    let provider = DefaultCryptoProvider;
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

// ── Section 3: Private hole-card delivery integrity ──────────────────────────

/// Every seated player must receive exactly two distinct private hole cards over
/// the encrypted TCP path. No two players share a card (the deck never repeats).
#[test]
fn g_each_player_receives_exactly_two_private_hole_cards_over_tcp() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-hole-cards", 93);

    let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
    let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
    let carol = connect_test_client(&provider, &host, "player-carol", "Carol");
    let _ = expect_snapshot_event(&alice);
    let _ = expect_snapshot_event(&bob);
    let _ = expect_snapshot_event(&carol);

    for (player_id, seat) in [
        ("player-alice", 0u8),
        ("player-bob", 1),
        ("player-carol", 2),
    ] {
        host.claim_seat(player_id, seat).expect("seat");
        host.set_ready_state(player_id, true).expect("ready");
    }
    host.start_tournament().expect("start");

    let alice_cards = wait_for_private_hole_cards(&alice);
    let bob_cards = wait_for_private_hole_cards(&bob);
    let carol_cards = wait_for_private_hole_cards(&carol);

    for (who, cards) in [
        ("alice", &alice_cards),
        ("bob", &bob_cards),
        ("carol", &carol_cards),
    ] {
        assert_eq!(
            cards.hole_cards.len(),
            2,
            "{who} must receive exactly 2 hole cards"
        );
    }

    // Build the combined set of (rank, suit) pairs — no card may appear twice.
    let mut all_cards: Vec<_> = alice_cards
        .hole_cards
        .iter()
        .chain(&bob_cards.hole_cards)
        .chain(&carol_cards.hole_cards)
        .map(|c| (c.rank, c.suit))
        .collect();
    all_cards.sort_unstable_by_key(|&(r, s)| (r as u8, s as u8));
    let before_dedup = all_cards.len();
    all_cards.dedup();
    assert_eq!(
        before_dedup,
        all_cards.len(),
        "no card should be dealt twice"
    );
}

/// Hole cards are private: the initial burst of public events after dealing must
/// not contain a non-null `hole_cards` field at the top level.
#[test]
fn h_hole_cards_are_not_visible_in_pre_showdown_public_events() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-no-leak", 94);

    let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
    let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
    let _ = expect_snapshot_event(&alice);
    let _ = expect_snapshot_event(&bob);

    host.claim_seat("player-alice", 0).expect("seat");
    host.claim_seat("player-bob", 1).expect("seat");
    host.set_ready_state("player-alice", true).expect("ready");
    host.set_ready_state("player-bob", true).expect("ready");
    host.start_tournament().expect("start");

    // Drain up to 8 non-showdown public events and assert none contain hole cards.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut checked = 0;
    while checked < 8 && Instant::now() < deadline {
        match alice.next_event(Duration::from_millis(100)) {
            Ok(ClientRuntimeEvent::PublicEvent {
                message_type,
                payload,
                ..
            }) => {
                // ShowdownHandRevealedEvent legitimately contains hole_cards. Skip it.
                if message_type == ProtocolMessageType::ShowdownHandRevealedEvent {
                    break;
                }
                let hole_cards_value = payload
                    .get("holeCards")
                    .or_else(|| payload.get("hole_cards"));
                assert!(
                    hole_cards_value.map(|v| v.is_null()).unwrap_or(true),
                    "public event {message_type:?} must not expose non-null hole cards"
                );
                checked += 1;
            }
            Ok(ClientRuntimeEvent::PrivateHoleCards(_)) => {}
            Ok(ClientRuntimeEvent::Snapshot(_)) => {}
            Ok(ClientRuntimeEvent::Reconnecting { .. }) => {}
            Ok(_) => {}
            Err(_) => {}
        }
    }
    assert!(
        checked > 0,
        "should have observed at least one public event"
    );
}

// ── Section 4: Observer-only enforcement after elimination ───────────────────

/// An eliminated player's action submission must be rejected by the host: the
/// action window must remain open, and the real actor can still advance the hand.
#[test]
fn i_eliminated_player_action_is_rejected_by_host() {
    let provider = DefaultCryptoProvider;
    let mut state = sample_tournament_state("table-elim-reject", 95);
    state.config.starting_stack = 100;
    let levels = vec![BlindLevel {
        level_index: 1,
        label: "L1".to_string(),
        small_blind: 30,
        big_blind: 60,
        ante: 0,
        duration_seconds: 600,
    }];
    state.config.blind_schedule = BlindSchedule {
        levels: levels.clone(),
    };
    state.blind_schedule = BlindSchedule { levels };
    let host = bind_test_host_with_state(&provider, "table-elim-reject", 95, state);

    let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
    let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
    let carol = connect_test_client(&provider, &host, "player-carol", "Carol");
    let _ = expect_snapshot_event(&alice);
    let _ = expect_snapshot_event(&bob);
    let _ = expect_snapshot_event(&carol);

    for (id, seat) in [
        ("player-alice", 0u8),
        ("player-bob", 1),
        ("player-carol", 2),
    ] {
        host.claim_seat(id, seat).expect("seat");
        host.set_ready_state(id, true).expect("ready");
    }
    host.start_tournament().expect("start");

    let seated: [(&str, &_); 3] = [
        ("player-alice", &alice),
        ("player-bob", &bob),
        ("player-carol", &carol),
    ];

    // Drive to the first elimination using the host's direct submit API so the
    // setup phase is not subject to TCP round-trip latency under parallel test
    // load. The critical assertion (eliminated player's TCP action is rejected)
    // still exercises the real TCP path below.
    let mut last_submitted: Option<String> = None;
    let mut folded_hand: Option<u32> = None;
    let deadline = Instant::now() + Duration::from_secs(20);

    let eliminated_player_id = loop {
        let s = host.authoritative_state().expect("state");
        if !s.placements.is_empty() && s.phase == TournamentPhase::Running {
            break s.placements.last().expect("placement").player_id.clone();
        }
        if let Some(window) = s
            .current_hand
            .as_ref()
            .and_then(|h| h.action_window.clone())
        {
            if last_submitted.as_deref() != Some(window.action_window_id.as_str()) {
                let hand_num = s.current_hand.as_ref().map(|h| h.hand_number).unwrap_or(0);
                let should_fold = folded_hand != Some(hand_num);
                if should_fold {
                    folded_hand = Some(hand_num);
                }
                let action = if should_fold && window.legal_actions.contains(&ActionType::Fold) {
                    ActionType::Fold
                } else if window.legal_actions.contains(&ActionType::AllIn) {
                    ActionType::AllIn
                } else if window.legal_actions.contains(&ActionType::Call) {
                    ActionType::Call
                } else {
                    ActionType::Check
                };
                let _ = host.submit_action(
                    &window.player_id,
                    window.action_window_id.clone(),
                    action,
                    None,
                );
                last_submitted = Some(window.action_window_id.clone());
            }
        }
        assert!(Instant::now() < deadline, "first elimination should occur");
        thread::sleep(Duration::from_millis(20));
    };

    let eliminated = seated
        .iter()
        .find(|(id, _)| *id == eliminated_player_id.as_str())
        .map(|(_, c)| *c)
        .expect("eliminated client");

    // Ensure all TCP command connections are ready before submitting via TCP.
    for (_, client) in &seated {
        wait_for_client_command_connection(client);
    }

    // Wait for the next hand's action window.
    let next_deadline = Instant::now() + Duration::from_secs(10);
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
        assert!(
            Instant::now() < next_deadline,
            "next hand window should open"
        );
        thread::sleep(Duration::from_millis(20));
    };

    // Eliminated player submits for the open window — must be rejected.
    let _ = eliminated.submit_action(
        window.action_window_id.clone(),
        window.seat_index,
        ActionType::Call,
        None,
    );
    thread::sleep(Duration::from_millis(300));

    let after_reject = host
        .authoritative_state()
        .expect("state")
        .current_hand
        .and_then(|h| h.action_window);
    assert_eq!(
        after_reject.as_ref().map(|w| w.action_window_id.as_str()),
        Some(window.action_window_id.as_str()),
        "eliminated player action must not advance the window"
    );

    // Real actor can still advance the hand.
    let live_window = after_reject.expect("window still open");
    if let Some((_, c)) = seated
        .iter()
        .find(|(id, _)| *id == live_window.player_id.as_str())
    {
        let action = if live_window.legal_actions.contains(&ActionType::Check) {
            ActionType::Check
        } else {
            ActionType::Call
        };
        c.submit_action(
            live_window.action_window_id.clone(),
            live_window.seat_index,
            action,
            None,
        )
        .expect("live actor should succeed");
    }
    let advance_deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let cur_id = host
            .authoritative_state()
            .expect("state")
            .current_hand
            .as_ref()
            .and_then(|h| h.action_window.as_ref())
            .map(|w| w.action_window_id.clone());
        if cur_id.as_deref() != Some(window.action_window_id.as_str()) {
            break;
        }
        assert!(
            Instant::now() < advance_deadline,
            "actor action should advance the hand"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

// ── Section 5: Signature replay protection across sessions ───────────────────

/// A signed envelope from session epoch N is rejected by the replay protector
/// for epoch N+1, and a repeated message_id is rejected even within the same
/// epoch. These are the two production guards preventing cross-session and
/// within-session replays.
#[test]
fn j_replay_protector_rejects_stale_epoch_and_duplicate_message_ids() {
    let provider = DefaultCryptoProvider;
    let signing_keys = provider.generate_signing_keypair();
    let encryption_keys = provider.generate_encryption_keypair();

    // Epoch 96 envelope (represents the "old" session).
    let old_payload =
        sample_join_payload_for_tests("table-replay", 96, signing_keys.public_key_base64());
    let stale_envelope = signed_join_envelope(
        &provider,
        &signing_keys,
        &encryption_keys,
        &old_payload,
        "player-replay",
        "Replay",
        &old_payload.join_token,
    );

    // Epoch 97 envelope for the same player (represents the "current" session).
    let new_payload =
        sample_join_payload_for_tests("table-replay", 97, signing_keys.public_key_base64());
    let fresh_envelope = signed_join_envelope(
        &provider,
        &signing_keys,
        &encryption_keys,
        &new_payload,
        "player-replay",
        "Replay",
        &new_payload.join_token,
    );

    // The host's ReplayProtector runs at epoch 97.
    let mut protector = ReplayProtector::new("table-replay", 97);

    // Stale epoch must be rejected.
    let stale_result = protector.validate_signed(&stale_envelope);
    assert!(
        stale_result.is_err(),
        "epoch-96 envelope must be rejected by epoch-97 protector"
    );
    let err_msg = stale_result.unwrap_err().to_string();
    assert!(
        err_msg.contains("session") || err_msg.contains("epoch"),
        "error must mention session/epoch mismatch, got: {err_msg}"
    );

    // Fresh epoch is accepted once.
    protector
        .validate_signed(&fresh_envelope)
        .expect("first submission at correct epoch should pass");

    // Duplicate message_id (same envelope replayed) is rejected.
    let duplicate_result = protector.validate_signed(&fresh_envelope);
    assert!(
        duplicate_result.is_err(),
        "replayed messageId must be rejected"
    );
}
