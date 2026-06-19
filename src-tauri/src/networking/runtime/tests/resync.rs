use std::{
    sync::{atomic::AtomicU64, Arc, Mutex},
    time::Duration,
};

use serde_json::json;

use super::super::handle_resync_request;
use crate::{
    crypto::{DefaultCryptoProvider, ProtocolCryptoProvider},
    domain::{
        ActionType, ActionWindow, BettingRoundState, Card, ConnectionState, HandCyclePhase,
        HandParticipationState, HandState, ParticipantState, Rank, SeatOccupancyState, SeatState,
        StreetPhase, Suit, TournamentPhase, TournamentSeatState,
    },
    networking::ClientRuntimeEvent,
    protocol::{
        test_support::sample_resync_request, ActionWindowOpened, PrivateHoleCardsEvent,
        ProtocolMessageType, TournamentStartedEvent,
    },
};

use super::support::*;

#[test]
fn resync_accepts_missing_sequence_and_replays_latest_snapshot() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let player_signing_keys = provider.generate_signing_keypair();
    let player_encryption_keys = provider.generate_encryption_keypair();
    let join_payload = sample_join_payload_for_tests(
        "table-resync-repeat",
        36,
        host_signing_keys.public_key_base64(),
    );
    let server_sequence = Arc::new(AtomicU64::new(5));
    let mut state = sample_tournament_state("table-resync-repeat", 36);
    state.participants.insert(
        "player-resync-repeat".to_string(),
        sample_participant_entry(
            "player-resync-repeat",
            "Resync Repeat",
            player_signing_keys.public_key_base64(),
            player_encryption_keys.public_key_base64(),
            ParticipantState::Admitted,
            ConnectionState::Connected,
            None,
            "resync-token",
        ),
    );
    let authoritative_state = Arc::new(Mutex::new(state));

    let first_snapshot = handle_resync_request(
        &provider,
        signed_resync_envelope(
            &provider,
            &player_signing_keys,
            &join_payload,
            "player-resync-repeat",
            sample_resync_request(None),
            1,
            "resync-repeat-1",
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    )
    .expect("resync should succeed without a sequence");
    assert_eq!(
        first_snapshot.payload.local_player_id,
        "player-resync-repeat"
    );

    authoritative_state
        .lock()
        .expect("authoritative state")
        .config
        .tournament_name = "Resync Repeat Updated".to_string();

    let second_snapshot = handle_resync_request(
        &provider,
        signed_resync_envelope(
            &provider,
            &player_signing_keys,
            &join_payload,
            "player-resync-repeat",
            sample_resync_request(None),
            2,
            "resync-repeat-2",
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    )
    .expect("repeated resync should succeed");

    assert_eq!(
        second_snapshot.payload.state.config.tournament_name,
        "Resync Repeat Updated"
    );
}

#[test]
fn resync_rejects_future_sequences() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let player_signing_keys = provider.generate_signing_keypair();
    let player_encryption_keys = provider.generate_encryption_keypair();
    let join_payload = sample_join_payload_for_tests(
        "table-resync-future",
        37,
        host_signing_keys.public_key_base64(),
    );
    let server_sequence = Arc::new(AtomicU64::new(2));
    let mut state = sample_tournament_state("table-resync-future", 37);
    state.participants.insert(
        "player-resync-future".to_string(),
        sample_participant_entry(
            "player-resync-future",
            "Resync Future",
            player_signing_keys.public_key_base64(),
            player_encryption_keys.public_key_base64(),
            ParticipantState::Admitted,
            ConnectionState::Connected,
            None,
            "resync-future-token",
        ),
    );
    let authoritative_state = Arc::new(Mutex::new(state));

    let result = handle_resync_request(
        &provider,
        signed_resync_envelope(
            &provider,
            &player_signing_keys,
            &join_payload,
            "player-resync-future",
            sample_resync_request(Some(99)),
            1,
            "resync-future",
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    );

    assert!(result.is_err());
    assert!(result
        .expect_err("future resync should fail")
        .to_string()
        .contains("ahead of the host sequence"));
}

#[test]
fn public_and_private_events_stay_ordered_and_scoped_across_two_clients() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-public-private-ordered", 38);

    let alice = connect_test_client(&provider, &host, "player-a", "Alice");
    let bob = connect_test_client(&provider, &host, "player-b", "Bob");
    let _ = expect_snapshot_event(&alice);
    let _ = expect_snapshot_event(&bob);

    host.broadcast_public_event(
        ProtocolMessageType::TournamentStartedEvent,
        &TournamentStartedEvent {
            tournament_name: "Ordered Events".to_string(),
            starting_stack: 1500,
            blind_schedule_preset: "FAST".to_string(),
            frozen_player_ids: vec!["player-a".to_string(), "player-b".to_string()],
        },
    )
    .expect("tournament started event");
    host.broadcast_public_event(
        ProtocolMessageType::ActionWindowOpenedEvent,
        &ActionWindowOpened {
            hand_number: 1,
            hand_phase: "AWAITING_ACTION".to_string(),
            action_window_id: "window-1".to_string(),
            player_id: "player-a".to_string(),
            seat_index: 0,
            legal_actions: vec![ActionType::Fold, ActionType::Call, ActionType::Raise],
            call_amount: 20,
            min_raise_to: Some(40),
            max_raise_to: Some(200),
            deadline_epoch_ms: 123_456,
        },
    )
    .expect("action window event");

    for client in [&alice, &bob] {
        let started_payload =
            assert_public_event(client, ProtocolMessageType::TournamentStartedEvent);
        assert_eq!(
            started_payload.get("tournamentName"),
            Some(&json!("Ordered Events"))
        );

        let action_window_payload =
            assert_public_event(client, ProtocolMessageType::ActionWindowOpenedEvent);
        assert_eq!(
            action_window_payload.get("playerId"),
            Some(&json!("player-a"))
        );
    }

    host.send_private_hole_cards(
        "player-a",
        &PrivateHoleCardsEvent {
            recipient_player_id: "player-a".to_string(),
            hole_cards: vec![
                Card {
                    rank: Rank::Ace,
                    suit: Suit::Spades,
                },
                Card {
                    rank: Rank::King,
                    suit: Suit::Spades,
                },
            ],
        },
    )
    .expect("alice private cards");

    match alice
        .next_event(Duration::from_secs(2))
        .expect("alice private event")
    {
        ClientRuntimeEvent::PrivateHoleCards(payload) => {
            assert_eq!(payload.recipient_player_id, "player-a");
            assert_eq!(payload.hole_cards.len(), 2);
        }
        other => panic!("expected alice private cards, got {other:?}"),
    }
    assert!(bob.next_event(Duration::from_millis(200)).is_err());

    host.send_private_hole_cards(
        "player-b",
        &PrivateHoleCardsEvent {
            recipient_player_id: "player-b".to_string(),
            hole_cards: vec![
                Card {
                    rank: Rank::Queen,
                    suit: Suit::Hearts,
                },
                Card {
                    rank: Rank::Jack,
                    suit: Suit::Hearts,
                },
            ],
        },
    )
    .expect("bob private cards");

    match bob
        .next_event(Duration::from_secs(2))
        .expect("bob private event")
    {
        ClientRuntimeEvent::PrivateHoleCards(payload) => {
            assert_eq!(payload.recipient_player_id, "player-b");
            assert_eq!(payload.hole_cards.len(), 2);
        }
        other => panic!("expected bob private cards, got {other:?}"),
    }
}

#[test]
fn reconnect_restores_mid_hand_snapshot_with_the_original_action_owner() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-mid-hand-reconnect", 39);
    let client = connect_test_client(&provider, &host, "player-midhand", "Midhand");
    let _ = expect_snapshot_event(&client);
    wait_for_host_participant_state(
        &host,
        "player-midhand",
        ParticipantState::Admitted,
        ConnectionState::Connected,
    );

    let mut updated_state = host.authoritative_state().expect("authoritative state");
    updated_state.phase = TournamentPhase::Running;
    updated_state.seats = vec![SeatState {
        seat_index: 0,
        occupancy: SeatOccupancyState::Occupied,
        tournament_state: TournamentSeatState::Active,
        participant_id: Some("player-midhand".to_string()),
        display_name: Some("Midhand".to_string()),
        chip_count: Some(1500),
        is_ready: true,
        marker: None,
    }];
    let participant = updated_state
        .participants
        .get_mut("player-midhand")
        .expect("participant");
    participant.state = ParticipantState::Active;
    participant.connection_state = ConnectionState::Connected;
    participant.seat_index = Some(0);
    updated_state.current_hand = Some(HandState {
        hand_number: 12,
        cycle_phase: HandCyclePhase::AwaitingAction,
        street: StreetPhase::Turn,
        dealer_seat_index: 0,
        small_blind_seat_index: 0,
        big_blind_seat_index: 0,
        board_cards: vec![Card {
            rank: Rank::Ace,
            suit: Suit::Clubs,
        }],
        hole_cards_by_player_id: [(
            "player-midhand".to_string(),
            vec![
                Card {
                    rank: Rank::King,
                    suit: Suit::Spades,
                },
                Card {
                    rank: Rank::King,
                    suit: Suit::Hearts,
                },
            ],
        )]
        .into_iter()
        .collect(),
        participation_by_player_id: [(
            "player-midhand".to_string(),
            HandParticipationState::Active,
        )]
        .into_iter()
        .collect(),
        betting_round: BettingRoundState {
            street: StreetPhase::Turn,
            current_bet: 40,
            min_raise_to: Some(80),
            max_raise_to: Some(200),
            pot_size: 120,
            contributions_by_player_id: [("player-midhand".to_string(), 40)].into_iter().collect(),
        },
        action_window: Some(ActionWindow {
            action_window_id: "window-midhand".to_string(),
            player_id: "player-midhand".to_string(),
            seat_index: 0,
            legal_actions: vec![ActionType::Fold, ActionType::Call, ActionType::Raise],
            call_amount: 40,
            min_raise_to: Some(80),
            max_raise_to: Some(200),
            deadline_epoch_ms: 456_789,
        }),
    });
    host.replace_authoritative_state(updated_state)
        .expect("replace authoritative state");

    disconnect_client(&host, "player-midhand");

    match client
        .next_event(Duration::from_secs(2))
        .expect("reconnecting event")
    {
        ClientRuntimeEvent::Reconnecting { player_id } => {
            assert_eq!(player_id, "player-midhand");
        }
        other => panic!("expected reconnecting event, got {other:?}"),
    }

    let snapshot = expect_snapshot_event(&client);
    assert_eq!(snapshot.local_player_id, "player-midhand");
    assert_eq!(snapshot.state.phase, TournamentPhase::Running);
    let action_window = snapshot
        .state
        .current_hand
        .as_ref()
        .and_then(|hand| hand.action_window.as_ref())
        .expect("action window after reconnect");
    assert_eq!(action_window.player_id, "player-midhand");
    assert_eq!(action_window.action_window_id, "window-midhand");
}
