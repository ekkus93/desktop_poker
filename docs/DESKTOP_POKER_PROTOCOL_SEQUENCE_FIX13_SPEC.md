# Desktop Poker Fix 13 Spec — Reconnect Stream Failure and Ledger Truth Cleanup

## Purpose

Fix 13 is a narrow cleanup pass after Fix 12. The goal is not to add features or change architecture. The goal is to remove the last reviewed quiet runtime failure in the reconnect path, correct misleading validation claims in `memory.md`, and re-run the same focused protocol/hardening audits.

The specific reviewed defect is in `src-tauri/src/networking/runtime/client.rs`: after a reconnect snapshot has passed sequence/key validation, the runtime attempts to clone and install the command stream using nested `if let Ok(...)` blocks. If either stream cloning or `command_connection` locking fails, the failure is silently ignored while the runtime continues and emits a `ClientRuntimeEvent::Snapshot`. That can make the UI believe the client is reconnected while action submission remains disconnected.

Fix 13 must make reconnect command-stream installation an explicit required step before accepting/exposing the reconnect snapshot.

## Non-goals

- Do not add Android implementation.
- Do not add Tauri Mobile configuration.
- Do not add a `poker-android-ffi` crate.
- Do not move networking into `poker-core`.
- Do not redesign the protocol.
- Do not rewrite unrelated runtime code just to reduce grep output.
- Do not introduce fallbacks that invent protocol ordering state.

## Current architecture constraints

- Frontend: React + TypeScript.
- Desktop adapter: Tauri 2 + Rust in `src-tauri`.
- Shared poker rules/state/projection: Rust crate in `crates/poker-core`.
- `poker-core` must remain platform-neutral: no Tauri, networking, keychain, socket, process, app-data-path, or UI dependencies.
- Android remains future work: native Kotlin/Compose app + Rust bindings, with Kotlin owning Android UI/lifecycle/networking/session transport.

## Required behavior

### Reconnect command stream installation

When reconnect/resync snapshot validation succeeds, the runtime must not emit the accepted snapshot unless the command stream is also successfully cloned and installed.

If stream cloning fails:

- emit `ClientRuntimeEvent::SafeError` with a clear message;
- do not emit the reconnect snapshot;
- break/stop the runtime path consistently with existing fatal reconnect errors;
- do not leave a partially installed stream.

If `command_connection` locking fails:

- emit `ClientRuntimeEvent::SafeError` with a clear message;
- do not emit the reconnect snapshot;
- break/stop the runtime path consistently with existing fatal reconnect errors;
- do not panic or unwrap.

The chosen implementation should keep the Fix 12 delay-installation approach: validate the snapshot first, then clone/install the command stream, then update reconnect state and emit the snapshot.

### Memory ledger honesty

`memory.md` must be corrected so it does not claim broad grep commands returned zero hits when they actually return expected false positives.

Specifically:

- The exact forbidden direct-assignment grep may be recorded as clean only if this exact command returns no hits:

```bash
rg -n "last_seen_server_sequence = envelope\.server_sequence" src-tauri/src/networking/runtime/client.rs
```

- Broader greps such as the following may still find safe assignments and must not be described as zero-hit if they are not:

```bash
rg -n "last_seen_server_sequence = .*server_sequence" src-tauri/src/networking/runtime/client.rs
```

- The `poker-core` forbidden-dependency grep may return expected `EngineCommand` false positives. The ledger should say so explicitly rather than claiming `0 hits`:

```bash
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn|Command" crates/poker-core
```

### Existing protocol hardening must remain intact

Fix 13 must preserve the previous hardening work:

- public-event required `server_sequence` is validated before stale checking and before `last_seen_server_sequence` mutation;
- missing public-event sequence warns and drops;
- missing live snapshot sequence warns and drops;
- stale live snapshot sequence warns and drops;
- no missing/stale sequence path defaults to `0` or the prior sequence;
- no exact assignment remains from `envelope.server_sequence` to `last_seen_server_sequence`;
- no empty-AAD private-message fallback;
- no acting NPC empty-hole-card fallback;
- no raw client runtime `thread::spawn`.

## Implementation guidance

Prefer replacing any reconnect stream installation like this:

```rust
if let Ok(cloned_stream) = stream.try_clone() {
    if let Ok(mut connection) = command_connection_for_thread.lock() {
        connection.stream = Some(Arc::new(Mutex::new(cloned_stream)));
    }
}
```

with an explicit required installation step:

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

Adjust names to match the current code. The important property is that failure to install the reconnect command stream is not ignored.

If there are multiple reconnect/resync snapshot-acceptance paths, apply this policy to each path that installs a command stream before or after snapshot acceptance.

## Testing expectations

Add a focused test if practical. If `TcpStream` construction makes direct testing too expensive, do not create brittle socket plumbing solely for this. Instead:

- make the production branch explicit and audit-friendly;
- add a small helper only if it simplifies production code;
- rely on the focused grep and runtime tests already in the suite;
- document why no direct stream-install failure test was added.

Do not add test-only production fallback behavior.

## Validation expectations

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
- The exact direct-assignment grep must return no hits.
- Broader sequence-assignment greps may find safe validated assignments; document them honestly if mentioned.
- Production dist should not contain `LayoutProbeApp` or `__DESKTOP_POKER_BROWSER_MOCKS__` in JS payloads.

## Definition of done

- Reconnect/resync snapshot acceptance cannot silently continue if command-stream cloning fails.
- Reconnect/resync snapshot acceptance cannot silently continue if `command_connection` locking fails.
- Reconnect snapshot is emitted only after command stream installation succeeds.
- `memory.md` correction distinguishes exact forbidden direct assignment from safe validated assignments.
- `memory.md` records expected `EngineCommand` false positives honestly.
- `memory.md` includes `npm run format:check` in the Fix 13 validation list if it was actually run.
- Previous hardening remains fixed: no test noise, no production mock chunk, no empty-AAD fallback, no empty NPC hole-card fallback, no raw client runtime `thread::spawn`, no sequence default-to-zero fallback.
- `poker-core` purity audit remains clean except expected harmless `EngineCommand` hits.
- No new hidden fallbacks or silent failures are introduced.
- No Android app, Tauri Mobile path, Android FFI crate, or networking-in-core is added.
- All final validation commands pass in an environment with Node 24 and Rust installed.
