# Desktop Poker Runtime Hardening Code Review TODO

Created: 2026-07-28
Repo: `ekkus93/desktop_poker`
Scope: follow-up work from a source-inspection code review of the Desktop Poker MVP.
Status: **COMPLETE — reconciled against committed source, tests, CI, and runtime evidence on 2026-07-28.**

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

- [x] Run frontend validation:
  - [x] `npm ci`
  - [x] `npm run format:check`
  - [x] `npm run lint`
  - [x] `npm run test`
  - [x] `npm run build`
- [x] Run Rust validation:
  - [x] `cargo fmt --check --manifest-path src-tauri/Cargo.toml`
  - [x] `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --all-features -- -D warnings`
  - [x] `cargo test --manifest-path src-tauri/Cargo.toml --workspace --all-targets --all-features`
  - [x] `cargo test --manifest-path crates/poker-core/Cargo.toml --all-targets`
- [x] If any command fails before changes, save the failure as baseline evidence instead of silently ignoring it.
- [x] Create or update a short note under `docs/runtime-validation/` with:
  - [x] command run
  - [x] pass/fail result
  - [x] relevant failure excerpt if any
  - [x] current commit SHA

### Acceptance criteria

- [x] There is a committed validation note showing the pre-fix baseline.
- [x] Existing failures are explicitly documented and not confused with regressions from this TODO.

---

# Phase 1 — P0 inbound frame-size hardening

## Problem

`src-tauri/src/networking/framing.rs` reads a 4-byte inbound length, casts it to `usize`, allocates `vec![0_u8; length]`, then reads the frame body. A malicious or broken LAN peer can advertise a huge frame length and force excessive memory allocation before sending a body.

## Task 1.1 — Add a maximum inbound frame size

### Target files

- `src-tauri/src/networking/framing.rs`
- `src-tauri/src/networking/mod.rs` if constants should be exported

### Subtasks

- [x] Add a constant for maximum inbound frame payload size.
  - Suggested name: `MAX_FRAME_PAYLOAD_BYTES`
  - Suggested initial value: `1_048_576` bytes, unless current snapshot sizes require more.
- [x] After reading the 4-byte frame length, reject frames larger than the limit before allocating the body buffer.
- [x] Return a clear `NetworkingError`, for example:
  - `frame payload exceeds maximum allowed size: {length} > {MAX_FRAME_PAYLOAD_BYTES}`
- [x] Make the limit apply to all inbound JSON frames: client, host, join, reconnect, resync, snapshot, public events, and private events.
- [x] Do not silently truncate oversized frames.
- [x] Do not read and discard oversized bodies; reject immediately after the length prefix.

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

- [x] `read_json_frame_rejects_payload_larger_than_max_before_allocation`
- [x] `read_json_frame_accepts_payload_at_max_size_when_json_valid` if practical
- [x] `read_json_frame_accepts_small_payload_after_limit_added`
- [x] Existing truncation and invalid JSON tests still pass.

### Acceptance criteria

- [x] Oversized inbound frame lengths fail before payload allocation.
- [x] The error is explicit and test-covered.
- [x] All existing frame tests still pass.

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

- [x] Add a `stop_signal: Arc<AtomicBool>` to `ClientRuntime`.
- [x] Add `runtime_thread: Option<JoinHandle<()>>` to `ClientRuntime`.
- [x] Pass `stop_signal` into the spawned client runtime thread.
- [x] Check `stop_signal` at the top of the client read/reconnect loop.
- [x] On shutdown, set `stop_signal = true`.
- [x] Wake a blocking read by shutting down or dropping the active stream.
- [x] Join the runtime thread in `Drop for ClientRuntime`.
- [x] Avoid panicking if the worker thread already panicked.
- [x] Ensure `leave_client_session` triggers this drop path.

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

- [x] Add a test that creates a client runtime, drops it, and verifies the runtime does not hang.
- [x] Add a test or runtime smoke script for joining then leaving a client session.
- [x] Add a reconnect/drop test if existing test harnesses support it.

### Acceptance criteria

- [x] Client runtime has explicit shutdown semantics comparable to `HostServer`.
- [x] Dropping a client session does not leave a long-lived runtime thread behind.
- [x] Tests prove drop does not hang.

## Task 2.2 — Distinguish normal shutdown from disconnected error

### Subtasks

- [x] When `stop_signal` is set, the runtime thread should exit without emitting a scary user-facing error.
- [x] Do not report `Disconnected from host` for intentional `leave_client_session`.
- [x] If a shutdown event is needed internally, add a distinct event variant rather than overloading `Disconnected`.

### Acceptance criteria

- [x] User-triggered leave is not shown as an unexpected host disconnect.
- [x] Unexpected network failure still surfaces as a terminated/disconnected session.

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

- [x] Add host runtime constants or config fields:
  - [x] `max_connected_clients`
  - [x] `max_pending_initial_joins`
  - [x] optional `initial_join_timeout_ms`
- [x] Default `max_connected_clients` should align with tournament max players plus a small buffer, not be unbounded.
- [x] Increment a pending-join counter before spawning `host-initial-join`.
- [x] Decrement pending-join counter on every thread exit path.
- [x] Reject or drop new joins when pending joins exceed limit.
- [x] Record limit rejections in `HostRuntimeHealth`.
- [x] Do not let rejected joins mutate authoritative tournament state.

### Suggested health additions

In `HostRuntimeHealth`, add fields similar to:

```rust
pub pending_join_limit_rejection_count: u64,
pub connected_client_limit_rejection_count: u64,
```

### Tests

- [x] Test pending join counter decrements on success.
- [x] Test pending join counter decrements on malformed join failure.
- [x] Test connections beyond limit are rejected and health counter increments.
- [x] Test rejection does not mutate tournament participants or seats.

### Acceptance criteria

- [x] The host cannot spawn unlimited join-handler threads.
- [x] Limit-related failures are visible in host runtime health.

## Task 3.2 — Ensure connected-client limit is enforced atomically

### Subtasks

- [x] Check connected-client count while holding the client registry lock.
- [x] Enforce the limit before inserting into `clients`.
- [x] Avoid races where two join handlers both see capacity and both insert.
- [x] Return a clear protocol error for a legitimate client rejected because the table is full or connection limit is reached.

### Acceptance criteria

- [x] Client registry cannot exceed the configured maximum through concurrent joins.
- [x] Legitimate users see a clear join rejection.

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

- [x] Add named constants for client operation timeouts:
  - [x] `INITIAL_JOIN_SNAPSHOT_TIMEOUT`
  - [x] `CLIENT_ACTION_ACK_TIMEOUT`
  - [x] `CLIENT_LOBBY_ACK_TIMEOUT`
  - [x] optional `READY_CHECK_TABLE_START_POLL_TIMEOUT`
- [x] Replace inline `Duration::from_secs(1)` with named constants.
- [x] Use a longer default for lobby/action acknowledgement, probably 5 seconds.
- [x] Keep tests deterministic by allowing test-only override or dependency injection where needed.
- [x] Include the timeout duration in error messages.

### Suggested code shape

```rust
const INITIAL_JOIN_SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(5);
const CLIENT_ACTION_ACK_TIMEOUT: Duration = Duration::from_secs(5);
const CLIENT_LOBBY_ACK_TIMEOUT: Duration = Duration::from_secs(5);
const READY_CHECK_TABLE_START_POLL_TIMEOUT: Duration = Duration::from_millis(250);
```

### Tests

- [x] Existing timeout tests updated for named constants.
- [x] Test error messages include the configured duration.
- [x] Test successful operation path is unchanged.

### Acceptance criteria

- [x] No hardcoded 1-second command deadlines remain in live client session code.
- [x] Timeout errors remain explicit.

## Task 4.2 — Improve UI behavior around pending operations

### Target files

- `src/screens/TournamentLobbyScreen.tsx`
- `src/screens/MainTableScreen.tsx`
- other screen files that call lobby/action commands

### Subtasks

- [x] Ensure buttons show a pending/disabled state while a command is in flight.
- [x] Avoid duplicate command submission while waiting for backend confirmation.
- [x] Show timeout errors as recoverable user-facing errors.
- [x] Do not automatically leave the session solely because a command timed out.

### Acceptance criteria

- [x] Slow command acknowledgement does not cause duplicate actions or confusing UI state.

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

- [x] Add a typed error enum for `next_event`, for example:

```rust
pub enum ClientRuntimePollError {
    Timeout,
    Disconnected,
}
```

- [x] Change `ClientRuntime::next_event` to distinguish:
  - [x] `RecvTimeoutError::Timeout`
  - [x] `RecvTimeoutError::Disconnected`
- [x] Update `DesktopClientSession::refresh`:
  - [x] `Timeout` means stop draining events normally.
  - [x] `Disconnected` means mark the session terminated or set a clear internal error.
- [x] Update `await_condition` to stop early on runtime disconnect.
- [x] Preserve public Tauri command error strings at the boundary if needed, but keep typed logic internally.

### Tests

- [x] Test normal timeout does not mark session terminated.
- [x] Test disconnected event channel marks session unhealthy/terminated.
- [x] Test `await_condition` exits on channel disconnect instead of spinning until timeout.

### Acceptance criteria

- [x] Dead runtime thread/channel cannot be mistaken for “no events available.”
- [x] UI can surface a clear fatal client-runtime error.

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

- [x] Define constants or an enum for protocol error codes.
- [x] Include at least:
  - [x] `JOIN_REJECTED`
  - [x] `RECONNECT_ALREADY_CONNECTED`
  - [x] `RECONNECT_REJECTED`
  - [x] `STALE_COUNTER`
  - [x] `INVALID_SIGNATURE`
  - [x] `TABLE_OR_SESSION_MISMATCH`
- [x] Ensure host rejection paths use these codes consistently.
- [x] Update `read_snapshot_response` so callers can inspect the error code, not just the message.
- [x] Replace `.contains("participant is already connected")` with a code comparison.

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

- [x] Test reconnect retries on `RECONNECT_ALREADY_CONNECTED`.
- [x] Test reconnect does not retry on unrelated rejection codes.
- [x] Test changing the human-readable message does not break retry behavior.

### Acceptance criteria

- [x] Reconnect behavior no longer depends on matching human-readable strings.

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

- [x] Replace or supplement `TournamentController::submit_action(...) -> Result<(), TournamentError>` with an explicit outcome.
- [x] Suggested enum:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionSubmissionOutcome {
    Committed,
    RejectedNoStateChange { reason: String },
    TimeoutAdvancedThenRejected { reason: String },
}
```

- [x] Ensure wrong-player, wrong-window, illegal-action, and invalid-raise rejections return `RejectedNoStateChange` and do not mutate state.
- [x] Ensure expired-window behavior is represented as `TimeoutAdvancedThenRejected` if it advances the game clock/state.
- [x] Ensure successful actions return `Committed`.
- [x] Keep state validation after every state-mutating path.
- [x] Update host publishing logic to publish only when the outcome allows a committed state change.

### Tests

- [x] Wrong player does not mutate state.
- [x] Stale action window id does not mutate state.
- [x] Illegal action does not mutate state.
- [x] Expired action window advances timeout state and returns `TimeoutAdvancedThenRejected`.
- [x] Host publishes timeout-advanced state exactly once.
- [x] Host does not publish after `RejectedNoStateChange`.

### Acceptance criteria

- [x] Rejected action behavior is explicit and test-covered.
- [x] State-changing rejected actions cannot happen accidentally.

## Task 7.2 — Add rollback protection to tick-time timeout advancement

### Subtasks

- [x] Audit `advance_time` and `commit_timeout` for partial mutation on failure.
- [x] Add rollback snapshots around timeout advancement where necessary.
- [x] Ensure `apply_action` clearing `hand.action_window = None` cannot leave state corrupted if a later validation error occurs.
- [x] Add tests that force validation failure after action-window clearing, if practical using test hooks or crafted state.

### Acceptance criteria

- [x] Failed timeout advancement leaves the controller in its previous valid state.

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

- [x] Keep lobby mutation authoritative-state success independent from client broadcast success.
- [x] Ensure every snapshot sync failure increments `snapshot_sync_error_count`.
- [x] Ensure `last_error` contains a useful message.
- [x] Ensure failed clients are marked reconnect-eligible or otherwise made recoverable.
- [x] Add comments explaining why this is a deliberate non-fatal path.

### Tests

- [x] Seat claim succeeds when authoritative mutation succeeds but one client write fails.
- [x] Failed client is removed from connected-client registry.
- [x] Failed participant becomes reconnect-eligible.
- [x] Health counter increments.

### Acceptance criteria

- [x] This best-effort path is observable and test-covered, not silently swallowed.

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

- [x] Define a serializable command error shape:

```rust
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCommandError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}
```

- [x] Start with the highest-value commands:
  - [x] `join_host_session`
  - [x] `submit_table_action`
  - [x] `client_claim_lobby_seat`
  - [x] `client_set_lobby_ready_state`
  - [x] `host_start_tournament`
- [x] Use stable codes such as:
  - [x] `NO_ACTIVE_SESSION`
  - [x] `OBSERVER_READ_ONLY`
  - [x] `NOT_ACTING_PLAYER`
  - [x] `STALE_ACTION_WINDOW`
  - [x] `NETWORK_TIMEOUT`
  - [x] `CLIENT_RUNTIME_DISCONNECTED`
  - [x] `HOST_REJECTED_ACTION`
  - [x] `INVALID_JOIN_PAYLOAD`
- [x] Update TypeScript API types/helpers to preserve `code`, `message`, and `recoverable`.
- [x] Keep user-facing text readable.

### Tests

- [x] Rust command helper tests assert codes and messages.
- [x] Frontend tests assert recoverable errors render correctly.
- [x] Existing string-based UI does not break during transition.

### Acceptance criteria

- [x] Critical commands expose stable typed errors.
- [x] Frontend does not need to parse error messages for control flow.

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

- [x] Add Rust-side JSON fixtures for representative DTOs:
  - [x] `DesktopBootstrapState`
  - [x] `HostSessionStatus`
  - [x] `ClientSessionStatus`
  - [x] `TableViewSnapshot`
  - [x] `DebugInspectorState`
  - [x] `NpcProfileListResult`
  - [x] LLM provider settings/config
- [x] Add TypeScript tests that load or embed those fixture shapes and validate expected field names.
- [x] Decide whether to generate TypeScript types from Rust in a later phase.
- [x] At minimum, add tests that fail if required frontend fields are missing or renamed.

### Acceptance criteria

- [x] Rust DTO changes that break frontend assumptions are caught by tests.

## Task 10.2 — Decide whether to generate TypeScript bindings

### Subtasks

- [x] Evaluate `specta`, `tauri-specta`, `ts-rs`, or a lightweight internal fixture generation approach.
- [x] Pick one approach and document it in the README or a developer doc.
- [x] If generation is adopted, do it in a separate focused patch.

### Acceptance criteria

- [x] There is a clear policy: generated bindings or explicit contract fixtures.

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

- [x] Add a helper like `writeStoredValueWithStatus` or `tryWriteStoredValue`.
- [x] It should catch storage exceptions and return a structured status.
- [x] Add a startup/runtime warning path for failed persistence writes.
- [x] Replace direct `localStorage.setItem` calls in `DesktopShellProvider`.
- [x] Ensure failed writes do not crash the app.

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

- [x] Mock `localStorage.setItem` throwing.
- [x] App does not crash.
- [x] Warning is surfaced in startup/runtime warning state.
- [x] Normal writes still work.

### Acceptance criteria

- [x] Local persistence write failures are non-fatal and visible.

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

- [x] Confirm all supported provider variants in Rust.
- [x] Confirm all supported provider variants in TypeScript.
- [x] Confirm settings UI exposes only supported variants.
- [x] Update README provider table to include `embeddedLocal` if it is genuinely supported.
- [x] If `embeddedLocal` is experimental or hidden, document that clearly.
- [x] Add tests for provider config load/save for every supported provider.

### Acceptance criteria

- [x] README, Rust provider storage, and TypeScript provider types agree.

---

# Phase 13 — Network/protocol abuse-case tests

## Task 13.1 — Add malformed peer tests

### Target files

- `src-tauri/src/networking/runtime/tests.rs`
- `src-tauri/src/networking/framing.rs`
- protocol tests

### Subtasks

Add tests or integration smoke scripts for:

- [x] oversized frame length
- [x] truncated join envelope
- [x] malformed JSON envelope
- [x] missing `messageType`
- [x] invalid signature
- [x] wrong table id
- [x] wrong session epoch
- [x] stale counter replay
- [x] private-hole-card decrypt failure
- [x] public event missing server sequence
- [x] snapshot missing server sequence
- [x] reconnect with stale token
- [x] reconnect while old connection is still being cleaned up

### Acceptance criteria

- [x] Bad peers cannot panic the host or client.
- [x] Bad peer behavior is rejected with typed, observable diagnostics.

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

- [x] Classify health counters:
  - [x] debug-only diagnostics
  - [x] recoverable user-facing warnings
  - [x] fatal session errors
- [x] Surface snapshot sync failures as recoverable reconnect/resync warnings.
- [x] Surface repeated publish failures as a host runtime warning.
- [x] Avoid exposing raw internal panic/poison text to normal users unless sanitized.
- [x] Keep detailed raw diagnostics in debug panel.

### Acceptance criteria

- [x] Runtime failures that affect gameplay are visible outside hidden debug tools.
- [x] Debug tools still retain detailed diagnostics.

---

# Phase 15 — Manual multi-instance runtime validation

## Task 15.1 — Validate common desktop flows after hardening

### Subtasks

Run these manually or through existing runtime scripts:

- [x] Host starts a table.
- [x] Client joins using invite payload.
- [x] Host and client claim seats.
- [x] Host and client ready up.
- [x] Tournament starts.
- [x] Local player submits fold/check/call/raise/all-in as applicable.
- [x] Observer mode cannot submit action.
- [x] Client leaves; no stale runtime thread remains.
- [x] Host stops; client gets clear termination/recovery state.
- [x] Client reconnects after temporary disconnect.
- [x] Malformed join is rejected cleanly.
- [x] Oversized inbound frame is rejected cleanly.
- [x] NPC-only or NPC-assisted tournament still runs.

### Evidence

- [x] Save logs/screenshots/summaries under `docs/runtime-validation/`.
- [x] Include the commit SHA and commands used.

### Acceptance criteria

- [x] Manual/runtime evidence exists for the major flows touched by this TODO.

---

# Phase 16 — CI and release-readiness checks

## Task 16.1 — Ensure CI catches the fixed classes of bugs

### Target files

- `.github/workflows/ci.yml`
- package scripts
- Rust test modules

### Subtasks

- [x] Ensure CI runs Rust unit tests for `poker-core` and Tauri crate.
- [x] Ensure CI runs frontend unit tests.
- [x] Ensure CI runs lint/format checks.
- [x] If runtime smoke tests are too heavy for every PR, document how to run them locally.
- [x] Add a lightweight test for inbound frame-size rejection to normal CI.
- [x] Add typed command/error tests to normal CI.

### Acceptance criteria

- [x] The most important hardening tests run automatically.

---

# Completion checklist

This TODO is complete only when all of the following are true:

- [x] Oversized inbound frames cannot allocate unbounded memory.
- [x] Client runtime has explicit shutdown and does not leave detached long-lived threads.
- [x] Host join handling is bounded by connection/thread limits.
- [x] Client operation timeouts are centralized, reasonable, and tested.
- [x] Runtime polling distinguishes ordinary timeout from dead runtime channel.
- [x] Reconnect logic no longer depends on string matching.
- [x] Action rejection vs timeout-advanced state changes are explicit and tested.
- [x] Intentional best-effort paths record structured diagnostics.
- [x] Critical Tauri commands expose stable typed errors.
- [x] Frontend/backend DTO drift is covered by contract tests or generated bindings.
- [x] localStorage write failures are non-fatal and visible.
- [x] LLM provider docs and provider enums agree.
- [x] Abuse-case protocol tests exist for malformed peers.
- [x] Runtime validation evidence is committed under `docs/runtime-validation/`.
- [x] `cargo test --workspace --all-targets --all-features` passes from `src-tauri/Cargo.toml`.
- [x] `npm run lint`, `npm run test`, and `npm run build` pass.

---

# Completion reconciliation — 2026-07-28

All checklist items above were reconciled against committed implementation, focused tests, general CI run `30403803338`, release-runtime run `30403803495`, and retained gameplay, reconnect, and NPC evidence under `docs/runtime-validation/`.

The detailed evidence map is committed at `docs/runtime-validation/runtime-hardening-reconciliation-2026-07-28.md`.

The separately tracked two-physical-machine LAN release gate remains explicitly deferred in `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`; it is not part of this completed code-review hardening checklist.

