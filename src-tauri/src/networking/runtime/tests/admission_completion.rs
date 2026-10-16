//! DP-REVIEW-001 transactional admission: deterministic fault injection over
//! the production `complete_admission` path, registry generation ownership
//! (N-3), lock/IO separation (N-4) and real-TCP abrupt disconnects.

use std::{
    collections::HashMap,
    io::Read,
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use super::super::{
    client_handle, complete_admission, handle_join_request, handle_reconnect_request,
    register_client, retire_failed_clients, write_to_client, ActiveClientSlotTracker,
    AdmissionFaults, AdmissionStage, HostRuntimeHealth, HostSessionContext,
    InitialRequestAcceptance, PendingAdmissionRollback,
};
use super::support::*;
use crate::{
    crypto::{
        DefaultCryptoProvider, EncryptionKeyMaterial, ProtocolCryptoProvider, SigningKeyMaterial,
    },
    domain::{ConnectionState, JoinPayload, ParticipantState, TournamentPhase, TournamentState},
    networking::{read_json_frame, write_json_frame, PublicEventLogEntry},
    protocol::{
        test_support::sample_reconnect_request, JsonSignedEnvelope, ProtocolMessageType,
        ReconnectTournamentRequest,
    },
};

const RECONNECT_TOKEN: &str = "fixture-reconnect-token";

struct Fixture {
    provider: DefaultCryptoProvider,
    host_signing: Arc<SigningKeyMaterial>,
    host_encryption: Arc<Mutex<EncryptionKeyMaterial>>,
    payload: JoinPayload,
    state: Arc<Mutex<TournamentState>>,
    sequence: Arc<AtomicU64>,
    clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
    health: Arc<Mutex<HostRuntimeHealth>>,
    slots: Arc<ActiveClientSlotTracker>,
    transition_lock: Arc<Mutex<()>>,
    public_events: Arc<Mutex<Vec<PublicEventLogEntry>>>,
}

impl Fixture {
    fn new(table_id: &str, epoch: u64) -> Self {
        let provider = DefaultCryptoProvider;
        let host_signing = Arc::new(provider.generate_signing_keypair());
        let payload =
            sample_join_payload_for_tests(table_id, epoch, host_signing.public_key_base64());
        Self {
            host_encryption: Arc::new(Mutex::new(provider.generate_encryption_keypair())),
            host_signing,
            payload,
            state: Arc::new(Mutex::new(sample_tournament_state(table_id, epoch))),
            sequence: Arc::new(AtomicU64::new(0)),
            clients: Arc::new(Mutex::new(HashMap::new())),
            health: Arc::new(Mutex::new(HostRuntimeHealth::default())),
            slots: Arc::new(ActiveClientSlotTracker::new(4)),
            transition_lock: Arc::new(Mutex::new(())),
            public_events: Arc::new(Mutex::new(Vec::new())),
            provider,
        }
    }

    fn context(&self) -> HostSessionContext {
        HostSessionContext {
            authoritative_state: Arc::clone(&self.state),
            tournament_runtime: Arc::new(Mutex::new(None)),
            transition_lock: Arc::clone(&self.transition_lock),
            clients: Arc::clone(&self.clients),
            join_payload: self.payload.clone(),
            server_sequence: Arc::clone(&self.sequence),
            host_signing_keys: Arc::clone(&self.host_signing),
            host_encryption_keys: Arc::clone(&self.host_encryption),
            public_events: Arc::clone(&self.public_events),
            runtime_health: Arc::clone(&self.health),
        }
    }

    fn join(&self, player_id: &str) -> InitialRequestAcceptance {
        let signing = self.provider.generate_signing_keypair();
        let keys = self.provider.generate_encryption_keypair();
        handle_join_request(
            &self.provider,
            signed_join_envelope(
                &self.provider,
                &signing,
                &keys,
                &self.payload,
                player_id,
                "Fixture Player",
                &self.payload.join_token,
            ),
            &self.payload,
            &self.state,
            &self.sequence,
            &self.host_signing,
            &self.host_encryption,
        )
        .expect("join admission")
    }

    /// Insert a disconnected, reconnect-eligible seated participant and return
    /// its signing keys.
    fn insert_reconnecting(&self, player_id: &str) -> SigningKeyMaterial {
        let signing = self.provider.generate_signing_keypair();
        let keys = self.provider.generate_encryption_keypair();
        self.state.lock().unwrap().participants.insert(
            player_id.to_string(),
            sample_participant_entry(
                player_id,
                "Reconnecting Player",
                signing.public_key_base64(),
                keys.public_key_base64(),
                ParticipantState::Reconnecting,
                ConnectionState::Reconnecting,
                Some(0),
                RECONNECT_TOKEN,
            ),
        );
        signing
    }

    fn reconnect(
        &self,
        player_id: &str,
        signing: &SigningKeyMaterial,
        token: &str,
        counter: u64,
    ) -> Result<InitialRequestAcceptance, crate::networking::NetworkingError> {
        handle_reconnect_request(
            &self.provider,
            signed_reconnect_envelope(
                &self.provider,
                signing,
                &self.payload,
                ReconnectTournamentRequest {
                    player_id: player_id.to_string(),
                    reconnect_token: token.to_string(),
                    ..sample_reconnect_request(Some(0))
                },
                counter,
                &format!("reconnect-{counter}"),
            ),
            &self.payload,
            &self.state,
            &self.sequence,
            &self.host_signing,
            &self.host_encryption,
        )
    }

    fn participant(&self, player_id: &str) -> Option<crate::domain::ParticipantRegistryEntry> {
        self.state
            .lock()
            .unwrap()
            .participants
            .get(player_id)
            .cloned()
    }

    fn last_error(&self) -> String {
        self.health
            .lock()
            .unwrap()
            .last_error
            .clone()
            .unwrap_or_default()
    }
}

/// A connected (host side, peer side) loopback pair.
fn tcp_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let peer = TcpStream::connect(listener.local_addr().unwrap()).expect("connect");
    let (host_side, _) = listener.accept().expect("accept");
    (host_side, peer)
}

/// The host must have closed the transport: the peer reaches EOF (after any
/// already-delivered snapshot bytes) instead of timing out.
fn assert_peer_sees_close(mut peer: TcpStream) {
    peer.set_read_timeout(Some(Duration::from_secs(5)))
        .expect("peer timeout");
    let mut sink = Vec::new();
    peer.read_to_end(&mut sink)
        .expect("host must close the failed transport, not leave it open");
}

fn wait_until(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(10));
    }
}

const ALL_STAGES: [AdmissionStage; 6] = [
    AdmissionStage::SnapshotWrite,
    AdmissionStage::ClearReadTimeout,
    AdmissionStage::StreamClone,
    AdmissionStage::CatchUpSnapshotBuild,
    AdmissionStage::Register,
    AdmissionStage::SessionSpawn,
];

#[test]
fn failed_new_join_at_every_stage_releases_seat_slot_and_transport() {
    for stage in ALL_STAGES {
        let fixture = Fixture::new("table-join-fault", 60);
        let acceptance = fixture.join("faulty-joiner");
        let slot = fixture.slots.try_acquire().expect("slot");
        let (host_side, peer) = tcp_pair();

        let failure = complete_admission(
            fixture.context(),
            host_side,
            acceptance,
            slot,
            AdmissionFaults::fail_at(stage),
        )
        .expect_err("injected failure must surface");

        assert_eq!(failure.stage, stage);
        assert!(
            fixture.participant("faulty-joiner").is_none(),
            "{stage:?}: uncommitted join must be removed (no phantom seat)"
        );
        assert_eq!(fixture.slots.in_use(), 0, "{stage:?}: slot leaked");
        assert!(
            fixture.clients.lock().unwrap().is_empty(),
            "{stage:?}: registration leaked"
        );
        assert!(
            fixture.last_error().starts_with("admission failed during"),
            "{stage:?}: categorized diagnostic missing: {}",
            fixture.last_error()
        );
        assert!(
            !fixture.last_error().contains(RECONNECT_TOKEN),
            "diagnostic must not carry secrets"
        );
        assert_peer_sees_close(peer);

        // The same identity can retry immediately after the failed attempt.
        fixture.join("faulty-joiner");
    }
}

#[test]
fn peer_disconnected_before_initial_snapshot_can_retry_without_phantom_or_slot_leak() {
    let fixture = Fixture::new("table-disconnected-before-snapshot", 72);

    for attempt in 0..4 {
        let acceptance = fixture.join("disconnected-before-snapshot");
        let slot = fixture.slots.try_acquire().expect("active slot");
        let (host_side, peer) = tcp_pair();
        peer.shutdown(Shutdown::Both)
            .expect("close peer before snapshot");
        drop(peer);

        let failure = complete_admission(
            fixture.context(),
            host_side,
            acceptance,
            slot,
            AdmissionFaults::none(),
        )
        .expect_err("closed peer must fail initial snapshot delivery");

        assert_eq!(failure.stage, AdmissionStage::SnapshotWrite);
        assert!(fixture
            .participant("disconnected-before-snapshot")
            .is_none());
        assert_eq!(fixture.slots.in_use(), 0, "attempt {attempt}: slot leaked");
        assert!(fixture.clients.lock().unwrap().is_empty());
    }

    let retry = fixture.join("disconnected-before-snapshot");
    assert_eq!(retry.player_id, "disconnected-before-snapshot");
}

#[test]
fn failed_reconnect_at_every_stage_restores_eligibility_and_later_retry_succeeds() {
    for stage in ALL_STAGES {
        let fixture = Fixture::new("table-reconnect-fault", 61);
        let signing = fixture.insert_reconnecting("returning");
        let before = fixture.participant("returning").unwrap();
        let acceptance = fixture
            .reconnect("returning", &signing, RECONNECT_TOKEN, 1)
            .expect("reconnect admission");
        let slot = fixture.slots.try_acquire().expect("slot");
        let (host_side, peer) = tcp_pair();

        complete_admission(
            fixture.context(),
            host_side,
            acceptance,
            slot,
            AdmissionFaults::fail_at(stage),
        )
        .expect_err("injected failure must surface");

        assert_eq!(
            fixture.participant("returning").unwrap(),
            before,
            "{stage:?}: reconnect state, token and expiry must be restored"
        );
        assert_eq!(fixture.slots.in_use(), 0, "{stage:?}: slot leaked");
        assert!(fixture.clients.lock().unwrap().is_empty());
        assert_peer_sees_close(peer);

        // Misuse is still rejected after the rollback ...
        let wrong_key = fixture.provider.generate_signing_keypair();
        assert!(fixture
            .reconnect("returning", &signing, "wrong-token", 2)
            .is_err());
        assert!(fixture
            .reconnect("returning", &wrong_key, RECONNECT_TOKEN, 3)
            .is_err());
        // ... and a legitimate retry succeeds.
        fixture
            .reconnect("returning", &signing, RECONNECT_TOKEN, 4)
            .expect("later valid reconnect must succeed");
    }
}

#[test]
fn poisoned_client_registry_fails_registration_and_rolls_back() {
    let fixture = Fixture::new("table-poisoned-registry", 62);
    let acceptance = fixture.join("poisoned-registry-joiner");
    let poison = Arc::clone(&fixture.clients);
    assert!(thread::spawn(move || {
        let _guard = poison.lock().unwrap();
        panic!("synthetic registry poison");
    })
    .join()
    .is_err());
    let (host_side, peer) = tcp_pair();

    let failure = complete_admission(
        fixture.context(),
        host_side,
        acceptance,
        fixture.slots.try_acquire().unwrap(),
        AdmissionFaults::none(),
    )
    .expect_err("poisoned registry must fail admission");

    assert_eq!(failure.stage, AdmissionStage::Register);
    assert!(fixture.participant("poisoned-registry-joiner").is_none());
    assert_eq!(fixture.slots.in_use(), 0);
    assert_eq!(
        fixture.health.lock().unwrap().client_registry_error_count,
        1
    );
    assert_peer_sees_close(peer);
}

#[test]
fn registration_never_replaces_an_existing_connection() {
    let fixture = Fixture::new("table-replacement", 63);
    let acceptance = fixture.join("contested");
    let (existing_host_side, _existing_peer) = tcp_pair();
    register_client(
        &fixture.clients,
        "contested",
        ConnectedClient {
            stream: Arc::new(Mutex::new(existing_host_side)),
            encryption_public_key: "existing-key".to_string(),
            generation: 7,
        },
    )
    .unwrap();
    let (host_side, peer) = tcp_pair();

    let failure = complete_admission(
        fixture.context(),
        host_side,
        acceptance,
        fixture.slots.try_acquire().unwrap(),
        AdmissionFaults::none(),
    )
    .expect_err("replacement conflict must fail admission");

    assert_eq!(failure.stage, AdmissionStage::Register);
    let existing = client_handle(&fixture.clients, "contested")
        .unwrap()
        .expect("existing registration kept");
    assert_eq!(existing.generation, 7);
    assert_eq!(existing.encryption_public_key, "existing-key");
    assert_peer_sees_close(peer);
}

#[test]
fn successful_admission_registers_then_session_exit_demotes_once() {
    let fixture = Fixture::new("table-success", 64);
    let acceptance = fixture.join("happy-joiner");
    let generation = acceptance.generation;
    let (host_side, mut peer) = tcp_pair();

    complete_admission(
        fixture.context(),
        host_side,
        acceptance,
        fixture.slots.try_acquire().unwrap(),
        AdmissionFaults::none(),
    )
    .expect("admission completes");

    peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let snapshot: JsonSignedEnvelope = read_json_frame(&mut peer).expect("initial snapshot");
    assert_eq!(snapshot.message_type, ProtocolMessageType::SnapshotEvent);
    let registered = client_handle(&fixture.clients, "happy-joiner")
        .unwrap()
        .expect("registered");
    assert_eq!(registered.generation, generation);
    assert_eq!(
        fixture
            .participant("happy-joiner")
            .unwrap()
            .connection_state,
        ConnectionState::Connected,
        "a successful attempt must not be rolled back"
    );
    assert_eq!(fixture.slots.in_use(), 1, "session owns the slot");

    peer.shutdown(Shutdown::Both).unwrap();
    wait_until("session exit", || fixture.slots.in_use() == 0);

    assert!(fixture.clients.lock().unwrap().is_empty());
    assert_eq!(
        fixture
            .participant("happy-joiner")
            .unwrap()
            .connection_state,
        ConnectionState::Reconnecting
    );
}

#[test]
fn admission_snapshot_sync_covers_public_event_before_registration() {
    let fixture = Fixture::new("table-admission-snapshot-gap", 71);
    let acceptance = fixture.join("snapshot-gap-client");
    let initial_sequence = acceptance
        .snapshot_envelope
        .server_sequence
        .expect("initial snapshot sequence");
    let initial_name = acceptance
        .snapshot_envelope
        .payload
        .state
        .config
        .tournament_name
        .clone();
    let changed_name = "Changed while snapshot was in flight";

    {
        let mut state = fixture.state.lock().unwrap();
        state.phase = TournamentPhase::ReadyCheck;
        state.config.tournament_name = changed_name.to_string();
    }
    let event_sequence = initial_sequence + 1;
    fixture.sequence.store(event_sequence, Ordering::SeqCst);
    fixture
        .public_events
        .lock()
        .unwrap()
        .push(PublicEventLogEntry {
            sequence: event_sequence,
            message_type: ProtocolMessageType::TournamentStartedEvent,
            payload: serde_json::json!({"tournamentName": changed_name}),
        });

    let (host_side, mut peer) = tcp_pair();
    complete_admission(
        fixture.context(),
        host_side,
        acceptance,
        fixture.slots.try_acquire().unwrap(),
        AdmissionFaults::none(),
    )
    .expect("admission with a catch-up snapshot completes");

    peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let initial: JsonSignedEnvelope = read_json_frame(&mut peer).expect("initial snapshot");
    let catch_up: JsonSignedEnvelope = read_json_frame(&mut peer).expect("catch-up snapshot");
    assert_eq!(initial.message_type, ProtocolMessageType::SnapshotEvent);
    assert_eq!(catch_up.message_type, ProtocolMessageType::SnapshotEvent);
    assert_eq!(
        initial.payload["state"]["config"]["tournamentName"].as_str(),
        Some(initial_name.as_str())
    );
    assert_eq!(
        catch_up.payload["state"]["config"]["tournamentName"].as_str(),
        Some(changed_name)
    );
    assert!(
        catch_up.server_sequence.expect("catch-up sequence")
            > initial.server_sequence.expect("initial sequence")
    );

    peer.shutdown(Shutdown::Both).unwrap();
}

#[test]
fn stale_session_exit_does_not_retire_or_demote_newer_registration() {
    let fixture = Fixture::new("table-stale-session", 65);
    let acceptance = fixture.join("rejoined");
    let (host_side, peer) = tcp_pair();
    complete_admission(
        fixture.context(),
        host_side,
        acceptance,
        fixture.slots.try_acquire().unwrap(),
        AdmissionFaults::none(),
    )
    .expect("first admission completes");

    // Model a newer connection that replaced the first registration (the old
    // session has not observed its disconnect yet).
    let (newer_host_side, _newer_peer) = tcp_pair();
    {
        let mut clients = fixture.clients.lock().unwrap();
        clients.insert(
            "rejoined".to_string(),
            ConnectedClient {
                stream: Arc::new(Mutex::new(newer_host_side)),
                encryption_public_key: "newer-key".to_string(),
                generation: u64::MAX,
            },
        );
    }

    peer.shutdown(Shutdown::Both).unwrap();
    wait_until("stale session exit", || fixture.slots.in_use() == 0);

    assert_eq!(
        client_handle(&fixture.clients, "rejoined")
            .unwrap()
            .expect("newer registration kept")
            .generation,
        u64::MAX
    );
    assert_eq!(
        fixture.participant("rejoined").unwrap().connection_state,
        ConnectionState::Connected,
        "stale session must not demote the newer connection"
    );
}

#[test]
fn failed_write_on_stale_handle_does_not_retire_newer_registration() {
    let fixture = Fixture::new("table-stale-writer", 66);
    let (stale_side, _stale_peer) = tcp_pair();
    register_client(
        &fixture.clients,
        "writer-target",
        ConnectedClient {
            stream: Arc::new(Mutex::new(stale_side)),
            encryption_public_key: "stale".to_string(),
            generation: 1,
        },
    )
    .unwrap();
    let stale_handle = client_handle(&fixture.clients, "writer-target")
        .unwrap()
        .unwrap();
    // The player reconnects: registration generation 2 replaces generation 1.
    fixture.clients.lock().unwrap().remove("writer-target");
    let (newer_side, _newer_peer) = tcp_pair();
    register_client(
        &fixture.clients,
        "writer-target",
        ConnectedClient {
            stream: Arc::new(Mutex::new(newer_side)),
            encryption_public_key: "newer".to_string(),
            generation: 2,
        },
    )
    .unwrap();

    // A broadcaster's write on the stale handle failed.
    retire_failed_clients(&fixture.clients, &[stale_handle], |player_id| {
        panic!("stale failure must not demote {player_id}")
    })
    .unwrap();

    assert_eq!(
        client_handle(&fixture.clients, "writer-target")
            .unwrap()
            .unwrap()
            .generation,
        2
    );
}

#[test]
fn blocked_client_write_does_not_hold_the_client_registry() {
    let fixture = Fixture::new("table-blocked-writer", 67);
    let (slow_side, _slow_peer) = tcp_pair();
    register_client(
        &fixture.clients,
        "slow",
        ConnectedClient {
            stream: Arc::new(Mutex::new(slow_side)),
            encryption_public_key: "slow".to_string(),
            generation: 1,
        },
    )
    .unwrap();
    let slow = client_handle(&fixture.clients, "slow").unwrap().unwrap();

    // Hold the slow client's stream as a stuck in-progress write would.
    let stream_lock = Arc::clone(&slow.stream);
    let (locked_tx, locked_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let holder = thread::spawn(move || {
        let _held = stream_lock.lock().unwrap();
        locked_tx.send(()).unwrap();
        release_rx.recv().unwrap();
    });
    locked_rx.recv().unwrap();
    let writer = thread::spawn(move || write_to_client(&slow, &serde_json::json!({"x": 1})));

    // While that write is blocked, admission and cleanup can still use the
    // registry.
    let (other_side, _other_peer) = tcp_pair();
    let (done_tx, done_rx) = mpsc::channel();
    let clients = Arc::clone(&fixture.clients);
    thread::spawn(move || {
        let result = register_client(
            &clients,
            "other",
            ConnectedClient {
                stream: Arc::new(Mutex::new(other_side)),
                encryption_public_key: "other".to_string(),
                generation: 2,
            },
        );
        done_tx.send(result).unwrap();
    });
    done_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("registry must not be held by a blocked client write")
        .unwrap();

    release_tx.send(()).unwrap();
    holder.join().unwrap();
    writer
        .join()
        .unwrap()
        .expect("write completes once unblocked");
}

#[test]
fn reconnect_rollback_keeps_gameplay_changes_made_while_pending() {
    let fixture = Fixture::new("table-precise-rollback", 68);
    let signing = fixture.insert_reconnecting("eliminated-meanwhile");
    let before = fixture.participant("eliminated-meanwhile").unwrap();
    let acceptance = fixture
        .reconnect("eliminated-meanwhile", &signing, RECONNECT_TOKEN, 1)
        .expect("reconnect admission");
    let pending = PendingAdmissionRollback::new(&fixture.state, &acceptance, &fixture.health);

    // A hand settles while the snapshot is in flight and eliminates the player.
    fixture
        .state
        .lock()
        .unwrap()
        .participants
        .get_mut("eliminated-meanwhile")
        .unwrap()
        .state = ParticipantState::EliminatedObserver;

    drop(pending);

    let after = fixture.participant("eliminated-meanwhile").unwrap();
    assert_eq!(
        after.state,
        ParticipantState::EliminatedObserver,
        "rollback must not resurrect an eliminated player"
    );
    assert_eq!(after.connection_state, before.connection_state);
    assert_eq!(after.reconnect_expiry_ms, before.reconnect_expiry_ms);
    assert_eq!(after.reconnect_token, before.reconnect_token);
}

#[test]
fn abrupt_disconnects_after_join_request_leave_no_connected_phantom_or_slot_leak() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-abrupt-join", 69);
    let payload = host.join_payload().clone();
    let signing = provider.generate_signing_keypair();
    let keys = provider.generate_encryption_keypair();

    // More attempts than the active-client slot limit (max_players + 1), so a
    // leaked slot would make the final join fail.
    let max_players = usize::from(host.authoritative_state().unwrap().config.max_players);
    for attempt in 0..(max_players + 1) * 2 {
        let mut stream = TcpStream::connect(host.listener_addr()).expect("connect");
        write_json_frame(
            &mut stream,
            &signed_join_envelope(
                &provider,
                &signing,
                &keys,
                &payload,
                "abrupt-joiner",
                "Abrupt",
                &payload.join_token,
            ),
        )
        .expect("send join request");
        // Disconnect immediately after transmitting the request. The host
        // settles state before closing, so EOF means this attempt is done.
        stream.shutdown(Shutdown::Write).expect("abrupt close");
        assert_peer_sees_close(stream);

        let state = host.authoritative_state().unwrap();
        if let Some(participant) = state.participants.get("abrupt-joiner") {
            assert_ne!(
                participant.connection_state,
                ConnectionState::Connected,
                "attempt {attempt}: connected phantom without a live session"
            );
        }
        assert!(
            !host.clients.lock().unwrap().contains_key("abrupt-joiner"),
            "attempt {attempt}: stale registration"
        );
    }

    let client = connect_test_client(&provider, &host, "after-abrupt", "After Abrupt");
    let snapshot = expect_snapshot_event(&client);
    assert_eq!(snapshot.local_player_id, "after-abrupt");
    assert!(
        snapshot.state.participants.contains_key("after-abrupt"),
        "a fresh client must still be admitted after the abrupt attempts"
    );
}

#[test]
fn concurrent_same_identity_attempts_have_exactly_one_winner() {
    const RACERS: usize = 8;
    for _ in 0..20 {
        // Joins racing for one playerId.
        let fixture = Arc::new(Fixture::new("table-join-race", 70));
        let barrier = Arc::new(std::sync::Barrier::new(RACERS));
        let joins = (0..RACERS)
            .map(|_| {
                let fixture = Arc::clone(&fixture);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    let signing = fixture.provider.generate_signing_keypair();
                    let keys = fixture.provider.generate_encryption_keypair();
                    let envelope = signed_join_envelope(
                        &fixture.provider,
                        &signing,
                        &keys,
                        &fixture.payload,
                        "racer",
                        "Racer",
                        &fixture.payload.join_token,
                    );
                    barrier.wait();
                    handle_join_request(
                        &fixture.provider,
                        envelope,
                        &fixture.payload,
                        &fixture.state,
                        &fixture.sequence,
                        &fixture.host_signing,
                        &fixture.host_encryption,
                    )
                    .is_ok()
                })
            })
            .collect::<Vec<_>>();
        let winners = joins
            .into_iter()
            .map(|racer| racer.join().expect("racer thread"))
            .filter(|won| *won)
            .count();
        assert_eq!(winners, 1, "exactly one concurrent join may win");

        // Reconnects racing with the same valid token.
        let fixture = Arc::new(Fixture::new("table-reconnect-race", 71));
        let signing = Arc::new(fixture.insert_reconnecting("returning-racer"));
        let barrier = Arc::new(std::sync::Barrier::new(RACERS));
        let reconnects = (0..RACERS as u64)
            .map(|counter| {
                let fixture = Arc::clone(&fixture);
                let signing = Arc::clone(&signing);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    fixture
                        .reconnect("returning-racer", &signing, RECONNECT_TOKEN, counter + 1)
                        .map_err(|error| error.code().map(str::to_owned))
                })
            })
            .collect::<Vec<_>>();
        let results = reconnects
            .into_iter()
            .map(|racer| racer.join().expect("racer thread"))
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert!(results
            .iter()
            .filter(|result| result.is_err())
            .all(|result| {
                result.as_ref().unwrap_err().as_deref()
                    == Some(crate::protocol::ERROR_CODE_RECONNECT_ALREADY_CONNECTED)
            }));
    }
}
