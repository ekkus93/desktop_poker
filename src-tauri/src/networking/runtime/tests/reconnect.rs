use std::{
    net::Shutdown,
    sync::{atomic::AtomicU64, Arc, Mutex},
    thread,
    time::Duration,
};

use super::super::{connect_and_join, reconnect_after_disconnect};
use super::super::{handle_reconnect_request, ClientReconnectIdentity};
use crate::{
    crypto::{DefaultCryptoProvider, ProtocolCryptoProvider},
    domain::{Card, ConnectionState, ParticipantState, Rank, Suit, TournamentPhase},
    networking::{
        ClientRuntime, ClientRuntimeConfig, ClientRuntimeEvent, HostRuntimeConfig, HostRuntimeMode,
        HostServer,
    },
    protocol::{
        test_support::sample_reconnect_request, PrivateHoleCardsEvent, ReconnectTournamentRequest,
    },
};

use super::support::*;

#[test]
fn private_encrypted_payload_can_be_delivered_and_decrypted() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

    let host = HostServer::bind(HostRuntimeConfig {
        bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
        advertised_host: "127.0.0.1".to_string(),
        session_epoch: 15,
        table_id: "table-private".to_string(),
        table_name: Some("Private".to_string()),
        join_token: "join-token".to_string(),
        host_signing_keys,
        host_encryption_keys,
        snapshot_state: sample_tournament_state("table-private", 15),
        runtime_mode: HostRuntimeMode::Test,
    })
    .expect("host should bind");

    let client = ClientRuntime::connect(ClientRuntimeConfig {
        join_payload: host.encoded_join_payload().to_string(),
        player_id: "player-private".to_string(),
        display_name: "Private".to_string(),
        signing_keys: provider.generate_signing_keypair(),
        encryption_keys: provider.generate_encryption_keypair(),
    })
    .expect("client should connect");
    let _ = client.next_event(Duration::from_secs(2)).expect("snapshot");

    host.send_private_hole_cards(
        "player-private",
        &PrivateHoleCardsEvent {
            recipient_player_id: "player-private".to_string(),
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
    .expect("private payload should send");

    match client
        .next_event(Duration::from_secs(2))
        .expect("private event")
    {
        ClientRuntimeEvent::PrivateHoleCards(payload) => {
            assert_eq!(payload.recipient_player_id, "player-private");
            assert_eq!(payload.hole_cards.len(), 2);
        }
        other => panic!("expected private payload, got {other:?}"),
    }
}

#[test]
fn reconnect_succeeds_only_with_original_keypair_and_valid_token() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let original_signing_keys = provider.generate_signing_keypair();
    let original_encryption_keys = provider.generate_encryption_keypair();

    let host = HostServer::bind(HostRuntimeConfig {
        bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
        advertised_host: "127.0.0.1".to_string(),
        session_epoch: 21,
        table_id: "table-reconnect-success".to_string(),
        table_name: Some("Reconnect Success".to_string()),
        join_token: "join-token".to_string(),
        host_signing_keys,
        host_encryption_keys,
        snapshot_state: sample_tournament_state("table-reconnect-success", 21),
        runtime_mode: HostRuntimeMode::Test,
    })
    .expect("host should bind");

    let client = ClientRuntime::connect(ClientRuntimeConfig {
        join_payload: host.encoded_join_payload().to_string(),
        player_id: "player-reconnect".to_string(),
        display_name: "Reconnect".to_string(),
        signing_keys: original_signing_keys,
        encryption_keys: original_encryption_keys,
    })
    .expect("client should connect");

    let initial_snapshot = client
        .next_event(Duration::from_secs(2))
        .expect("initial snapshot");
    match initial_snapshot {
        ClientRuntimeEvent::Snapshot(snapshot) => {
            assert!(snapshot.reconnect_token.is_some());
        }
        other => panic!("expected snapshot event, got {other:?}"),
    }

    disconnect_client(&host, "player-reconnect");

    match client
        .next_event(Duration::from_secs(2))
        .expect("reconnecting event")
    {
        ClientRuntimeEvent::Reconnecting { player_id } => {
            assert_eq!(player_id, "player-reconnect");
        }
        other => panic!("expected reconnecting event, got {other:?}"),
    }

    match client
        .next_event(Duration::from_secs(2))
        .expect("reconnected snapshot")
    {
        ClientRuntimeEvent::Snapshot(snapshot) => {
            assert_eq!(snapshot.local_player_id, "player-reconnect");
            assert!(snapshot.reconnect_token.is_some());
        }
        other => panic!("expected reconnected snapshot, got {other:?}"),
    }

    wait_for_host_participant_state(
        &host,
        "player-reconnect",
        ParticipantState::Admitted,
        ConnectionState::Connected,
    );
}

#[test]
fn reconnect_fails_with_regenerated_keypair() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

    let host = HostServer::bind(HostRuntimeConfig {
        bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
        advertised_host: "127.0.0.1".to_string(),
        session_epoch: 22,
        table_id: "table-reconnect-fail".to_string(),
        table_name: Some("Reconnect Fail".to_string()),
        join_token: "join-token".to_string(),
        host_signing_keys,
        host_encryption_keys,
        snapshot_state: sample_tournament_state("table-reconnect-fail", 22),
        runtime_mode: HostRuntimeMode::Test,
    })
    .expect("host should bind");

    let client = ClientRuntime::connect(ClientRuntimeConfig {
        join_payload: host.encoded_join_payload().to_string(),
        player_id: "player-regenerated".to_string(),
        display_name: "Regenerated".to_string(),
        signing_keys: provider.generate_signing_keypair(),
        encryption_keys: provider.generate_encryption_keypair(),
    })
    .expect("client should connect");
    let _ = client.next_event(Duration::from_secs(2)).expect("snapshot");

    client
        .replace_reconnect_identity(
            provider.generate_signing_keypair(),
            provider.generate_encryption_keypair(),
        )
        .expect("replace reconnect identity");

    disconnect_client(&host, "player-regenerated");

    match client
        .next_event(Duration::from_secs(2))
        .expect("reconnecting event")
    {
        ClientRuntimeEvent::Reconnecting { player_id } => {
            assert_eq!(player_id, "player-regenerated");
        }
        other => panic!("expected reconnecting event, got {other:?}"),
    }

    match client
        .next_event(Duration::from_secs(2))
        .expect("safe error event")
    {
        ClientRuntimeEvent::SafeError { player_id, message } => {
            assert_eq!(player_id, "player-regenerated");
            assert!(message.contains("signature verification failed"));
        }
        other => panic!("expected safe error event, got {other:?}"),
    }
}

#[test]
fn reconnect_rejects_stale_tokens() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let player_signing_keys = provider.generate_signing_keypair();
    let player_encryption_keys = provider.generate_encryption_keypair();
    let join_payload = sample_join_payload_for_tests(
        "table-reconnect-stale-token",
        31,
        host_signing_keys.public_key_base64(),
    );
    let server_sequence = Arc::new(AtomicU64::new(0));
    let mut state = sample_tournament_state("table-reconnect-stale-token", 31);
    state.participants.insert(
        "player-stale-token".to_string(),
        sample_participant_entry(
            "player-stale-token",
            "Stale Token",
            player_signing_keys.public_key_base64(),
            player_encryption_keys.public_key_base64(),
            ParticipantState::Reconnecting,
            ConnectionState::Reconnecting,
            Some(0),
            "expected-token",
        ),
    );
    let authoritative_state = Arc::new(Mutex::new(state));

    let result = handle_reconnect_request(
        &provider,
        signed_reconnect_envelope(
            &provider,
            &player_signing_keys,
            &join_payload,
            ReconnectTournamentRequest {
                player_id: "player-stale-token".to_string(),
                reconnect_token: "wrong-token".to_string(),
                ..sample_reconnect_request(Some(1))
            },
            1,
            "reconnect-stale-token",
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    );

    assert!(result.is_err());
    assert!(result
        .expect_err("reconnect should fail")
        .to_string()
        .contains("reconnect token mismatch"));
}

#[test]
fn reconnect_after_disconnect_retries_until_prior_connection_cleans_up() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-reconnect-race", 31);
    let join_payload = crate::protocol::decode_join_payload(host.encoded_join_payload())
        .expect("join payload should decode");
    let reconnect_identity = Arc::new(Mutex::new(ClientReconnectIdentity {
        signing_keys: Some(provider.generate_signing_keypair()),
        encryption_keys: Some(provider.generate_encryption_keypair()),
    }));
    let (original_stream, snapshot) = connect_and_join(
        &provider,
        &join_payload,
        "player-race",
        "Race",
        &reconnect_identity,
    )
    .expect("client should join");
    let reconnect_token = snapshot
        .payload
        .reconnect_token
        .clone()
        .expect("reconnect token");

    wait_for_host_participant_state(
        &host,
        "player-race",
        ParticipantState::Admitted,
        ConnectionState::Connected,
    );

    let mut next_counter = 2;
    let (reconnected_stream, reconnected_snapshot) = thread::scope(|scope| {
        scope.spawn(|| {
            thread::sleep(Duration::from_millis(60));
            original_stream
                .shutdown(Shutdown::Both)
                .expect("shutdown original stream");
        });

        reconnect_after_disconnect(
            &provider,
            &join_payload,
            "player-race",
            &reconnect_identity,
            Some(reconnect_token.as_str()),
            snapshot.server_sequence.unwrap_or(0),
            &mut next_counter,
        )
    })
    .expect("reconnect should succeed after cleanup");

    assert_eq!(reconnected_snapshot.payload.local_player_id, "player-race");
    wait_for_host_participant_state(
        &host,
        "player-race",
        ParticipantState::Admitted,
        ConnectionState::Connected,
    );
    let state = host.authoritative_state().expect("authoritative state");
    let participant = state.participants.get("player-race").expect("participant");
    assert!(participant.reconnect_expiry_ms.is_none());
    assert_eq!(host.clients.lock().expect("client registry").len(), 1);
    let _ = reconnected_stream.shutdown(Shutdown::Both);
}

#[test]
fn reconnect_after_disconnect_reports_retry_exhaustion_without_creating_a_duplicate_session() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-reconnect-exhausted", 31);
    let join_payload = crate::protocol::decode_join_payload(host.encoded_join_payload())
        .expect("join payload should decode");
    let reconnect_identity = Arc::new(Mutex::new(ClientReconnectIdentity {
        signing_keys: Some(provider.generate_signing_keypair()),
        encryption_keys: Some(provider.generate_encryption_keypair()),
    }));
    let (original_stream, snapshot) = connect_and_join(
        &provider,
        &join_payload,
        "player-exhausted",
        "Exhausted",
        &reconnect_identity,
    )
    .expect("client should join");
    let reconnect_token = snapshot
        .payload
        .reconnect_token
        .clone()
        .expect("reconnect token");

    wait_for_host_participant_state(
        &host,
        "player-exhausted",
        ParticipantState::Admitted,
        ConnectionState::Connected,
    );

    let mut next_counter = 2;
    let error = reconnect_after_disconnect(
        &provider,
        &join_payload,
        "player-exhausted",
        &reconnect_identity,
        Some(reconnect_token.as_str()),
        snapshot.server_sequence.unwrap_or(0),
        &mut next_counter,
    )
    .expect_err("reconnect should exhaust retries while the original stream stays connected");

    assert_eq!(
        error.to_string(),
        "reconnect retries exhausted while waiting for prior connection cleanup"
    );
    let state = host.authoritative_state().expect("authoritative state");
    let participant = state
        .participants
        .get("player-exhausted")
        .expect("participant");
    assert_eq!(participant.connection_state, ConnectionState::Connected);
    assert_eq!(participant.state, ParticipantState::Admitted);
    assert_eq!(host.clients.lock().expect("client registry").len(), 1);
    let _ = original_stream.shutdown(Shutdown::Both);
}

#[test]
fn reconnect_rejects_already_connected_participants() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let player_signing_keys = provider.generate_signing_keypair();
    let player_encryption_keys = provider.generate_encryption_keypair();
    let join_payload = sample_join_payload_for_tests(
        "table-reconnect-connected",
        32,
        host_signing_keys.public_key_base64(),
    );
    let server_sequence = Arc::new(AtomicU64::new(4));
    let mut state = sample_tournament_state("table-reconnect-connected", 32);
    state.phase = TournamentPhase::Running;
    state.participants.insert(
        "player-connected".to_string(),
        sample_participant_entry(
            "player-connected",
            "Connected",
            player_signing_keys.public_key_base64(),
            player_encryption_keys.public_key_base64(),
            ParticipantState::Active,
            ConnectionState::Connected,
            Some(0),
            "connected-token",
        ),
    );
    let authoritative_state = Arc::new(Mutex::new(state));

    let result = handle_reconnect_request(
        &provider,
        signed_reconnect_envelope(
            &provider,
            &player_signing_keys,
            &join_payload,
            ReconnectTournamentRequest {
                player_id: "player-connected".to_string(),
                reconnect_token: "connected-token".to_string(),
                last_known_server_seq: Some(4),
            },
            1,
            "reconnect-connected",
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    );

    assert!(result.is_err());
    assert!(result
        .expect_err("reconnect should fail")
        .to_string()
        .contains("already connected"));
}

#[test]
fn reconnect_rejects_removed_participants() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let player_signing_keys = provider.generate_signing_keypair();
    let player_encryption_keys = provider.generate_encryption_keypair();
    let join_payload = sample_join_payload_for_tests(
        "table-reconnect-removed",
        33,
        host_signing_keys.public_key_base64(),
    );
    let server_sequence = Arc::new(AtomicU64::new(2));
    let mut state = sample_tournament_state("table-reconnect-removed", 33);
    state.participants.insert(
        "player-removed".to_string(),
        sample_participant_entry(
            "player-removed",
            "Removed",
            player_signing_keys.public_key_base64(),
            player_encryption_keys.public_key_base64(),
            ParticipantState::Removed,
            ConnectionState::Reconnecting,
            Some(0),
            "removed-token",
        ),
    );
    let authoritative_state = Arc::new(Mutex::new(state));

    let result = handle_reconnect_request(
        &provider,
        signed_reconnect_envelope(
            &provider,
            &player_signing_keys,
            &join_payload,
            ReconnectTournamentRequest {
                player_id: "player-removed".to_string(),
                reconnect_token: "removed-token".to_string(),
                last_known_server_seq: Some(2),
            },
            1,
            "reconnect-removed",
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    );

    assert!(result.is_err());
    assert!(result
        .expect_err("reconnect should fail")
        .to_string()
        .contains("not reconnect-eligible"));
}

#[test]
fn reconnect_after_tournament_complete_restores_seated_state() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let player_signing_keys = provider.generate_signing_keypair();
    let player_encryption_keys = provider.generate_encryption_keypair();
    let join_payload = sample_join_payload_for_tests(
        "table-reconnect-complete",
        34,
        host_signing_keys.public_key_base64(),
    );
    let server_sequence = Arc::new(AtomicU64::new(8));
    let mut state = sample_tournament_state("table-reconnect-complete", 34);
    state.phase = TournamentPhase::Complete;
    state.participants.insert(
        "player-complete".to_string(),
        sample_participant_entry(
            "player-complete",
            "Complete",
            player_signing_keys.public_key_base64(),
            player_encryption_keys.public_key_base64(),
            ParticipantState::Reconnecting,
            ConnectionState::Reconnecting,
            Some(0),
            "complete-token",
        ),
    );
    let authoritative_state = Arc::new(Mutex::new(state));

    let accepted = handle_reconnect_request(
        &provider,
        signed_reconnect_envelope(
            &provider,
            &player_signing_keys,
            &join_payload,
            ReconnectTournamentRequest {
                player_id: "player-complete".to_string(),
                reconnect_token: "complete-token".to_string(),
                last_known_server_seq: Some(8),
            },
            1,
            "reconnect-complete",
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    )
    .expect("reconnect should succeed");

    assert_eq!(accepted.player_id, "player-complete");
    let state = authoritative_state.lock().expect("authoritative state");
    let participant = state
        .participants
        .get("player-complete")
        .expect("participant after reconnect");
    assert_eq!(participant.state, ParticipantState::Seated);
    assert_eq!(participant.connection_state, ConnectionState::Connected);
}

#[test]
fn reconnect_preserves_eliminated_observer_state() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let player_signing_keys = provider.generate_signing_keypair();
    let player_encryption_keys = provider.generate_encryption_keypair();
    let join_payload = sample_join_payload_for_tests(
        "table-reconnect-observer",
        35,
        host_signing_keys.public_key_base64(),
    );
    let server_sequence = Arc::new(AtomicU64::new(3));
    let mut state = sample_tournament_state("table-reconnect-observer", 35);
    state.phase = TournamentPhase::Running;
    state.participants.insert(
        "player-observer".to_string(),
        sample_participant_entry(
            "player-observer",
            "Observer",
            player_signing_keys.public_key_base64(),
            player_encryption_keys.public_key_base64(),
            ParticipantState::EliminatedObserver,
            ConnectionState::Reconnecting,
            Some(2),
            "observer-token",
        ),
    );
    let authoritative_state = Arc::new(Mutex::new(state));

    let accepted = handle_reconnect_request(
        &provider,
        signed_reconnect_envelope(
            &provider,
            &player_signing_keys,
            &join_payload,
            ReconnectTournamentRequest {
                player_id: "player-observer".to_string(),
                reconnect_token: "observer-token".to_string(),
                last_known_server_seq: Some(3),
            },
            1,
            "reconnect-observer",
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    )
    .expect("observer reconnect should succeed");

    assert_eq!(accepted.player_id, "player-observer");
    let state = authoritative_state.lock().expect("authoritative state");
    let participant = state
        .participants
        .get("player-observer")
        .expect("participant after reconnect");
    assert_eq!(participant.state, ParticipantState::EliminatedObserver);
    assert_eq!(participant.connection_state, ConnectionState::Connected);
}
