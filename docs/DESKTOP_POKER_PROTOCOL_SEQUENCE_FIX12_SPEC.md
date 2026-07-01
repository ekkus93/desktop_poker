# Desktop Poker Fix 12 Spec — Final Protocol Ledger and Snapshot Ordering Cleanup

## Purpose

Fix 12 is a narrow cleanup pass after Fix 11. The shared-core extraction and sequence-hardening work is largely complete, but review found three remaining issues that should be resolved before ending this stabilization loop:

1. `memory.md` contains a misleading Fix 11 audit claim.
2. reconnect/resync snapshot validation failures can leave a cloned command stream installed after the runtime decides to fail/break.
3. live `SNAPSHOT_EVENT` frames validate that `server_sequence` exists, but do not reject stale snapshot sequences.

This pass should not add product features, UI changes, Android code, Tauri Mobile configuration, Android FFI crates, or networking inside `poker-core`.

## Current architecture boundary

The current architecture remains:

- `crates/poker-core` owns platform-neutral poker rules, state, projection, and deterministic engine/facade logic.
- `src-tauri` owns the desktop adapter, desktop networking/session runtime, Tauri commands, storage integration, and desktop-specific runtime wiring.
- Future Android support is native Kotlin/Compose plus Rust bindings to the shared core.
- Android Kotlin owns Android UI/lifecycle/storage/networking/session transport.
- `poker-core` must not import Tauri, Android, keychain, networking sockets, app-data-path logic, provider/LLM code, or process/thread runtime adapters.

## Fix 12 scope

Fix 12 should do only the following:

1. Correct the misleading Fix 11 `memory.md` audit statement.
2. Ensure reconnect/resync snapshot validation failure does not leave `command_connection.stream` populated with a newly cloned stream.
3. Define and enforce policy for stale live `SNAPSHOT_EVENT` frames.
4. Add tests for snapshot stale-sequence behavior and reconnect/resync stream cleanup if practical.
5. Re-run the prior protocol/hardening audits.
6. Confirm `poker-core` remains platform-neutral.

## Non-goals

Do not do any of the following in Fix 12:

- add Android Gradle files;
- add Kotlin source files;
- add Tauri Mobile configuration;
- add `poker-android-ffi`;
- move networking into `poker-core`;
- redesign the protocol;
- rewrite the client runtime wholesale;
- alter user-visible gameplay behavior except for rejecting malformed/stale snapshot frames;
- silence tests or greps by deleting coverage;
- replace missing/stale sequence errors with fallback defaults.

## Required behavior

### 1. `memory.md` honesty

`memory.md` must not claim that a broad grep returned zero hits if it did not. The exact Fix 11 audit that matters is:

```bash
rg -n "last_seen_server_sequence = envelope\.server_sequence" src-tauri/src/networking/runtime/client.rs
```

It is acceptable if broader greps find safe assignments such as:

```rust
last_seen_server_sequence = Some(envelope.server_sequence);
last_seen_server_sequence = Some(server_sequence);
```

when the sequence is already a validated `u64` or comes from a non-optional envelope field. The ledger should distinguish the exact forbidden pattern from safe validated assignments.

### 2. reconnect/resync stream lifecycle

When reconnect or resync obtains a cloned command stream but then fails validation of the snapshot envelope, the runtime must not leave the cloned stream installed in `command_connection.stream`.

Preferred behavior:

- validate the reconnect/resync snapshot sequence and host encryption key before installing the cloned command stream, if the current control flow allows that safely;
- otherwise, if the stream must be installed earlier, explicitly clear it before emitting `SafeError` and breaking/returning;
- add a short comment explaining the cleanup so future agents do not treat it as accidental.

### 3. stale live `SNAPSHOT_EVENT` policy

A live host-originated `SNAPSHOT_EVENT` with a present but stale `server_sequence` must not silently overwrite `last_seen_server_sequence` or emit a snapshot event.

Policy:

- missing snapshot sequence: emit `ProtocolWarning`, drop frame, preserve previous `last_seen_server_sequence`;
- stale snapshot sequence: emit `ProtocolWarning`, drop frame, preserve previous `last_seen_server_sequence`;
- fresh snapshot sequence: accept, update `last_seen_server_sequence`, update reconnect token / host encryption key as before, emit `ClientRuntimeEvent::Snapshot`.

Do not trigger a resync from a stale snapshot unless the current runtime already has a well-defined safe way to do that from this branch. The narrow Fix 12 behavior is warning + drop for stale live snapshots.

Reconnect/resync response snapshots may have different control-flow semantics from unsolicited live `SNAPSHOT_EVENT` frames. Missing required sequence is always malformed. Stale reconnect/resync response handling can be left as a `SafeError`/break if implementing live snapshot stale drop would otherwise grow the patch too much.

## Expected implementation direction

The client runtime already has sequence validation helpers from Fix 10/11. Reuse them rather than duplicating logic.

Recommended helper direction:

```rust
fn is_stale_required_server_sequence(
    last_seen_server_sequence: Option<u64>,
    next_sequence: u64,
) -> bool {
    is_stale_server_sequence(last_seen_server_sequence, Some(next_sequence))
}
```

Recommended live snapshot branch shape:

```rust
let Some(snapshot_sequence) = snapshot_server_sequence_or_warn(
    &sender,
    &mut protocol_warning_counts,
    &player_id,
    envelope.server_sequence,
) else {
    // Malformed sequenced snapshot. Preserve last_seen_server_sequence.
    continue;
};

if is_stale_required_server_sequence(last_seen_server_sequence, snapshot_sequence) {
    emit_protocol_warning(
        &sender,
        &mut protocol_warning_counts,
        &player_id,
        "stale snapshot server sequence",
    );
    continue;
}

reconnect_token = envelope.payload.reconnect_token.clone();
last_seen_server_sequence = Some(snapshot_sequence);

if let Some(next_host_encryption_public_key) =
    envelope.payload.host_encryption_public_key.clone()
{
    host_encryption_public_key = next_host_encryption_public_key;
}

let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(envelope.payload)));
```

If `let _ = sender.send(...)` remains in this branch, it should be because the receiver can legitimately be gone during shutdown. Add or preserve comments for intentional best-effort event delivery where appropriate.

## Required tests

Add or update tests that prove:

1. missing snapshot sequence warns and preserves previous `last_seen_server_sequence`;
2. present fresh snapshot sequence updates `last_seen_server_sequence` after validation;
3. stale snapshot sequence warns, preserves previous `last_seen_server_sequence`, and does not emit a snapshot;
4. reconnect/resync snapshot validation failure does not leave `command_connection.stream` populated, if that state is practical to inspect without broad runtime surgery.

If direct stream-state inspection is too invasive, test the extracted cleanup helper or add a clear comment in the code explaining why the runtime clears/defers stream installation.

## Validation requirements

Run from repository root with Node 24 active and Rust installed:

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
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn|Command" crates/poker-core
```

Expected notes:

- `EngineCommand` in `poker-core` is an expected false positive for the broad `Command` grep.
- `src-tauri/src/tournament` may not exist after extraction; that is okay.
- The exact direct-assignment grep must return no hits.
- Broader sequence assignment greps may find safe validated assignments; do not claim otherwise in `memory.md`.

## Definition of done

- `memory.md` contains an honest correction for the Fix 11 grep/audit claim.
- Reconnect/resync snapshot validation failure does not leave a newly cloned stream installed.
- Live `SNAPSHOT_EVENT` with missing sequence warns, drops, and preserves previous sequence.
- Live `SNAPSHOT_EVENT` with stale sequence warns, drops, and preserves previous sequence.
- Live `SNAPSHOT_EVENT` with fresh sequence updates sequence after validation and emits snapshot as before.
- No missing/stale sequence path defaults to `0` or to the prior sequence.
- No direct assignment remains from `envelope.server_sequence` to `last_seen_server_sequence`.
- Previous hardening remains fixed.
- `poker-core` purity audit remains clean.
- No new hidden fallbacks or silent failures are introduced.
- No Android app, Tauri Mobile path, Android FFI crate, or networking-in-core is added.
- Final validation passes in an environment with Node 24 and Rust installed.
