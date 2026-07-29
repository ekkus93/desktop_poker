from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    text = file_path.read_text(encoding="utf-8")
    if old not in text:
        if new in text:
            return
        raise SystemExit(f"expected text not found in {path}: {old[:160]!r}")
    file_path.write_text(text.replace(old, new, 1), encoding="utf-8")


replace_once(
    "src-tauri/src/networking/runtime/handlers.rs",
    '''    let next_state = controller.state().clone();

    let invalid_transition = match &action_outcome {
        ActionSubmissionOutcome::Committed => next_state == before_state,
        ActionSubmissionOutcome::RejectedNoStateChange { .. } => next_state != before_state,
        ActionSubmissionOutcome::TimeoutAdvancedThenRejected { .. } => next_state == before_state,
    };''',
    '''    let next_state = controller.state().clone();
    let mut normalized_next_state = next_state.clone();
    merge_networking_state(&before_state, &mut normalized_next_state);
    let gameplay_state_changed = normalized_next_state != before_state;

    let invalid_transition = match &action_outcome {
        ActionSubmissionOutcome::Committed => !gameplay_state_changed,
        ActionSubmissionOutcome::RejectedNoStateChange { .. } => gameplay_state_changed,
        ActionSubmissionOutcome::TimeoutAdvancedThenRejected { .. } => !gameplay_state_changed,
    };''',
)

replace_once(
    "src-tauri/src/networking/runtime/tests/action_outcomes.rs",
    "    domain::{ActionType, PlayerIdentity, TournamentState},",
    "    domain::{ActionType, ConnectionState, PlayerIdentity, TournamentState},",
)

action_tests_path = Path("src-tauri/src/networking/runtime/tests/action_outcomes.rs")
action_tests = action_tests_path.read_text(encoding="utf-8")
rejection_test = r'''

#[test]
fn networking_only_authoritative_fields_do_not_turn_rejection_into_internal_error() {
    let fixture = started_runtime(now_epoch_ms());
    let window = current_window(&fixture);
    let wrong_player = if window.player_id == "player-a" {
        "player-b"
    } else {
        "player-a"
    };
    {
        let mut authoritative = fixture
            .authoritative_state
            .lock()
            .expect("authoritative state");
        let participant = authoritative
            .participants
            .get_mut(wrong_player)
            .expect("wrong-player participant");
        participant.connection_state = ConnectionState::Reconnecting;
        participant.reconnect_token = Some("network-only-rejection-token".to_string());
        participant.admitted_at_ms = 991;
    }
    let envelope = signed_action(
        &fixture,
        wrong_player,
        window.action_window_id,
        window.seat_index,
        ActionType::Fold,
        None,
    );

    let outcome = handle_action_submission_request(
        &fixture.provider,
        envelope,
        &fixture.authoritative_state,
        &fixture.tournament_runtime,
    )
    .expect("networking-only metadata must not convert a gameplay rejection into an internal error");

    let RemoteActionSubmissionOutcome::RejectedNoStateChange { error } = outcome else {
        panic!("expected a no-state-change rejection");
    };
    assert!(error.to_string().contains("does not own the action window"));

    let authoritative = fixture
        .authoritative_state
        .lock()
        .expect("authoritative state after rejection")
        .clone();
    let participant = authoritative
        .participants
        .get(wrong_player)
        .expect("wrong-player participant after rejection");
    assert_eq!(participant.connection_state, ConnectionState::Reconnecting);
    assert_eq!(
        participant.reconnect_token.as_deref(),
        Some("network-only-rejection-token")
    );
    assert_eq!(participant.admitted_at_ms, 991);

    let mut normalized_controller_state = fixture
        .tournament_runtime
        .lock()
        .expect("runtime")
        .as_ref()
        .expect("controller")
        .state()
        .clone();
    merge_networking_state(&authoritative, &mut normalized_controller_state);
    assert_eq!(normalized_controller_state, authoritative);
}
'''
if "networking_only_authoritative_fields_do_not_turn_rejection_into_internal_error" not in action_tests:
    action_tests_path.write_text(action_tests + rejection_test, encoding="utf-8")

replace_once(
    "src-tauri/src/commands.rs",
    '''    fn new(code: &'static str, message: String, recoverable: bool) -> Self {
        Self {
            code: code.to_string(),
            message,
            recoverable,
        }
    }

    fn from_table_action(error: DesktopTableActionError) -> Self {''',
    '''    fn new(code: &'static str, message: String, recoverable: bool) -> Self {
        Self {
            code: code.to_string(),
            message,
            recoverable,
        }
    }

    fn invalid_join_payload(message: String) -> Self {
        Self::new("INVALID_JOIN_PAYLOAD", message, true)
    }

    fn from_table_action(error: DesktopTableActionError) -> Self {''',
)

replace_once(
    "src-tauri/src/commands.rs",
    '''    let result = state
        .join_host_session(request)
        .map_err(|message| DesktopCommandError::new("INVALID_JOIN_PAYLOAD", message, true))?;''',
    '''    let result = state
        .join_host_session(request)
        .map_err(DesktopCommandError::invalid_join_payload)?;''',
)

replace_once(
    "src-tauri/src/commands.rs",
    '''    #[test]
    fn command_error_serializes_stable_fields() {
        let error = DesktopCommandError::from_table_action(DesktopTableActionError::new(
            DesktopTableActionErrorCode::ObserverReadOnly,
            "spectators cannot act",
        ));
        let value = serde_json::to_value(error).expect("command error serializes");
        assert_eq!(value["code"], "OBSERVER_READ_ONLY");
        assert_eq!(value["recoverable"], true);
        assert_eq!(value["message"], "spectators cannot act");
    }
}''',
    '''    #[test]
    fn command_error_serializes_stable_fields() {
        let error = DesktopCommandError::from_table_action(DesktopTableActionError::new(
            DesktopTableActionErrorCode::ObserverReadOnly,
            "spectators cannot act",
        ));
        let value = serde_json::to_value(error).expect("command error serializes");
        assert_eq!(value["code"], "OBSERVER_READ_ONLY");
        assert_eq!(value["recoverable"], true);
        assert_eq!(value["message"], "spectators cannot act");
    }

    #[test]
    fn typed_not_acting_player_code_is_recoverable_independent_of_wording() {
        for message in ["turn belongs to another participant", "rewritten ownership copy"] {
            let error = DesktopCommandError::from_table_action(DesktopTableActionError::new(
                DesktopTableActionErrorCode::NotActingPlayer,
                message,
            ));
            assert_eq!(error.code, "NOT_ACTING_PLAYER");
            assert!(error.recoverable);
            assert_eq!(error.message, message);
        }
    }

    #[test]
    fn invalid_join_payload_code_is_recoverable_independent_of_wording() {
        for message in ["invite envelope was invalid", "rewritten join failure copy"] {
            let error = DesktopCommandError::invalid_join_payload(message.to_string());
            assert_eq!(error.code, "INVALID_JOIN_PAYLOAD");
            assert!(error.recoverable);
            assert_eq!(error.message, message);
        }
    }
}''',
)
