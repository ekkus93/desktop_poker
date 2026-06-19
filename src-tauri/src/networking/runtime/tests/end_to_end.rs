use std::{
    thread,
    time::{Duration, Instant},
};

use crate::{
    crypto::DefaultCryptoProvider,
    domain::{ActionType, BlindLevel, BlindSchedule, TournamentPhase},
    protocol::ProtocolMessageType,
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
