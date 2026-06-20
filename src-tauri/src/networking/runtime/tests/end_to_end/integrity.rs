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

use super::super::support::*;

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
