# Desktop Poker Fix 13 TODO — Reconnect Command Stream Failure Cleanup

This TODO is intentionally explicit for Claude Code. Do tasks in priority order. Prefer small, test-backed patches. Do not introduce hidden fallback behavior to make tests pass.

Fix 13 is a narrow cleanup pass. Do not add features. Do not start Android implementation.

---

## P0.1 — Make reconnect command-stream installation fail visibly

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- tests only if practical

### Problem

Fix 12 delayed reconnect command-stream installation until after snapshot validation. That was the correct direction, but review found the installation itself can still fail silently.

The current code may contain a shape equivalent to:

```rust
// Snapshot accepted — install the command stream now.
if let Ok(cloned_stream) = stream.try_clone() {
    if let Ok(mut connection) = command_connection_for_thread.lock() {
        connection.stream = Some(Arc::new(Mutex::new(cloned_stream)));
    }
}

reconnect_token = snapshot_envelope.payload.reconnect_token.clone();
last_seen_server_sequence = Some(snapshot_sequence);
host_encryption_public_key = next_host_encryption_public_key;

let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(
    snapshot_envelope.payload,
)));
```

That is a quiet runtime failure. If `try_clone()` fails, or the command-connection lock is poisoned, the runtime still accepts the reconnect snapshot and continues. The UI can appear reconnected while command submission remains disconnected.

### Required behavior

After reconnect snapshot validation succeeds, command-stream installation is required before the accepted snapshot is exposed.

If `stream.try_clone()` fails:

- emit `ClientRuntimeEvent::SafeError`;
- do not emit `ClientRuntimeEvent::Snapshot` for that reconnect response;
- break/stop the runtime path consistently with existing fatal reconnect failures;
- do not panic or unwrap.

If `command_connection_for_thread.lock()` fails:

- emit `ClientRuntimeEvent::SafeError`;
- do not emit `ClientRuntimeEvent::Snapshot` for that reconnect response;
- break/stop the runtime path consistently with existing fatal reconnect failures;
- do not panic or unwrap.

### Required code change

Replace the silent nested `if let Ok(...)` install block with explicit error handling.

Suggested target shape:

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

reconnect_token = snapshot_envelope.payload.reconnect_token.clone();
last_seen_server_sequence = Some(snapshot_sequence);
host_encryption_public_key = next_host_encryption_public_key;

let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(
    snapshot_envelope.payload,
)));
```

Adjust variable names to match the current code. If the code uses a different command connection variable name, preserve the current naming style.

### Important

Do **not** replace the silent failure with a fallback such as:

```rust
// Wrong: leaves command path disconnected while receive path continues.
if stream.try_clone().is_err() {
    // keep going anyway
}
```

Do **not** use `unwrap()` or `expect()` around `try_clone()` or the mutex lock.

### Acceptance

- Reconnect snapshot is emitted only after command-stream clone succeeds.
- Reconnect snapshot is emitted only after command connection lock succeeds.
- Clone failure emits `SafeError` and stops the reconnect/runtime path.
- Lock failure emits `SafeError` and stops the reconnect/runtime path.
- No silent `if let Ok(...)` ignores reconnect command-stream installation failure.
- No panic path is added.

---

## P0.2 — Check all reconnect/resync stream-install paths for the same quiet failure

**Files:**

- `src-tauri/src/networking/runtime/client.rs`

### Problem

The reviewed issue was found in the reconnect-after-disconnect path. There may be more than one place where the runtime handles reconnect or resync snapshots and installs a command stream.

### Required audit

Search for silent command-stream installation patterns:

```bash
rg -n "try_clone\(|command_connection.*lock\(|connection\.stream = Some|if let Ok\(cloned_stream\)|if let Ok\(mut connection\)" \
  src-tauri/src/networking/runtime/client.rs
```

For every reconnect/resync command-stream installation path:

- make stream clone failure explicit;
- make lock failure explicit;
- emit `SafeError` or an existing structured fatal runtime event;
- do not expose an accepted snapshot until command-stream installation succeeds;
- add a short comment explaining why the install is required before snapshot emission.

### Acceptance

- Every reconnect/resync stream-install path has explicit clone/lock failure behavior.
- There is no nested `if let Ok(...)` block that silently ignores stream installation failure.
- Existing normal successful reconnect behavior is preserved.

---

## P0.3 — Correct `memory.md` audit overclaims

**Files:**

- `memory.md`

### Problem

Fix 12 corrected one misleading Fix 11 ledger claim, but review found another overclaim: the Fix 12 entry may say something like:

```text
poker-core purity → 0 hits
```

for this broad grep:

```bash
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn|Command" crates/poker-core
```

That grep can return expected `EngineCommand` hits. The code is fine, but the ledger must not claim `0 hits` if there are expected false positives.

The Fix 12 validation list may also omit `npm run format:check` even if it was run.

### Required change

Add a new append-only correction entry using the project timestamp convention:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

Suggested entry:

```md
- <timestamp> — Fix 13 ledger correction: the `poker-core` forbidden-dependency grep `rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn|Command" crates/poker-core` returns only expected `EngineCommand` false positives; it should not be recorded as `0 hits`. No Tauri, networking, keychain, socket, thread-spawn, process-spawn, or app-data-path dependencies were found in `poker-core`. The exact forbidden direct assignment audit remains `rg -n "last_seen_server_sequence = envelope\.server_sequence" src-tauri/src/networking/runtime/client.rs`, which returns no hits. Fix 13 validation run in this environment: <exact commands actually run and pass/fail status, including `npm run format:check` if run>.
```

Also add the actual Fix 13 completion entry after validation:

```md
- <timestamp> — Fix 13 completed: reconnect snapshot acceptance now requires command-stream clone and command-connection lock success before emitting the accepted snapshot; clone/lock failures emit `SafeError` and stop the runtime path instead of leaving command submission disconnected. Previous protocol hardening remains intact: missing/stale public or snapshot sequence values warn/drop without defaulting to `0` or prior sequence, and no direct assignment from `envelope.server_sequence` to `last_seen_server_sequence` remains. Validation run in this environment: <exact commands actually run and pass/fail status>. No Android app, Tauri Mobile path, FFI crate, or networking-in-core was added.
```

### Acceptance

- `memory.md` does not claim broad greps returned zero hits when they returned expected false positives.
- `memory.md` clearly distinguishes exact forbidden patterns from safe validated assignments and expected false positives.
- `memory.md` includes `npm run format:check` in Fix 13 validation if it was actually run.
- `memory.md` lists exact commands actually run.
- No false completion claims.

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
- tests touched by Fix 13

### Required audit

For code touched by Fix 13, run:

```bash
rg -n "let _ =|\.ok\(\)|unwrap_or\(|unwrap_or_else\(|unwrap_or_default\(|thread::spawn|continue;|return;|if let Ok" \
  src-tauri/src/networking/runtime/client.rs
```

For every hit in touched production code:

- convert real silent failures to structured errors/diagnostics;
- add a short comment for intentional best-effort cleanup;
- leave harmless presentation-only/test-only defaults alone;
- do not rewrite unrelated stable code just to reduce grep output.

### Specific warning

Do not accept a reconnect snapshot if the command path was not installed. This shape is not allowed:

```rust
if let Ok(cloned_stream) = stream.try_clone() {
    if let Ok(mut connection) = command_connection_for_thread.lock() {
        connection.stream = Some(Arc::new(Mutex::new(cloned_stream)));
    }
}

// Wrong: snapshot is accepted even if clone/lock failed.
let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(snapshot_envelope.payload)));
```

### Acceptance

- No newly touched runtime code contains unexplained silent failure behavior.
- Any ignored error has a short comment explaining why it is safe.
- No new fallback invents authoritative protocol ordering state.
- No reconnect path leaves command submission disconnected while exposing an accepted snapshot.

---

## P2.2 — Do not start Android implementation in Fix 13

**Files:**

- docs only, if needed

### Required rule

Do not add:

- Android Gradle project files;
- Kotlin source files;
- Tauri Mobile configuration;
- `poker-android-ffi` crate;
- networking inside `poker-core`.

Fix 13 is reconnect/ledger cleanup only.

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
- Broader assignment greps may find safe validated assignments; document them honestly if mentioned.

## Definition of done

- [ ] Reconnect snapshot acceptance requires successful command-stream clone.
- [ ] Reconnect snapshot acceptance requires successful command-connection lock.
- [ ] Reconnect snapshot is not emitted if command-stream installation fails.
- [ ] Clone/lock failure emits `SafeError` and stops the runtime path.
- [ ] No silent reconnect command-stream install failure pattern remains.
- [ ] `memory.md` corrects the `poker-core` grep overclaim and records expected `EngineCommand` false positives honestly.
- [ ] `memory.md` includes exact commands actually run, including `npm run format:check` if run.
- [ ] Previous protocol hardening remains fixed: missing/stale public or snapshot sequence values warn/drop without defaulting to `0` or prior sequence.
- [ ] No direct assignment remains from `envelope.server_sequence` to `last_seen_server_sequence`.
- [ ] Previous hardening remains fixed: no test noise, no production mock chunk, no empty-AAD fallback, no empty NPC hole-card fallback, no raw client runtime `thread::spawn`.
- [ ] `poker-core` purity audit remains clean except expected harmless `EngineCommand` hits.
- [ ] No new hidden fallbacks or silent failures are introduced.
- [ ] No Android app, Tauri Mobile path, Android FFI crate, or networking-in-core is added.
- [ ] All final validation commands pass in an environment with Node 24 and Rust installed.
