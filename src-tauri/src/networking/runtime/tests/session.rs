use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::json;

use crate::{
    crypto::{DefaultCryptoProvider, ProtocolCryptoProvider},
    domain::{Card, Rank, SeatOccupancyState, Suit, TournamentPhase},
    networking::{
        ClientRuntime, ClientRuntimeConfig, ClientRuntimeEvent, HostRuntimeConfig, HostRuntimeMode,
        HostServer,
    },
    protocol::{PrivateHoleCardsEvent, ProtocolMessageType, TournamentStartedEvent},
};

use super::support::*;

#[test]
fn host_can_open_listener() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

    let host = HostServer::bind(HostRuntimeConfig {
        bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
        advertised_host: "127.0.0.1".to_string(),
        session_epoch: 1,
        table_id: "table-1".to_string(),
        table_name: Some("Test".to_string()),
        join_token: "join-token".to_string(),
        host_signing_keys,
        host_encryption_keys,
        snapshot_state: sample_tournament_state("table-1", 1),
        runtime_mode: HostRuntimeMode::Test,
    })
    .expect("host should bind");

    assert!(host.listener_addr().port() > 0);
    assert!(host.encoded_join_payload().starts_with("pkr1_"));
}

#[test]
fn client_can_connect_and_join_using_canonical_payload() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

    let host = HostServer::bind(HostRuntimeConfig {
        bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
        advertised_host: "127.0.0.1".to_string(),
        session_epoch: 7,
        table_id: "table-join".to_string(),
        table_name: Some("Join Table".to_string()),
        join_token: "join-token".to_string(),
        host_signing_keys,
        host_encryption_keys,
        snapshot_state: sample_tournament_state("table-join", 7),
        runtime_mode: HostRuntimeMode::Test,
    })
    .expect("host should bind");

    let client = ClientRuntime::connect(ClientRuntimeConfig {
        join_payload: host.encoded_join_payload().to_string(),
        player_id: "player-a".to_string(),
        display_name: "Alice".to_string(),
        signing_keys: provider.generate_signing_keypair(),
        encryption_keys: provider.generate_encryption_keypair(),
    })
    .expect("client should connect");

    match client
        .next_event(Duration::from_secs(2))
        .expect("snapshot event")
    {
        ClientRuntimeEvent::Snapshot(snapshot) => {
            assert_eq!(snapshot.local_player_id, "player-a");
        }
        other => panic!("expected snapshot event, got {other:?}"),
    }
}

#[test]
fn multiple_clients_keep_distinct_identity_and_reconnect_state() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

    let host = HostServer::bind(HostRuntimeConfig {
        bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
        advertised_host: "127.0.0.1".to_string(),
        session_epoch: 8,
        table_id: "table-multi-client".to_string(),
        table_name: Some("Multi Client".to_string()),
        join_token: "join-token".to_string(),
        host_signing_keys,
        host_encryption_keys,
        snapshot_state: sample_tournament_state("table-multi-client", 8),
        runtime_mode: HostRuntimeMode::Test,
    })
    .expect("host should bind");

    let alice_signing_keys = provider.generate_signing_keypair();
    let alice_signing_public_key = alice_signing_keys.public_key_base64();
    let alice_encryption_keys = provider.generate_encryption_keypair();
    let alice_encryption_public_key = alice_encryption_keys.public_key_base64();
    let alice = ClientRuntime::connect(ClientRuntimeConfig {
        join_payload: host.encoded_join_payload().to_string(),
        player_id: "player-a".to_string(),
        display_name: "Alice".to_string(),
        signing_keys: alice_signing_keys,
        encryption_keys: alice_encryption_keys,
    })
    .expect("alice should connect");

    let bob_signing_keys = provider.generate_signing_keypair();
    let bob_signing_public_key = bob_signing_keys.public_key_base64();
    let bob_encryption_keys = provider.generate_encryption_keypair();
    let bob_encryption_public_key = bob_encryption_keys.public_key_base64();
    let bob = ClientRuntime::connect(ClientRuntimeConfig {
        join_payload: host.encoded_join_payload().to_string(),
        player_id: "player-b".to_string(),
        display_name: "Bob".to_string(),
        signing_keys: bob_signing_keys,
        encryption_keys: bob_encryption_keys,
    })
    .expect("bob should connect");

    let alice_reconnect_token = match alice
        .next_event(Duration::from_secs(2))
        .expect("alice snapshot")
    {
        ClientRuntimeEvent::Snapshot(snapshot) => snapshot
            .reconnect_token
            .clone()
            .expect("alice reconnect token"),
        other => panic!("expected alice snapshot, got {other:?}"),
    };
    let bob_reconnect_token = match bob
        .next_event(Duration::from_secs(2))
        .expect("bob snapshot")
    {
        ClientRuntimeEvent::Snapshot(snapshot) => snapshot
            .reconnect_token
            .clone()
            .expect("bob reconnect token"),
        other => panic!("expected bob snapshot, got {other:?}"),
    };

    assert_ne!(alice_reconnect_token, bob_reconnect_token);

    let state = host.authoritative_state().expect("authoritative state");
    assert_eq!(state.participants.len(), 2);
    let alice_participant = state.participants.get("player-a").expect("alice state");
    let bob_participant = state.participants.get("player-b").expect("bob state");
    assert_eq!(
        alice_participant.identity.signing_public_key,
        alice_signing_public_key
    );
    assert_eq!(
        alice_participant.identity.encryption_public_key,
        alice_encryption_public_key
    );
    assert_eq!(
        bob_participant.identity.signing_public_key,
        bob_signing_public_key
    );
    assert_eq!(
        bob_participant.identity.encryption_public_key,
        bob_encryption_public_key
    );
    assert_ne!(
        alice_participant.identity.signing_public_key,
        bob_participant.identity.signing_public_key
    );
}

#[test]
fn two_local_clients_can_join_and_receive_live_table_events_on_one_machine() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

    let host = HostServer::bind(HostRuntimeConfig {
        bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
        advertised_host: "127.0.0.1".to_string(),
        session_epoch: 9,
        table_id: "table-local-play".to_string(),
        table_name: Some("Local Play".to_string()),
        join_token: "join-token".to_string(),
        host_signing_keys,
        host_encryption_keys,
        snapshot_state: sample_tournament_state("table-local-play", 9),
        runtime_mode: HostRuntimeMode::Test,
    })
    .expect("host should bind");

    let alice = ClientRuntime::connect(ClientRuntimeConfig {
        join_payload: host.encoded_join_payload().to_string(),
        player_id: "player-local-a".to_string(),
        display_name: "Alice".to_string(),
        signing_keys: provider.generate_signing_keypair(),
        encryption_keys: provider.generate_encryption_keypair(),
    })
    .expect("alice should connect");
    let bob = ClientRuntime::connect(ClientRuntimeConfig {
        join_payload: host.encoded_join_payload().to_string(),
        player_id: "player-local-b".to_string(),
        display_name: "Bob".to_string(),
        signing_keys: provider.generate_signing_keypair(),
        encryption_keys: provider.generate_encryption_keypair(),
    })
    .expect("bob should connect");

    let _ = alice
        .next_event(Duration::from_secs(2))
        .expect("alice snapshot");
    let _ = bob
        .next_event(Duration::from_secs(2))
        .expect("bob snapshot");

    host.broadcast_public_event(
        ProtocolMessageType::TournamentStartedEvent,
        &TournamentStartedEvent {
            tournament_name: "Local Play".to_string(),
            starting_stack: 1500,
            blind_schedule_preset: "FAST".to_string(),
            frozen_player_ids: vec!["player-local-a".to_string(), "player-local-b".to_string()],
        },
    )
    .expect("broadcast tournament started");

    for client in [&alice, &bob] {
        match client
            .next_event(Duration::from_secs(2))
            .expect("public event")
        {
            ClientRuntimeEvent::PublicEvent {
                message_type,
                payload,
                ..
            } => {
                assert_eq!(message_type, ProtocolMessageType::TournamentStartedEvent);
                assert_eq!(payload.get("tournamentName"), Some(&json!("Local Play")));
            }
            other => panic!("expected public event, got {other:?}"),
        }
    }

    host.send_private_hole_cards(
        "player-local-a",
        &PrivateHoleCardsEvent {
            recipient_player_id: "player-local-a".to_string(),
            hole_cards: vec![
                Card {
                    rank: Rank::Ace,
                    suit: Suit::Spades,
                },
                Card {
                    rank: Rank::Ace,
                    suit: Suit::Hearts,
                },
            ],
        },
    )
    .expect("send alice private cards");
    host.send_private_hole_cards(
        "player-local-b",
        &PrivateHoleCardsEvent {
            recipient_player_id: "player-local-b".to_string(),
            hole_cards: vec![
                Card {
                    rank: Rank::King,
                    suit: Suit::Clubs,
                },
                Card {
                    rank: Rank::Queen,
                    suit: Suit::Diamonds,
                },
            ],
        },
    )
    .expect("send bob private cards");

    match alice
        .next_event(Duration::from_secs(2))
        .expect("alice private cards")
    {
        ClientRuntimeEvent::PrivateHoleCards(payload) => {
            assert_eq!(payload.recipient_player_id, "player-local-a");
            assert_eq!(payload.hole_cards.len(), 2);
        }
        other => panic!("expected alice private cards, got {other:?}"),
    }

    match bob
        .next_event(Duration::from_secs(2))
        .expect("bob private cards")
    {
        ClientRuntimeEvent::PrivateHoleCards(payload) => {
            assert_eq!(payload.recipient_player_id, "player-local-b");
            assert_eq!(payload.hole_cards.len(), 2);
        }
        other => panic!("expected bob private cards, got {other:?}"),
    }
}

#[test]
fn public_events_flow_from_host_to_client_over_real_tcp() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

    let host = HostServer::bind(HostRuntimeConfig {
        bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
        advertised_host: "127.0.0.1".to_string(),
        session_epoch: 11,
        table_id: "table-events".to_string(),
        table_name: Some("Events".to_string()),
        join_token: "join-token".to_string(),
        host_signing_keys,
        host_encryption_keys,
        snapshot_state: sample_tournament_state("table-events", 11),
        runtime_mode: HostRuntimeMode::Test,
    })
    .expect("host should bind");

    let client = ClientRuntime::connect(ClientRuntimeConfig {
        join_payload: host.encoded_join_payload().to_string(),
        player_id: "player-public".to_string(),
        display_name: "Public".to_string(),
        signing_keys: provider.generate_signing_keypair(),
        encryption_keys: provider.generate_encryption_keypair(),
    })
    .expect("client should connect");
    let _ = client.next_event(Duration::from_secs(2)).expect("snapshot");

    host.broadcast_public_event(
        ProtocolMessageType::TournamentStartedEvent,
        &TournamentStartedEvent {
            tournament_name: "LAN Test".to_string(),
            starting_stack: 1500,
            blind_schedule_preset: "FAST".to_string(),
            frozen_player_ids: vec!["player-public".to_string()],
        },
    )
    .expect("host should broadcast");

    match client
        .next_event(Duration::from_secs(2))
        .expect("public event")
    {
        ClientRuntimeEvent::PublicEvent {
            message_type,
            payload,
            ..
        } => {
            assert_eq!(message_type, ProtocolMessageType::TournamentStartedEvent);
            assert_eq!(payload.get("tournamentName"), Some(&json!("LAN Test")));
        }
        other => panic!("expected public event, got {other:?}"),
    }
}

#[test]
fn client_lobby_requests_sync_seat_and_ready_state_across_host_and_clients() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-lobby-sync", 60);
    let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
    let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
    let _ = expect_snapshot_event(&alice);
    let _ = expect_snapshot_event(&bob);

    alice.claim_seat(1).expect("alice should claim a seat");
    let alice_seated = expect_snapshot_where(&alice, |snapshot| {
        snapshot
            .state
            .participants
            .get("player-alice")
            .and_then(|participant| participant.seat_index)
            == Some(1)
    });
    assert_eq!(alice_seated.state.phase, TournamentPhase::WaitingForPlayers);

    bob.claim_seat(2).expect("bob should claim a seat");
    let host_view_after_bob = expect_snapshot_where(&alice, |snapshot| {
        snapshot
            .state
            .participants
            .get("player-bob")
            .and_then(|participant| participant.seat_index)
            == Some(2)
    });
    let bob_view_after_claim = expect_snapshot_where(&bob, |snapshot| {
        snapshot
            .state
            .participants
            .get("player-bob")
            .and_then(|participant| participant.seat_index)
            == Some(2)
    });
    assert_eq!(
        host_view_after_bob.state.phase,
        TournamentPhase::WaitingForPlayers
    );
    assert_eq!(
        bob_view_after_claim.state.phase,
        TournamentPhase::WaitingForPlayers
    );

    alice
        .set_ready_state(true)
        .expect("alice should toggle ready");
    let alice_ready = expect_snapshot_where(&alice, |snapshot| {
        snapshot
            .state
            .seats
            .iter()
            .find(|seat| seat.seat_index == 1)
            .is_some_and(|seat| seat.is_ready)
    });
    let bob_sees_alice_ready = expect_snapshot_where(&bob, |snapshot| {
        snapshot
            .state
            .seats
            .iter()
            .find(|seat| seat.seat_index == 1)
            .is_some_and(|seat| seat.is_ready)
    });
    assert_eq!(alice_ready.state.phase, TournamentPhase::WaitingForPlayers);
    assert_eq!(
        bob_sees_alice_ready.state.phase,
        TournamentPhase::WaitingForPlayers
    );

    bob.set_ready_state(true).expect("bob should toggle ready");
    let alice_ready_check = expect_snapshot_where(&alice, |snapshot| {
        snapshot.state.phase == TournamentPhase::ReadyCheck
    });
    let bob_ready_check = expect_snapshot_where(&bob, |snapshot| {
        snapshot.state.phase == TournamentPhase::ReadyCheck
    });
    assert_eq!(alice_ready_check.state.phase, TournamentPhase::ReadyCheck);
    assert_eq!(bob_ready_check.state.phase, TournamentPhase::ReadyCheck);

    let host_state = host.authoritative_state().expect("host state");
    assert_eq!(host_state.phase, TournamentPhase::ReadyCheck);
    assert!(host_state
        .seats
        .iter()
        .filter(|seat| seat.occupancy == SeatOccupancyState::Occupied)
        .all(|seat| seat.is_ready));
}
