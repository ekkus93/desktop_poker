# Desktop Poker Runtime Hardening Follow-up TODO

Created: 2026-07-28  
Repo: `ekkus93/desktop_poker`  
Branch: `master`  
Status: **OPEN**

## Purpose

This TODO captures the follow-up fixes from the post-reconciliation code review of `docs/DESKTOP_POKER_RUNTIME_HARDENING_CODE_REVIEW_TODO.md`.

The previous runtime-hardening pass materially improved the codebase, but the review found that the original TODO was marked complete too aggressively. The most important remaining issue is a real remote-client action correctness bug: the local host action path uses the new explicit action outcome enum, but the remote client action path still uses the old `submit_action(...).into_result()` adapter and can lose timeout-advanced state.

This file is intended to be handed to Claude Code or another coding agent. Keep every referenced file committed at the exact path named here.

## Non-negotiable implementation rules

1. Work directly on `master` unless the user explicitly asks for a branch or PR.
2. Do not create a PR unless the user explicitly asks for one.
3. Do not hide runtime failures behind silent fallbacks.
4. Keep Rust backend state authoritative. Do not move poker-rule decisions to the frontend.
5. Prefer typed outcomes/errors over string matching.
6. Add regression tests before marking a task complete.
7. If a best-effort path is genuinely necessary, record it in structured health/debug state or make the reason explicit in a comment and test.
8. Do not mark this TODO complete until the validation checklist at the end passes.

## Key files to inspect

- `src-tauri/src/networking/runtime/handlers.rs`
- `src-tauri/src/networking/runtime/host_session.rs`
- `src-tauri/src/networking/runtime/host.rs`
- `crates/poker-core/src/tournament/mod.rs`
- `crates/poker-core/src/tournament/controller_core.rs`
- `src-tauri/src/networking/runtime/tests/`
- `src/api/desktop.ts`
- `src/api/desktop.contract.test.ts`
- `src/fixtures/desktop-contract.json`
- `src/components/shell/DebugPanel.tsx`
- `src-tauri/src/commands.rs`
- `src-tauri/src/networking/framing.rs`

---

# Phase 0 — Baseline and guardrails

## Task 0.1 — Confirm current repository state

### Subtasks

- [ ] Confirm the working target is `master`.
- [ ] Confirm no temporary workflow/helper files from the prior Ralph loop remain.
- [ ] Read the current source for:
  - [ ] `src-tauri/src/networking/runtime/handlers.rs`
  - [ ] `src-tauri/src/networking/runtime/host_session.rs`
  - [ ] `src-tauri/src/networking/runtime/host.rs`
  - [ ] `crates/poker-core/src/tournament/controller_core.rs`
  - [ ] `src/api/desktop.ts`
  - [ ] `src/fixtures/desktop-contract.json`
  - [ ] `src/components/shell/DebugPanel.tsx`
- [ ] Run or inspect the latest CI/runtime evidence before making behavior changes.
- [ ] Record any pre-existing failures separately from regressions introduced by this TODO.

### Acceptance criteria

- [ ] The implementation agent knows the exact current baseline.
- [ ] No existing CI/runtime failures are silently reclassified as TODO regressions.

---

# Phase 1 — P0 remote client action outcome hardening

## Problem statement

The local host action path is hardened correctly: `HostServer::submit_action` calls `submit_action_with_outcome`, handles `Committed`, `RejectedNoStateChange`, and `TimeoutAdvancedThenRejected`, and publishes timeout-advanced state before returning the rejected action error.

The remote client path is not equivalent. `handle_action_submission_request` still calls `controller.submit_action(...)`, which delegates to `submit_action_with_outcome(...).into_result()`. For `TimeoutAdvancedThenRejected`, the controller state advances, but the adapter returns `Err`; the remote handler then exits before committing the advanced state into `authoritative_state` or publishing the transition to clients.

This can leave `tournament_runtime` advanced while `authoritative_state` and remote clients remain stale.

## Task 1.1 — Add a remote action outcome type

Add a typed return value for remote action handling. The exact shape can differ, but it must distinguish:

- committed action with a publishable transition;
- rejected action with no state change;
- timeout-advanced rejection with a publishable transition plus a rejection error.

Suggested shape:

```rust
pub(crate) enum RemoteActionSubmissionOutcome {
    Committed {
        previous_state: TournamentState,
        after_state: TournamentState,
    },
    RejectedNoStateChange {
        error: NetworkingError,
    },
    TimeoutAdvancedThenRejected {
        previous_state: TournamentState,
        after_state: TournamentState,
        error: NetworkingError,
    },
}
```

### Subtasks

- [ ] Define the outcome near the remote action handler or in a small helper module under `src-tauri/src/networking/runtime/`.
- [ ] Keep the type internal to the networking runtime unless a wider API is genuinely required.
- [ ] Do not encode these outcomes as strings.
- [ ] Preserve the existing `NetworkingError` behavior for command/rejection messages.

### Acceptance criteria

- [ ] Remote action handling has an explicit typed outcome.
- [ ] Code no longer needs to infer timeout-vs-rejection behavior from an `Err` after calling `submit_action`.

## Task 1.2 — Refactor `handle_action_submission_request` to use `submit_action_with_outcome`

### Required behavior

The remote handler must call `submit_action_with_outcome` directly. It must not call the old `submit_action(...).into_result()` adapter for remote client actions.

The remote path must enforce the same semantic guarantees as `HostServer::submit_action`:

- `Committed`:
  - controller state must differ from the previous state;
  - authoritative state must be committed;
  - transition must be publishable by the caller.
- `RejectedNoStateChange`:
  - controller state must remain identical to the previous state;
  - no authoritative commit;
  - no transition publication;
  - a protocol error reply is allowed and expected.
- `TimeoutAdvancedThenRejected`:
  - controller state must differ from the previous state;
  - authoritative state must be committed;
  - the timeout-advanced transition must be published;
  - the original action must still be rejected with a protocol error reply.

### Suggested code shape

This is only a shape, not a mandatory patch. Adapt names to the existing module layout.

```rust
let before_state = authoritative_state
    .lock()
    .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))?
    .clone();

let (next_state, action_outcome) = {
    let mut runtime = tournament_runtime
        .lock()
        .map_err(|_| NetworkingError::new("tournament runtime lock poisoned"))?;
    let controller = runtime
        .as_mut()
        .ok_or_else(|| NetworkingError::new("live tournament runtime is unavailable"))?;
    let rollback_controller = controller.clone();

    let action_outcome = match controller.submit_action_with_outcome(request, now_epoch_ms()) {
        Ok(outcome) => outcome,
        Err(error) => {
            *controller = rollback_controller;
            return Err(NetworkingError::new(error.to_string()));
        }
    };

    let next_state = controller.state().clone();
    if matches!(action_outcome, ActionSubmissionOutcome::RejectedNoStateChange { .. })
        && next_state != before_state
    {
        *controller = rollback_controller;
        return Err(NetworkingError::new(
            "rejected remote action mutated controller state; mutation was rolled back",
        ));
    }

    (next_state, action_outcome)
};
```

Then convert the outcome into `RemoteActionSubmissionOutcome` and call `commit_runtime_state` only for the publishable cases.

### Subtasks

- [ ] Parse and verify `PlayerActionSubmission` exactly as the current handler does.
- [ ] Keep signature verification before runtime mutation.
- [ ] Keep `sender_id` as the authoritative player ID; do not trust a payload player ID for authority.
- [ ] Replace `controller.submit_action(...)` with `controller.submit_action_with_outcome(...)`.
- [ ] Clone/rollback the controller on internal errors exactly as the local path does.
- [ ] Assert or explicitly check that `RejectedNoStateChange` does not mutate controller state.
- [ ] Call `commit_runtime_state` for `Committed` and `TimeoutAdvancedThenRejected` only.
- [ ] Return a typed result that tells the caller whether a transition must be published.

### Acceptance criteria

- [ ] No remote action code path uses `TournamentController::submit_action` for live remote client actions.
- [ ] Timeout-advanced remote submissions commit authoritative state before returning a rejection to the client.
- [ ] Rejected no-state-change submissions cannot mutate controller or authoritative state.

## Task 1.3 — Update `spawn_host_client_session` action handling

The remote client session loop currently captures `previous_state`, calls `handle_action_submission_request`, then publishes only when the handler returns `Ok(())`. That structure is not sufficient for timeout-advanced rejections because the action is rejected but state still changed.

### Required behavior

- [ ] Remove the old `previous_state` capture outside the handler unless it is still needed for diagnostics.
- [ ] Match on the new `RemoteActionSubmissionOutcome`.
- [ ] For `Committed`, publish the returned `(previous_state, after_state)` transition.
- [ ] For `TimeoutAdvancedThenRejected`, publish the returned transition first, then send a signed `ACTION_SUBMISSION_REJECTED` protocol error for the attempted action.
- [ ] For `RejectedNoStateChange`, do not publish; send only the signed protocol error.
- [ ] If publishing a committed or timeout-advanced transition fails, do not leave the client believing it is synchronized. Disconnect the client or force reconnect using the existing failure path.
- [ ] Keep rejection writes best-effort only after the authoritative/publish semantics are already correct.

### Suggested caller shape

```rust
match handle_action_submission_request(...) {
    Ok(RemoteActionSubmissionOutcome::Committed { previous_state, after_state }) => {
        publish_runtime_transition(..., &previous_state, &after_state, ...)?;
    }
    Ok(RemoteActionSubmissionOutcome::TimeoutAdvancedThenRejected {
        previous_state,
        after_state,
        error,
    }) => {
        if publish_runtime_transition(..., &previous_state, &after_state, ...).is_err() {
            disconnect_client(...);
            break;
        }
        send_action_rejection_protocol_error(error);
    }
    Ok(RemoteActionSubmissionOutcome::RejectedNoStateChange { error }) => {
        send_action_rejection_protocol_error(error);
    }
    Err(error) => {
        send_action_rejection_protocol_error(error);
    }
}
```

### Acceptance criteria

- [ ] Remote committed actions still publish exactly once.
- [ ] Remote timeout-advanced rejected actions publish exactly once.
- [ ] Remote no-state-change rejected actions publish zero state transitions.
- [ ] A failed publish does not result in a quiet stale client.

## Task 1.4 — Consider extracting shared local/remote action outcome logic

The local host path and remote client path should not drift again.

### Subtasks

- [ ] Evaluate whether `HostServer::submit_action` and the remote handler can share a helper for:
  - [ ] calling `submit_action_with_outcome`;
  - [ ] rollback on internal errors;
  - [ ] no-state-change mutation checks;
  - [ ] committing `Committed` and `TimeoutAdvancedThenRejected` states.
- [ ] If a helper improves clarity, extract it.
- [ ] If a helper would make the code harder to read, keep duplicated logic but add comments explaining that both paths must remain semantically equivalent.

### Acceptance criteria

- [ ] Future changes to action outcome semantics have an obvious single place or mirrored tests to update.

---

# Phase 2 — Remote action regression tests

## Task 2.1 — Add tests for remote timeout-advanced rejection

Add a deterministic test proving that an expired action submitted by a remote client advances timeout state, commits authoritative state, and publishes the resulting transition.

### Suggested location

Prefer one of:

- `src-tauri/src/networking/runtime/tests/session.rs`
- `src-tauri/src/networking/runtime/tests/end_to_end/integrity.rs`
- a new focused file such as `src-tauri/src/networking/runtime/tests/action_outcomes.rs`

If you create a new file, add it to the relevant `mod.rs` so it runs in normal Rust test suites.

### Subtasks

- [ ] Start a real test host with a short action timer.
- [ ] Connect at least one real `ClientRuntime` remote player.
- [ ] Seat and ready enough players to start a tournament.
- [ ] Start the tournament and wait for an action window owned by the remote client or construct the test so the remote client is next to act.
- [ ] Wait until the action deadline has passed.
- [ ] Submit the stale action from the remote client.
- [ ] Assert the remote client receives an action rejection or safe error.
- [ ] Assert the host authoritative state has advanced past the timed-out window.
- [ ] Assert connected clients receive the state transition, snapshot, or public event proving the timeout was published.
- [ ] Assert the runtime controller and `authoritative_state` are not divergent after the stale action.

### Acceptance criteria

- [ ] The test fails against the pre-fix remote handler.
- [ ] The test passes after Phase 1.
- [ ] The test does not rely on flaky long sleeps when a shorter deterministic wait/poll is possible.

## Task 2.2 — Add tests for remote no-state-change rejections

### Cases to cover

- [ ] Remote wrong-player action is rejected without state change.
- [ ] Remote stale action-window ID is rejected without state change.
- [ ] Remote invalid raise/bet amount is rejected without state change.

### Subtasks

- [ ] Capture host authoritative state before the rejected remote submission.
- [ ] Submit the bad action from a real remote runtime or signed remote envelope.
- [ ] Assert the host authoritative state equals the pre-action state.
- [ ] Assert no new publishable runtime event was emitted for the rejected no-state-change action.
- [ ] Assert the remote client sees a clear rejection, not a timeout/hang.

### Acceptance criteria

- [ ] No-state-change rejected remote actions cannot mutate either controller state or authoritative state.
- [ ] The client receives a rejection path that is visible to the UI/session layer.

## Task 2.3 — Add tests for remote committed action parity

### Subtasks

- [ ] Submit a legal action from a remote client.
- [ ] Assert authoritative state changes.
- [ ] Assert clients receive the corresponding transition.
- [ ] Assert the behavior remains equivalent to local `HostServer::submit_action` for the same action.

### Acceptance criteria

- [ ] Remote committed action behavior is unchanged except for any intentional cleanup from Phase 1.

---

# Phase 3 — HostRuntimeHealth DTO drift and UI coverage

## Problem statement

Rust `HostRuntimeHealth` includes these newer fields:

- `pending_join_limit_rejection_count`
- `connected_client_limit_rejection_count`

The TypeScript `HostRuntimeHealth` type and debug UI do not include or render those fields. The top-level DTO fixture also misses nested `HostRuntimeHealth` because its `DebugInspectorState` sample uses `hostRuntimeHealth: null`.

## Task 3.1 — Update TypeScript DTO type

### Subtasks

- [ ] Add `pendingJoinLimitRejectionCount: number` to `HostRuntimeHealth` in `src/api/desktop.ts`.
- [ ] Add `connectedClientLimitRejectionCount: number` to `HostRuntimeHealth` in `src/api/desktop.ts`.
- [ ] Keep camelCase names consistent with Rust `#[serde(rename_all = "camelCase")]`.

### Acceptance criteria

- [ ] TypeScript has all serialized Rust `HostRuntimeHealth` fields.

## Task 3.2 — Update debug panel rendering

### Subtasks

- [ ] In `src/components/shell/DebugPanel.tsx`, include the two new counters in the condition that decides whether to render `Host runtime health`.
- [ ] Render a line for pending join safety-limit rejections when `pendingJoinLimitRejectionCount > 0`.
- [ ] Render a line for connected client safety-limit rejections when `connectedClientLimitRejectionCount > 0`.
- [ ] Add `data-testid` attributes if existing tests use them or if new tests need stable selectors.

### Acceptance criteria

- [ ] Debug UI exposes all host runtime health counters.
- [ ] Safety-limit rejections are visible in debug diagnostics.

## Task 3.3 — Strengthen DTO contract fixtures

### Subtasks

- [ ] Extend `src/fixtures/desktop-contract.json` to include a `HostRuntimeHealth` key list, or include nested expected keys under `DebugInspectorState` in a way the tests actually check.
- [ ] Update `src/api/desktop.contract.test.ts` to instantiate a non-null `hostRuntimeHealth` object.
- [ ] Assert `Object.keys(debugState.hostRuntimeHealth)` matches the fixture keys.
- [ ] Add all current fields, including:
  - [ ] `acceptErrorCount`
  - [ ] `streamTimeoutErrorCount`
  - [ ] `tickAdvanceErrorCount`
  - [ ] `publishErrorCount`
  - [ ] `stateLockErrorCount`
  - [ ] `streamCloneErrorCount`
  - [ ] `clientRegistryErrorCount`
  - [ ] `reconnectMarkErrorCount`
  - [ ] `snapshotSyncErrorCount`
  - [ ] `pendingJoinLimitRejectionCount`
  - [ ] `connectedClientLimitRejectionCount`
  - [ ] `lastError`
  - [ ] `lastSuccessfulTickMs`
  - [ ] `lastSuccessfulPublishMs`
- [ ] Add a Rust-side serialization-key test if it does not already exist, so Rust and TypeScript fixtures cannot drift silently.

### Acceptance criteria

- [ ] A missing or extra `HostRuntimeHealth` serialized field fails tests.
- [ ] Nested DTO drift is covered, not just top-level object keys.

## Task 3.4 — Extend runtime warning tests

### Subtasks

- [ ] Add/extend tests in `src-tauri/src/app_state/host_shutdown.rs` for both safety-limit counters.
- [ ] Assert `pending_join_limit_rejection_count > 0` produces the sanitized safety-limit warning.
- [ ] Assert `connected_client_limit_rejection_count > 0` produces the sanitized safety-limit warning.
- [ ] Assert raw `last_error` detail still does not leak to normal UI warnings.

### Acceptance criteria

- [ ] Normal runtime warnings remain sanitized and actionable.
- [ ] Both admission-limit counters feed the normal UI warning path.

---

# Phase 4 — Replace backend command-error string matching

## Problem statement

The frontend no longer string-matches command errors, which is good. But `DesktopCommandError::from_message` still classifies backend errors by substring matching human-readable text. That makes the command boundary fragile.

## Task 4.1 — Introduce typed command/app error codes

### Suggested shape

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopCommandErrorCode {
    NoActiveSession,
    ObserverReadOnly,
    NotActingPlayer,
    StaleActionWindow,
    NetworkTimeout,
    ClientRuntimeDisconnected,
    InvalidJoinPayload,
    HostRejectedAction,
    CommandFailed,
}
```

Then provide a conversion into the serialized `DesktopCommandError`.

### Subtasks

- [ ] Define stable internal error codes near `DesktopCommandError` or in a dedicated app error module.
- [ ] Map codes to the existing serialized strings:
  - [ ] `NO_ACTIVE_SESSION`
  - [ ] `OBSERVER_READ_ONLY`
  - [ ] `NOT_ACTING_PLAYER`
  - [ ] `STALE_ACTION_WINDOW`
  - [ ] `NETWORK_TIMEOUT`
  - [ ] `CLIENT_RUNTIME_DISCONNECTED`
  - [ ] `INVALID_JOIN_PAYLOAD`
  - [ ] `HOST_REJECTED_ACTION`
  - [ ] `COMMAND_FAILED`
- [ ] Preserve `message` as human-readable text.
- [ ] Preserve `recoverable` semantics.

### Acceptance criteria

- [ ] There is an internal typed code path for command errors.
- [ ] The serialized frontend contract remains `{ code, message, recoverable }`.

## Task 4.2 — Remove substring classification from critical command paths

### Critical commands

- [ ] `host_start_tournament`
- [ ] `join_host_session`
- [ ] `client_claim_lobby_seat`
- [ ] `client_set_lobby_ready_state`
- [ ] `submit_table_action`

### Subtasks

- [ ] Replace `DesktopCommandError::from_message(...)` calls on critical commands with explicit typed construction.
- [ ] Where source functions currently return `Result<T, String>`, either:
  - [ ] migrate those source functions to return a typed app error, or
  - [ ] wrap specific call sites with explicit known codes instead of substring parsing.
- [ ] Do not attempt to classify a generic unknown string as a specific code.
- [ ] Keep a conservative fallback code for truly unknown failures.

### Acceptance criteria

- [ ] Changing a human-readable error message does not change the serialized `code` for these commands.
- [ ] There is no substring-matching decision tree for critical command error codes.

## Task 4.3 — Add command error tests

### Subtasks

- [ ] Add Rust tests proving observer action rejection returns `OBSERVER_READ_ONLY` without relying on message text.
- [ ] Add Rust tests proving not-acting-player action rejection returns `NOT_ACTING_PLAYER` without relying on message text.
- [ ] Add Rust tests proving invalid join payload returns `INVALID_JOIN_PAYLOAD`.
- [ ] Add frontend tests proving `DesktopCommandFailure` preserves backend `code`, `message`, and `recoverable`.

### Acceptance criteria

- [ ] Error-code behavior is stable under wording changes.

---

# Phase 5 — Remove or type `ClientRuntime::next_event`

## Problem statement

Production code uses `ClientRuntime::poll_event`, which distinguishes timeout from disconnected runtime channel. But `ClientRuntime::next_event` still exists and converts all `recv_timeout` failures into a generic `NetworkingError`. Tests still call it directly, which leaves a footgun for future production code.

## Task 5.1 — Decide the policy for `next_event`

Choose one:

- [ ] Remove `next_event` and update tests to call `poll_event`.
- [ ] Rename it to `next_event_for_test` and guard it with `#[cfg(test)]`.
- [ ] Change it to return `Result<ClientRuntimeEvent, ClientRuntimePollError>` and delegate to `poll_event`.

Preferred option: change it to delegate to `poll_event` with the typed error, then update tests accordingly.

### Acceptance criteria

- [ ] There is no public production method that erases timeout vs disconnected-channel distinction.

## Task 5.2 — Update tests using `next_event`

### Subtasks

- [ ] Search all Rust tests for `.next_event(`.
- [ ] Update tests to handle `ClientRuntimePollError::Timeout` and `ClientRuntimePollError::Disconnected` explicitly where relevant.
- [ ] Keep helper functions if they improve test readability, but name them as test helpers.

### Acceptance criteria

- [ ] Test code no longer normalizes typed poll failures into generic networking failures.

---

# Phase 6 — Outbound frame-size policy

## Problem statement

Inbound frame reading enforces `MAX_FRAME_PAYLOAD_BYTES` before allocation. Outbound frame writing only checks whether the payload length fits in `u32`. That asymmetry can let this process emit frames that hardened peers reject.

## Task 6.1 — Decide outbound policy

Preferred policy: outbound JSON payloads should also be capped at `MAX_FRAME_PAYLOAD_BYTES`.

### Subtasks

- [ ] Confirm there is no valid application message expected to exceed `MAX_FRAME_PAYLOAD_BYTES`.
- [ ] If large snapshots are possible, decide whether to implement paging/compression later; do not silently raise the max here.
- [ ] Document the decision in `src-tauri/src/networking/framing.rs`.

### Acceptance criteria

- [ ] The outbound/inbound size policy is explicit.

## Task 6.2 — Enforce outbound cap

### Suggested code shape

```rust
if payload_bytes.len() > MAX_FRAME_PAYLOAD_BYTES {
    return Err(NetworkingError::new(format!(
        "frame payload exceeds maximum allowed size: {} > {}",
        payload_bytes.len(),
        MAX_FRAME_PAYLOAD_BYTES,
    )));
}
```

### Subtasks

- [ ] Add the check before writing the length prefix.
- [ ] Keep the existing `u32::try_from` guard as a secondary safety check.
- [ ] Ensure error wording is clear enough for logs/tests.

### Acceptance criteria

- [ ] The process cannot emit outbound JSON frames larger than the configured max.

## Task 6.3 — Add outbound frame tests

### Subtasks

- [ ] Add a test that writes a payload at exactly `MAX_FRAME_PAYLOAD_BYTES` and succeeds.
- [ ] Add a test that attempts to write a payload at `MAX_FRAME_PAYLOAD_BYTES + 1` and fails before writing body bytes.
- [ ] Add a test that the writer receives no partial oversized body.

### Acceptance criteria

- [ ] Inbound and outbound frame-size behavior are both regression-tested.

---

# Phase 7 — Abuse-test discoverability and coverage index

## Task 7.1 — Add an abuse coverage index

The current abuse tests are useful but easy to miss. Add a short index comment or markdown note that maps hostile input cases to test names.

### Suggested location

Prefer an in-repo note such as:

- `docs/runtime-validation/runtime-hardening-abuse-coverage.md`

or a module-level comment in:

- `src-tauri/src/networking/runtime/tests/abuse.rs`

### Cases to map

- [ ] Oversized frame prefix.
- [ ] Truncated frame body.
- [ ] Malformed JSON.
- [ ] Wrong protocol `messageType` as first message.
- [ ] Invalid join token.
- [ ] Duplicate player ID join.
- [ ] Already-connected reconnect rejection.
- [ ] Reconnect after host-side disconnect.
- [ ] Resync after stale server sequence.
- [ ] Unsupported post-connect request.
- [ ] Bad signature.

### Acceptance criteria

- [ ] A reviewer can quickly see which hostile input cases are tested and where.
- [ ] Missing cases are explicitly listed as open rather than implied complete.

## Task 7.2 — Fill any obvious abuse-test gaps

### Subtasks

- [ ] If any mapped hostile input case has no deterministic test, add one.
- [ ] Do not rely only on runtime smoke tests for protocol-level rejection semantics.
- [ ] Keep tests fast enough for normal CI where possible.

### Acceptance criteria

- [ ] The abuse matrix is not just documentation; it corresponds to actual runnable tests or clearly deferred items.

---

# Phase 8 — Validation and evidence

## Task 8.1 — Run Rust validation

### Commands

```bash
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --workspace --all-targets --all-features
cargo test --manifest-path crates/poker-core/Cargo.toml --all-targets
```

### Acceptance criteria

- [ ] All Rust validation commands pass.
- [ ] New remote-action tests fail before the fix and pass after the fix, or the commit message/test note explains how the regression was verified.

## Task 8.2 — Run frontend validation

### Commands

```bash
npm ci
npm run format:check
npm run lint
npm run test
npm run build
```

### Acceptance criteria

- [ ] All frontend validation commands pass.
- [ ] DTO fixture tests fail if `HostRuntimeHealth` fields drift.

## Task 8.3 — Run runtime validation

### Required automated evidence

- [ ] General CI passes.
- [ ] Release runtime validation passes.
- [ ] Multi-instance host/client smoke passes.
- [ ] Full gameplay runtime evidence remains passing.
- [ ] Reconnect/host-loss runtime evidence remains passing.
- [ ] NPC runtime evidence remains passing.

### Acceptance criteria

- [ ] Runtime validation evidence is committed under `docs/runtime-validation/` or otherwise retained by the repository's existing evidence publisher.
- [ ] The evidence names the commit SHA being validated.

## Task 8.4 — Update this TODO only after verification

### Subtasks

- [ ] Check off each completed task only after implementation and tests are committed.
- [ ] Add a short reconciliation note under `docs/runtime-validation/` summarizing:
  - [ ] fixed remote action outcome semantics;
  - [ ] added tests;
  - [ ] CI result;
  - [ ] runtime evidence result;
  - [ ] any intentionally deferred physical-LAN validation.
- [ ] Do not mark `Status: COMPLETE` until every non-deferred acceptance criterion is satisfied.

### Acceptance criteria

- [ ] This TODO's final status matches committed code and evidence.
- [ ] No task is marked complete solely because CI is green; it must also satisfy the behavior described here.

---

# Final completion checklist

- [ ] Remote client action submissions use explicit action outcomes, not `submit_action(...).into_result()`.
- [ ] Remote timeout-advanced rejections commit and publish timeout-advanced state.
- [ ] Remote no-state-change rejections cannot mutate controller or authoritative state.
- [ ] Remote committed actions still publish normally.
- [ ] `HostRuntimeHealth` Rust and TypeScript fields are in sync.
- [ ] Debug UI renders all runtime health counters.
- [ ] DTO contract tests catch nested `HostRuntimeHealth` drift.
- [ ] Critical Tauri command errors no longer use substring matching for error codes.
- [ ] `ClientRuntime::next_event` no longer erases typed poll failures in production API surface.
- [ ] Outbound frame-size behavior is explicitly documented and tested.
- [ ] Abuse-test coverage is indexed and missing cases are explicit.
- [ ] Rust validation passes.
- [ ] Frontend validation passes.
- [ ] Runtime validation passes.
- [ ] Reconciliation evidence is committed.
