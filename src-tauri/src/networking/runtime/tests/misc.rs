use std::{
    sync::{atomic::Ordering, Arc, Mutex},
    time::Duration,
};

use serde_json::json;

use super::super::{infer_new_elimination_events, validate_production_host_ip};
use crate::{
    crypto::{DefaultCryptoProvider, ProtocolCryptoProvider},
    domain::{
        ConnectionState, ParticipantState, PlacementEntry, SeatOccupancyState, SeatState,
        TournamentPhase, TournamentSeatState,
    },
    networking::{
        resolve_connectable_host_ip, ClientRuntime, ClientRuntimeConfig, ClientRuntimeEvent,
        HostRuntimeConfig, HostRuntimeMode, HostServer,
    },
    protocol::{
        EliminationEvent, ProtocolMessageType, TournamentCompleteEvent, TournamentStartedEvent,
    },
};

use super::support::*;

#[test]
fn resync_after_a_sequence_gap_allows_followup_public_events_to_continue() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-resync-continue", 40);
    let client = connect_test_client(&provider, &host, "player-resync-continue", "Resync");
    let _ = expect_snapshot_event(&client);

    let mut updated_state = host.authoritative_state().expect("authoritative state");
    updated_state.phase = TournamentPhase::ReadyCheck;
    updated_state.config.tournament_name = "Resync Continue".to_string();
    host.replace_authoritative_state(updated_state)
        .expect("replace authoritative state");

    host.server_sequence.store(0, Ordering::SeqCst);
    host.broadcast_public_event(
        ProtocolMessageType::TournamentStartedEvent,
        &TournamentStartedEvent {
            tournament_name: "stale".to_string(),
            starting_stack: 1500,
            blind_schedule_preset: "FAST".to_string(),
            frozen_player_ids: vec!["player-resync-continue".to_string()],
        },
    )
    .expect("broadcast stale event");

    match client
        .next_event(Duration::from_secs(2))
        .expect("resync requested")
    {
        ClientRuntimeEvent::ResyncRequested {
            player_id,
            last_seen_server_sequence,
        } => {
            assert_eq!(player_id, "player-resync-continue");
            assert_eq!(last_seen_server_sequence, 1);
        }
        other => panic!("expected resync requested event, got {other:?}"),
    }

    let snapshot = expect_snapshot_event(&client);
    assert_eq!(snapshot.state.config.tournament_name, "Resync Continue");

    host.broadcast_public_event(
        ProtocolMessageType::TournamentCompleteEvent,
        &TournamentCompleteEvent {
            winner_player_id: "player-resync-continue".to_string(),
            placements: vec![PlacementEntry {
                player_id: "player-resync-continue".to_string(),
                place: 1,
                busted_at_hand_number: None,
            }],
        },
    )
    .expect("broadcast tournament complete");

    let completion_payload =
        assert_public_event(&client, ProtocolMessageType::TournamentCompleteEvent);
    assert_eq!(
        completion_payload.get("winnerPlayerId"),
        Some(&json!("player-resync-continue"))
    );
}

#[test]
fn elimination_and_completion_events_stay_in_sync_across_clients() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-elimination-sync", 41);
    let alice = connect_test_client(&provider, &host, "player-a", "Alice");
    let bob = connect_test_client(&provider, &host, "player-b", "Bob");
    let _ = expect_snapshot_event(&alice);
    let _ = expect_snapshot_event(&bob);

    host.broadcast_public_event(
        ProtocolMessageType::EliminationEvent,
        &EliminationEvent {
            player_id: "player-b".to_string(),
            place: 2,
        },
    )
    .expect("elimination event");
    host.broadcast_public_event(
        ProtocolMessageType::TournamentCompleteEvent,
        &TournamentCompleteEvent {
            winner_player_id: "player-a".to_string(),
            placements: vec![
                PlacementEntry {
                    player_id: "player-a".to_string(),
                    place: 1,
                    busted_at_hand_number: None,
                },
                PlacementEntry {
                    player_id: "player-b".to_string(),
                    place: 2,
                    busted_at_hand_number: Some(12),
                },
            ],
        },
    )
    .expect("completion event");

    for client in [&alice, &bob] {
        let elimination_payload =
            assert_public_event(client, ProtocolMessageType::EliminationEvent);
        assert_eq!(
            elimination_payload.get("playerId"),
            Some(&json!("player-b"))
        );
        let completion_payload =
            assert_public_event(client, ProtocolMessageType::TournamentCompleteEvent);
        assert_eq!(
            completion_payload.get("winnerPlayerId"),
            Some(&json!("player-a"))
        );
    }
}

#[test]
fn host_marks_disconnect_as_reconnect_eligible() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

    let host = HostServer::bind(HostRuntimeConfig {
        bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
        advertised_host: "127.0.0.1".to_string(),
        session_epoch: 23,
        table_id: "table-disconnect".to_string(),
        table_name: Some("Disconnect".to_string()),
        join_token: "join-token".to_string(),
        host_signing_keys,
        host_encryption_keys,
        snapshot_state: sample_tournament_state("table-disconnect", 23),
        runtime_mode: HostRuntimeMode::Test,
    })
    .expect("host should bind");

    let client = ClientRuntime::connect(ClientRuntimeConfig {
        join_payload: host.encoded_join_payload().to_string(),
        player_id: "player-disconnect".to_string(),
        display_name: "Disconnect".to_string(),
        signing_keys: provider.generate_signing_keypair(),
        encryption_keys: provider.generate_encryption_keypair(),
    })
    .expect("client should connect");
    let _ = client.next_event(Duration::from_secs(2)).expect("snapshot");
    client
        .clear_reconnect_identity()
        .expect("clear reconnect identity");

    let mut updated_state = host.authoritative_state().expect("authoritative state");
    updated_state.phase = TournamentPhase::Running;
    updated_state.seats = vec![SeatState {
        seat_index: 0,
        occupancy: SeatOccupancyState::Occupied,
        tournament_state: TournamentSeatState::Active,
        participant_id: Some("player-disconnect".to_string()),
        display_name: Some("Disconnect".to_string()),
        chip_count: Some(1500),
        is_ready: true,
        marker: None,
    }];
    let participant = updated_state
        .participants
        .get_mut("player-disconnect")
        .expect("participant");
    participant.state = ParticipantState::Active;
    participant.connection_state = ConnectionState::Connected;
    participant.seat_index = Some(0);
    host.replace_authoritative_state(updated_state)
        .expect("replace authoritative state");

    disconnect_client(&host, "player-disconnect");

    wait_for_host_participant_state(
        &host,
        "player-disconnect",
        ParticipantState::Reconnecting,
        ConnectionState::Reconnecting,
    );

    let state = host.authoritative_state().expect("authoritative state");
    let participant = state
        .participants
        .get("player-disconnect")
        .expect("participant registry entry");
    assert_eq!(participant.seat_index, Some(0));
    assert!(participant.reconnect_expiry_ms.is_some());
    assert_eq!(
        state
            .seats
            .first()
            .and_then(|seat| seat.participant_id.as_deref()),
        Some("player-disconnect")
    );
}

#[test]
fn resync_replaces_local_state_from_authoritative_snapshot() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

    let host = HostServer::bind(HostRuntimeConfig {
        bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
        advertised_host: "127.0.0.1".to_string(),
        session_epoch: 24,
        table_id: "table-resync".to_string(),
        table_name: Some("Resync".to_string()),
        join_token: "join-token".to_string(),
        host_signing_keys,
        host_encryption_keys,
        snapshot_state: sample_tournament_state("table-resync", 24),
        runtime_mode: HostRuntimeMode::Test,
    })
    .expect("host should bind");

    let client = ClientRuntime::connect(ClientRuntimeConfig {
        join_payload: host.encoded_join_payload().to_string(),
        player_id: "player-resync".to_string(),
        display_name: "Resync".to_string(),
        signing_keys: provider.generate_signing_keypair(),
        encryption_keys: provider.generate_encryption_keypair(),
    })
    .expect("client should connect");
    let _ = client.next_event(Duration::from_secs(2)).expect("snapshot");

    let mut updated_state = host.authoritative_state().expect("authoritative state");
    updated_state.phase = TournamentPhase::ReadyCheck;
    updated_state.config.tournament_name = "Resynced Tournament".to_string();
    host.replace_authoritative_state(updated_state)
        .expect("replace authoritative state");

    host.server_sequence.store(0, Ordering::SeqCst);
    host.broadcast_public_event(
        ProtocolMessageType::TournamentStartedEvent,
        &TournamentStartedEvent {
            tournament_name: "stale".to_string(),
            starting_stack: 1500,
            blind_schedule_preset: "FAST".to_string(),
            frozen_player_ids: vec!["player-resync".to_string()],
        },
    )
    .expect("broadcast stale event");

    let _ = client
        .next_event(Duration::from_secs(2))
        .expect("resync requested");
    match client
        .next_event(Duration::from_secs(2))
        .expect("resynced snapshot")
    {
        ClientRuntimeEvent::Snapshot(snapshot) => {
            assert_eq!(snapshot.state.phase, TournamentPhase::ReadyCheck);
            assert_eq!(snapshot.state.config.tournament_name, "Resynced Tournament");
        }
        other => panic!("expected snapshot event, got {other:?}"),
    }
}

#[test]
fn event_sequence_mismatch_triggers_resync() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

    let host = HostServer::bind(HostRuntimeConfig {
        bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
        advertised_host: "127.0.0.1".to_string(),
        session_epoch: 25,
        table_id: "table-sequence".to_string(),
        table_name: Some("Sequence".to_string()),
        join_token: "join-token".to_string(),
        host_signing_keys,
        host_encryption_keys,
        snapshot_state: sample_tournament_state("table-sequence", 25),
        runtime_mode: HostRuntimeMode::Test,
    })
    .expect("host should bind");

    let client = ClientRuntime::connect(ClientRuntimeConfig {
        join_payload: host.encoded_join_payload().to_string(),
        player_id: "player-sequence".to_string(),
        display_name: "Sequence".to_string(),
        signing_keys: provider.generate_signing_keypair(),
        encryption_keys: provider.generate_encryption_keypair(),
    })
    .expect("client should connect");
    let _ = client.next_event(Duration::from_secs(2)).expect("snapshot");

    host.server_sequence.store(0, Ordering::SeqCst);
    host.broadcast_public_event(
        ProtocolMessageType::TournamentStartedEvent,
        &TournamentStartedEvent {
            tournament_name: "stale".to_string(),
            starting_stack: 1500,
            blind_schedule_preset: "FAST".to_string(),
            frozen_player_ids: vec!["player-sequence".to_string()],
        },
    )
    .expect("broadcast stale event");

    match client
        .next_event(Duration::from_secs(2))
        .expect("resync requested")
    {
        ClientRuntimeEvent::ResyncRequested {
            player_id,
            last_seen_server_sequence,
        } => {
            assert_eq!(player_id, "player-sequence");
            assert_eq!(last_seen_server_sequence, 1);
        }
        other => panic!("expected resync requested event, got {other:?}"),
    }

    match client
        .next_event(Duration::from_secs(2))
        .expect("resynced snapshot")
    {
        ClientRuntimeEvent::Snapshot(snapshot) => {
            assert_eq!(snapshot.local_player_id, "player-sequence");
        }
        other => panic!("expected snapshot event, got {other:?}"),
    }
}

#[test]
fn invalid_host_ip_blocks_host_startup() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

    let host = HostServer::bind(HostRuntimeConfig {
        bind_addr: "0.0.0.0:0".parse().expect("socket addr"),
        advertised_host: "0.0.0.0".to_string(),
        session_epoch: 1,
        table_id: "table-invalid".to_string(),
        table_name: Some("Invalid".to_string()),
        join_token: "join-token".to_string(),
        host_signing_keys,
        host_encryption_keys,
        snapshot_state: sample_tournament_state("table-invalid", 1),
        runtime_mode: HostRuntimeMode::Production,
    });

    assert!(host.is_err());
}

#[test]
fn invalid_payload_blocks_join_before_connect() {
    let provider = DefaultCryptoProvider;

    let result = ClientRuntime::connect(ClientRuntimeConfig {
            join_payload: "{\"payloadVersion\":1,\"hostAddress\":\"0.0.0.0\",\"hostPort\":43818,\"tableId\":\"table-1\",\"sessionEpoch\":1,\"hostSigningPublicKey\":\"host\",\"joinToken\":\"join\",\"generatedAtMs\":1}".to_string(),
            player_id: "player-invalid".to_string(),
            display_name: "Invalid".to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        });

    assert!(result.is_err());
}

#[test]
fn host_ip_resolution_returns_error_when_no_connectable_address_exists() {
    let result = validate_production_host_ip("127.0.0.1".parse().expect("ip address"));
    assert!(result.is_err());
    let _ = resolve_connectable_host_ip;
}

// P1.1 — HostRuntimeHealth helper methods record the right counter and last_error.

#[test]
fn host_runtime_health_record_stream_clone_error_increments_counter() {
    use crate::networking::HostRuntimeHealth;

    let mut health = HostRuntimeHealth::default();
    assert_eq!(health.stream_clone_error_count, 0);
    assert!(health.last_error.is_none());

    health.record_stream_clone_error("simulated OS error");

    assert_eq!(health.stream_clone_error_count, 1);
    assert!(
        health
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("simulated OS error"),
        "last_error must include the original error message; got: {:?}",
        health.last_error
    );
}

#[test]
fn host_runtime_health_record_client_registry_error_increments_counter() {
    use crate::networking::HostRuntimeHealth;

    let mut health = HostRuntimeHealth::default();
    assert_eq!(health.client_registry_error_count, 0);
    assert!(health.last_error.is_none());

    health.record_client_registry_error();

    assert_eq!(health.client_registry_error_count, 1);
    assert!(
        health
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("registry lock poisoned"),
        "last_error must describe the lock poison; got: {:?}",
        health.last_error
    );
}

#[test]
fn host_runtime_health_record_reconnect_mark_error_increments_counter() {
    use crate::networking::HostRuntimeHealth;

    let mut health = HostRuntimeHealth::default();
    assert_eq!(health.reconnect_mark_error_count, 0);
    assert!(health.last_error.is_none());

    health.record_reconnect_mark_error("player-x", "state lock poisoned");

    assert_eq!(health.reconnect_mark_error_count, 1);
    assert!(
        health
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("player-x"),
        "last_error must name the affected player; got: {:?}",
        health.last_error
    );
    assert!(
        health
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("state lock poisoned"),
        "last_error must include the cause; got: {:?}",
        health.last_error
    );
}

#[test]
fn host_runtime_health_counters_accumulate_independently() {
    use crate::networking::HostRuntimeHealth;

    let mut health = HostRuntimeHealth::default();

    health.record_stream_clone_error("err1");
    health.record_stream_clone_error("err2");
    health.record_client_registry_error();
    health.record_reconnect_mark_error("p1", "cause");

    assert_eq!(
        health.stream_clone_error_count, 2,
        "two stream-clone errors"
    );
    assert_eq!(health.client_registry_error_count, 1, "one registry error");
    assert_eq!(health.reconnect_mark_error_count, 1, "one reconnect error");
}

#[test]
fn established_connections_clear_handshake_read_timeouts() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-established-timeouts", 87);
    let client = connect_test_client(
        &provider,
        &host,
        "player-established-timeouts",
        "Timeout Test",
    );
    let _ = expect_snapshot_event(&client);

    let connected_clients = host.clients.lock().expect("host client registry");
    let connected_client = connected_clients
        .get("player-established-timeouts")
        .expect("registered host client");
    let host_stream = connected_client.stream.lock().expect("host client stream");
    assert_eq!(
        host_stream.read_timeout().expect("host read timeout"),
        None,
        "the host must clear the handshake read timeout for an established session"
    );
    drop(host_stream);
    drop(connected_clients);

    let command_connection = client
        .command_connection
        .lock()
        .expect("client command connection");
    let client_stream = command_connection
        .stream
        .as_ref()
        .expect("established client command stream")
        .lock()
        .expect("client command stream lock");
    assert_eq!(
        client_stream.read_timeout().expect("client read timeout"),
        None,
        "the client must clear the handshake read timeout for an established session"
    );
}

#[test]
fn completion_transition_never_emits_winner_as_eliminated() {
    let mut before = sample_tournament_state("table-completion-events", 91);
    before.placements.clear();
    let mut after = before.clone();
    after.placements = vec![
        PlacementEntry {
            player_id: "winner".to_string(),
            place: 1,
            busted_at_hand_number: None,
        },
        PlacementEntry {
            player_id: "loser".to_string(),
            place: 2,
            busted_at_hand_number: Some(3),
        },
    ];

    let events = infer_new_elimination_events(&before, &after);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].player_id, "loser");
    assert_eq!(events[0].place, 2);

    before.placements = vec![after.placements[1].clone()];
    let winner_only_added = infer_new_elimination_events(&before, &after);
    assert!(
        winner_only_added.is_empty(),
        "adding the first-place placement must not emit an elimination"
    );
}
