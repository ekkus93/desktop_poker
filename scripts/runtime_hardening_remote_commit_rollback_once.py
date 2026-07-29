from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    text = file_path.read_text(encoding="utf-8")
    if old not in text:
        if new in text:
            return
        raise SystemExit(f"expected text not found in {path}: {old[:120]!r}")
    file_path.write_text(text.replace(old, new, 1), encoding="utf-8")


replace_once(
    "src-tauri/src/networking/runtime/handlers.rs",
    """pub(crate) fn handle_action_submission_request(
    crypto_provider: &impl ProtocolCryptoProvider,""",
    """pub(crate) fn commit_remote_action_state(
    controller: &mut TournamentController,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    next_state: TournamentState,
    rollback_controller: TournamentController,
) -> Result<(TournamentState, TournamentState), NetworkingError> {
    match commit_runtime_state(authoritative_state, next_state) {
        Ok(states) => Ok(states),
        Err(error) => {
            *controller = rollback_controller;
            Err(error)
        }
    }
}

pub(crate) fn handle_action_submission_request(
    crypto_provider: &impl ProtocolCryptoProvider,""",
)

old_block = """    let (next_state, action_outcome) = {
        let mut runtime = tournament_runtime
            .lock()
            .map_err(|_| NetworkingError::new("tournament runtime lock poisoned"))?;
        let controller = runtime
            .as_mut()
            .ok_or_else(|| NetworkingError::new("live tournament runtime is unavailable"))?;
        if controller.state() != &before_state {
            return Err(NetworkingError::new(
                "tournament runtime diverged from authoritative state before remote action",
            ));
        }
        let rollback_controller = controller.clone();
        let action_outcome =
            match controller.submit_action_with_outcome(action_request, now_epoch_ms()) {
                Ok(outcome) => outcome,
                Err(error) => {
                    *controller = rollback_controller;
                    return Err(NetworkingError::new(error.to_string()));
                }
            };
        let next_state = controller.state().clone();

        let invalid_transition = match &action_outcome {
            ActionSubmissionOutcome::Committed => next_state == before_state,
            ActionSubmissionOutcome::RejectedNoStateChange { .. } => next_state != before_state,
            ActionSubmissionOutcome::TimeoutAdvancedThenRejected { .. } => {
                next_state == before_state
            }
        };
        if invalid_transition {
            *controller = rollback_controller;
            return Err(NetworkingError::new(
                "remote action outcome did not match its controller state transition; mutation was rolled back",
            ));
        }

        (next_state, action_outcome)
    };

    match action_outcome {
        ActionSubmissionOutcome::Committed => {
            let (previous_state, after_state) =
                commit_runtime_state(authoritative_state, next_state)?;
            Ok(RemoteActionSubmissionOutcome::Committed {
                previous_state,
                after_state,
            })
        }
        ActionSubmissionOutcome::RejectedNoStateChange { error } => {
            Ok(RemoteActionSubmissionOutcome::RejectedNoStateChange {
                error: NetworkingError::new(error.to_string()),
            })
        }
        ActionSubmissionOutcome::TimeoutAdvancedThenRejected { error } => {
            let (previous_state, after_state) =
                commit_runtime_state(authoritative_state, next_state)?;
            Ok(RemoteActionSubmissionOutcome::TimeoutAdvancedThenRejected {
                previous_state,
                after_state,
                error: NetworkingError::new(error.to_string()),
            })
        }
    }
"""
new_block = """    let mut runtime = tournament_runtime
        .lock()
        .map_err(|_| NetworkingError::new("tournament runtime lock poisoned"))?;
    let controller = runtime
        .as_mut()
        .ok_or_else(|| NetworkingError::new("live tournament runtime is unavailable"))?;
    if controller.state() != &before_state {
        return Err(NetworkingError::new(
            "tournament runtime diverged from authoritative state before remote action",
        ));
    }
    let rollback_controller = controller.clone();
    let action_outcome = match controller.submit_action_with_outcome(action_request, now_epoch_ms()) {
        Ok(outcome) => outcome,
        Err(error) => {
            *controller = rollback_controller;
            return Err(NetworkingError::new(error.to_string()));
        }
    };
    let next_state = controller.state().clone();

    let invalid_transition = match &action_outcome {
        ActionSubmissionOutcome::Committed => next_state == before_state,
        ActionSubmissionOutcome::RejectedNoStateChange { .. } => next_state != before_state,
        ActionSubmissionOutcome::TimeoutAdvancedThenRejected { .. } => next_state == before_state,
    };
    if invalid_transition {
        *controller = rollback_controller;
        return Err(NetworkingError::new(
            "remote action outcome did not match its controller state transition; mutation was rolled back",
        ));
    }

    match action_outcome {
        ActionSubmissionOutcome::Committed => {
            let (previous_state, after_state) = commit_remote_action_state(
                controller,
                authoritative_state,
                next_state,
                rollback_controller,
            )?;
            Ok(RemoteActionSubmissionOutcome::Committed {
                previous_state,
                after_state,
            })
        }
        ActionSubmissionOutcome::RejectedNoStateChange { error } => {
            Ok(RemoteActionSubmissionOutcome::RejectedNoStateChange {
                error: NetworkingError::new(error.to_string()),
            })
        }
        ActionSubmissionOutcome::TimeoutAdvancedThenRejected { error } => {
            let (previous_state, after_state) = commit_remote_action_state(
                controller,
                authoritative_state,
                next_state,
                rollback_controller,
            )?;
            Ok(RemoteActionSubmissionOutcome::TimeoutAdvancedThenRejected {
                previous_state,
                after_state,
                error: NetworkingError::new(error.to_string()),
            })
        }
    }
"""
replace_once("src-tauri/src/networking/runtime/handlers.rs", old_block, new_block)

replace_once(
    "src-tauri/src/networking/runtime/tests/action_outcomes.rs",
    """    tournament::{RegisteredPlayer, TournamentController},""",
    """    tournament::{ActionRequest, RegisteredPlayer, TournamentController},""",
)

path = Path("src-tauri/src/networking/runtime/tests/action_outcomes.rs")
text = path.read_text(encoding="utf-8")
test = r'''

#[test]
fn authoritative_commit_failure_rolls_controller_back() {
    let fixture = started_runtime(now_epoch_ms());
    let window = current_window(&fixture);
    let mut runtime = fixture.tournament_runtime.lock().expect("runtime");
    let controller = runtime.as_mut().expect("controller");
    let rollback_controller = controller.clone();
    controller
        .submit_action(
            ActionRequest {
                player_id: window.player_id,
                action_window_id: window.action_window_id,
                action_type: ActionType::Fold,
                raise_to_amount: None,
            },
            now_epoch_ms(),
        )
        .expect("controller advances before simulated commit failure");
    let advanced_state = controller.state().clone();
    assert_ne!(advanced_state, *rollback_controller.state());

    let poisoned_authoritative = Arc::new(Mutex::new(rollback_controller.state().clone()));
    let poison_target = Arc::clone(&poisoned_authoritative);
    let _ = std::thread::spawn(move || {
        let _guard = poison_target.lock().expect("lock before poisoning");
        panic!("poison authoritative state for rollback test");
    })
    .join();

    let error = commit_remote_action_state(
        controller,
        &poisoned_authoritative,
        advanced_state,
        rollback_controller.clone(),
    )
    .expect_err("poisoned authoritative commit should fail");

    assert!(error.to_string().contains("authoritative state lock poisoned"));
    assert_eq!(controller.state(), rollback_controller.state());
}
'''
if "authoritative_commit_failure_rolls_controller_back" not in text:
    path.write_text(text + test, encoding="utf-8")
