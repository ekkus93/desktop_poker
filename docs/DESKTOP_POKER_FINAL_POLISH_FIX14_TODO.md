# Desktop Poker Fix 14 TODO — Final Validation Ledger and Cleanup Comments

This TODO is intentionally explicit for Claude Code. Do tasks in priority order. Prefer tiny, auditable patches. Do not introduce hidden fallback behavior to make tests pass.

Fix 14 is a final polish pass. Do not add features. Do not start Android implementation.

---

## P0.1 — Correct `memory.md` validation command list

**Files:**

- `memory.md`

### Problem

Fix 13 corrected the major `poker-core` grep overclaim, but review found that the Fix 13 validation list in `memory.md` omitted:

```bash
cargo test -p poker-core
```

The final validation command set explicitly includes that command. If it was run, record it. If it was not run, say it was not run. Do not let the ledger imply more validation than actually happened.

### Required change

Add a new append-only `memory.md` correction entry using the project timestamp convention:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

Suggested entry if `cargo test -p poker-core` **was run and passed**:

```md
- <timestamp> — Fix 14 ledger correction: the Fix 13 validation ledger omitted `cargo test -p poker-core` from the recorded command list. Validation in this environment now includes `cargo test -p poker-core` passing, along with the previously recorded workspace validation. The `poker-core` forbidden-dependency grep returns only expected `EngineCommand` false positives; no Tauri, networking, keychain, socket, thread-spawn, process-spawn, or app-data-path dependencies were found in `poker-core`.
```

Suggested entry if `cargo test -p poker-core` **was not run**:

```md
- <timestamp> — Fix 14 ledger correction: the Fix 13 validation ledger omitted `cargo test -p poker-core` from the recorded command list. That command was not run in the Fix 13 environment and should not be implied as passed. Current validation run in this environment: <exact commands actually run and pass/fail status>. The `poker-core` forbidden-dependency grep returns only expected `EngineCommand` false positives; no Tauri, networking, keychain, socket, thread-spawn, process-spawn, or app-data-path dependencies were found in `poker-core`.
```

If you run the full validation set as part of Fix 14, prefer one combined completion entry:

```md
- <timestamp> — Fix 14 completed: corrected the validation ledger so `cargo test -p poker-core` is recorded honestly; cleanup-only command-stream lock paths in `ClientRuntime` now explain why ignored cleanup lock failures are best effort; reconnect snapshot acceptance still requires command-stream clone and command-connection lock success before emitting the accepted snapshot. Validation run in this environment: `npm ci` pass; `npm run format:check` pass; `npm run lint` pass; `npm run build` pass; `npm test` pass; `cargo fmt --check` pass; `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass; `cargo test --workspace --all-targets --all-features` pass; `cargo test -p poker-core` pass; `cargo tree -p poker-core` pass; focused audits pass except expected `EngineCommand` false positives and expected missing `src-tauri/src/tournament` path after core extraction. No Android app, Tauri Mobile path, FFI crate, or networking-in-core was added.
```

Adjust pass/fail statuses to exactly match what actually happened. Do not claim Rust commands passed unless Rust validation actually ran and passed.

### Acceptance

- `memory.md` lists exact commands actually run.
- `memory.md` includes `cargo test -p poker-core` if it was run.
- If `cargo test -p poker-core` was not run, `memory.md` explicitly says it was not run.
- `memory.md` documents expected `EngineCommand` false positives honestly.
- No false completion claims.

---

## P0.2 — Add comments to best-effort command stream cleanup locks

**Files:**

- `src-tauri/src/networking/runtime/client.rs`

### Problem

Fix 13 correctly made reconnect command-stream installation fail visibly. However, review found cleanup-only blocks such as:

```rust
if let Ok(mut connection) = command_connection_for_thread.lock() {
    connection.stream = None;
}
```

These are not the prohibited reconnect install pattern. They are cleanup paths, usually after disconnect, reconnect failure, or runtime shutdown. But without comments, future reviewers and agents may confuse them with silent failure bugs.

### Required change

Find cleanup-only command stream clearing blocks:

```bash
rg -n "connection\.stream = None|command_connection_for_thread\.lock\(\)|if let Ok\(mut connection\)" \
  src-tauri/src/networking/runtime/client.rs
```

For every block that intentionally ignores a cleanup lock failure, add a short comment explaining why the ignored lock failure is safe.

Preferred comment:

```rust
// Best-effort cleanup: if this lock is poisoned, command submission is already
// unusable and the runtime is disconnecting or surfacing an error below.
if let Ok(mut connection) = command_connection_for_thread.lock() {
    connection.stream = None;
}
```

If the cleanup block is in a more specific context, make the comment more specific:

```rust
// Best-effort reconnect cleanup: failure to acquire this lock only prevents
// clearing an already-invalid command stream; the reconnect failure is reported below.
if let Ok(mut connection) = command_connection_for_thread.lock() {
    connection.stream = None;
}
```

### Important

Do not add `unwrap()` or `expect()` around cleanup locks.

Do not convert cleanup-only lock failures into noisy user-facing errors unless there is a real runtime state that can still recover. For disconnect/shutdown cleanup, a comment is usually enough.

Do not change the successful reconnect command-stream installation path except to preserve explicit clone/lock error handling from Fix 13.

### Acceptance

- Cleanup-only `connection.stream = None` blocks have explanatory comments.
- The comments distinguish best-effort cleanup from required reconnect command-stream installation.
- No panic path is added.
- No unrelated refactor is introduced.

---

## P0.3 — Verify reconnect command-stream installation still fails visibly

**Files:**

- `src-tauri/src/networking/runtime/client.rs`

### Required audit

Run:

```bash
rg -n "try_clone\(|command_connection.*lock\(|connection\.stream = Some|if let Ok\(cloned_stream\)|if let Ok\(mut connection\)" \
  src-tauri/src/networking/runtime/client.rs
```

Inspect every hit.

### Required behavior

The reconnect snapshot acceptance path must still have explicit failure handling like this:

```rust
// Snapshot validation has succeeded. Command stream installation is required
// before exposing the reconnect snapshot to the UI; otherwise the client can
// appear reconnected while action submission remains disconnected.
let cloned_stream = match stream.try_clone() {
    Ok(cloned_stream) => cloned_stream,
    Err(error) => {
        let _ = sender.send(ClientRuntimeEvent::SafeError {
            player_id: player_id.clone(),
            message: format!("failed to clone reconnect command stream: {error}"),
        });
        break;
    }
};

match command_connection_for_thread.lock() {
    Ok(mut connection) => {
        connection.stream = Some(Arc::new(Mutex::new(cloned_stream)));
    }
    Err(_) => {
        let _ = sender.send(ClientRuntimeEvent::SafeError {
            player_id: player_id.clone(),
            message: "client command connection lock poisoned after reconnect".to_string(),
        });
        break;
    }
}
```

Adjust variable names to match the current code. The important property is that the snapshot is not emitted after clone/lock failure.

### Forbidden shape

This shape must not exist for reconnect snapshot acceptance:

```rust
if let Ok(cloned_stream) = stream.try_clone() {
    if let Ok(mut connection) = command_connection_for_thread.lock() {
        connection.stream = Some(Arc::new(Mutex::new(cloned_stream)));
    }
}

// Wrong if this can run after clone or lock failure.
let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(snapshot_envelope.payload)));
```

### Acceptance

- Reconnect snapshot acceptance requires successful command-stream clone.
- Reconnect snapshot acceptance requires successful command-connection lock.
- Clone failure emits `SafeError` and stops the runtime path.
- Lock failure emits `SafeError` and stops the runtime path.
- No silent reconnect stream-install pattern remains.

---

## P0.4 — Re-run previous protocol and runtime hardening audits

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- `src-tauri/src/npc/runner/action.rs`
- `src-tauri/src/npc/provider_storage.rs`
- `src/main.tsx`
- `src/test/setup.ts`
- `crates/poker-core/src/**`

### Required audits

Run:

```bash
npm test 2>&1 | tee /tmp/desktop-poker-npm-test.log
rg -n "Failed to initialize window state persistence|currentWindow" /tmp/desktop-poker-npm-test.log

npm run build
rg -n "__DESKTOP_POKER_BROWSER_MOCKS__|LayoutProbeApp" dist || true

rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated" src-tauri/src/networking
rg -n "hole_cards_by_player_id.*unwrap_or\(|unwrap_or\(&\[\]\)" src-tauri/src/npc src-tauri/src/tournament crates/poker-core/src || true
rg -n "thread::spawn" src-tauri/src/networking/runtime/client.rs
rg -n "server_sequence.*unwrap_or_default|unwrap_or_default\(\).*server_sequence" src-tauri/src/networking/runtime/client.rs
rg -n "last_seen_server_sequence = envelope\.server_sequence" src-tauri/src/networking/runtime/client.rs
rg -n "if let Ok\(cloned_stream\).*try_clone|if let Ok\(mut connection\).*command_connection" src-tauri/src/networking/runtime/client.rs
```

### Acceptance

- No window-persistence test noise.
- Production dist does not contain browser mock/probe setup code.
- No private-message empty-AAD fallback.
- No acting NPC empty-hole-card fallback.
- No raw client runtime `thread::spawn`.
- No public-event/snapshot missing-sequence default to `0`.
- No assignment from `envelope.server_sequence` directly into `last_seen_server_sequence`.
- No silent reconnect command-stream install failure pattern remains.
- Cleanup-only `if let Ok(mut connection)` hits are commented as best effort.

---

## P1.1 — Keep `poker-core` platform-neutral

**Files:**

- `crates/poker-core/Cargo.toml`
- `crates/poker-core/src/**`
- `memory.md`

### Required validation

Run:

```bash
cargo tree -p poker-core
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|Mutex<.*Tcp|thread::spawn|Command" crates/poker-core
```

Expected:

- no Tauri dependency;
- no socket/networking dependency;
- no LLM/provider/keychain dependency;
- no desktop/Android UI dependency;
- no thread spawning;
- no process spawning;
- no platform app-data-path logic.

`EngineCommand` is an expected false positive for the broad `Command` grep. Do not remove or rename `EngineCommand` just to satisfy the grep.

### Acceptance

- `cargo tree -p poker-core` remains small and platform-neutral.
- Forbidden dependency grep has no code hits except expected harmless terms such as `EngineCommand`.
- Any textual/comment hits are reviewed and documented honestly.

---

## P2.1 — Silent-failure audit for touched runtime code

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- tests touched by Fix 14, if any

### Required audit

For touched runtime code, run:

```bash
rg -n "let _ =|\.ok\(\)|unwrap_or\(|unwrap_or_else\(|unwrap_or_default\(|thread::spawn|continue;|return;|if let Ok" \
  src-tauri/src/networking/runtime/client.rs
```

For every hit in touched production code:

- convert real silent failures to structured errors/diagnostics;
- add a short comment for intentional best-effort cleanup;
- leave harmless presentation-only/test-only defaults alone;
- do not rewrite unrelated stable code just to reduce grep output.

### Specific guidance

`let _ = sender.send(...)` is often acceptable when the receiver may be gone during shutdown or fatal error handling. If the touched area already has an explanatory comment, leave it alone. If the touched area ignores a lock failure or cleanup failure, add a short comment.

### Acceptance

- No newly touched runtime code contains unexplained silent failure behavior.
- Any ignored cleanup error has a short comment explaining why it is safe.
- No new fallback invents authoritative protocol ordering state.
- No reconnect path leaves command submission disconnected while exposing an accepted snapshot.

---

## P2.2 — Do not start Android implementation in Fix 14

**Files:**

- docs only, if needed

### Required rule

Do not add:

- Android Gradle project files;
- Kotlin source files;
- Tauri Mobile configuration;
- `poker-android-ffi` crate;
- networking inside `poker-core`.

Fix 14 is final validation/ledger/comment cleanup only.

### Acceptance

- No half-built Android implementation appears.
- Existing Android architecture doc remains accurate.

---

## Final validation commands

Run from repo root with Node 24 active and Rust installed:

```bash
npm ci
npm run format:check
npm run lint
npm run build
npm test
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test -p poker-core
cargo tree -p poker-core
```

Focused audits:

```bash
npm test 2>&1 | tee /tmp/desktop-poker-npm-test.log
rg -n "Failed to initialize window state persistence|currentWindow" /tmp/desktop-poker-npm-test.log

npm run build
rg -n "__DESKTOP_POKER_BROWSER_MOCKS__|LayoutProbeApp" dist || true

rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated" src-tauri/src/networking
rg -n "hole_cards_by_player_id.*unwrap_or\(|unwrap_or\(&\[\]\)" src-tauri/src/npc src-tauri/src/tournament crates/poker-core/src || true
rg -n "thread::spawn" src-tauri/src/networking/runtime/client.rs
rg -n "server_sequence.*unwrap_or_default|unwrap_or_default\(\).*server_sequence" src-tauri/src/networking/runtime/client.rs
rg -n "last_seen_server_sequence = envelope\.server_sequence" src-tauri/src/networking/runtime/client.rs
rg -n "if let Ok\(cloned_stream\).*try_clone|if let Ok\(mut connection\).*command_connection" src-tauri/src/networking/runtime/client.rs
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn|Command" crates/poker-core
```

Expected notes:

- `EngineCommand` in `poker-core` is an expected false positive for the broad `Command` grep.
- `src-tauri/src/tournament` may not exist after the `poker-core` extraction; this is okay.
- Production dist grep should have no JS payload hits for `LayoutProbeApp` or `__DESKTOP_POKER_BROWSER_MOCKS__`.
- The exact direct-assignment grep must return no hits.
- Broad `if let Ok(mut connection)` greps may find cleanup-only blocks; those should have comments explaining best-effort cleanup.

## Definition of done

- [ ] `memory.md` records exact validation commands actually run.
- [ ] `memory.md` includes `cargo test -p poker-core` if it was run, or explicitly says it was not run.
- [ ] `memory.md` documents expected `EngineCommand` false positives honestly.
- [ ] Cleanup-only command stream lock ignore paths have explanatory best-effort comments.
- [ ] Reconnect snapshot acceptance still requires successful command-stream clone.
- [ ] Reconnect snapshot acceptance still requires successful command-connection lock.
- [ ] Reconnect snapshot is not emitted if command-stream installation fails.
- [ ] Clone/lock failure still emits `SafeError` and stops the runtime path.
- [ ] Previous protocol hardening remains fixed: missing/stale public or snapshot sequence values warn/drop without defaulting to `0` or prior sequence.
- [ ] No direct assignment remains from `envelope.server_sequence` to `last_seen_server_sequence`.
- [ ] Previous hardening remains fixed: no test noise, no production mock chunk, no empty-AAD fallback, no empty NPC hole-card fallback, no raw client runtime `thread::spawn`.
- [ ] `poker-core` purity audit remains clean except expected harmless `EngineCommand` hits.
- [ ] No new hidden fallbacks or silent failures are introduced.
- [ ] No Android app, Tauri Mobile path, Android FFI crate, or networking-in-core is added.
- [ ] All final validation commands pass in an environment with Node 24 and Rust installed, or any command that was not run is clearly recorded as not run.
