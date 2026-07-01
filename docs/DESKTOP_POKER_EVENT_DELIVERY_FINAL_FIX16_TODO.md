# Desktop Poker Fix 16 TODO — Final Event Send Consistency and Ledger Closure

This TODO is intentionally explicit for Claude Code. Fix 16 is a tiny closure pass after Fix 15. Prefer the smallest auditable patch. Do not introduce hidden fallback behavior to make tests pass.

Fix 16 is not a feature pass. Do not start Android implementation.

---

## P0.1 — Convert the remaining raw live snapshot event send to the helper

**Files:**

- `src-tauri/src/networking/runtime/client.rs`

### Problem

Fix 15 introduced `send_runtime_event_best_effort(...)` so intentional ignored `ClientRuntimeEvent` send failures are explicit and auditable.

Review found one remaining raw event send in the live `SNAPSHOT_EVENT` branch:

```rust
// Best-effort event delivery: receiver may be gone during shutdown.
let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(envelope.payload)));
```

This is not a severe runtime bug because it has a comment, but it is inconsistent with the helper and can make `memory.md` overclaim that every ignored event send was replaced.

### Required change

Replace the remaining raw live snapshot send with the existing helper.

Expected replacement:

```rust
send_runtime_event_best_effort(
    &sender,
    ClientRuntimeEvent::Snapshot(Box::new(envelope.payload)),
);
```

Keep the surrounding snapshot sequence validation behavior unchanged.

### Acceptance

- The live `SNAPSHOT_EVENT` branch uses `send_runtime_event_best_effort(...)` when emitting `ClientRuntimeEvent::Snapshot`.
- No behavior change is introduced other than using the helper for event-channel delivery.
- No protocol validation failure is hidden.
- No command-stream clone/lock failure is routed through the helper as a harmless condition.
- No `unwrap()`, `expect()`, or panic path is added.

---

## P0.2 — Verify no raw `sender.send(...)` ignores remain outside the helper

**Files:**

- `src-tauri/src/networking/runtime/client.rs`

### Required audit

Run:

```bash
rg -n "let _ = sender\.send" src-tauri/src/networking/runtime/client.rs
```

Expected result after P0.1:

- the only hit should be inside `send_runtime_event_best_effort(...)` itself.

If more hits remain, inspect each one.

### Allowed outcomes

Preferred: convert every intentional ignored `ClientRuntimeEvent` send to the helper.

If a raw `let _ = sender.send(...)` must remain, add a precise comment explaining why the helper is not appropriate and record that exception in `memory.md`.

### Acceptance

- `rg -n "let _ = sender\.send" src-tauri/src/networking/runtime/client.rs` returns only the helper implementation, or every remaining non-helper hit has a specific comment and is recorded honestly.
- `memory.md` does not claim all sends were replaced unless that is true.
- No real runtime failure is downgraded into best-effort behavior.

---

## P0.3 — Keep reconnect command-stream installation strict

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

The reconnect snapshot acceptance path must still require successful command-stream clone and command-connection lock before emitting the accepted snapshot.

Required shape:

```rust
// Snapshot validation has succeeded. Command stream installation is required
// before exposing the reconnect snapshot to the UI; otherwise the client can
// appear reconnected while action submission remains disconnected.
let cloned_stream = match stream.try_clone() {
    Ok(cloned_stream) => cloned_stream,
    Err(error) => {
        send_runtime_event_best_effort(
            &sender,
            ClientRuntimeEvent::SafeError {
                player_id: player_id.clone(),
                message: format!("failed to clone reconnect command stream: {error}"),
            },
        );
        break;
    }
};

match command_connection_for_thread.lock() {
    Ok(mut connection) => {
        connection.stream = Some(Arc::new(Mutex::new(cloned_stream)));
    }
    Err(_) => {
        send_runtime_event_best_effort(
            &sender,
            ClientRuntimeEvent::SafeError {
                player_id: player_id.clone(),
                message: "client command connection lock poisoned after reconnect".to_string(),
            },
        );
        break;
    }
}
```

### Forbidden shape

This must not exist in reconnect snapshot acceptance:

```rust
if let Ok(cloned_stream) = stream.try_clone() {
    if let Ok(mut connection) = command_connection_for_thread.lock() {
        connection.stream = Some(Arc::new(Mutex::new(cloned_stream)));
    }
}

// Wrong if this can run after clone or lock failure.
send_runtime_event_best_effort(
    &sender,
    ClientRuntimeEvent::Snapshot(Box::new(snapshot_envelope.payload)),
);
```

### Acceptance

- Reconnect snapshot acceptance still requires successful command-stream clone.
- Reconnect snapshot acceptance still requires successful command-connection lock.
- Clone/lock failure emits `SafeError` and stops the runtime path.
- Reconnect snapshot is not emitted if command-stream installation fails.
- Cleanup-only `if let Ok(mut connection)` blocks remain commented as best-effort cleanup.

---

## P0.4 — Preserve public/snapshot sequence hardening

**Files:**

- `src-tauri/src/networking/runtime/client.rs`

### Required audit

Run:

```bash
rg -n "server_sequence.*unwrap_or_default|unwrap_or_default\(\).*server_sequence" src-tauri/src/networking/runtime/client.rs
rg -n "last_seen_server_sequence = envelope\.server_sequence" src-tauri/src/networking/runtime/client.rs
```

Also inspect the live public-event and live `SNAPSHOT_EVENT` branches manually.

### Required behavior

- Missing live public-event `server_sequence` emits `ProtocolWarning` and drops the event.
- Missing live snapshot `server_sequence` emits `ProtocolWarning` and drops the snapshot.
- Stale live snapshot `server_sequence` emits `ProtocolWarning` and drops the snapshot.
- Fresh live snapshot updates `last_seen_server_sequence` only after validation.
- No missing/stale sequence path defaults to `0` or the prior sequence.

### Acceptance

- The two grep commands above return no hits.
- Live public-event sequence hardening remains intact.
- Live snapshot missing/stale sequence hardening remains intact.
- No new fallback invents authoritative protocol ordering state.

---

## P0.5 — Re-run previous frontend and runtime hardening audits

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- `src-tauri/src/npc/runner/action.rs`
- `src-tauri/src/npc/provider_storage.rs`
- `src/main.tsx`
- `src/test/setup.ts`
- `crates/poker-core/src/**`

### Required commands

Run from repo root:

```bash
npm ci
npm run format:check
npm run lint
npm run build
npm test
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
rg -n "let _ = sender\.send" src-tauri/src/networking/runtime/client.rs
```

### Expected notes

- `src-tauri/src/tournament` may not exist after core extraction; that is okay.
- Production dist should have no JS payload hits for `LayoutProbeApp` or `__DESKTOP_POKER_BROWSER_MOCKS__`.
- Cleanup-only `if let Ok(mut connection)` hits are acceptable only if commented as best-effort cleanup.
- `rg -n "let _ = sender\.send" ...` should only hit inside `send_runtime_event_best_effort(...)` after P0.1.

### Acceptance

- `npm ci`, `npm run format:check`, `npm run lint`, `npm run build`, and `npm test` pass.
- No window-persistence test noise.
- No production browser mock/probe bundle.
- No private-message empty-AAD fallback.
- No acting NPC empty-hole-card fallback.
- No raw client runtime `thread::spawn`.
- No public/snapshot sequence default-to-zero fallback.
- No direct assignment from `envelope.server_sequence` to `last_seen_server_sequence`.
- No silent reconnect command-stream install failure pattern.

---

## P1.1 — Run and record full Rust validation in the real environment

**Files:**

- `memory.md`
- Rust workspace files touched by Fix 16

### Required commands

Run from repo root in an environment with Rust installed:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test -p poker-core
cargo tree -p poker-core
```

Also run:

```bash
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|Mutex<.*Tcp|thread::spawn|Command" crates/poker-core
```

Expected:

- no Tauri dependency;
- no socket/networking dependency;
- no keychain/provider/LLM dependency;
- no thread spawning;
- no process spawning;
- no app-data-path logic;
- only expected harmless false positives such as `EngineCommand`.

### Acceptance

- Rust validation commands pass, or any command not run is explicitly recorded as not run.
- `poker-core` dependency tree remains platform-neutral.
- Expected `EngineCommand` hits are documented honestly.

---

## P1.2 — Update `memory.md` honestly after Fix 16

**Files:**

- `memory.md`

### Required change

Add an append-only entry using the project timestamp convention:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

Suggested entry if the final raw send is converted:

```md
- <timestamp> — Fix 16 completed: the final raw live snapshot `let _ = sender.send(...)` call in `ClientRuntime` was converted to `send_runtime_event_best_effort(...)`, so intentional ignored `ClientRuntimeEvent` sends are centralized through the helper. Reconnect snapshot acceptance still requires successful command-stream clone and command-connection lock before emitting the accepted snapshot. Previous protocol hardening remains intact: missing/stale public or snapshot sequence values warn/drop without defaulting to `0` or prior sequence, and no direct assignment from `envelope.server_sequence` to `last_seen_server_sequence` remains. Validation run in this environment: <exact commands actually run and pass/fail status>. `poker-core` purity audit: <exact grep/tree result, including expected `EngineCommand` false positives if present>. No Android app, Tauri Mobile path, FFI crate, or networking-in-core was added.
```

Suggested entry if a raw send remains intentionally:

```md
- <timestamp> — Fix 16 completed: reviewed remaining raw `let _ = sender.send(...)` uses in `ClientRuntime`. Remaining non-helper uses are intentionally retained and documented because <specific reason>. Reconnect snapshot acceptance and protocol sequence hardening remain intact. Validation run in this environment: <exact commands actually run and pass/fail status>. No Android app, Tauri Mobile path, FFI crate, or networking-in-core was added.
```

Do not claim all raw event sends were replaced unless `rg -n "let _ = sender\.send" src-tauri/src/networking/runtime/client.rs` only hits inside the helper.

### Acceptance

- `memory.md` lists exact commands actually run.
- `memory.md` does not claim Rust commands passed unless they actually passed.
- `memory.md` documents expected grep false positives honestly.
- `memory.md` does not overclaim event-send replacement status.
- No false completion claims.

---

## P2.1 — Silent-failure audit for touched runtime code

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- tests touched by Fix 16, if any

### Required audit

Run:

```bash
rg -n "let _ =|\.ok\(\)|unwrap_or\(|unwrap_or_else\(|unwrap_or_default\(|thread::spawn|continue;|return;|if let Ok" \
  src-tauri/src/networking/runtime/client.rs
```

For every hit in touched production code:

- convert real silent failures to structured errors/diagnostics;
- use `send_runtime_event_best_effort(...)` for intentional event-channel sends;
- add a short comment for intentional cleanup lock ignores;
- leave harmless presentation/test-only defaults alone;
- do not rewrite unrelated stable code just to reduce grep output.

### Acceptance

- No newly touched runtime code contains unexplained silent failure behavior.
- No reconnect path leaves command submission disconnected while exposing an accepted snapshot.
- No new fallback invents authoritative protocol ordering state.

---

## P2.2 — Do not start Android implementation in Fix 16

**Files:**

- docs only, if needed

### Required rule

Do not add:

- Android Gradle project files;
- Kotlin source files;
- Tauri Mobile configuration;
- `poker-android-ffi` crate;
- networking inside `poker-core`.

Fix 16 is final event-delivery/ledger closure only.

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
rg -n "let _ = sender\.send" src-tauri/src/networking/runtime/client.rs
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn|Command" crates/poker-core
```

Expected notes:

- `EngineCommand` in `poker-core` is an expected false positive for the broad `Command` grep.
- `src-tauri/src/tournament` may not exist after the `poker-core` extraction; this is okay.
- Production dist grep should have no JS payload hits for `LayoutProbeApp` or `__DESKTOP_POKER_BROWSER_MOCKS__`.
- The exact direct-assignment grep must return no hits.
- Cleanup-only lock ignores must be commented as best-effort cleanup.
- `let _ = sender.send` should only appear inside `send_runtime_event_best_effort(...)`, unless a remaining exception is explicitly documented.

## Definition of done

- [ ] The live snapshot event send uses `send_runtime_event_best_effort(...)` or a remaining raw send is explicitly justified.
- [ ] `rg -n "let _ = sender\.send" src-tauri/src/networking/runtime/client.rs` only hits inside the helper, unless a remaining exception is documented honestly.
- [ ] Reconnect snapshot acceptance still requires successful command-stream clone.
- [ ] Reconnect snapshot acceptance still requires successful command-connection lock.
- [ ] Reconnect snapshot is not emitted if command-stream installation fails.
- [ ] Clone/lock failure still emits `SafeError` and stops the runtime path.
- [ ] Previous protocol hardening remains fixed: missing/stale public or snapshot sequence values warn/drop without defaulting to `0` or prior sequence.
- [ ] No direct assignment remains from `envelope.server_sequence` to `last_seen_server_sequence`.
- [ ] Previous hardening remains fixed: no test noise, no production mock chunk, no empty-AAD fallback, no empty NPC hole-card fallback, no raw client runtime `thread::spawn`.
- [ ] `poker-core` purity audit remains clean except expected harmless `EngineCommand` hits.
- [ ] `memory.md` records exact commands actually run.
- [ ] `memory.md` does not overclaim event-send replacement status.
- [ ] No new hidden fallbacks or silent failures are introduced.
- [ ] No Android app, Tauri Mobile path, Android FFI crate, or networking-in-core is added.
- [ ] All final validation commands pass in an environment with Node 24 and Rust installed, or any command that was not run is clearly recorded as not run.
