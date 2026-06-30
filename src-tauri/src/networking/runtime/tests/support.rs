use std::{
    collections::BTreeMap,
    net::Shutdown,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use base64::Engine as _;

use super::super::now_epoch_ms;
use crate::{
    crypto::{key_fingerprint, DefaultCryptoProvider, ProtocolCryptoProvider},
    domain::{
        BlindLevel, BlindSchedule, ConnectionState, JoinPayload, ParticipantRegistryEntry,
        ParticipantState, PlayerIdentity, TournamentConfig, TournamentPhase, TournamentState,
    },
    networking::{
        ClientRuntime, ClientRuntimeConfig, ClientRuntimeEvent, HostRuntimeConfig, HostRuntimeMode,
        HostServer,
    },
    protocol::{
        JoinTournamentRequest, JsonSignedEnvelope, PrivateHoleCardsEvent, ProtocolMessageType,
        ReconnectTournamentRequest, ResyncRequest, SignedEnvelope, SnapshotEvent, PROTOCOL_VERSION,
    },
};

pub(super) fn sample_tournament_state(table_id: &str, session_epoch: u64) -> TournamentState {
    TournamentState {
        table_id: table_id.to_string(),
        session_epoch,
        phase: TournamentPhase::WaitingForPlayers,
        config: TournamentConfig {
            tournament_name: "LAN Test".to_string(),
            table_name: Some("Test Table".to_string()),
            max_players: 6,
            starting_stack: 1500,
            turn_timer_seconds: 20,
            blind_schedule: BlindSchedule {
                levels: vec![BlindLevel {
                    level_index: 1,
                    label: "Level 1".to_string(),
                    small_blind: 10,
                    big_blind: 20,
                    ante: 0,
                    duration_seconds: 180,
                }],
            },
        },
        blind_schedule: BlindSchedule {
            levels: vec![BlindLevel {
                level_index: 1,
                label: "Level 1".to_string(),
                small_blind: 10,
                big_blind: 20,
                ante: 0,
                duration_seconds: 180,
            }],
        },
        blind_level_index: 0,
        participants: BTreeMap::new(),
        seats: Vec::new(),
        current_hand: None,
        hand_results: Vec::new(),
        placements: Vec::new(),
    }
}

pub(super) fn bind_test_host(
    provider: &DefaultCryptoProvider,
    table_id: &str,
    session_epoch: u64,
) -> HostServer {
    bind_test_host_with_state(
        provider,
        table_id,
        session_epoch,
        sample_tournament_state(table_id, session_epoch),
    )
}

/// Bind a test host with a caller-supplied initial state, so tests can tune the
/// config (stacks, blinds, turn timer) for end-to-end scenarios.
pub(super) fn bind_test_host_with_state(
    provider: &DefaultCryptoProvider,
    table_id: &str,
    session_epoch: u64,
    snapshot_state: TournamentState,
) -> HostServer {
    let host_signing_keys = Arc::new(provider.generate_signing_keypair());
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));

    HostServer::bind(HostRuntimeConfig {
        bind_addr: "127.0.0.1:0".parse().expect("socket addr"),
        advertised_host: "127.0.0.1".to_string(),
        session_epoch,
        table_id: table_id.to_string(),
        table_name: Some(format!("Table {session_epoch}")),
        join_token: "join-token".to_string(),
        host_signing_keys,
        host_encryption_keys,
        snapshot_state,
        runtime_mode: HostRuntimeMode::Test,
    })
    .expect("host should bind")
}

pub(super) fn connect_test_client(
    provider: &DefaultCryptoProvider,
    host: &HostServer,
    player_id: &str,
    display_name: &str,
) -> ClientRuntime {
    ClientRuntime::connect(ClientRuntimeConfig {
        join_payload: host.encoded_join_payload().to_string(),
        player_id: player_id.to_string(),
        display_name: display_name.to_string(),
        signing_keys: provider.generate_signing_keypair(),
        encryption_keys: provider.generate_encryption_keypair(),
    })
    .expect("client should connect")
}

pub(super) fn expect_snapshot_event(client: &ClientRuntime) -> SnapshotEvent {
    match client
        .next_event(Duration::from_secs(2))
        .expect("snapshot event")
    {
        ClientRuntimeEvent::Snapshot(snapshot) => *snapshot,
        other => panic!("expected snapshot event, got {other:?}"),
    }
}

pub(super) fn assert_public_event(
    client: &ClientRuntime,
    expected_message_type: ProtocolMessageType,
) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        match client.next_event(Duration::from_millis(200)) {
            Ok(ClientRuntimeEvent::PublicEvent {
                message_type,
                payload,
                ..
            }) => {
                assert_eq!(message_type, expected_message_type);
                return payload;
            }
            Ok(ClientRuntimeEvent::Snapshot(_)) => {}
            Ok(ClientRuntimeEvent::Reconnecting { .. }) => {}
            Ok(ClientRuntimeEvent::Disconnected { .. }) => {}
            Ok(ClientRuntimeEvent::ResyncRequested { .. }) => {}
            Ok(ClientRuntimeEvent::ProtocolWarning { .. }) => {}
            Ok(ClientRuntimeEvent::PrivateHoleCards(other)) => {
                panic!("expected public event, got private hole cards: {other:?}");
            }
            Ok(ClientRuntimeEvent::SafeError { message, .. }) => {
                panic!("expected public event, got safe error: {message}");
            }
            // A poll-window timeout is not a failure under load; keep waiting
            // until the outer deadline rather than panicking on the first gap.
            Err(_) => {}
        }

        assert!(
            Instant::now() < deadline,
            "expected public event before timeout"
        );
    }
}

pub(super) fn wait_for_public_event(
    client: &ClientRuntime,
    expected_message_type: ProtocolMessageType,
) -> serde_json::Value {
    wait_for_public_event_where(client, expected_message_type, |_| true)
}

pub(super) fn wait_for_public_event_where(
    client: &ClientRuntime,
    expected_message_type: ProtocolMessageType,
    predicate: impl Fn(&serde_json::Value) -> bool,
) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        match client.next_event(Duration::from_millis(200)) {
            Ok(ClientRuntimeEvent::PublicEvent {
                message_type,
                payload,
                ..
            }) if message_type == expected_message_type && predicate(&payload) => return payload,
            Ok(ClientRuntimeEvent::Snapshot(_)) => {}
            Ok(ClientRuntimeEvent::PublicEvent { .. }) => {}
            Ok(ClientRuntimeEvent::PrivateHoleCards(_)) => {}
            Ok(ClientRuntimeEvent::Reconnecting { .. }) => {}
            Ok(ClientRuntimeEvent::Disconnected { .. }) => {}
            Ok(ClientRuntimeEvent::ResyncRequested { .. }) => {}
            Ok(ClientRuntimeEvent::ProtocolWarning { .. }) => {}
            Ok(ClientRuntimeEvent::SafeError { message, .. }) => {
                panic!("expected public event, got safe error: {message}");
            }
            // A poll-window timeout is not a failure under load; keep waiting
            // until the outer deadline rather than panicking on the first gap.
            Err(_) => {}
        }

        assert!(
            Instant::now() < deadline,
            "expected matching public event before timeout"
        );
    }
}

pub(super) fn wait_for_private_hole_cards(client: &ClientRuntime) -> PrivateHoleCardsEvent {
    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        match client.next_event(Duration::from_millis(200)) {
            Ok(ClientRuntimeEvent::PrivateHoleCards(payload)) => return payload,
            Ok(ClientRuntimeEvent::Snapshot(_)) => {}
            Ok(ClientRuntimeEvent::PublicEvent { .. }) => {}
            Ok(ClientRuntimeEvent::Reconnecting { .. }) => {}
            Ok(ClientRuntimeEvent::Disconnected { .. }) => {}
            Ok(ClientRuntimeEvent::ResyncRequested { .. }) => {}
            Ok(ClientRuntimeEvent::ProtocolWarning { .. }) => {}
            Ok(ClientRuntimeEvent::SafeError { message, .. }) => {
                panic!("expected private hole cards, got safe error: {message}");
            }
            // A poll-window timeout is not a failure under load; keep waiting
            // until the outer deadline rather than panicking on the first gap.
            Err(_) => {}
        }

        assert!(
            Instant::now() < deadline,
            "expected private hole cards before timeout"
        );
    }
}

pub(super) fn expect_snapshot_where(
    client: &ClientRuntime,
    predicate: impl Fn(&SnapshotEvent) -> bool,
) -> SnapshotEvent {
    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        // A poll-window timeout is not a failure under load; keep waiting until the
        // outer deadline rather than panicking on the first gap.
        if let Ok(ClientRuntimeEvent::Snapshot(snapshot)) =
            client.next_event(Duration::from_millis(200))
        {
            if predicate(&snapshot) {
                return *snapshot;
            }
        }

        assert!(
            Instant::now() < deadline,
            "expected matching snapshot before timeout"
        );
    }
}

pub(super) fn disconnect_client(host: &HostServer, player_id: &str) {
    let deadline = Instant::now() + Duration::from_secs(2);
    let stream = loop {
        if let Some(stream) = host
            .clients
            .lock()
            .expect("client registry")
            .get(player_id)
            .map(|client| {
                client
                    .stream
                    .lock()
                    .expect("client stream")
                    .try_clone()
                    .expect("clone stream")
            })
        {
            break stream;
        }

        assert!(Instant::now() < deadline, "connected client");
        thread::sleep(Duration::from_millis(20));
    };
    stream
        .shutdown(Shutdown::Both)
        .expect("shutdown client stream");
}

/// Write a raw JSON frame to the client's stream (from the host side).
/// Blocks until the client is registered (i.e., the join handshake completed).
pub(super) fn send_frame_to_client(host: &HostServer, player_id: &str, value: &serde_json::Value) {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut write_stream = loop {
        if let Some(stream) = host
            .clients
            .lock()
            .expect("client registry")
            .get(player_id)
            .map(|client| {
                client
                    .stream
                    .lock()
                    .expect("client stream")
                    .try_clone()
                    .expect("clone stream")
            })
        {
            break stream;
        }

        assert!(Instant::now() < deadline, "client did not connect in time");
        thread::sleep(Duration::from_millis(20));
    };
    crate::networking::write_json_frame(&mut write_stream, value).expect("write frame to client");
}

pub(super) fn wait_for_host_participant_state(
    host: &HostServer,
    player_id: &str,
    expected_state: ParticipantState,
    expected_connection_state: ConnectionState,
) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let state = host.authoritative_state().expect("authoritative state");
        if let Some(participant) = state.participants.get(player_id) {
            if participant.state == expected_state
                && participant.connection_state == expected_connection_state
            {
                return;
            }
        }

        assert!(
            Instant::now() < deadline,
            "participant state did not converge"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

pub(super) fn wait_for_client_command_connection(client: &ClientRuntime) {
    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        if client
            .command_connection
            .lock()
            .expect("client command connection")
            .stream
            .is_some()
        {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "client command connection should become available"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

pub(super) fn sample_join_payload_for_tests(
    table_id: &str,
    session_epoch: u64,
    host_signing_public_key: String,
) -> JoinPayload {
    JoinPayload {
        payload_version: PROTOCOL_VERSION,
        host_address: "127.0.0.1".to_string(),
        host_port: 43_818,
        table_id: table_id.to_string(),
        session_epoch,
        host_signing_public_key,
        join_token: "join-token".to_string(),
        generated_at_ms: 1,
        table_name: Some("Reconnect Test".to_string()),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn sample_participant_entry(
    player_id: &str,
    display_name: &str,
    signing_public_key: String,
    encryption_public_key: String,
    state: ParticipantState,
    connection_state: ConnectionState,
    seat_index: Option<u8>,
    reconnect_token: &str,
) -> ParticipantRegistryEntry {
    let signing_key_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(signing_public_key.as_bytes())
        .expect("valid signing key bytes");

    ParticipantRegistryEntry {
        identity: PlayerIdentity {
            player_id: player_id.to_string(),
            display_name: display_name.to_string(),
            signing_public_key,
            encryption_public_key,
            signing_key_fingerprint: key_fingerprint(&signing_key_bytes),
        },
        state,
        connection_state,
        seat_index,
        admitted_at_ms: 1,
        reconnect_token: Some(reconnect_token.to_string()),
        reconnect_expiry_ms: Some(now_epoch_ms() + 60_000),
        is_host: false,
    }
}

/// Poll `host.authoritative_state()` until the in-progress hand number is ≥ `n`.
/// Returns the authoritative `TournamentState` at that point.
pub(super) fn wait_for_hand_number(
    host: &HostServer,
    n: u32,
    deadline: Instant,
) -> TournamentState {
    loop {
        let state = host.authoritative_state().expect("authoritative state");
        if state
            .current_hand
            .as_ref()
            .map(|h| h.hand_number)
            .unwrap_or(0)
            >= n
        {
            return state;
        }
        assert!(
            Instant::now() < deadline,
            "expected hand {n} to start before deadline"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

pub(super) fn signed_reconnect_envelope(
    provider: &DefaultCryptoProvider,
    signing_keys: &crate::crypto::SigningKeyMaterial,
    join_payload: &JoinPayload,
    request: ReconnectTournamentRequest,
    counter: u64,
    message_id: &str,
) -> JsonSignedEnvelope {
    let mut envelope = SignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::ReconnectTournamentRequest,
        table_id: join_payload.table_id.clone(),
        session_epoch: join_payload.session_epoch,
        sender_id: request.player_id.clone(),
        counter,
        message_id: message_id.to_string(),
        server_sequence: None,
        payload: serde_json::to_value(request).expect("reconnect request payload"),
        signature: None,
    };
    envelope
        .sign(provider, signing_keys)
        .expect("reconnect request should sign");

    envelope
}

pub(super) fn signed_join_envelope(
    provider: &DefaultCryptoProvider,
    signing_keys: &crate::crypto::SigningKeyMaterial,
    encryption_keys: &crate::crypto::EncryptionKeyMaterial,
    join_payload: &JoinPayload,
    player_id: &str,
    display_name: &str,
    join_token: &str,
) -> JsonSignedEnvelope {
    let request = JoinTournamentRequest {
        display_name: display_name.to_string(),
        join_token: join_token.to_string(),
        signing_public_key: signing_keys.public_key_base64(),
        encryption_public_key: encryption_keys.public_key_base64(),
    };
    let mut envelope = SignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::JoinTournamentRequest,
        table_id: join_payload.table_id.clone(),
        session_epoch: join_payload.session_epoch,
        sender_id: player_id.to_string(),
        counter: 1,
        message_id: format!("join-test-{player_id}"),
        server_sequence: None,
        payload: serde_json::to_value(request).expect("join request payload"),
        signature: None,
    };
    envelope
        .sign(provider, signing_keys)
        .expect("join request should sign");

    envelope
}

pub(super) fn signed_resync_envelope(
    provider: &DefaultCryptoProvider,
    signing_keys: &crate::crypto::SigningKeyMaterial,
    join_payload: &JoinPayload,
    sender_id: &str,
    request: ResyncRequest,
    counter: u64,
    message_id: &str,
) -> JsonSignedEnvelope {
    let mut envelope = SignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::ResyncRequest,
        table_id: join_payload.table_id.clone(),
        session_epoch: join_payload.session_epoch,
        sender_id: sender_id.to_string(),
        counter,
        message_id: message_id.to_string(),
        server_sequence: None,
        payload: serde_json::to_value(request).expect("resync request payload"),
        signature: None,
    };
    envelope
        .sign(provider, signing_keys)
        .expect("resync request should sign");

    envelope
}
