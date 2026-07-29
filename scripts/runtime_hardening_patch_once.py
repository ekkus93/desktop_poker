from __future__ import annotations

import json
import re
from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    text = file_path.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"expected text not found in {path}: {old[:100]!r}")
    file_path.write_text(text.replace(old, new, 1), encoding="utf-8")


# Finish rustfmt changes and ensure authoritative commit failure rolls back the
# tournament controller instead of creating a new divergence.
handlers_path = Path("src-tauri/src/networking/runtime/handlers.rs")
handlers = handlers_path.read_text(encoding="utf-8")
handlers = handlers.replace(
    '''    let request: JoinTournamentRequest = serde_json::from_value(request_envelope.payload.clone())
        .map_err(|error| NetworkingError::new(format!("invalid join request payload: {error}")))?;''',
    '''    let request: JoinTournamentRequest = serde_json::from_value(request_envelope.payload.clone())
        .map_err(|error| {
            NetworkingError::new(format!("invalid join request payload: {error}"))
        })?;''',
    1,
)
start = handlers.index(
    "    let before_state = authoritative_state",
    handlers.index("pub(crate) fn handle_action_submission_request"),
)
handlers_tail = '''    let action_request = ActionRequest {
        player_id: request_envelope.sender_id,
        action_window_id: request.action_window_id,
        action_type: request.action_type,
        raise_to_amount: request.raise_to_amount,
    };

    let mut runtime = tournament_runtime
        .lock()
        .map_err(|_| NetworkingError::new("tournament runtime lock poisoned"))?;
    let controller = runtime
        .as_mut()
        .ok_or_else(|| NetworkingError::new("live tournament runtime is unavailable"))?;
    let rollback_controller = controller.clone();
    let controller_before = controller.state().clone();
    let action_outcome = match controller.submit_action_with_outcome(action_request, now_epoch_ms()) {
        Ok(outcome) => outcome,
        Err(error) => {
            *controller = rollback_controller;
            return Err(NetworkingError::new(error.to_string()));
        }
    };
    let next_state = controller.state().clone();

    let invalid_transition = match &action_outcome {
        ActionSubmissionOutcome::Committed => next_state == controller_before,
        ActionSubmissionOutcome::RejectedNoStateChange { .. } => next_state != controller_before,
        ActionSubmissionOutcome::TimeoutAdvancedThenRejected { .. } => {
            next_state == controller_before
        }
    };
    if invalid_transition {
        *controller = rollback_controller;
        return Err(NetworkingError::new(
            "remote action outcome did not match its controller state transition; mutation was rolled back",
        ));
    }

    match action_outcome {
        ActionSubmissionOutcome::Committed => {
            let (previous_state, after_state) =
                match commit_runtime_state(authoritative_state, next_state) {
                    Ok(states) => states,
                    Err(error) => {
                        *controller = rollback_controller;
                        return Err(error);
                    }
                };
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
                match commit_runtime_state(authoritative_state, next_state) {
                    Ok(states) => states,
                    Err(commit_error) => {
                        *controller = rollback_controller;
                        return Err(commit_error);
                    }
                };
            Ok(RemoteActionSubmissionOutcome::TimeoutAdvancedThenRejected {
                previous_state,
                after_state,
                error: NetworkingError::new(error.to_string()),
            })
        }
    }
}
'''
handlers_path.write_text(handlers[:start] + handlers_tail, encoding="utf-8")

replace_once(
    "src-tauri/src/networking/runtime/tests/action_outcomes.rs",
    "use std::{collections::HashMap, sync::{Arc, Mutex}};",
    '''use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};''',
)
replace_once(
    "src-tauri/src/networking/runtime/tests/action_outcomes.rs",
    '''    let mut controller = TournamentController::new(
        "table-action-outcomes",
        401,
        config,
        registered_players,
    )
    .expect("controller builds");''',
    '''    let mut controller =
        TournamentController::new("table-action-outcomes", 401, config, registered_players)
            .expect("controller builds");''',
)

# Keep the nested HostRuntimeHealth DTO and debug diagnostics synchronized.
replace_once(
    "src/api/desktop.ts",
    '''  snapshotSyncErrorCount: number;
  lastError: string | null;''',
    '''  snapshotSyncErrorCount: number;
  pendingJoinLimitRejectionCount: number;
  connectedClientLimitRejectionCount: number;
  lastError: string | null;''',
)
replace_once(
    "src/components/debug/DebugPanel.tsx",
    '''        debugState.hostRuntimeHealth.snapshotSyncErrorCount > 0 ||
        debugState.hostRuntimeHealth.lastError != null)''',
    '''        debugState.hostRuntimeHealth.snapshotSyncErrorCount > 0 ||
        debugState.hostRuntimeHealth.pendingJoinLimitRejectionCount > 0 ||
        debugState.hostRuntimeHealth.connectedClientLimitRejectionCount > 0 ||
        debugState.hostRuntimeHealth.lastError != null)''',
)
replace_once(
    "src/components/debug/DebugPanel.tsx",
    '''            {debugState.hostRuntimeHealth.snapshotSyncErrorCount > 0 && (
              <li>
                <strong>Snapshot sync errors:</strong>{" "}
                {debugState.hostRuntimeHealth.snapshotSyncErrorCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.lastError != null && (''',
    '''            {debugState.hostRuntimeHealth.snapshotSyncErrorCount > 0 && (
              <li>
                <strong>Snapshot sync errors:</strong>{" "}
                {debugState.hostRuntimeHealth.snapshotSyncErrorCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.pendingJoinLimitRejectionCount > 0 && (
              <li data-testid="host-runtime-pending-join-limit-rejections">
                <strong>Pending join limit rejections:</strong>{" "}
                {debugState.hostRuntimeHealth.pendingJoinLimitRejectionCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.connectedClientLimitRejectionCount > 0 && (
              <li data-testid="host-runtime-connected-client-limit-rejections">
                <strong>Connected client limit rejections:</strong>{" "}
                {debugState.hostRuntimeHealth.connectedClientLimitRejectionCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.lastError != null && (''',
)
replace_once(
    "src/components/debug/DebugPanel.test.tsx",
    '''    snapshotSyncErrorCount: 0,
    lastError: null,''',
    '''    snapshotSyncErrorCount: 0,
    pendingJoinLimitRejectionCount: 0,
    connectedClientLimitRejectionCount: 0,
    lastError: null,''',
)
replace_once(
    "src/components/debug/DebugPanel.test.tsx",
    '''          snapshotSyncErrorCount: 9,
        }),''',
    '''          snapshotSyncErrorCount: 9,
          pendingJoinLimitRejectionCount: 10,
          connectedClientLimitRejectionCount: 11,
        }),''',
)
replace_once(
    "src/components/debug/DebugPanel.test.tsx",
    "    expect(screen.getByText(/Snapshot sync errors:/i)).toBeTruthy();",
    '''    expect(screen.getByText(/Snapshot sync errors:/i)).toBeTruthy();
    expect(screen.getByText(/Pending join limit rejections:/i)).toBeTruthy();
    expect(screen.getByText(/Connected client limit rejections:/i)).toBeTruthy();''',
)

fixture_path = Path("src/fixtures/desktop-contract.json")
fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
fixture["HostRuntimeHealth"] = [
    "acceptErrorCount",
    "streamTimeoutErrorCount",
    "tickAdvanceErrorCount",
    "publishErrorCount",
    "stateLockErrorCount",
    "streamCloneErrorCount",
    "clientRegistryErrorCount",
    "reconnectMarkErrorCount",
    "snapshotSyncErrorCount",
    "pendingJoinLimitRejectionCount",
    "connectedClientLimitRejectionCount",
    "lastError",
    "lastSuccessfulTickMs",
    "lastSuccessfulPublishMs",
]
fixture_path.write_text(json.dumps(fixture, indent=2) + "\n", encoding="utf-8")

replace_once(
    "src/api/desktop.contract.test.ts",
    '''  HostSessionStatus,
  LlmProviderConfig,''',
    '''  HostRuntimeHealth,
  HostSessionStatus,
  LlmProviderConfig,''',
)
replace_once(
    "src/api/desktop.contract.test.ts",
    "    const debugState: DebugInspectorState = {",
    '''    const hostRuntimeHealth: HostRuntimeHealth = {
      acceptErrorCount: 0,
      streamTimeoutErrorCount: 0,
      tickAdvanceErrorCount: 0,
      publishErrorCount: 0,
      stateLockErrorCount: 0,
      streamCloneErrorCount: 0,
      clientRegistryErrorCount: 0,
      reconnectMarkErrorCount: 0,
      snapshotSyncErrorCount: 0,
      pendingJoinLimitRejectionCount: 0,
      connectedClientLimitRejectionCount: 0,
      lastError: null,
      lastSuccessfulTickMs: null,
      lastSuccessfulPublishMs: null,
    };
    const debugState: DebugInspectorState = {''',
)
replace_once(
    "src/api/desktop.contract.test.ts",
    "      hostRuntimeHealth: null,",
    "      hostRuntimeHealth,",
)
replace_once(
    "src/api/desktop.contract.test.ts",
    '''    expect(sortedKeys(debugState)).toEqual(expectedKeys("DebugInspectorState"));''',
    '''    expect(sortedKeys(debugState)).toEqual(expectedKeys("DebugInspectorState"));
    expect(sortedKeys(hostRuntimeHealth)).toEqual(
      expectedKeys("HostRuntimeHealth"),
    );''',
)

# Preserve typed timeout-vs-disconnect polling in every public runtime method.
replace_once(
    "src-tauri/src/networking/runtime/client.rs",
    '''    pub fn next_event(&self, timeout: Duration) -> Result<ClientRuntimeEvent, NetworkingError> {
        self.incoming.recv_timeout(timeout).map_err(|error| {
            NetworkingError::new(format!("timed out waiting for client event: {error}"))
        })
    }''',
    '''    pub fn next_event(
        &self,
        timeout: Duration,
    ) -> Result<ClientRuntimeEvent, ClientRuntimePollError> {
        self.poll_event(timeout)
    }''',
)

# Apply the same payload limit to outbound and inbound frames.
replace_once(
    "src-tauri/src/networking/framing.rs",
    "/// Maximum accepted JSON frame payload size.",
    "/// Maximum JSON frame payload size for both inbound and outbound frames.",
)
replace_once(
    "src-tauri/src/networking/framing.rs",
    '''fn write_frame_bytes<W: Write>(
    writer: &mut W,
    payload_bytes: &[u8],
    payload_len: u64,
) -> Result<(), NetworkingError> {
    let length = u32::try_from(payload_len)''',
    '''fn write_frame_bytes<W: Write>(
    writer: &mut W,
    payload_bytes: &[u8],
    payload_len: u64,
) -> Result<(), NetworkingError> {
    if payload_bytes.len() > MAX_FRAME_PAYLOAD_BYTES {
        return Err(NetworkingError::new(format!(
            "frame payload exceeds maximum allowed size: {} > {}",
            payload_bytes.len(),
            MAX_FRAME_PAYLOAD_BYTES
        )));
    }

    let length = u32::try_from(payload_len)''',
)
replace_once(
    "src-tauri/src/networking/framing.rs",
    '''    #[test]
    fn write_frame_bytes_rejects_payloads_larger_than_u32() {''',
    '''    #[test]
    fn write_frame_bytes_accepts_payload_at_exact_maximum() {
        let payload = vec![b'a'; MAX_FRAME_PAYLOAD_BYTES];
        let mut writer = Vec::new();

        write_frame_bytes(&mut writer, &payload, payload.len() as u64)
            .expect("payload at the maximum should write");

        assert_eq!(writer.len(), MAX_FRAME_PAYLOAD_BYTES + 4);
    }

    #[test]
    fn write_frame_bytes_rejects_payload_above_max_without_partial_write() {
        let payload = vec![b'a'; MAX_FRAME_PAYLOAD_BYTES + 1];
        let mut writer = Vec::new();

        let error = write_frame_bytes(&mut writer, &payload, payload.len() as u64)
            .expect_err("payload above the maximum should fail");

        assert_eq!(
            error.to_string(),
            format!(
                "frame payload exceeds maximum allowed size: {} > {}",
                MAX_FRAME_PAYLOAD_BYTES + 1,
                MAX_FRAME_PAYLOAD_BYTES
            )
        );
        assert!(
            writer.is_empty(),
            "oversized frames must not be partially written"
        );
    }

    #[test]
    fn write_frame_bytes_rejects_payloads_larger_than_u32() {''',
)

# Add focused normal-UI warning tests for both admission counters.
host_shutdown_path = Path("src-tauri/src/app_state/host_shutdown.rs")
host_shutdown = host_shutdown_path.read_text(encoding="utf-8")
if "pending_join_limit_counter_produces_sanitized_warning" not in host_shutdown:
    tests = '''

    #[test]
    fn pending_join_limit_counter_produces_sanitized_warning() {
        let health = HostRuntimeHealth {
            pending_join_limit_rejection_count: 1,
            last_error: Some("raw pending join detail".to_string()),
            ..HostRuntimeHealth::default()
        };
        let warnings = runtime_health_warning_messages(&health);
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("safety limit")));
        assert!(warnings
            .iter()
            .all(|warning| !warning.contains("raw pending join detail")));
    }

    #[test]
    fn connected_client_limit_counter_produces_sanitized_warning() {
        let health = HostRuntimeHealth {
            connected_client_limit_rejection_count: 1,
            last_error: Some("raw connected client detail".to_string()),
            ..HostRuntimeHealth::default()
        };
        let warnings = runtime_health_warning_messages(&health);
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("safety limit")));
        assert!(warnings
            .iter()
            .all(|warning| !warning.contains("raw connected client detail")));
    }
'''
    if not host_shutdown.endswith("\n}\n"):
        raise SystemExit("unexpected host_shutdown.rs ending")
    host_shutdown_path.write_text(host_shutdown[:-3] + tests + "\n}\n", encoding="utf-8")

# Replace command-boundary message parsing with explicit typed codes.
commands_path = Path("src-tauri/src/commands.rs")
commands = commands_path.read_text(encoding="utf-8")
old_impl = re.compile(
    r"impl DesktopCommandError \{\n    fn from_message\(.*?\n    \}\n\}\n",
    re.DOTALL,
)
new_impl = '''#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DesktopCommandErrorCode {
    NoActiveSession,
    ObserverReadOnly,
    NotActingPlayer,
    ClientRuntimeDisconnected,
    InvalidJoinPayload,
    HostRejectedAction,
    CommandFailed,
}

impl DesktopCommandErrorCode {
    fn as_str(self) -> &'static str {
        match self {
            Self::NoActiveSession => "NO_ACTIVE_SESSION",
            Self::ObserverReadOnly => "OBSERVER_READ_ONLY",
            Self::NotActingPlayer => "NOT_ACTING_PLAYER",
            Self::ClientRuntimeDisconnected => "CLIENT_RUNTIME_DISCONNECTED",
            Self::InvalidJoinPayload => "INVALID_JOIN_PAYLOAD",
            Self::HostRejectedAction => "HOST_REJECTED_ACTION",
            Self::CommandFailed => "COMMAND_FAILED",
        }
    }

    fn recoverable(self) -> bool {
        !matches!(self, Self::ClientRuntimeDisconnected | Self::CommandFailed)
    }
}

impl DesktopCommandError {
    fn new(code: DesktopCommandErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.as_str().to_string(),
            message: message.into(),
            recoverable: code.recoverable(),
        }
    }
}

fn command_failed(message: String) -> DesktopCommandError {
    DesktopCommandError::new(DesktopCommandErrorCode::CommandFailed, message)
}

fn host_rejected_action(message: String) -> DesktopCommandError {
    DesktopCommandError::new(DesktopCommandErrorCode::HostRejectedAction, message)
}

fn require_active_host_session(active: bool) -> Result<(), DesktopCommandError> {
    if active {
        Ok(())
    } else {
        Err(DesktopCommandError::new(
            DesktopCommandErrorCode::NoActiveSession,
            "no active host session",
        ))
    }
}

fn require_active_client_session(
    status: Option<&ClientSessionStatus>,
) -> Result<(), DesktopCommandError> {
    let status = status.ok_or_else(|| {
        DesktopCommandError::new(
            DesktopCommandErrorCode::NoActiveSession,
            "no active client session",
        )
    })?;
    if status.terminated {
        return Err(DesktopCommandError::new(
            DesktopCommandErrorCode::ClientRuntimeDisconnected,
            status
                .last_error
                .clone()
                .unwrap_or_else(|| "client runtime is disconnected".to_string()),
        ));
    }
    Ok(())
}

fn require_table_action_context(
    viewer_mode: TableViewerMode,
    has_active_session: bool,
    client_terminated: bool,
    action_tray_available: bool,
) -> Result<(), DesktopCommandError> {
    if matches!(viewer_mode, TableViewerMode::Observer) {
        return Err(DesktopCommandError::new(
            DesktopCommandErrorCode::ObserverReadOnly,
            "observer mode cannot submit actions",
        ));
    }
    if !has_active_session {
        return Err(DesktopCommandError::new(
            DesktopCommandErrorCode::NoActiveSession,
            "no active live session is available for table actions",
        ));
    }
    if client_terminated {
        return Err(DesktopCommandError::new(
            DesktopCommandErrorCode::ClientRuntimeDisconnected,
            "client runtime is disconnected",
        ));
    }
    if !action_tray_available {
        return Err(DesktopCommandError::new(
            DesktopCommandErrorCode::NotActingPlayer,
            "the local player does not currently own the action window",
        ));
    }
    Ok(())
}

fn validate_join_payload_for_command(payload: &str) -> Result<JoinPayload, DesktopCommandError> {
    decode_join_payload(payload.trim()).map_err(|error| {
        DesktopCommandError::new(DesktopCommandErrorCode::InvalidJoinPayload, error.to_string())
    })
}
'''
commands, count = old_impl.subn(new_impl, commands, count=1)
if count != 1:
    raise SystemExit("DesktopCommandError impl replacement failed")

commands = commands.replace(
    '''    let result = state.host_start_tournament().map_err(|message| {
        DesktopCommandError::from_message(message, "HOST_REJECTED_ACTION", true)
    })?;''',
    '''    let active = state.host_session_status().map_err(command_failed)?.is_some();
    require_active_host_session(active)?;
    let result = state
        .host_start_tournament()
        .map_err(host_rejected_action)?;''',
    1,
)
commands = commands.replace(
    '''    let result = state.join_host_session(request).map_err(|message| {
        DesktopCommandError::from_message(message, "INVALID_JOIN_PAYLOAD", true)
    })?;''',
    '''    validate_join_payload_for_command(&request.join_payload)?;
    let result = state.join_host_session(request).map_err(host_rejected_action)?;''',
    1,
)
commands = commands.replace(
    '''    let result = state.client_claim_lobby_seat(request).map_err(|message| {
        DesktopCommandError::from_message(message, "HOST_REJECTED_ACTION", true)
    })?;''',
    '''    let status = state.client_session_status().map_err(command_failed)?;
    require_active_client_session(status.as_ref())?;
    let result = state
        .client_claim_lobby_seat(request)
        .map_err(host_rejected_action)?;''',
    1,
)
commands = commands.replace(
    '''    let result = state
        .client_set_lobby_ready_state(request)
        .map_err(|message| {
            DesktopCommandError::from_message(message, "HOST_REJECTED_ACTION", true)
        })?;''',
    '''    let status = state.client_session_status().map_err(command_failed)?;
    require_active_client_session(status.as_ref())?;
    let result = state
        .client_set_lobby_ready_state(request)
        .map_err(host_rejected_action)?;''',
    1,
)
commands = commands.replace(
    '''    let result = submit_table_action_inner(
        viewer_mode,
        action_kind,
        raise_to_amount,
        |next_viewer_mode, next_action_kind, next_raise_to_amount| {
            state.submit_table_action(next_viewer_mode, next_action_kind, next_raise_to_amount)
        },
    )
    .map_err(|message| DesktopCommandError::from_message(message, "HOST_REJECTED_ACTION", true))?;''',
    '''    let host_active = state.host_session_status().map_err(command_failed)?.is_some();
    let client_status = state.client_session_status().map_err(command_failed)?;
    let has_active_session = host_active || client_status.is_some();
    let client_terminated = client_status.as_ref().is_some_and(|status| status.terminated);
    let action_tray_available = if has_active_session && !client_terminated {
        state
            .table_view(TableViewerMode::Local)
            .map_err(command_failed)?
            .action_tray
            .is_some()
    } else {
        false
    };
    require_table_action_context(
        viewer_mode,
        has_active_session,
        client_terminated,
        action_tray_available,
    )?;
    let result = submit_table_action_inner(
        viewer_mode,
        action_kind,
        raise_to_amount,
        |next_viewer_mode, next_action_kind, next_raise_to_amount| {
            state.submit_table_action(next_viewer_mode, next_action_kind, next_raise_to_amount)
        },
    )
    .map_err(host_rejected_action)?;''',
    1,
)
if "DesktopCommandError::from_message" in commands:
    raise SystemExit("substring command error classification remains")

tests = '''

    #[test]
    fn observer_action_error_code_is_typed_not_message_parsed() {
        let error = require_table_action_context(TableViewerMode::Observer, true, false, true)
            .expect_err("observer action must fail");
        assert_eq!(error.code, "OBSERVER_READ_ONLY");
        assert!(error.recoverable);
    }

    #[test]
    fn missing_action_tray_returns_not_acting_player_code() {
        let error = require_table_action_context(TableViewerMode::Local, true, false, false)
            .expect_err("non-acting player must fail");
        assert_eq!(error.code, "NOT_ACTING_PLAYER");
    }

    #[test]
    fn invalid_join_payload_returns_stable_typed_code() {
        let error = validate_join_payload_for_command("not-a-poker-invite")
            .expect_err("invalid invite must fail");
        assert_eq!(error.code, "INVALID_JOIN_PAYLOAD");
        assert!(error.recoverable);
    }

    #[test]
    fn command_error_code_is_independent_of_human_message_wording() {
        let first = DesktopCommandError::new(
            DesktopCommandErrorCode::HostRejectedAction,
            "first wording",
        );
        let second = DesktopCommandError::new(
            DesktopCommandErrorCode::HostRejectedAction,
            "completely different wording",
        );
        assert_eq!(first.code, second.code);
        assert_eq!(first.recoverable, second.recoverable);
        assert_ne!(first.message, second.message);
    }
'''
if "observer_action_error_code_is_typed_not_message_parsed" not in commands:
    if not commands.endswith("\n}\n"):
        raise SystemExit("unexpected commands.rs ending")
    commands = commands[:-3] + tests + "\n}\n"
commands_path.write_text(commands, encoding="utf-8")

Path("docs/runtime-validation/runtime-hardening-abuse-coverage.md").write_text(
    '''# Runtime hardening abuse-test coverage

Updated: 2026-07-28

This index maps hostile or malformed transport/protocol inputs to runnable tests. A case marked **Deferred** is not implied complete.

| Case | Status | Evidence |
|---|---|---|
| Oversized frame prefix | Covered | `networking::framing::tests::read_json_frame_rejects_payload_larger_than_max_before_allocation`; `runtime::tests::abuse::host_accept_loop_survives_oversized_truncated_and_malformed_join_frames` |
| Truncated frame body | Covered | `networking::framing::tests::read_json_frame_reports_truncated_bodies`; live host survival test in `runtime/tests/abuse.rs` |
| Malformed JSON | Covered | `networking::framing::tests::read_json_frame_reports_invalid_json_syntax`; live host survival test in `runtime/tests/abuse.rs` |
| Wrong first-message type | Covered | Initial-request tests under `runtime/tests/join.rs` |
| Invalid join token | Covered | Join rejection tests under `runtime/tests/join.rs` |
| Duplicate player ID join | Covered | Identity/join tests under `runtime/tests/join.rs` and `runtime/tests/session.rs` |
| Already-connected reconnect | Covered | Stable-code retry tests in `runtime/client_connect.rs`; integration coverage in `runtime/tests/reconnect.rs` |
| Reconnect after host-side disconnect | Covered | `runtime/tests/reconnect.rs` |
| Resync after stale server sequence | Covered | `runtime/tests/resync.rs` and `runtime/tests/end_to_end/integrity.rs` |
| Unsupported post-connect request | Covered | Host session rejection coverage under `runtime/tests/session.rs` |
| Bad signature | Covered | Signature/integrity tests under `runtime/tests/end_to_end/integrity.rs` |
| Oversized outbound frame | Covered | `networking::framing::tests::write_frame_bytes_rejects_payload_above_max_without_partial_write` |

When adding a protocol message or admission path, update this table with its abuse tests.
''',
    encoding="utf-8",
)
