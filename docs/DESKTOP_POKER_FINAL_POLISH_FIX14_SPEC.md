# Desktop Poker Fix 14 Spec — Final Validation Ledger and Cleanup Comments

## Purpose

Fix 14 is a deliberately small final-polish pass after the Fix 13 reconnect command-stream cleanup. The goal is not to change protocol architecture or add features. The goal is to remove the last process/maintenance ambiguity found in review:

1. `memory.md` must accurately record the exact validation commands that were actually run, including `cargo test -p poker-core` if it was run.
2. Best-effort cleanup locks in `ClientRuntime` must be explicitly documented so future reviewers do not confuse cleanup-only ignored lock failures with reconnect command-stream installation failures.
3. The previous protocol/runtime hardening checks must be rerun and recorded honestly.

Fix 14 should be small, easy to review, and mostly documentation/commentary plus validation. Do not make broad refactors.

## Context

The Fix 13 review found that the major runtime issue from Fix 12 was fixed: reconnect snapshot acceptance now requires command-stream clone success and command-connection lock success before emitting the accepted snapshot. Clone/lock failure now emits `ClientRuntimeEvent::SafeError` and stops the runtime path.

The remaining issues were narrow:

- The Fix 13 `memory.md` validation list omitted `cargo test -p poker-core`, even though the final validation command set includes it.
- Some cleanup-only blocks still use `if let Ok(mut connection) = command_connection_for_thread.lock() { connection.stream = None; }`. These are not the prohibited reconnect install pattern, but they should be commented as intentional best-effort cleanup.
- Full Rust validation still needs to be run in an environment with Rust installed and Node 24 active.

## High-level requirements

### 1. Accurate project ledger

`memory.md` must not overclaim. It must list exactly what was run and what passed or failed. If `cargo test -p poker-core` was run, record it. If it was not run, say it was not run.

Acceptable wording:

```md
Validation run in this environment:
- npm ci — pass
- npm run format:check — pass
- npm run lint — pass
- npm run build — pass
- npm test — pass
- cargo fmt --check — pass
- cargo clippy --workspace --all-targets --all-features -- -D warnings — pass
- cargo test --workspace --all-targets --all-features — pass
- cargo test -p poker-core — pass
- cargo tree -p poker-core — pass
```

If any command was not run, do not list it as passed.

### 2. Comment cleanup-only ignored lock failures

Best-effort cleanup blocks are acceptable when the runtime is already disconnecting, failing, or shutting down and the block only clears `command_connection.stream = None`.

However, any ignored lock failure must have a short comment explaining why it is safe. This prevents future agents from confusing it with the previously fixed reconnect install bug.

Preferred comment shape:

```rust
// Best-effort cleanup: if this lock is poisoned, command submission is already
// unusable and the runtime is disconnecting or surfacing an error below.
if let Ok(mut connection) = command_connection_for_thread.lock() {
    connection.stream = None;
}
```

Do not add `unwrap()` or `expect()` around cleanup locks.

### 3. Preserve Fix 13 reconnect behavior

Do not weaken the Fix 13 reconnect path. The accepted reconnect snapshot must still be emitted only after command-stream installation succeeds.

This pattern must remain forbidden:

```rust
if let Ok(cloned_stream) = stream.try_clone() {
    if let Ok(mut connection) = command_connection_for_thread.lock() {
        connection.stream = Some(Arc::new(Mutex::new(cloned_stream)));
    }
}

// Wrong if this can run after clone/lock failure.
let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(snapshot_envelope.payload)));
```

### 4. Preserve sequence hardening

Fix 14 must not weaken the public/snapshot sequence rules:

- missing required public-event `server_sequence` warns and drops;
- missing required snapshot `server_sequence` warns and drops;
- stale live snapshot sequence warns and drops;
- no missing/stale sequence path defaults to `0` or prior sequence;
- no direct assignment remains from `envelope.server_sequence` to `last_seen_server_sequence`.

### 5. Keep `poker-core` platform-neutral

No Tauri, networking, keychain, desktop UI, Android UI, socket, process-spawn, or thread-spawn dependency should enter `crates/poker-core`.

The broad `Command` grep may still find `EngineCommand`; that is expected and should be documented honestly.

### 6. Do not start Android implementation

Fix 14 must not add:

- Android Gradle files;
- Kotlin source files;
- Tauri Mobile configuration;
- `poker-android-ffi` crate;
- networking inside `poker-core`.

## Files likely touched

- `memory.md`
- `src-tauri/src/networking/runtime/client.rs`

No TypeScript code should need changes unless validation reveals a real issue.
No Rust protocol logic should need changes unless validation reveals a real issue.

## Final validation command set

Run from the repository root with Node 24 active and Rust installed:

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
- Broad cleanup greps may find intentionally best-effort cleanup blocks; those should now have comments.

## Definition of done

- `memory.md` records exact Fix 14 validation commands actually run.
- `memory.md` includes `cargo test -p poker-core` if it was run, or explicitly says it was not run.
- `memory.md` documents expected `EngineCommand` false positives honestly.
- Cleanup-only `command_connection_for_thread.lock()` ignore paths in `client.rs` have explanatory comments.
- Reconnect snapshot acceptance still requires successful command-stream clone and command-connection lock.
- No accepted reconnect snapshot is emitted if command-stream installation fails.
- Previous sequence hardening remains intact.
- Previous hardening remains fixed: no test noise, no production mock chunk, no empty-AAD fallback, no empty NPC hole-card fallback, no raw client runtime `thread::spawn`.
- `poker-core` purity audit remains clean except expected harmless `EngineCommand` hits.
- No new hidden fallbacks or silent failures are introduced.
- No Android app, Tauri Mobile path, Android FFI crate, or networking-in-core is added.
- All final validation commands pass in an environment with Node 24 and Rust installed, or any command that was not run is clearly recorded as not run.
