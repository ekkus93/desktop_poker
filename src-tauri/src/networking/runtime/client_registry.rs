//! Connected-client registry helpers (DP-REVIEW-001, invariants N-3/N-4).
//!
//! Lock order: `transition_lock` -> `authoritative_state`. The client registry
//! is a leaf lock: never acquire the authoritative state or a transition lock,
//! and never perform socket I/O, while holding it. A per-client stream mutex is
//! held only while writing one frame to that client, so frames from different
//! host writers do not interleave on the wire.
//!
//! Every registration carries a unique generation. A writer or session that
//! observed a failure can only retire the exact registration it used, so a
//! stale failure never removes or demotes a newer connection for the same
//! player.

use std::{
    collections::HashMap,
    net::{Shutdown, TcpStream},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use serde::Serialize;

use crate::networking::write_json_frame;

use super::{ConnectedClient, NetworkingError};

static NEXT_CLIENT_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Allocate a process-unique registration generation for one admission attempt.
pub(crate) fn next_client_generation() -> u64 {
    NEXT_CLIENT_GENERATION.fetch_add(1, Ordering::Relaxed)
}

/// A registration copied out of the registry so I/O can happen without the
/// registry lock.
#[derive(Clone, Debug)]
pub(crate) struct ClientHandle {
    pub(crate) player_id: String,
    pub(crate) generation: u64,
    pub(crate) stream: Arc<Mutex<TcpStream>>,
    pub(crate) encryption_public_key: String,
}

fn registry_poisoned() -> NetworkingError {
    NetworkingError::new("client registry lock poisoned")
}

fn handle_from(player_id: &str, client: &ConnectedClient) -> ClientHandle {
    ClientHandle {
        player_id: player_id.to_string(),
        generation: client.generation,
        stream: Arc::clone(&client.stream),
        encryption_public_key: client.encryption_public_key.clone(),
    }
}

pub(crate) fn client_handles(
    clients: &Mutex<HashMap<String, ConnectedClient>>,
) -> Result<Vec<ClientHandle>, NetworkingError> {
    let registry = clients.lock().map_err(|_| registry_poisoned())?;
    Ok(registry
        .iter()
        .map(|(player_id, client)| handle_from(player_id, client))
        .collect())
}

pub(crate) fn client_handle(
    clients: &Mutex<HashMap<String, ConnectedClient>>,
    player_id: &str,
) -> Result<Option<ClientHandle>, NetworkingError> {
    let registry = clients.lock().map_err(|_| registry_poisoned())?;
    Ok(registry
        .get(player_id)
        .map(|client| handle_from(player_id, client)))
}

/// Write one frame to a registered client. Holds only that client's stream
/// mutex, never the registry lock.
pub(crate) fn write_to_client<T: Serialize>(
    handle: &ClientHandle,
    payload: &T,
) -> Result<(), NetworkingError> {
    let mut stream = handle
        .stream
        .lock()
        .map_err(|_| NetworkingError::new("client stream lock poisoned"))?;
    write_json_frame(&mut stream, payload)
}

/// Register a newly admitted client. Refuses to replace an existing
/// registration for the same player (a replacement conflict is an admission
/// failure, not a silent takeover).
pub(crate) fn register_client(
    clients: &Mutex<HashMap<String, ConnectedClient>>,
    player_id: &str,
    client: ConnectedClient,
) -> Result<(), NetworkingError> {
    let mut registry = clients.lock().map_err(|_| registry_poisoned())?;
    if registry.contains_key(player_id) {
        return Err(NetworkingError::new(
            "client registry already holds a connection for this player",
        ));
    }
    registry.insert(player_id.to_string(), client);
    Ok(())
}

/// Remove `player_id` only while it is still registered with `generation`.
/// Returns the removed registration; only a caller that got it back may demote
/// the participant. No I/O happens here: callers update authoritative state
/// first and then `close_client`, so a peer that observes the close can retry
/// against already-settled state.
pub(crate) fn take_client_if_current(
    clients: &Mutex<HashMap<String, ConnectedClient>>,
    player_id: &str,
    generation: u64,
) -> Result<Option<ConnectedClient>, NetworkingError> {
    let mut registry = clients.lock().map_err(|_| registry_poisoned())?;
    if registry
        .get(player_id)
        .is_some_and(|client| client.generation == generation)
    {
        Ok(registry.remove(player_id))
    } else {
        Ok(None)
    }
}

/// Shut down a retired registration's transport so its session thread exits.
pub(crate) fn close_client(client: &ConnectedClient) {
    if let Ok(stream) = client.stream.lock() {
        // Best-effort close: the peer may already be gone.
        let _ = stream.shutdown(Shutdown::Both);
    }
}

/// Retire every failed handle that is still current, run `on_retired` for each
/// retired player (with the registry lock released), then close the retired
/// transports.
pub(crate) fn retire_failed_clients(
    clients: &Mutex<HashMap<String, ConnectedClient>>,
    failed: &[ClientHandle],
    mut on_retired: impl FnMut(&str) -> Result<(), NetworkingError>,
) -> Result<(), NetworkingError> {
    let mut retired = Vec::new();
    for handle in failed {
        if let Some(client) = take_client_if_current(clients, &handle.player_id, handle.generation)?
        {
            retired.push((handle.player_id.as_str(), client));
        }
    }
    let mut result = Ok(());
    for (player_id, _) in &retired {
        if result.is_ok() {
            result = on_retired(player_id);
        }
    }
    for (_, client) in &retired {
        close_client(client);
    }
    result
}
