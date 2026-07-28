use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicU64},
        mpsc::{Receiver, RecvTimeoutError},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use local_ip_address::list_afinet_netifas;
use serde::Serialize;
use serde_json::Value;

use crate::{
    crypto::{EncryptionKeyMaterial, SigningKeyMaterial},
    domain::{JoinPayload, TournamentState},
    protocol::{PrivateHoleCardsEvent, ProtocolMessageType, SignedEnvelope, SnapshotEvent},
    tournament::TournamentController,
};

mod client;
mod client_connect;
mod events;
mod handlers;
mod host;
mod host_broadcast;
mod host_session;
mod host_shutdown;
mod lobby;
mod reconnect;
mod snapshot;

// Re-export the moved free functions so the rest of the `runtime` module (and
// its tests) keeps the original flat namespace.
pub(crate) use client_connect::*;
pub(crate) use events::*;
pub(crate) use handlers::*;
pub(crate) use host_session::*;
pub(crate) use lobby::*;
pub(crate) use reconnect::*;
pub(crate) use snapshot::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkingError {
    message: String,
}

impl NetworkingError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NetworkingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NetworkingError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientRuntimePollError {
    Timeout,
    Disconnected,
}

impl std::fmt::Display for ClientRuntimePollError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "timed out waiting for client event"),
            Self::Disconnected => write!(f, "client runtime event channel disconnected"),
        }
    }
}

impl std::error::Error for ClientRuntimePollError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostRuntimeMode {
    Production,
    Test,
}

#[derive(Debug)]
pub struct HostRuntimeConfig {
    pub bind_addr: SocketAddr,
    pub advertised_host: String,
    pub session_epoch: u64,
    pub table_id: String,
    pub table_name: Option<String>,
    pub join_token: String,
    pub host_signing_keys: Arc<SigningKeyMaterial>,
    pub host_encryption_keys: Arc<Mutex<EncryptionKeyMaterial>>,
    pub snapshot_state: TournamentState,
    pub runtime_mode: HostRuntimeMode,
}

#[derive(Debug)]
pub(crate) struct ConnectedClient {
    stream: Arc<Mutex<TcpStream>>,
    encryption_public_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicEventLogEntry {
    pub sequence: u64,
    pub message_type: ProtocolMessageType,
    pub payload: Value,
}

/// Live health counters for the host background loops.
///
/// Counts stay at zero while the host is healthy.  Non-zero values or a
/// non-None `last_error` indicate that at least one background failure was
/// silently swallowed (accept error, tick error, publish error, lock poison).
/// Readable via `HostServer::runtime_health()`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostRuntimeHealth {
    pub accept_error_count: u64,
    pub stream_timeout_error_count: u64,
    pub tick_advance_error_count: u64,
    pub publish_error_count: u64,
    pub state_lock_error_count: u64,
    /// Incremented when a newly-accepted TCP stream cannot be cloned.
    pub stream_clone_error_count: u64,
    /// Incremented when the connected-client registry mutex is poisoned.
    pub client_registry_error_count: u64,
    /// Incremented when marking a disconnected participant reconnect-eligible fails.
    pub reconnect_mark_error_count: u64,
    /// Incremented when a lobby mutation succeeded but broadcasting the updated
    /// snapshot to connected clients failed.  The authoritative state is valid;
    /// affected clients may need a manual resync.
    pub snapshot_sync_error_count: u64,
    pub last_error: Option<String>,
    pub last_successful_tick_ms: Option<u64>,
    pub last_successful_publish_ms: Option<u64>,
}

impl HostRuntimeHealth {
    fn record_error(&mut self, message: impl Into<String>) {
        self.last_error = Some(message.into());
    }

    pub(super) fn record_stream_clone_error(&mut self, error: impl std::fmt::Display) {
        self.stream_clone_error_count += 1;
        self.record_error(format!("failed to clone client stream: {error}"));
    }

    pub(super) fn record_client_registry_error(&mut self) {
        self.client_registry_error_count += 1;
        self.record_error("connected-client registry lock poisoned");
    }

    pub(super) fn record_reconnect_mark_error(
        &mut self,
        player_id: &str,
        error: impl std::fmt::Display,
    ) {
        self.reconnect_mark_error_count += 1;
        self.record_error(format!(
            "failed to mark {player_id} reconnect-eligible: {error}"
        ));
    }

    pub(super) fn record_snapshot_sync_error(&mut self, error: impl std::fmt::Display) {
        self.snapshot_sync_error_count += 1;
        self.record_error(format!(
            "lobby mutation succeeded but snapshot sync to clients failed: {error}"
        ));
    }
}

/// Update the health state from any thread that holds an `Arc<Mutex<HostRuntimeHealth>>`.
/// Silently ignores a poisoned mutex (the health update is best-effort).
pub(super) fn update_health(
    health: &Arc<Mutex<HostRuntimeHealth>>,
    update: impl FnOnce(&mut HostRuntimeHealth),
) {
    if let Ok(mut guard) = health.lock() {
        update(&mut guard);
    }
}

/// One NPC participant to register, seat, and mark ready atomically.
#[derive(Clone, Debug)]
pub struct NpcSeatAssignment {
    pub player_id: String,
    pub display_name: String,
    pub seat_index: u8,
}

pub struct HostServer {
    listener_addr: SocketAddr,
    join_payload: JoinPayload,
    encoded_join_payload: String,
    authoritative_state: Arc<Mutex<TournamentState>>,
    tournament_runtime: Arc<Mutex<Option<TournamentController>>>,
    transition_lock: Arc<Mutex<()>>,
    clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
    server_sequence: Arc<AtomicU64>,
    stop_signal: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
    tick_thread: Option<JoinHandle<()>>,
    host_signing_keys: Arc<SigningKeyMaterial>,
    host_encryption_keys: Arc<Mutex<EncryptionKeyMaterial>>,
    public_events: Arc<Mutex<Vec<PublicEventLogEntry>>>,
    runtime_health: Arc<Mutex<HostRuntimeHealth>>,
}

#[derive(Debug)]
pub(crate) struct InitialRequestAcceptance {
    player_id: String,
    snapshot_envelope: SignedEnvelope<SnapshotEvent>,
    encryption_public_key: String,
}

#[derive(Debug)]
pub(crate) struct ClientReconnectIdentity {
    signing_keys: Option<SigningKeyMaterial>,
    encryption_keys: Option<EncryptionKeyMaterial>,
}

#[derive(Debug)]
struct ClientCommandConnection {
    player_id: String,
    table_id: String,
    session_epoch: u64,
    next_counter: u64,
    stream: Option<Arc<Mutex<TcpStream>>>,
}

pub struct ClientRuntime {
    incoming: Receiver<ClientRuntimeEvent>,
    reconnect_identity: Arc<Mutex<ClientReconnectIdentity>>,
    command_connection: Arc<Mutex<ClientCommandConnection>>,
}

impl ClientRuntime {
    pub fn poll_event(
        &self,
        timeout: Duration,
    ) -> Result<ClientRuntimeEvent, ClientRuntimePollError> {
        self.incoming
            .recv_timeout(timeout)
            .map_err(|error| match error {
                RecvTimeoutError::Timeout => ClientRuntimePollError::Timeout,
                RecvTimeoutError::Disconnected => ClientRuntimePollError::Disconnected,
            })
    }
}

pub struct ClientRuntimeConfig {
    pub join_payload: String,
    pub player_id: String,
    pub display_name: String,
    pub signing_keys: SigningKeyMaterial,
    pub encryption_keys: EncryptionKeyMaterial,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClientRuntimeEvent {
    Snapshot(Box<SnapshotEvent>),
    PublicEvent {
        message_type: ProtocolMessageType,
        server_sequence: u64,
        payload: Value,
    },
    PrivateHoleCards(PrivateHoleCardsEvent),
    Reconnecting {
        player_id: String,
    },
    ResyncRequested {
        player_id: String,
        last_seen_server_sequence: u64,
    },
    ProtocolWarning {
        player_id: String,
        reason: String,
        count: u64,
    },
    SafeError {
        player_id: String,
        message: String,
    },
    Disconnected {
        player_id: String,
    },
}

pub(crate) fn commit_runtime_state(
    authoritative_state: &Arc<Mutex<TournamentState>>,
    mut next_state: TournamentState,
) -> Result<(TournamentState, TournamentState), NetworkingError> {
    let mut authoritative = authoritative_state
        .lock()
        .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))?;
    let previous_state = authoritative.clone();

    if next_state.table_id != previous_state.table_id
        || next_state.session_epoch != previous_state.session_epoch
    {
        return Err(NetworkingError::new(
            "runtime state writeback must match the active table/session",
        ));
    }
    if !next_state
        .hand_results
        .starts_with(&previous_state.hand_results)
    {
        return Err(NetworkingError::new(
            "refusing runtime state writeback that rewrites or removes settled hand history",
        ));
    }
    if previous_state.phase == crate::domain::TournamentPhase::Complete
        && next_state.phase != crate::domain::TournamentPhase::Complete
    {
        return Err(NetworkingError::new(
            "refusing runtime state writeback that reopens a completed tournament",
        ));
    }

    merge_networking_state(&previous_state, &mut next_state);
    *authoritative = next_state.clone();
    Ok((previous_state, next_state))
}

pub fn resolve_connectable_host_ip() -> Result<IpAddr, NetworkingError> {
    let interfaces = list_afinet_netifas()
        .map_err(|error| NetworkingError::new(format!("failed to list interfaces: {error}")))?;

    interfaces
        .into_iter()
        .map(|(_, ip)| ip)
        .find(|ip| ip.is_ipv4() && !ip.is_loopback() && !ip.is_unspecified())
        .ok_or_else(|| NetworkingError::new("no valid LAN IP address is available for hosting"))
}

pub(crate) fn clear_established_read_timeout(
    stream: &TcpStream,
    context: &str,
) -> Result<(), NetworkingError> {
    stream.set_read_timeout(None).map_err(|error| {
        NetworkingError::new(format!("failed to clear {context} read timeout: {error}"))
    })
}

fn validate_production_host_ip(ip_addr: IpAddr) -> Result<(), NetworkingError> {
    if ip_addr.is_unspecified() || ip_addr.is_loopback() {
        return Err(NetworkingError::new(
            "production host flow requires a non-loopback, connectable LAN IP address",
        ));
    }

    Ok(())
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod poll_tests {
    use std::sync::mpsc;

    use super::*;

    fn runtime_with_receiver(receiver: Receiver<ClientRuntimeEvent>) -> ClientRuntime {
        ClientRuntime {
            incoming: receiver,
            reconnect_identity: Arc::new(Mutex::new(ClientReconnectIdentity {
                signing_keys: None,
                encryption_keys: None,
            })),
            command_connection: Arc::new(Mutex::new(ClientCommandConnection {
                player_id: "player-1".to_string(),
                table_id: "table-1".to_string(),
                session_epoch: 1,
                next_counter: 1,
                stream: None,
            })),
        }
    }

    #[test]
    fn poll_event_returns_queued_event() {
        let (sender, receiver) = mpsc::channel();
        sender
            .send(ClientRuntimeEvent::Reconnecting {
                player_id: "player-1".to_string(),
            })
            .expect("event should queue");
        let runtime = runtime_with_receiver(receiver);

        let event = runtime
            .poll_event(Duration::from_millis(10))
            .expect("queued event should be returned");

        assert!(matches!(event, ClientRuntimeEvent::Reconnecting { .. }));
    }

    #[test]
    fn poll_event_distinguishes_timeout() {
        let (_sender, receiver) = mpsc::channel();
        let runtime = runtime_with_receiver(receiver);

        let error = runtime
            .poll_event(Duration::from_millis(1))
            .expect_err("empty live channel should time out");

        assert_eq!(error, ClientRuntimePollError::Timeout);
    }

    #[test]
    fn poll_event_distinguishes_disconnected_channel() {
        let (sender, receiver) = mpsc::channel();
        drop(sender);
        let runtime = runtime_with_receiver(receiver);

        let error = runtime
            .poll_event(Duration::from_millis(1))
            .expect_err("closed channel should report disconnect");

        assert_eq!(error, ClientRuntimePollError::Disconnected);
    }
}

#[cfg(test)]
mod tests;
