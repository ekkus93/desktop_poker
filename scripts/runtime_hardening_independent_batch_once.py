from __future__ import annotations

import json
from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    text = file_path.read_text(encoding="utf-8")
    if old not in text:
        if new in text:
            return
        raise SystemExit(f"expected text not found in {path}: {old[:120]!r}")
    file_path.write_text(text.replace(old, new, 1), encoding="utf-8")


# Phase 3: keep HostRuntimeHealth's TypeScript contract and debug UI in sync.
replace_once(
    "src/api/desktop.ts",
    """  snapshotSyncErrorCount: number;
  lastError: string | null;""",
    """  snapshotSyncErrorCount: number;
  pendingJoinLimitRejectionCount: number;
  connectedClientLimitRejectionCount: number;
  lastError: string | null;""",
)

replace_once(
    "src/components/debug/DebugPanel.tsx",
    """        debugState.hostRuntimeHealth.snapshotSyncErrorCount > 0 ||
        debugState.hostRuntimeHealth.lastError != null)""",
    """        debugState.hostRuntimeHealth.snapshotSyncErrorCount > 0 ||
        debugState.hostRuntimeHealth.pendingJoinLimitRejectionCount > 0 ||
        debugState.hostRuntimeHealth.connectedClientLimitRejectionCount > 0 ||
        debugState.hostRuntimeHealth.lastError != null)""",
)

replace_once(
    "src/components/debug/DebugPanel.tsx",
    """            {debugState.hostRuntimeHealth.snapshotSyncErrorCount > 0 && (
              <li>
                <strong>Snapshot sync errors:</strong>{" "}
                {debugState.hostRuntimeHealth.snapshotSyncErrorCount}
              </li>
            )}
            {debugState.hostRuntimeHealth.lastError != null && (""",
    """            {debugState.hostRuntimeHealth.snapshotSyncErrorCount > 0 && (
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
            {debugState.hostRuntimeHealth.lastError != null && (""",
)

replace_once(
    "src/components/debug/DebugPanel.test.tsx",
    """    snapshotSyncErrorCount: 0,
    lastError: null,""",
    """    snapshotSyncErrorCount: 0,
    pendingJoinLimitRejectionCount: 0,
    connectedClientLimitRejectionCount: 0,
    lastError: null,""",
)
replace_once(
    "src/components/debug/DebugPanel.test.tsx",
    """          snapshotSyncErrorCount: 9,
        }),""",
    """          snapshotSyncErrorCount: 9,
          pendingJoinLimitRejectionCount: 10,
          connectedClientLimitRejectionCount: 11,
        }),""",
)
replace_once(
    "src/components/debug/DebugPanel.test.tsx",
    "    expect(screen.getByText(/Snapshot sync errors:/i)).toBeTruthy();",
    """    expect(screen.getByText(/Snapshot sync errors:/i)).toBeTruthy();
    expect(screen.getByText(/Pending join limit rejections:/i)).toBeTruthy();
    expect(screen.getByText(/Connected client limit rejections:/i)).toBeTruthy();""",
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
    """  DebugInspectorState,
  HostSessionStatus,""",
    """  DebugInspectorState,
  HostRuntimeHealth,
  HostSessionStatus,""",
)
replace_once(
    "src/api/desktop.contract.test.ts",
    "    const debugState: DebugInspectorState = {",
    """    const hostRuntimeHealth: HostRuntimeHealth = {
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

    const debugState: DebugInspectorState = {""",
)
replace_once(
    "src/api/desktop.contract.test.ts",
    "      hostRuntimeHealth: null,",
    "      hostRuntimeHealth,",
)
replace_once(
    "src/api/desktop.contract.test.ts",
    "    expect(sortedKeys(debugState)).toEqual(expectedKeys(\"DebugInspectorState\"));",
    """    expect(sortedKeys(debugState)).toEqual(expectedKeys("DebugInspectorState"));
    expect(sortedKeys(hostRuntimeHealth)).toEqual(
      expectedKeys("HostRuntimeHealth"),
    );""",
)

# Add focused normal-UI warning and serialization-key coverage.
replace_once(
    "src-tauri/src/app_state/host_shutdown.rs",
    """    #[test]
    fn healthy_runtime_has_no_normal_ui_warning() {""",
    """    #[test]
    fn pending_join_limit_rejection_is_visible_without_raw_error_detail() {
        let health = HostRuntimeHealth {
            pending_join_limit_rejection_count: 1,
            last_error: Some("raw pending-join transport detail".to_string()),
            ..HostRuntimeHealth::default()
        };

        let warnings = runtime_health_warning_messages(&health);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("safety limit"));
        assert!(!warnings[0].contains("raw pending-join transport detail"));
    }

    #[test]
    fn connected_client_limit_rejection_is_visible_without_raw_error_detail() {
        let health = HostRuntimeHealth {
            connected_client_limit_rejection_count: 1,
            last_error: Some("raw connected-client transport detail".to_string()),
            ..HostRuntimeHealth::default()
        };

        let warnings = runtime_health_warning_messages(&health);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("safety limit"));
        assert!(!warnings[0].contains("raw connected-client transport detail"));
    }

    #[test]
    fn host_runtime_health_serialization_keys_are_stable() {
        let value = serde_json::to_value(HostRuntimeHealth::default())
            .expect("host runtime health serializes");
        let mut actual = value
            .as_object()
            .expect("host runtime health serializes as an object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        actual.sort();
        let mut expected = vec![
            "acceptErrorCount",
            "clientRegistryErrorCount",
            "connectedClientLimitRejectionCount",
            "lastError",
            "lastSuccessfulPublishMs",
            "lastSuccessfulTickMs",
            "pendingJoinLimitRejectionCount",
            "publishErrorCount",
            "reconnectMarkErrorCount",
            "snapshotSyncErrorCount",
            "stateLockErrorCount",
            "streamCloneErrorCount",
            "streamTimeoutErrorCount",
            "tickAdvanceErrorCount",
        ];
        expected.sort();

        assert_eq!(actual, expected);
    }

    #[test]
    fn healthy_runtime_has_no_normal_ui_warning() {""",
)

# Phase 5: preserve timeout-vs-disconnect typing in the public polling API.
replace_once(
    "src-tauri/src/networking/runtime/client.rs",
    """    pub fn next_event(&self, timeout: Duration) -> Result<ClientRuntimeEvent, NetworkingError> {
        self.incoming.recv_timeout(timeout).map_err(|error| {
            NetworkingError::new(format!("timed out waiting for client event: {error}"))
        })
    }""",
    """    pub fn next_event(
        &self,
        timeout: Duration,
    ) -> Result<ClientRuntimeEvent, ClientRuntimePollError> {
        self.poll_event(timeout)
    }""",
)

# Phase 6: enforce the same frame-size cap in both directions.
replace_once(
    "src-tauri/src/networking/framing.rs",
    "/// Maximum accepted JSON frame payload size.",
    "/// Maximum JSON frame payload size for both inbound and outbound frames.",
)
replace_once(
    "src-tauri/src/networking/framing.rs",
    """fn write_frame_bytes<W: Write>(
    writer: &mut W,
    payload_bytes: &[u8],
    payload_len: u64,
) -> Result<(), NetworkingError> {
    let length = u32::try_from(payload_len)""",
    """fn write_frame_bytes<W: Write>(
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

    let length = u32::try_from(payload_len)""",
)
replace_once(
    "src-tauri/src/networking/framing.rs",
    """    #[test]
    fn write_frame_bytes_rejects_payloads_larger_than_u32() {""",
    """    #[test]
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
        assert!(writer.is_empty(), "oversized frames must not be partially written");
    }

    #[test]
    fn write_frame_bytes_rejects_payloads_larger_than_u32() {""",
)

# Phase 2: cover two additional no-state-change remote rejection classes.
action_tests = Path("src-tauri/src/networking/runtime/tests/action_outcomes.rs")
action_text = action_tests.read_text(encoding="utf-8")
addition = r'''

#[test]
fn remote_stale_window_rejection_does_not_mutate_state() {
    let fixture = started_runtime(now_epoch_ms());
    let before = fixture
        .authoritative_state
        .lock()
        .expect("authoritative state")
        .clone();
    let window = current_window(&fixture);
    let envelope = signed_action(
        &fixture,
        &window.player_id,
        "stale-action-window".to_string(),
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
    .expect("stale-window rejection is typed");

    assert!(matches!(
        outcome,
        RemoteActionSubmissionOutcome::RejectedNoStateChange { .. }
    ));
    assert_eq!(
        *fixture
            .authoritative_state
            .lock()
            .expect("authoritative state after rejection"),
        before
    );
    assert_eq!(
        fixture
            .tournament_runtime
            .lock()
            .expect("runtime")
            .as_ref()
            .expect("controller")
            .state(),
        &before
    );
}

#[test]
fn remote_invalid_raise_rejection_does_not_mutate_state() {
    let fixture = started_runtime(now_epoch_ms());
    let before = fixture
        .authoritative_state
        .lock()
        .expect("authoritative state")
        .clone();
    let window = current_window(&fixture);
    let envelope = signed_action(
        &fixture,
        &window.player_id,
        window.action_window_id,
        window.seat_index,
        ActionType::Raise,
        Some(0),
    );

    let outcome = handle_action_submission_request(
        &fixture.provider,
        envelope,
        &fixture.authoritative_state,
        &fixture.tournament_runtime,
    )
    .expect("invalid-raise rejection is typed");

    assert!(matches!(
        outcome,
        RemoteActionSubmissionOutcome::RejectedNoStateChange { .. }
    ));
    assert_eq!(
        *fixture
            .authoritative_state
            .lock()
            .expect("authoritative state after rejection"),
        before
    );
    assert_eq!(
        fixture
            .tournament_runtime
            .lock()
            .expect("runtime")
            .as_ref()
            .expect("controller")
            .state(),
        &before
    );
}
'''
if "remote_stale_window_rejection_does_not_mutate_state" not in action_text:
    action_tests.write_text(action_text + addition, encoding="utf-8")

# Phase 7: retain a reviewer-facing hostile-input coverage matrix.
coverage = Path("docs/runtime-validation/runtime-hardening-abuse-coverage.md")
coverage.write_text(
    """# Runtime hardening abuse-test coverage

This index maps hostile or malformed peer behavior to deterministic tests in the normal Rust suite. A row marked **Open** is intentionally not represented as complete.

| Hostile input or condition | Coverage | Runnable test |
|---|---|---|
| Oversized frame prefix | Covered | `networking::runtime::tests::abuse::host_accept_loop_survives_oversized_truncated_and_malformed_join_frames` |
| Truncated frame body | Covered | `networking::runtime::tests::abuse::host_accept_loop_survives_oversized_truncated_and_malformed_join_frames` |
| Malformed JSON | Covered | `networking::runtime::tests::abuse::host_accept_loop_survives_oversized_truncated_and_malformed_join_frames` |
| Wrong first-message `messageType` | Covered | initial-request rejection tests under `networking::runtime::tests` |
| Invalid join token | Covered | join-request rejection tests under `networking::runtime::tests` |
| Duplicate player ID join | Covered | join identity/capacity tests under `networking::runtime::tests` |
| Already-connected reconnect | Covered | reconnect tests using `RECONNECT_ALREADY_CONNECTED` |
| Reconnect after host-side disconnect | Covered | `networking::runtime::tests::reconnect` and end-to-end reconnect tests |
| Resync after stale server sequence | Covered | `networking::runtime::tests::resync` |
| Unsupported post-connect request | Covered | connected-session protocol rejection tests |
| Bad signature | Covered | integrity and protocol-warning tests |
| Remote stale action-window ID | Covered | `networking::runtime::tests::action_outcomes::remote_stale_window_rejection_does_not_mutate_state` |
| Remote invalid raise amount | Covered | `networking::runtime::tests::action_outcomes::remote_invalid_raise_rejection_does_not_mutate_state` |

## Explicitly deferred

- Physical-LAN packet loss, router isolation, and cross-device firewall behavior remain release/manual validation concerns rather than deterministic protocol-unit tests.
- Resource-exhaustion behavior above the configured connection limits is covered by bounded-counter/unit tests; sustained distributed load testing is not part of normal CI.
""",
    encoding="utf-8",
)
