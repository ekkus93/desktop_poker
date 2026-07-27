#!/usr/bin/env python3
"""Apply the host runtime transition serialization repair deterministically."""

from __future__ import annotations

from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "src-tauri/src/networking/runtime/mod.rs",
    """    tournament_runtime: Arc<Mutex<Option<TournamentController>>>,
    clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
""",
    """    tournament_runtime: Arc<Mutex<Option<TournamentController>>>,
    transition_lock: Arc<Mutex<()>>,
    clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
""",
)

replace_once(
    "src-tauri/src/networking/runtime/mod.rs",
    """pub fn resolve_connectable_host_ip() -> Result<IpAddr, NetworkingError> {
""",
    """pub(crate) fn commit_runtime_state(
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
""",
)

replace_once(
    "src-tauri/src/networking/runtime/host.rs",
    """        let authoritative_state = Arc::new(Mutex::new(config.snapshot_state.clone()));
        let tournament_runtime = Arc::new(Mutex::new(None));
        let clients = Arc::new(Mutex::new(HashMap::new()));
""",
    """        let authoritative_state = Arc::new(Mutex::new(config.snapshot_state.clone()));
        let tournament_runtime = Arc::new(Mutex::new(None));
        let transition_lock = Arc::new(Mutex::new(()));
        let clients = Arc::new(Mutex::new(HashMap::new()));
""",
)

replace_once(
    "src-tauri/src/networking/runtime/host.rs",
    """            let authoritative_state = Arc::clone(&authoritative_state);
            let tournament_runtime = Arc::clone(&tournament_runtime);
            let clients = Arc::clone(&clients);
""",
    """            let authoritative_state = Arc::clone(&authoritative_state);
            let tournament_runtime = Arc::clone(&tournament_runtime);
            let transition_lock = Arc::clone(&transition_lock);
            let clients = Arc::clone(&clients);
""",
)

replace_once(
    "src-tauri/src/networking/runtime/host.rs",
    """                        let authoritative_state = Arc::clone(&authoritative_state);
                        let tournament_runtime = Arc::clone(&tournament_runtime);
                        let server_sequence = Arc::clone(&server_sequence);
""",
    """                        let authoritative_state = Arc::clone(&authoritative_state);
                        let tournament_runtime = Arc::clone(&tournament_runtime);
                        let transition_lock = Arc::clone(&transition_lock);
                        let server_sequence = Arc::clone(&server_sequence);
""",
)

replace_once(
    "src-tauri/src/networking/runtime/host.rs",
    """                                                authoritative_state,
                                                tournament_runtime,
                                                clients,
""",
    """                                                authoritative_state,
                                                tournament_runtime,
                                                transition_lock,
                                                clients,
""",
)

replace_once(
    "src-tauri/src/networking/runtime/host.rs",
    """            let authoritative_state = Arc::clone(&authoritative_state);
            let tournament_runtime = Arc::clone(&tournament_runtime);
            let clients = Arc::clone(&clients);
            let stop_signal = Arc::clone(&stop_signal);
""",
    """            let authoritative_state = Arc::clone(&authoritative_state);
            let tournament_runtime = Arc::clone(&tournament_runtime);
            let transition_lock = Arc::clone(&transition_lock);
            let clients = Arc::clone(&clients);
            let stop_signal = Arc::clone(&stop_signal);
""",
)

host_path = Path("src-tauri/src/networking/runtime/host.rs")
host_text = host_path.read_text()
start_marker = """                        // Acquire the lock only for the duration of advance_time,
"""
end_marker = """                        thread::sleep(Duration::from_millis(50));
"""
start = host_text.index(start_marker)
end = host_text.index(end_marker, start)
new_tick = """                        // Serialize controller mutation, authoritative writeback, and
                        // event publication. Without this lock, a delayed tick candidate can
                        // overwrite a newer player action after the controller lock is released.
                        let transition_guard = match transition_lock.lock() {
                            Ok(guard) => guard,
                            Err(_) => {
                                update_health(&runtime_health, |h| {
                                    h.state_lock_error_count += 1;
                                    h.record_error("host transition lock poisoned");
                                });
                                break;
                            }
                        };

                        let transition = match tournament_runtime.lock() {
                            Err(_) => {
                                update_health(&runtime_health, |h| {
                                    h.state_lock_error_count += 1;
                                    h.record_error("tournament runtime lock poisoned");
                                });
                                break;
                            }
                            Ok(mut runtime) => match runtime.as_mut() {
                                None => None,
                                Some(controller) => {
                                    let before = controller.state().clone();
                                    match controller.advance_time(now) {
                                        Ok(()) => {
                                            update_health(&runtime_health, |h| {
                                                h.last_successful_tick_ms = Some(now);
                                            });
                                            let after = controller.state().clone();
                                            if after == before {
                                                None
                                            } else {
                                                match commit_runtime_state(
                                                    &authoritative_state,
                                                    after,
                                                ) {
                                                    Ok(states) => Some(states),
                                                    Err(error) => {
                                                        update_health(&runtime_health, |h| {
                                                            h.state_lock_error_count += 1;
                                                            h.record_error(format!(
                                                                "runtime tick writeback rejected: {error}"
                                                            ));
                                                        });
                                                        None
                                                    }
                                                }
                                            }
                                        }
                                        Err(error) => {
                                            update_health(&runtime_health, |h| {
                                                h.tick_advance_error_count += 1;
                                                h.record_error(format!(
                                                    "advance_time failed: {error}"
                                                ));
                                            });
                                            None
                                        }
                                    }
                                }
                            },
                        };

                        if let Some((previous_state, state)) = transition {
                            match publish_runtime_transition(
                                &join_payload,
                                &authoritative_state,
                                &previous_state,
                                &state,
                                &clients,
                                &server_sequence,
                                &host_signing_keys,
                                &host_encryption_keys,
                                &public_events,
                            ) {
                                Ok(()) => update_health(&runtime_health, |h| {
                                    h.last_successful_publish_ms = Some(now_epoch_ms());
                                }),
                                Err(error) => update_health(&runtime_health, |h| {
                                    h.publish_error_count += 1;
                                    h.record_error(format!(
                                        "runtime transition publish failed: {error}"
                                    ));
                                }),
                            }
                        }

                        drop(transition_guard);

"""
host_path.write_text(host_text[:start] + new_tick + host_text[end:])

replace_once(
    "src-tauri/src/networking/runtime/host.rs",
    """            authoritative_state,
            tournament_runtime,
            clients,
""",
    """            authoritative_state,
            tournament_runtime,
            transition_lock,
            clients,
""",
)

replace_once(
    "src-tauri/src/networking/runtime/host.rs",
    """    pub fn replace_authoritative_state(
        &self,
        next_state: TournamentState,
    ) -> Result<(), NetworkingError> {
""",
    """    pub fn replace_authoritative_state(
        &self,
        next_state: TournamentState,
    ) -> Result<(), NetworkingError> {
        let _transition_guard = self
            .transition_lock
            .lock()
            .map_err(|_| NetworkingError::new("host transition lock poisoned"))?;
""",
)

replace_once(
    "src-tauri/src/networking/runtime/host.rs",
    """    pub fn start_tournament(&self) -> Result<(), NetworkingError> {
        let before_state = self.authoritative_state()?;
""",
    """    pub fn start_tournament(&self) -> Result<(), NetworkingError> {
        let _transition_guard = self
            .transition_lock
            .lock()
            .map_err(|_| NetworkingError::new("host transition lock poisoned"))?;
        let before_state = self.authoritative_state()?;
""",
)

submit_start = host_path.read_text().index("""    pub fn submit_action(
""")
submit_end = host_path.read_text().index(
    """    /// Attempt to broadcast updated snapshots after a successful lobby mutation.
""",
    submit_start,
)
host_text = host_path.read_text()
new_submit = """    pub fn submit_action(
        &self,
        player_id: &str,
        action_window_id: String,
        action_type: crate::domain::ActionType,
        raise_to_amount: Option<u32>,
    ) -> Result<(), NetworkingError> {
        let _transition_guard = self
            .transition_lock
            .lock()
            .map_err(|_| NetworkingError::new("host transition lock poisoned"))?;
        let before_state = self.authoritative_state()?;
        let (next_state, action_result) = {
            let mut runtime = self
                .tournament_runtime
                .lock()
                .map_err(|_| NetworkingError::new("tournament runtime lock poisoned"))?;
            let controller = runtime
                .as_mut()
                .ok_or_else(|| NetworkingError::new("live tournament runtime is unavailable"))?;

            let action_result = controller
                .submit_action(
                    ActionRequest {
                        player_id: player_id.to_string(),
                        action_window_id,
                        action_type,
                        raise_to_amount,
                    },
                    now_epoch_ms(),
                )
                .map_err(|error| NetworkingError::new(error.to_string()));
            (controller.state().clone(), action_result)
        };

        let publish_result = if next_state != before_state {
            let (previous_state, after_state) =
                commit_runtime_state(&self.authoritative_state, next_state)?;
            publish_runtime_transition(
                &self.join_payload,
                &self.authoritative_state,
                &previous_state,
                &after_state,
                &self.clients,
                &self.server_sequence,
                &self.host_signing_keys,
                &self.host_encryption_keys,
                &self.public_events,
            )
        } else {
            Ok(())
        };

        match (action_result, publish_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(action_error), Ok(())) => Err(action_error),
            (Ok(()), Err(publish_error)) => Err(publish_error),
            (Err(action_error), Err(publish_error)) => Err(NetworkingError::new(format!(
                "{action_error}; additionally failed to publish committed runtime state: {publish_error}"
            ))),
        }
    }

"""
host_path.write_text(host_text[:submit_start] + new_submit + host_text[submit_end:])

replace_once(
    "src-tauri/src/networking/runtime/host_session.rs",
    """    authoritative_state: Arc<Mutex<TournamentState>>,
    tournament_runtime: Arc<Mutex<Option<TournamentController>>>,
    clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
""",
    """    authoritative_state: Arc<Mutex<TournamentState>>,
    tournament_runtime: Arc<Mutex<Option<TournamentController>>>,
    transition_lock: Arc<Mutex<()>>,
    clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,
""",
)

replace_once(
    "src-tauri/src/networking/runtime/host_session.rs",
    """                    ProtocolMessageType::ActionSubmissionRequest => {
                        let rejected_message_id = request_envelope.message_id.clone();
""",
    """                    ProtocolMessageType::ActionSubmissionRequest => {
                        let _transition_guard = match transition_lock.lock() {
                            Ok(guard) => guard,
                            Err(_) => {
                                record_state_lock_error(
                                    &runtime_health,
                                    "host transition lock poisoned during remote action",
                                );
                                break;
                            }
                        };
                        let rejected_message_id = request_envelope.message_id.clone();
""",
)

replace_once(
    "src-tauri/src/networking/runtime/handlers.rs",
    """    authoritative_state
        .lock()
        .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))
        .map(|mut state| {
            *state = next_state;
        })?;

    Ok(())
""",
    """    commit_runtime_state(authoritative_state, next_state)?;

    Ok(())
""",
)

test_path = Path("src-tauri/src/networking/runtime/tests/tournament.rs")
test_text = test_path.read_text()
test_text = test_text.replace(
    """    sync::{atomic::Ordering, Arc, Mutex},
""",
    """    sync::{atomic::Ordering, mpsc, Arc, Mutex},
""",
    1,
)
test_text += """

#[test]
fn runtime_state_writeback_rejects_settled_history_regression() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-history-regression", 190);
    host.stop_signal.store(true, Ordering::SeqCst);

    for (player_id, display_name, seat_index) in
        [("player-a", "Alice", 0_u8), ("player-b", "Bob", 1_u8)]
    {
        host.register_npc_participant(player_id, display_name)
            .expect("participant registers");
        host.claim_seat(player_id, seat_index)
            .expect("participant claims seat");
        host.set_ready_state(player_id, true)
            .expect("participant becomes ready");
    }
    host.start_tournament().expect("tournament starts");
    let window = host
        .authoritative_state()
        .expect("running state")
        .current_hand
        .as_ref()
        .and_then(|hand| hand.action_window.clone())
        .expect("action window");
    host.submit_action(
        &window.player_id,
        window.action_window_id,
        ActionType::Fold,
        None,
    )
    .expect("fold settles the hand");

    let committed = host.authoritative_state().expect("committed state");
    assert_eq!(committed.hand_results.len(), 1);
    let mut stale_candidate = committed.clone();
    stale_candidate.hand_results.clear();

    let error = super::super::commit_runtime_state(
        &host.authoritative_state,
        stale_candidate,
    )
    .expect_err("settled history regression must be rejected");
    assert!(error.to_string().contains("settled hand history"));
    assert_eq!(
        host.authoritative_state()
            .expect("authoritative state remains intact")
            .hand_results,
        committed.hand_results
    );
}

#[test]
fn host_action_waits_for_transition_serialization_lock() {
    let provider = DefaultCryptoProvider;
    let host = Arc::new(bind_test_host(&provider, "table-transition-lock", 191));
    host.stop_signal.store(true, Ordering::SeqCst);

    for (player_id, display_name, seat_index) in
        [("player-a", "Alice", 0_u8), ("player-b", "Bob", 1_u8)]
    {
        host.register_npc_participant(player_id, display_name)
            .expect("participant registers");
        host.claim_seat(player_id, seat_index)
            .expect("participant claims seat");
        host.set_ready_state(player_id, true)
            .expect("participant becomes ready");
    }
    host.start_tournament().expect("tournament starts");
    let window = host
        .authoritative_state()
        .expect("running state")
        .current_hand
        .as_ref()
        .and_then(|hand| hand.action_window.clone())
        .expect("action window");

    let transition_guard = host
        .transition_lock
        .lock()
        .expect("transition lock is available");
    let host_for_action = Arc::clone(&host);
    let (sender, receiver) = mpsc::channel();
    let action_thread = thread::spawn(move || {
        let result = host_for_action.submit_action(
            &window.player_id,
            window.action_window_id,
            ActionType::Fold,
            None,
        );
        sender.send(result).expect("result receiver remains live");
    });

    assert!(matches!(
        receiver.recv_timeout(Duration::from_millis(150)),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    drop(transition_guard);
    receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("action completes after transition lock release")
        .expect("fold succeeds");
    action_thread.join().expect("action thread joins");
}
"""
test_path.write_text(test_text)
