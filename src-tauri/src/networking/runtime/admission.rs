//! Completion of an accepted join/reconnect request (DP-REVIEW-001).
//!
//! `handle_initial_client_request` admits the participant (Prepared ->
//! SnapshotBuilt). `complete_admission` then delivers the snapshot, makes the
//! stream long-lived, registers it and hands it to a session thread
//! (SnapshotDelivered -> Registered -> Active). Every failure goes through one
//! cleanup path that settles state before the peer can observe it: retire a
//! registration made by this attempt, roll the admission back if this attempt
//! still owns it, release the active-client slot, and only then close the
//! transport. No lock is held across socket I/O; see `client_registry` for the
//! lock order.

use std::{
    collections::HashMap,
    net::{Shutdown, TcpStream},
    sync::{atomic::AtomicU64, Arc, Mutex},
};

use crate::{
    crypto::{DefaultCryptoProvider, EncryptionKeyMaterial, SigningKeyMaterial},
    domain::{JoinPayload, TournamentState},
    networking::write_json_frame,
    tournament::TournamentController,
};

use super::*;

/// Post-acceptance steps that can fail, in execution order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AdmissionStage {
    SnapshotWrite,
    ClearReadTimeout,
    StreamClone,
    CatchUpSnapshotBuild,
    Register,
    SessionSpawn,
}

impl AdmissionStage {
    fn label(self) -> &'static str {
        match self {
            Self::SnapshotWrite => "snapshot delivery",
            Self::ClearReadTimeout => "read-timeout reconfiguration",
            Self::StreamClone => "stream clone",
            Self::CatchUpSnapshotBuild => "admission snapshot synchronization",
            Self::Register => "client registration",
            Self::SessionSpawn => "session spawn",
        }
    }
}

/// Deterministic failure injection for admission tests. Production always
/// passes `AdmissionFaults::none()`.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AdmissionFaults {
    fail_at: Option<AdmissionStage>,
}

impl AdmissionFaults {
    pub(crate) const fn none() -> Self {
        Self { fail_at: None }
    }

    #[cfg(test)]
    pub(crate) const fn fail_at(stage: AdmissionStage) -> Self {
        Self {
            fail_at: Some(stage),
        }
    }

    fn check(self, stage: AdmissionStage) -> Result<(), NetworkingError> {
        if self.fail_at == Some(stage) {
            return Err(NetworkingError::new("injected admission failure"));
        }
        Ok(())
    }
}

/// Shared host state a client session needs.
pub(crate) struct HostSessionContext {
    pub(crate) authoritative_state: Arc<Mutex<TournamentState>>,
    pub(crate) tournament_runtime: Arc<Mutex<Option<TournamentController>>>,
    pub(crate) transition_lock: Arc<Mutex<()>>,
    pub(crate) clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
    pub(crate) join_payload: JoinPayload,
    pub(crate) server_sequence: Arc<AtomicU64>,
    pub(crate) host_signing_keys: Arc<SigningKeyMaterial>,
    pub(crate) host_encryption_keys: Arc<Mutex<EncryptionKeyMaterial>>,
    pub(crate) public_events: Arc<Mutex<Vec<PublicEventLogEntry>>>,
    pub(crate) runtime_health: Arc<Mutex<HostRuntimeHealth>>,
}

/// An admission failure after acceptance, tagged with the failing stage.
#[derive(Debug)]
pub(crate) struct AdmissionFailure {
    pub(crate) stage: AdmissionStage,
    pub(crate) error: NetworkingError,
}

fn at(stage: AdmissionStage) -> impl FnOnce(NetworkingError) -> AdmissionFailure {
    move |error| AdmissionFailure { stage, error }
}

fn record_admission_failure(
    runtime_health: &Arc<Mutex<HostRuntimeHealth>>,
    failure: &AdmissionFailure,
) {
    update_health(runtime_health, |health| {
        match failure.stage {
            AdmissionStage::ClearReadTimeout => health.stream_timeout_error_count += 1,
            AdmissionStage::StreamClone => health.record_stream_clone_error(&failure.error),
            AdmissionStage::Register => health.client_registry_error_count += 1,
            AdmissionStage::SnapshotWrite
            | AdmissionStage::CatchUpSnapshotBuild
            | AdmissionStage::SessionSpawn => {
                health.accept_error_count += 1;
            }
        }
        health.record_error(format!(
            "admission failed during {}: {}",
            failure.stage.label(),
            failure.error
        ));
    });
}

/// Finish an accepted admission, or undo it completely.
pub(crate) fn complete_admission(
    context: HostSessionContext,
    stream: TcpStream,
    acceptance: InitialRequestAcceptance,
    active_client_slot_guard: ActiveClientSlotGuard,
    faults: AdmissionFaults,
) -> Result<(), AdmissionFailure> {
    let mut rollback = PendingAdmissionRollback::new(
        &context.authoritative_state,
        &acceptance,
        &context.runtime_health,
    );
    let runtime_health = Arc::clone(&context.runtime_health);
    let result = deliver_and_register(
        context,
        stream,
        acceptance,
        active_client_slot_guard,
        faults,
        &mut rollback,
    );
    match &result {
        Ok(()) => rollback.disarm(),
        Err(failure) => record_admission_failure(&runtime_health, failure),
    }
    result
}

fn deliver_and_register(
    context: HostSessionContext,
    mut stream: TcpStream,
    acceptance: InitialRequestAcceptance,
    active_client_slot_guard: ActiveClientSlotGuard,
    faults: AdmissionFaults,
    rollback: &mut PendingAdmissionRollback,
) -> Result<(), AdmissionFailure> {
    let InitialRequestAcceptance {
        player_id,
        snapshot_envelope,
        encryption_public_key,
        generation,
        ..
    } = acceptance;
    let initial_snapshot_sequence = snapshot_envelope.server_sequence.unwrap_or_default();
    let mut registered = false;
    let clients = Arc::clone(&context.clients);

    let registered_stream = (|| {
        use AdmissionStage::*;
        faults.check(SnapshotWrite).map_err(at(SnapshotWrite))?;
        write_json_frame(&mut stream, &snapshot_envelope).map_err(at(SnapshotWrite))?;

        faults
            .check(ClearReadTimeout)
            .map_err(at(ClearReadTimeout))?;
        clear_established_read_timeout(&stream, "established host client")
            .map_err(at(ClearReadTimeout))?;

        faults.check(StreamClone).map_err(at(StreamClone))?;
        let stream_handle = stream
            .try_clone()
            .map(|cloned| Arc::new(Mutex::new(cloned)))
            .map_err(|error| {
                NetworkingError::new(format!("failed to clone client stream: {error}"))
            })
            .map_err(at(StreamClone))?;

        let transition_guard = context.transition_lock.lock().map_err(|_| {
            at(CatchUpSnapshotBuild)(NetworkingError::new("host transition lock poisoned"))
        })?;
        faults
            .check(CatchUpSnapshotBuild)
            .map_err(at(CatchUpSnapshotBuild))?;
        let has_public_event_after_snapshot = context
            .public_events
            .lock()
            .map_err(|_| {
                at(CatchUpSnapshotBuild)(NetworkingError::new("public event log lock poisoned"))
            })?
            .iter()
            .any(|event| event.sequence > initial_snapshot_sequence);
        let catch_up_snapshot = if has_public_event_after_snapshot {
            Some(
                build_snapshot_envelope(
                    &DefaultCryptoProvider,
                    &context.join_payload,
                    &context.authoritative_state,
                    &context.server_sequence,
                    &context.host_signing_keys,
                    &context.host_encryption_keys,
                    &player_id,
                )
                .map_err(at(CatchUpSnapshotBuild))?,
            )
        } else {
            None
        };

        // The writer is not yet in the registry, so this lock is uncontended.
        // Acquire it before registration while following the transition-lock
        // ordering; subsequent broadcasts then wait until any catch-up snapshot
        // has been written.
        let mut writer = stream_handle
            .lock()
            .map_err(|_| at(Register)(NetworkingError::new("client stream lock poisoned")))?;
        faults.check(Register).map_err(at(Register))?;
        register_client(
            &context.clients,
            &player_id,
            ConnectedClient {
                stream: Arc::clone(&stream_handle),
                encryption_public_key,
                generation,
            },
        )
        .map_err(at(Register))?;
        registered = true;
        drop(transition_guard);

        if let Some(snapshot) = catch_up_snapshot {
            write_json_frame(&mut writer, &snapshot).map_err(at(SnapshotWrite))?;
        }
        drop(writer);
        Ok::<_, AdmissionFailure>(stream_handle)
    })();

    let writer = match registered_stream {
        Ok(writer) => writer,
        Err(failure) => {
            let retired = registered
                .then(|| {
                    take_client_if_current(&clients, &player_id, generation)
                        .ok()
                        .flatten()
                })
                .flatten();
            rollback.rollback_now();
            drop(active_client_slot_guard);
            if let Some(client) = retired {
                close_client(&client);
            }
            // Best-effort close: the peer may already be gone.
            let _ = stream.shutdown(Shutdown::Both);
            return Err(failure);
        }
    };

    let spawned = faults
        .check(AdmissionStage::SessionSpawn)
        .and_then(|()| {
            spawn_host_client_session(
                player_id.clone(),
                generation,
                stream,
                writer,
                context,
                active_client_slot_guard,
            )
        })
        .map_err(at(AdmissionStage::SessionSpawn));

    if spawned.is_err() {
        // The slot guard and original stream were dropped with the unspawned
        // session; the registered clone keeps the socket open until retired.
        let retired = take_client_if_current(&clients, &player_id, generation);
        rollback.rollback_now();
        if let Ok(Some(client)) = retired {
            close_client(&client);
        }
    }
    spawned
}
