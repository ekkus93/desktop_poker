# Desktop Poker Runtime Hardening Code Review TODO

Created: 2026-07-28
Repo: `ekkus93/desktop_poker`
Scope: follow-up work from a source-inspection code review of the Desktop Poker MVP.

## Purpose

This TODO file turns the code-review findings into an implementation checklist for Claude Code or another coding agent.

The current codebase has a good overall architecture: `poker-core` owns deterministic poker rules and tournament state, the Tauri/Rust backend owns networking/protocol/crypto/session state, and the React frontend mostly renders backend-projected state. The issues below are mostly runtime hardening, failure-mode visibility, networking robustness, and contract drift problems.

Do **not** rewrite the app. Fix these issues incrementally, with tests after each phase.

## Non-negotiable implementation rules

1. Do not hide runtime failures behind silent fallbacks.
2. Do not replace explicit errors with best-effort behavior unless the TODO explicitly says the failure is non-fatal.
3. When best-effort behavior is genuinely necessary, record it in structured health/debug state.
4. Prefer typed errors or typed outcome enums over string matching.
5. Add tests for each bug fix before marking a task complete.
6. Keep the Rust backend authoritative. Do not move game-rule decisions to the frontend.
7. Keep Android/Desktop protocol compatibility in mind; do not casually rename serialized protocol fields.
8. Avoid creating new assistant-only reference files unless they are committed to the repository.

---

# Phase 0 — Baseline validation

## Task 0.1 — Record current validation status

Before editing behavior, run the current validation suite and capture the result in a runtime-validation note.

### Subtasks

- [ ] Run frontend validation:
  - [ ] `npm ci`
  - [ ] `npm run format:check`
  - [ ] `npm run lint`
  - [ ] `npm run test`
  - [ ] `npm run build`
- [ ] Run Rust validation:
  - [ ] `cargo fmt --check --manifest-path src-tauri/Cargo.toml`
  - [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --all-features -- -D warnings`
  - [ ] `cargo test --manifest-path src-tauri/Cargo.toml --workspace --all-targets --all-features`
  - [ ] `cargo test --manifest-path crates/poker-core/Cargo.toml --all-targets`
- [ ] If any command fails before changes, save the failure as baseline evidence instead of silently ignoring it.
- [ ] Create or update a short note under `docs/runtime-validation/` with:
  - [ ] command run
  - [ ] pass/fail result
  - [ ] relevant failure excerpt if any
  - [ ] current commit SHA

### Acceptance criteria

- [ ] There is a committed validation note showing the pre-fix baseline.
- [ ] Existing failures are explicitly documented and not confused with regressions from this TODO.

---

# Phase 1 — P0 inbound frame-size hardening

## Problem

`src-tauri/src/networking/framing.rs` reads a 4-byte inbound length, casts it to `usize`, allocates `vec![0_u8; length]`, then reads the frame body. A malicious or broken LAN peer can advertise a huge frame length and force excessive memory allocation before sending a body.

## Task 1.1 — Add a maximum inbound frame size

### Target files

- `src-tauri/src/networking/framing.rs`
- `src-tauri/src/networking/mod.rs` if constants should be exported

### Subtasks

- [ ] Add a constant for maximum inbound frame payload size.
  - Suggested name: `MAX_FRAME_PAYLOAD_BYTES`
  - Suggested initial value: `1_048_576` bytes, unless current snapshot sizes require more.
- [ ] After reading the 4-byte frame length, reject frames larger than the limit before allocating the body buffer.
- [ ] Return a clear `NetworkingError`, for example:
  - `frame payload exceeds maximum allowed size: {length} > {MAX_FRAME_PAYLOAD_BYTES}`
- [ ] Make the limit apply to all inbound JSON frames: client, host, join, reconnect, resync, snapshot, public events, and private events.
- [ ] Do not silently truncate oversized frames.
- [ ] Do not read and discard oversized bodies; reject immediately after the length prefix.

### Suggested code shape

```rust
pub const MAX_FRAME_PAYLOAD_BYTES: usize = 1_048_576;

fn read_json_frame_from_reader<T: DeserializeOwned, R: Read>(
    reader: &mut R,
) -> Result<T, NetworkingError> {
    let mut length_bytes = [0_u8; 4];
    reader
        .read_exact(&mut length_bytes)
        .map_err(|error| NetworkingError::new(format!("failed to read frame length: {error}")))?;

    let length = u32::from_be_bytes(length_bytes) as usize;
    if length > MAX_FRAME_PAYLOAD_BYTES {
        return Err(NetworkingError::new(format!(
            "frame payload exceeds maximum allowed size: {length} > {MAX_FRAME_PAYLOAD_BYTES}"
        )));
    }

    let mut payload_bytes = vec![0_u8; length];
    reader
        .read_exact(&mut payload_bytes)
        .map_err(|error| NetworkingError::new(format!("failed to read frame body: {error}")))?;

    serde_json::from_slice(&payload_bytes)
        .map_err(|error| NetworkingError::new(format!("invalid frame JSON: {error}")))
}
```

### Tests

Add tests in `src-tauri/src/networking/framing.rs`:

- [ ] `read_json_frame_rejects_payload_larger_than_max_before_allocation`
- [ ] `read_json_frame_accepts_payload_at_max_size_when_json_valid` if practical
- [ ] `read_json_frame_accepts_small_payload_after_limit_added`
- [ ] Existing truncation and invalid JSON tests still pass.

### Acceptance criteria

- [ ] Oversized inbound frame lengths fail before payload allocation.
- [ ] The error is explicit and test-covered.
- [ ] All existing frame tests still pass.

---

# Phase 2 — P0/P1 explicit client runtime shutdown

## Problem

`ClientRuntime` spawns a detached runtime thread and does not store a stop flag or `JoinHandle`. `leave_client_session` drops the session, but the runtime thread may remain blocked on a TCP read or reconnecting. The host runtime has explicit shutdown; the client should too.

## Task 2.1 — Add client runtime stop signal and join handle

### Target files

- `src-tauri/src/networking/runtime/mod.rs`
- `src-tauri/src/networking/runtime/client.rs`
- `src-tauri/src/app_state/session.rs`
- tests under `src-tauri/src/networking/runtime/tests.rs` or existing runtime test modules

### Subtasks

- [ ] Add a `stop_signal: Arc<AtomicBool>` to `ClientRuntime`.
- [ ] Add `runtime_thread: Option<JoinHandle<()>>` to `ClientRuntime`.
- [ ] Pass `stop_signal` into the spawned client runtime thread.
- [ ] Check `stop_signal` at the top of the client read/reconnect loop.
- [ ] On shutdown, set `stop_signal = true`.
- [ ] Wake a blocking read by shutting down or dropping the active stream.
- [ ] Join the runtime thread in `Drop for ClientRuntime`.
- [ ] Avoid panicking if the worker thread already panicked.
- [ ] Ensure `leave_client_session` triggers this drop path.

### Suggested code shape

```rust
pub struct ClientRuntime {
    incoming: Receiver<ClientRuntimeEvent>,
    reconnect_identity: Arc<Mutex<ClientReconnectIdentity>>,
    command_connection: Arc<Mutex<ClientCommandConnection>>,
    stop_signal: Arc<AtomicBool>,
    runtime_thread: Option<JoinHandle<()>>,
}

impl Drop for ClientRuntime {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);

        if let Ok(mut connection) = self.command_connection.lock() {
            if let Some(stream_handle) = connection.stream.take() {
                if let Ok(stream) = stream_handle.lock() {
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                }
            }
        }

        if let Some(handle) = self.runtime_thread.take() {
            let _ = handle.join();
        }
    }
}
```

Adjust as needed if the active read stream is not the same as `command_connection.stream`.

### Tests

- [ ] Add a test that creates a client runtime, drops it, and verifies the runtime does not hang.
- [ ] Add a test or runtime smoke script for joining then leaving a client session.
- [ ] Add a reconnect/drop test if existing test harnesses support it.

### Acceptance criteria

- [ ] Client runtime has explicit shutdown semantics comparable to `HostServer`.
- [ ] Dropping a client session does not leave a long-lived runtime thread behind.
- [ ] Tests prove drop does not hang.

## Task 2.2 — Distinguish normal shutdown from disconnected error

### Subtasks

- [ ] When `stop_signal` is set, the runtime thread should exit without emitting a scary user-facing error.
- [ ] Do not report `Disconnected from host` for intentional `leave_client_session`.
- [ ] If a shutdown event is needed internally, add a distinct event variant rather than overloading `Disconnected`.

### Acceptance criteria

- [ ] User-triggered leave is not shown as an unexpected host disconnect.
- [ ] Unexpected network failure still surfaces as a terminated/disconnected session.

---

# Phase 3 — Host connection and thread-spawn hardening

## Problem

The host accept loop spawns a new thread for every accepted initial join. Combined with unbounded inbound frames, this can exhaust memory or threads on a LAN. Frame size must be fixed first, but connection limits are also needed.

## Task 3.1 — Add explicit host connection limits

### Target files

- `src-tauri/src/networking/runtime/mod.rs`
- `src-tauri/src/networking/runtime/host.rs`
- `src-tauri/src/networking/runtime/host_session.rs`
- runtime tests

### Subtasks

- [ ] Add host runtime constants or config fields:
  - [ ] `max_connected_clients`
  - [ ] `max_pending_initial_joins`
  - [ ] optional `initial_join_timeout_ms`
- [ ] Default `max_connected_clients` should align with tournament max players plus a small buffer, not be unbounded.
- [ ] Increment a pending-join counter before spawning `host-initial-join`.
- [ ] Decrement pending-join counter on every thread exit path.
- [ ] Reject or drop new joins when pending joins exceed limit.
- [ ] Record limit rejections in `HostRuntimeHealth`.
- [ ] Do not let rejected joins mutate authoritative tournament state.

### Suggested health additions

In `HostRuntimeHealth`, add fields similar to:

```rust
pub pending_join_limit_rejection_count: u64,
pub connected_client_limit_rejection_count: u64,
```

### Tests

- [ ] Test pending join counter decrements on success.
- [ ] Test pending join counter decrements on malformed join failure.
- [ ] Test connections beyond limit are rejected and health counter increments.
- [ ] Test rejection does not mutate tournament participants or seats.

### Acceptance criteria

- [ ] The host cannot spawn unlimited join-handler threads.
- [ ] Limit-related failures are visible in host runtime health.

## Task 3.2 — Ensure connected-client limit is enforced atomically

### Subtasks

- [ ] Check connected-client count while holding the client registry lock.
- [ ] Enforce the limit before inserting into `clients`.
- [ ] Avoid races where two join handlers both see capacity and both insert.
- [ ] Return a clear protocol error for a legitimate client rejected because the table is full or connection limit is reached.

### Acceptance criteria

- [ ] Client registry cannot exceed the configured maximum through concurrent joins.
- [ ] Legitimate users see a clear join rejection.

---

# Phase 4 — Replace fragile 1-second command deadlines

## Problem

Desktop app commands wait only 1 second for join snapshots, action acknowledgements, seat claims, and ready-state confirmation. This is likely to cause false failures in debug builds, slow machines, CPU contention, Wi-Fi latency, or local LLM/NPC-heavy sessions.

## Task 4.1 — Centralize client operation deadlines

### Target files

- `src-tauri/src/app_state/session.rs`
- `src-tauri/src/app_state/mod.rs` if constants are shared
- related tests

### Subtasks

- [ ] Add named constants for client operation timeouts:
  - [ ] `INITIAL_JOIN_SNAPSHOT_TIMEOUT`
  - [ ] `CLIENT_ACTION_ACK_TIMEOUT`
  - [ ] `CLIENT_LOBBY_ACK_TIMEOUT`
  - [ ] optional `READY_CHECK_TABLE_START_POLL_TIMEOUT`
- [ ] Replace inline `Duration::from_secs(1)` with named constants.
- [ ] Use a longer default for lobby/action acknowledgement, probably 5 seconds.
- [ ] Keep tests deterministic by allowing test-only override or dependency injection where needed.
- [ ] Include the timeout duration in error messages.

### Suggested code shape

```rust
const INITIAL_JOIN_SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(5);
const CLIENT_ACTION_ACK_TIMEOUT: Duration = Duration::from_secs(5);
const CLIENT_LOBBY_ACK_TIMEOUT: Duration = Duration::from_secs(5);
const READY_CHECK_TABLE_START_POLL_TIMEOUT: Duration = Duration::from_millis(250);
```

### Tests

- [ ] Existing timeout tests updated for named constants.
- [ ] Test error messages include the configured duration.
- [ ] Test successful operation path is unchanged.

### Acceptance criteria

- [ ] No hardcoded 1-second command deadlines remain in live client session code.
- [ ] Timeout errors remain explicit.

## Task 4.2 — Improve UI behavior around pending operations

### Target files

- `src/screens/TournamentLobbyScreen.tsx`
- `src/screens/MainTableScreen.tsx`
- other screen files that call lobby/action commands

### Subtasks

- [ ] Ensure buttons show a pending/disabled state while a command is in flight.
- [ ] Avoid duplicate command submission while waiting for backend confirmation.
- [ ] Show timeout errors as recoverable user-facing errors.
- [ ] Do not automatically leave the session solely because a command timed out.

### Acceptance criteria

- [ ] Slow command acknowledgement does not cause duplicate actions or confusing UI state.

---

# Phase 5 — Typed client event receive errors

## Problem

`ClientRuntime::next_event` maps all `recv_timeout` failures into a generic `NetworkingError`. `DesktopClientSession::refresh` treats any error as "no more events right now." This conflates normal polling timeout with channel disconnect/dead runtime.

## Task 5.1 — Add typed next-event result/error

### Target files

- `src-tauri/src/networking/runtime/mod.rs`
- `src-tauri/src/networking/runtime/client.rs`
- `src-tauri/src/app_state/session.rs`

### Subtasks

- [ ] Add a typed error enum for `next_event`, for example:

```rust
pub enum ClientRuntimePollError {
    Timeout,
    Disconnected,
}
```

- [ ] Change `ClientRuntime::next_event` to distinguish:
  - [ ] `RecvTimeoutError::Timeout`
  - [ ] `RecvTimeoutError::Disconnected`
- [ ] Update `DesktopClientSession::refresh`:
  - [ ] `Timeout` means stop draining events normally.
  - [ ] `Disconnected` means mark the session terminated or set a clear internal error.
- [ ] Update `await_condition` to stop early on runtime disconnect.
- [ ] Preserve public Tauri command error strings at the boundary if needed, but keep typed logic internally.

### Tests

- [ ] Test normal timeout does not mark session terminated.
- [ ] Test disconnected event channel marks session unhealthy/terminated.
- [ ] Test `await_condition` exits on channel disconnect instead of spinning until timeout.

### Acceptance criteria

- [ ] Dead runtime thread/channel cannot be mistaken for “no events available.”
- [ ] UI can surface a clear fatal client-runtime error.

---

# Phase 6 — Remove string-matching from reconnect behavior

## Problem

Reconnect retry logic checks whether an error string contains `participant is already connected`. This is brittle. Protocol behavior should use typed error codes.

## Task 6.1 — Add protocol error codes for reconnectable rejection causes

### Target files

- `src-tauri/src/protocol/models/mod.rs`
- `src-tauri/src/networking/runtime/client_connect.rs`
- `src-tauri/src/networking/runtime/handlers.rs`
- any function building protocol error envelopes

### Subtasks

- [ ] Define constants or an enum for protocol error codes.
- [ ] Include at least:
  - [ ] `JOIN_REJECTED`
  - [ ] `RECONNECT_ALREADY_CONNECTED`
  - [ ] `RECONNECT_REJECTED`
  - [ ] `STALE_COUNTER`
  - [ ] `INVALID_SIGNATURE`
  - [ ] `TABLE_OR_SESSION_MISMATCH`
- [ ] Ensure host rejection paths use these codes consistently.
- [ ] Update `read_snapshot_response` so callers can inspect the error code, not just the message.
- [ ] Replace `.contains("participant is already connected")` with a code comparison.

### Suggested code shape

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolRejection {
    pub code: String,
    pub message: String,
    pub rejected_message_id: Option<String>,
}
```

Then have `read_snapshot_response` return either snapshot or typed rejection.

### Tests

- [ ] Test reconnect retries on `RECONNECT_ALREADY_CONNECTED`.
- [ ] Test reconnect does not retry on unrelated rejection codes.
- [ ] Test changing the human-readable message does not break retry behavior.

### Acceptance criteria

- [ ] Reconnect behavior no longer depends on matching human-readable strings.

---

# Phase 7 — Action/timeout transition outcome hardening

## Problem

Action submission and timeout handling currently have subtle semantics. A submitted stale action can trigger timeout advancement and return an error. Host-side publish logic may publish changed state even when `action_result` is an error. This may be intentional, but it is too implicit for an authoritative poker engine.

## Task 7.1 — Introduce explicit action outcome types

### Target files

- `crates/poker-core/src/tournament/mod.rs`
- `crates/poker-core/src/tournament/controller_core.rs`
- `src-tauri/src/networking/runtime/host.rs`
- tests in `crates/poker-core/src/tournament/tests.rs`

### Subtasks

- [ ] Replace or supplement `TournamentController::submit_action(...) -> Result<(), TournamentError>` with an explicit outcome.
- [ ] Suggested enum:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionSubmissionOutcome {
    Committed,
    RejectedNoStateChange { reason: String },
    TimeoutAdvancedThenRejected { reason: String },
}
```

- [ ] Ensure wrong-player, wrong-window, illegal-action, and invalid-raise rejections return `RejectedNoStateChange` and do not mutate state.
- [ ] Ensure expired-window behavior is represented as `TimeoutAdvancedThenRejected` if it advances the game clock/state.
- [ ] Ensure successful actions return `Committed`.
- [ ] Keep state validation after every state-mutating path.
- [ ] Update host publishing logic to publish only when the outcome allows a committed state change.

### Tests

- [ ] Wrong player does not mutate state.
- [ ] Stale action window id does not mutate state.
- [ ] Illegal action does not mutate state.
- [ ] Expired action window advances timeout state and returns `TimeoutAdvancedThenRejected`.
- [ ] Host publishes timeout-advanced state exactly once.
- [ ] Host does not publish after `RejectedNoStateChange`.

### Acceptance criteria

- [ ] Rejected action behavior is explicit and test-covered.
- [ ] State-changing rejected actions cannot happen accidentally.

## Task 7.2 — Add rollback protection to tick-time timeout advancement

### Subtasks

- [ ] Audit `advance_time` and `commit_timeout` for partial mutation on failure.
- [ ] Add rollback snapshots around timeout advancement where necessary.
- [ ] Ensure `apply_action` clearing `hand.action_window = None` cannot leave state corrupted if a later validation error occurs.
- [ ] Add tests that force validation failure after action-window clearing, if practical using test hooks or crafted state.

### Acceptance criteria

- [ ] Failed timeout advancement leaves the controller in its previous valid state.

---

# Phase 8 — Strengthen host lobby snapshot sync failure semantics

## Problem

`sync_snapshots_after_lobby_mutation` intentionally records snapshot-broadcast failures as health diagnostics instead of failing the mutation. That is reasonable, but callers need clear semantics and tests so this does not become an accidental silent failure pattern.

## Task 8.1 — Document and test post-mutation snapshot sync failure behavior

### Target files

- `src-tauri/src/networking/runtime/host.rs`
- `src-tauri/src/networking/runtime/tests.rs` or equivalent
- `src-tauri/src/app_state/debug.rs` if debug surfacing changes

### Subtasks

- [ ] Keep lobby mutation authoritative-state success independent from client broadcast success.
- [ ] Ensure every snapshot sync failure increments `snapshot_sync_error_count`.
- [ ] Ensure `last_error` contains a useful message.
- [ ] Ensure failed clients are marked reconnect-eligible or otherwise made recoverable.
- [ ] Add comments explaining why this is a deliberate non-fatal path.

### Tests

- [ ] Seat claim succeeds when authoritative mutation succeeds but one client write fails.
- [ ] Failed client is removed from connected-client registry.
- [ ] Failed participant becomes reconnect-eligible.
- [ ] Health counter increments.

### Acceptance criteria

- [ ] This best-effort path is observable and test-covered, not silently swallowed.

---

# Phase 9 — Structured Tauri command errors

## Problem

Most Tauri commands return `Result<..., String>`. Strings are easy to display but weak for recovery logic, tests, and frontend branching.

## Task 9.1 — Add a serializable command error type

### Target files

- `src-tauri/src/commands.rs`
- `src-tauri/src/app_state/*`
- `src/api/desktop.ts`

### Subtasks

- [ ] Define a serializable command error shape:

```rust
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCommandError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}
```

- [ ] Start with the highest-value commands:
  - [ ] `join_host_session`
  - [ ] `submit_table_action`
  - [ ] `client_claim_lobby_seat`
  - [ ] `client_set_lobby_ready_state`
  - [ ] `host_start_tournament`
- [ ] Use stable codes such as:
  - [ ] `NO_ACTIVE_SESSION`
  - [ ] `OBSERVER_READ_ONLY`
  - [ ] `NOT_ACTING_PLAYER`
  - [ ] `STALE_ACTION_WINDOW`
  - [ ] `NETWORK_TIMEOUT`
  - [ ] `CLIENT_RUNTIME_DISCONNECTED`
  - [ ] `HOST_REJECTED_ACTION`
  - [ ] `INVALID_JOIN_PAYLOAD`
- [ ] Update TypeScript API types/helpers to preserve `code`, `message`, and `recoverable`.
- [ ] Keep user-facing text readable.

### Tests

- [ ] Rust command helper tests assert codes and messages.
- [ ] Frontend tests assert recoverable errors render correctly.
- [ ] Existing string-based UI does not break during transition.

### Acceptance criteria

- [ ] Critical commands expose stable typed errors.
- [ ] Frontend does not need to parse error messages for control flow.

---

# Phase 10 — Rust/TypeScript DTO contract drift prevention

## Problem

Frontend DTOs in `src/api/desktop.ts` manually mirror Rust structs. This can drift. One visible mismatch is that README provider docs list four providers, while frontend types include `embeddedLocal`.

## Task 10.1 — Add contract tests for backend/frontend DTOs

### Target files

- `src/api/desktop.ts`
- `src-tauri/src/app_state/mod.rs`
- `src-tauri/src/commands.rs`
- `src/**/*.test.tsx` or `src/**/*.test.ts`
- optional new fixtures under `src/fixtures/` or `src-tauri/tests/fixtures/`

### Subtasks

- [ ] Add Rust-side JSON fixtures for representative DTOs:
  - [ ] `DesktopBootstrapState`
  - [ ] `HostSessionStatus`
  - [ ] `ClientSessionStatus`
  - [ ] `TableViewSnapshot`
  - [ ] `DebugInspectorState`
  - [ ] `NpcProfileListResult`
  - [ ] LLM provider settings/config
- [ ] Add TypeScript tests that load or embed those fixture shapes and validate expected field names.
- [ ] Decide whether to generate TypeScript types from Rust in a later phase.
- [ ] At minimum, add tests that fail if required frontend fields are missing or renamed.

### Acceptance criteria

- [ ] Rust DTO changes that break frontend assumptions are caught by tests.

## Task 10.2 — Decide whether to generate TypeScript bindings

### Subtasks

- [ ] Evaluate `specta`, `tauri-specta`, `ts-rs`, or a lightweight internal fixture generation approach.
- [ ] Pick one approach and document it in the README or a developer doc.
- [ ] If generation is adopted, do it in a separate focused patch.

### Acceptance criteria

- [ ] There is a clear policy: generated bindings or explicit contract fixtures.

---

# Phase 11 — Frontend localStorage hardening

## Problem

`DesktopShellProvider` reads localStorage with parse-error handling, which is good. But writes use direct `localStorage.setItem` calls that can throw due to quota, storage denial, corrupted environment, or WebView oddities.

## Task 11.1 — Add safe localStorage write wrapper

### Target files

- `src/app/DesktopShellProvider.tsx`
- `src/app/shell.ts`
- `src/app/persistence.ts`
- frontend tests

### Subtasks

- [ ] Add a helper like `writeStoredValueWithStatus` or `tryWriteStoredValue`.
- [ ] It should catch storage exceptions and return a structured status.
- [ ] Add a startup/runtime warning path for failed persistence writes.
- [ ] Replace direct `localStorage.setItem` calls in `DesktopShellProvider`.
- [ ] Ensure failed writes do not crash the app.

### Suggested TypeScript shape

```ts
export type StoredWriteStatus = {
  ok: boolean;
  error?: string;
};

export function writeStoredValueWithStatus(
  key: string,
  value: unknown,
): StoredWriteStatus {
  try {
    localStorage.setItem(key, JSON.stringify(value));
    return { ok: true };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}
```

### Tests

- [ ] Mock `localStorage.setItem` throwing.
- [ ] App does not crash.
- [ ] Warning is surfaced in startup/runtime warning state.
- [ ] Normal writes still work.

### Acceptance criteria

- [ ] Local persistence write failures are non-fatal and visible.

---

# Phase 12 — LLM provider/documentation consistency

## Problem

The README documents Anthropic, OpenAI, Ollama, and llama-server. The frontend type also includes `embeddedLocal`. The codebase should have one clear provider contract.

## Task 12.1 — Audit LLM provider enum and docs

### Target files

- `README.md`
- `src/api/desktop.ts`
- `src-tauri/src/npc/provider.rs`
- `src-tauri/src/npc/embedded_llm.rs`
- `src-tauri/src/npc/provider_storage.rs`
- settings UI files

### Subtasks

- [ ] Confirm all supported provider variants in Rust.
- [ ] Confirm all supported provider variants in TypeScript.
- [ ] Confirm settings UI exposes only supported variants.
- [ ] Update README provider table to include `embeddedLocal` if it is genuinely supported.
- [ ] If `embeddedLocal` is experimental or hidden, document that clearly.
- [ ] Add tests for provider config load/save for every supported provider.

### Acceptance criteria

- [ ] README, Rust provider storage, and TypeScript provider types agree.

---

# Phase 13 — Network/protocol abuse-case tests

## Task 13.1 — Add malformed peer tests

### Target files

- `src-tauri/src/networking/runtime/tests.rs`
- `src-tauri/src/networking/framing.rs`
- protocol tests

### Subtasks

Add tests or integration smoke scripts for:

- [ ] oversized frame length
- [ ] truncated join envelope
- [ ] malformed JSON envelope
- [ ] missing `messageType`
- [ ] invalid signature
- [ ] wrong table id
- [ ] wrong session epoch
- [ ] stale counter replay
- [ ] private-hole-card decrypt failure
- [ ] public event missing server sequence
- [ ] snapshot missing server sequence
- [ ] reconnect with stale token
- [ ] reconnect while old connection is still being cleaned up

### Acceptance criteria

- [ ] Bad peers cannot panic the host or client.
- [ ] Bad peer behavior is rejected with typed, observable diagnostics.

---

# Phase 14 — Runtime health surfacing in normal UI

## Problem

Host runtime health exists, but it is primarily debug-inspector data. Some failures are important enough to show in normal player UX or recovery UI.

## Task 14.1 — Decide which runtime health failures are user-visible

### Target files

- `src-tauri/src/app_state/debug.rs`
- `src-tauri/src/app_state/session.rs`
- `src/screens/ErrorStateScreen.tsx`
- `src/screens/MainTableScreen.tsx`
- `src/components/debug/DebugPanel.tsx`

### Subtasks

- [ ] Classify health counters:
  - [ ] debug-only diagnostics
  - [ ] recoverable user-facing warnings
  - [ ] fatal session errors
- [ ] Surface snapshot sync failures as recoverable reconnect/resync warnings.
- [ ] Surface repeated publish failures as a host runtime warning.
- [ ] Avoid exposing raw internal panic/poison text to normal users unless sanitized.
- [ ] Keep detailed raw diagnostics in debug panel.

### Acceptance criteria

- [ ] Runtime failures that affect gameplay are visible outside hidden debug tools.
- [ ] Debug tools still retain detailed diagnostics.

---

# Phase 15 — Manual multi-instance runtime validation

## Task 15.1 — Validate common desktop flows after hardening

### Subtasks

Run these manually or through existing runtime scripts:

- [ ] Host starts a table.
- [ ] Client joins using invite payload.
- [ ] Host and client claim seats.
- [ ] Host and client ready up.
- [ ] Tournament starts.
- [ ] Local player submits fold/check/call/raise/all-in as applicable.
- [ ] Observer mode cannot submit action.
- [ ] Client leaves; no stale runtime thread remains.
- [ ] Host stops; client gets clear termination/recovery state.
- [ ] Client reconnects after temporary disconnect.
- [ ] Malformed join is rejected cleanly.
- [ ] Oversized inbound frame is rejected cleanly.
- [ ] NPC-only or NPC-assisted tournament still runs.

### Evidence

- [ ] Save logs/screenshots/summaries under `docs/runtime-validation/`.
- [ ] Include the commit SHA and commands used.

### Acceptance criteria

- [ ] Manual/runtime evidence exists for the major flows touched by this TODO.

---

# Phase 16 — CI and release-readiness checks

## Task 16.1 — Ensure CI catches the fixed classes of bugs

### Target files

- `.github/workflows/ci.yml`
- package scripts
- Rust test modules

### Subtasks

- [ ] Ensure CI runs Rust unit tests for `poker-core` and Tauri crate.
- [ ] Ensure CI runs frontend unit tests.
- [ ] Ensure CI runs lint/format checks.
- [ ] If runtime smoke tests are too heavy for every PR, document how to run them locally.
- [ ] Add a lightweight test for inbound frame-size rejection to normal CI.
- [ ] Add typed command/error tests to normal CI.

### Acceptance criteria

- [ ] The most important hardening tests run automatically.

---

# Completion checklist

This TODO is complete only when all of the following are true:

- [ ] Oversized inbound frames cannot allocate unbounded memory.
- [ ] Client runtime has explicit shutdown and does not leave detached long-lived threads.
- [ ] Host join handling is bounded by connection/thread limits.
- [ ] Client operation timeouts are centralized, reasonable, and tested.
- [ ] Runtime polling distinguishes ordinary timeout from dead runtime channel.
- [ ] Reconnect logic no longer depends on string matching.
- [ ] Action rejection vs timeout-advanced state changes are explicit and tested.
- [ ] Intentional best-effort paths record structured diagnostics.
- [ ] Critical Tauri commands expose stable typed errors.
- [ ] Frontend/backend DTO drift is covered by contract tests or generated bindings.
- [ ] localStorage write failures are non-fatal and visible.
- [ ] LLM provider docs and provider enums agree.
- [ ] Abuse-case protocol tests exist for malformed peers.
- [ ] Runtime validation evidence is committed under `docs/runtime-validation/`.
- [ ] `cargo test --workspace --all-targets --all-features` passes from `src-tauri/Cargo.toml`.
- [ ] `npm run lint`, `npm run test`, and `npm run build` pass.
