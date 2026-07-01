# Desktop Poker Fix 12 TODO — Final Protocol Ledger and Snapshot Ordering Cleanup

This TODO is intentionally explicit for Claude Code. Do tasks in priority order. Prefer small, test-backed patches. Do not introduce hidden fallback behavior to make tests pass.

Fix 12 is a narrow cleanup pass. Do not add features. Do not start Android implementation.

---

## P0.1 — Correct the misleading Fix 11 `memory.md` audit claim

**Files:**

- `memory.md`

### Problem

The Fix 11 review found that `memory.md` claimed a broad audit equivalent to:

```bash
rg -n "last_seen_server_sequence = .*server_sequence" src-tauri/src/networking/runtime/client.rs
```

returned zero hits.

That is misleading because the current code can still contain safe assignments such as:

```rust
last_seen_server_sequence = Some(envelope.server_sequence);
last_seen_server_sequence = Some(server_sequence);
```

Those are acceptable only when the sequence is already a validated `u64` or comes from a non-optional field. The exact forbidden pattern is the optional direct assignment:

```rust
last_seen_server_sequence = envelope.server_sequence;
```

### Required change

Add a corrective `memory.md` entry. Do not edit history to pretend the original claim was accurate unless the project convention requires append-only corrections.

Use the project timestamp convention:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

Suggested entry:

```md
- <timestamp> — Fix 12 correction: the exact forbidden direct assignment audit `rg -n "last_seen_server_sequence = envelope\.server_sequence" src-tauri/src/networking/runtime/client.rs` returns no hits. A broader grep such as `rg -n "last_seen_server_sequence = .*server_sequence" src-tauri/src/networking/runtime/client.rs` may still find safe assignments where the sequence is already a validated `u64` or comes from a non-optional envelope field. The Fix 11 ledger wording was corrected to avoid overstating the audit result.
```

### Acceptance

- `memory.md` clearly distinguishes the exact forbidden pattern from safe validated assignments.
- No ledger entry falsely claims a broad grep has zero hits if it does not.
- The correction lists exact commands or exact patterns, not vague claims.

---

## P0.2 — Prevent reconnect/resync snapshot validation failure from leaving a stream installed

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- tests if practical, likely inline `#[cfg(test)]` tests or existing runtime tests

### Problem

Review found a lifecycle smell in reconnect/resync paths: a cloned stream can be installed in `command_connection.stream` before snapshot validation finishes. If snapshot sequence or host encryption key validation then fails, the runtime emits `SafeError` and breaks, but the command connection may still hold a stream handle.

This is not the original missing-sequence fallback, but it is a quiet lifecycle issue. A failed reconnect/resync snapshot should not leave a partially accepted command stream installed.

### Required behavior

On reconnect/resync snapshot validation failure:

- emit the existing `ClientRuntimeEvent::SafeError` or protocol warning path as appropriate;
- do not leave a newly cloned stream in `command_connection.stream`;
- either validate before installing the stream, or explicitly clear it before breaking/returning;
- add a short comment for intentional cleanup.

### Preferred implementation direction

Prefer delaying installation until after snapshot validation if current control flow allows it.

Instead of this shape:

```rust
let command_stream = stream.try_clone()?;
if let Ok(mut connection) = command_connection.lock() {
    connection.stream = Some(command_stream);
}

let Some(snapshot_sequence) = snapshot_server_sequence_or_warn(...) else {
    let _ = sender.send(ClientRuntimeEvent::SafeError { /* ... */ });
    break;
};
```

Prefer this shape:

```rust
let command_stream = stream.try_clone()?;

let Some(snapshot_sequence) = snapshot_server_sequence_or_warn(
    &sender,
    &mut protocol_warning_counts,
    &player_id,
    snapshot_envelope.server_sequence,
) else {
    let _ = sender.send(ClientRuntimeEvent::SafeError {
        player_id: player_id.clone(),
        message: "reconnect snapshot envelope missing server sequence".to_string(),
    });
    break;
};

// Install the command stream only after the reconnect snapshot is accepted.
if let Ok(mut connection) = command_connection.lock() {
    connection.stream = Some(command_stream);
}

last_seen_server_sequence = Some(snapshot_sequence);
```

If the current code needs the stream installed earlier, use explicit cleanup before failure:

```rust
fn clear_command_stream_after_failed_reconnect(
    command_connection: &std::sync::Arc<std::sync::Mutex<CommandConnection>>,
) {
    if let Ok(mut connection) = command_connection.lock() {
        // The reconnect snapshot was rejected, so this cloned stream must not remain usable.
        connection.stream = None;
    }
}
```

Then call it before `break`/`return` on validation failure:

```rust
let Some(snapshot_sequence) = snapshot_server_sequence_or_warn(...) else {
    clear_command_stream_after_failed_reconnect(&command_connection);
    let _ = sender.send(ClientRuntimeEvent::SafeError {
        player_id: player_id.clone(),
        message: "reconnect snapshot envelope missing server sequence".to_string(),
    });
    break;
};
```

Adjust type names to match the current code. Do not introduce panics or unwraps to access the connection.

### Tests

Add a test if practical. The test should prove that failed reconnect/resync snapshot validation leaves no command stream installed.

If inspecting a real stream is too heavy, extract the cleanup helper and test it:

```rust
#[test]
fn clear_command_stream_after_failed_reconnect_removes_installed_stream() {
    let command_connection = std::sync::Arc::new(std::sync::Mutex::new(CommandConnection {
        stream: Some(dummy_stream_for_test()),
    }));

    clear_command_stream_after_failed_reconnect(&command_connection);

    let connection = command_connection
        .lock()
        .expect("command connection lock should not be poisoned in test");
    assert!(connection.stream.is_none());
}
```

If constructing `dummy_stream_for_test()` is too awkward because the field is a `TcpStream`, prefer delaying stream installation rather than adding test-only socket complexity.

### Acceptance

- Failed reconnect/resync snapshot validation cannot leave the newly cloned stream installed.
- The chosen ordering/cleanup has a short comment explaining why it is necessary.
- No `.unwrap()` or panic path is added around `command_connection` locking.
- No failure is silently ignored.

---

## P0.3 — Reject stale live `SNAPSHOT_EVENT` frames

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- tests under `src-tauri/src/networking/runtime/tests/**` or inline `#[cfg(test)]` tests in `client.rs`

### Problem

Fix 11 validates that a live `SNAPSHOT_EVENT` has a `server_sequence`, but it still appears permissive for present-but-stale snapshot sequences.

A host-originated live snapshot with `server_sequence: Some(5)` should not overwrite `last_seen_server_sequence: Some(10)` and emit a snapshot. That weakens protocol-ordering guarantees.

### Required behavior

For live `SNAPSHOT_EVENT` frames:

- missing `server_sequence`: warn, drop, preserve previous last-seen sequence;
- stale `server_sequence`: warn, drop, preserve previous last-seen sequence;
- fresh `server_sequence`: accept, update `last_seen_server_sequence`, update reconnect token / host encryption key as before, emit snapshot.

For Fix 12, stale live snapshots should **warn and drop**. Do not trigger resync from this branch unless the current runtime already has a safe, obvious resync path and adding it does not expand the patch.

### Suggested helper

If useful, add a small helper for stale required sequences:

```rust
fn is_stale_required_server_sequence(
    last_seen_server_sequence: Option<u64>,
    next_sequence: u64,
) -> bool {
    is_stale_server_sequence(last_seen_server_sequence, Some(next_sequence))
}
```

Or just call the existing helper directly:

```rust
if is_stale_server_sequence(last_seen_server_sequence, Some(snapshot_sequence)) {
    emit_protocol_warning(
        &sender,
        &mut protocol_warning_counts,
        &player_id,
        "stale snapshot server sequence",
    );
    continue;
}
```

### Suggested snapshot branch shape

```rust
ProtocolMessageType::SnapshotEvent => {
    let Ok(envelope) = serde_json::from_value::<SnapshotEnvelope>(frame_value.clone()) else {
        emit_protocol_warning(
            &sender,
            &mut protocol_warning_counts,
            &player_id,
            "malformed snapshot envelope",
        );
        continue;
    };

    let Some(snapshot_sequence) = snapshot_server_sequence_or_warn(
        &sender,
        &mut protocol_warning_counts,
        &player_id,
        envelope.server_sequence,
    ) else {
        // Malformed sequenced snapshot. Preserve last_seen_server_sequence.
        continue;
    };

    if is_stale_server_sequence(last_seen_server_sequence, Some(snapshot_sequence)) {
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
}
```

Keep existing type names and field names if they differ.

### Tests

Add a stale snapshot sequence test. If the existing tests are helper-level, test the production-used helper or extract one.

Suggested test:

```rust
#[test]
fn stale_snapshot_sequence_warns_and_preserves_last_seen_sequence() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut warning_counts = std::collections::BTreeMap::new();
    let mut last_seen_server_sequence = Some(42);

    let sequence = snapshot_server_sequence_or_warn(
        &sender,
        &mut warning_counts,
        "player-1",
        Some(41),
    );

    let Some(snapshot_sequence) = sequence else {
        panic!("present snapshot sequence should validate before stale check");
    };

    if is_stale_server_sequence(last_seen_server_sequence, Some(snapshot_sequence)) {
        emit_protocol_warning(
            &sender,
            &mut warning_counts,
            "player-1",
            "stale snapshot server sequence",
        );
    } else {
        last_seen_server_sequence = Some(snapshot_sequence);
    }

    assert_eq!(last_seen_server_sequence, Some(42));

    let warning = receiver.try_recv().expect("expected stale snapshot warning");
    assert!(matches!(
        warning,
        ClientRuntimeEvent::ProtocolWarning { reason, .. }
            if reason.contains("stale snapshot server sequence")
    ));

    assert!(
        receiver.try_recv().is_err(),
        "stale snapshot should not emit additional events"
    );
}
```

A stronger production-used helper would be better:

```rust
enum RequiredSequenceDecision {
    Accept(u64),
    Missing,
    Stale { last_seen: u64, received: u64 },
}

fn validate_required_ordered_sequence_or_warn(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    last_seen_server_sequence: Option<u64>,
    received_sequence: Option<u64>,
    missing_reason: &'static str,
    stale_reason: &'static str,
) -> RequiredSequenceDecision {
    let Some(sequence) = required_server_sequence_or_warn(
        sender,
        counts,
        player_id,
        received_sequence,
        missing_reason,
    ) else {
        return RequiredSequenceDecision::Missing;
    };

    if is_stale_server_sequence(last_seen_server_sequence, Some(sequence)) {
        emit_protocol_warning(sender, counts, player_id, stale_reason);
        return RequiredSequenceDecision::Stale {
            last_seen: last_seen_server_sequence.unwrap_or(0),
            received: sequence,
        };
    }

    RequiredSequenceDecision::Accept(sequence)
}
```

Only introduce this enum/helper if it simplifies production code rather than making the patch larger.

### Acceptance

- Live stale `SNAPSHOT_EVENT` emits `ClientRuntimeEvent::ProtocolWarning`.
- Live stale `SNAPSHOT_EVENT` does not mutate `last_seen_server_sequence`.
- Live stale `SNAPSHOT_EVENT` does not emit `ClientRuntimeEvent::Snapshot`.
- Fresh snapshot behavior remains unchanged except that sequence validation happens first.
- Missing snapshot behavior from Fix 11 remains intact.

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
```

### Acceptance

- No window-persistence test noise.
- Production dist does not contain browser mock/probe setup code.
- No private-message empty-AAD fallback.
- No acting NPC empty-hole-card fallback.
- No raw client runtime `thread::spawn`.
- No public-event/snapshot missing-sequence default to `0`.
- No assignment from `envelope.server_sequence` directly into `last_seen_server_sequence`.

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
- Any textual/comment hits are reviewed and documented if necessary.

---

## P1.2 — Update `memory.md` honestly after Fix 12 validation

**Files:**

- `memory.md`

### Required change

After completing Fix 12 and running validation, add a new entry using the project timestamp convention:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

Suggested entry:

```md
- <timestamp> — Fix 12 completed: corrected the Fix 11 ledger wording for the sequence-assignment audit; reconnect/resync snapshot validation failure no longer leaves a newly cloned command stream installed; live snapshot events now reject missing or stale required `server_sequence` values with `ProtocolWarning` while preserving previous `last_seen_server_sequence`; fresh snapshots update last-seen only after validation. Validation run in this environment: <exact commands actually run and pass/fail status>. Previous hardening remains intact: no test-noise regression, no production browser mock/probe bundle, no empty-AAD fallback, no empty NPC hole-card fallback, no raw client runtime `thread::spawn`, no direct assignment from `envelope.server_sequence` to `last_seen_server_sequence`. `poker-core` remains platform-neutral. No Android app, Tauri Mobile path, FFI crate, or networking-in-core was added.
```

### Acceptance

- `memory.md` lists exact commands actually run.
- `memory.md` does not claim broad grep results that are untrue.
- Android/core architecture decision remains recorded.
- No false completion claims.

---

## P2.1 — Silent-failure audit for touched runtime code

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- tests touched by Fix 12

### Required audit

For code touched by Fix 12, run:

```bash
rg -n "let _ =|\.ok\(\)|unwrap_or\(|unwrap_or_else\(|unwrap_or_default\(|thread::spawn|continue;|return;" \
  src-tauri/src/networking/runtime/client.rs
```

For every hit in touched production code:

- convert real silent failures to structured errors/diagnostics;
- add a short comment for intentional best-effort cleanup;
- leave harmless presentation-only/test-only defaults alone;
- do not rewrite unrelated stable code just to reduce grep output.

### Specific warnings

Do not replace missing or stale sequence defects with fallbacks like:

```rust
let snapshot_sequence = envelope.server_sequence.unwrap_or(last_seen_server_sequence.unwrap_or(0));
let snapshot_sequence = envelope.server_sequence.unwrap_or(0);
let snapshot_sequence = envelope.server_sequence.unwrap_or(previous_sequence);
```

Those are all wrong. Missing sequence is malformed input. Stale sequence is invalid ordering input. Warn and drop.

### Acceptance

- No newly touched runtime code contains unexplained silent failure behavior.
- Any ignored error has a short comment explaining why it is safe.
- No new fallback invents authoritative protocol ordering state.

---

## P2.2 — Do not start Android implementation in Fix 12

**Files:**

- docs only, if needed

### Required rule

Do not add:

- Android Gradle project files;
- Kotlin source files;
- Tauri Mobile configuration;
- `poker-android-ffi` crate;
- networking inside `poker-core`.

Fix 12 is protocol/ledger cleanup only.

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
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn|Command" crates/poker-core
```

Expected notes:

- `EngineCommand` in `poker-core` is an expected false positive for the broad `Command` grep.
- `src-tauri/src/tournament` may not exist after the `poker-core` extraction; this is okay.
- Production dist grep should have no JS payload hits for `LayoutProbeApp` or `__DESKTOP_POKER_BROWSER_MOCKS__`.
- The exact direct-assignment grep must return no hits.
- Broader assignment greps may find safe validated assignments; document them honestly if mentioned.

## Definition of done

- [ ] `memory.md` correction distinguishes exact forbidden direct assignment from safe validated assignments.
- [ ] Reconnect/resync snapshot validation failure cannot leave a newly cloned command stream installed.
- [ ] Missing live snapshot sequence warns, drops the frame, and preserves previous `last_seen_server_sequence`.
- [ ] Stale live snapshot sequence warns, drops the frame, and preserves previous `last_seen_server_sequence`.
- [ ] Fresh live snapshot sequence updates `last_seen_server_sequence` only after validation.
- [ ] Tests cover stale snapshot warning and last-seen preservation.
- [ ] Missing snapshot tests from Fix 11 still pass.
- [ ] Live public-event sequence hardening remains intact.
- [ ] No missing/stale sequence path defaults to `0` or to prior sequence.
- [ ] No direct assignment remains from `envelope.server_sequence` to `last_seen_server_sequence`.
- [ ] Previous hardening remains fixed: no test noise, no production mock chunk, no empty-AAD fallback, no empty NPC hole-card fallback, no raw client runtime `thread::spawn`.
- [ ] `poker-core` purity audit remains clean.
- [ ] `memory.md` accurately records commands actually run.
- [ ] No new hidden fallbacks or silent failures are introduced.
- [ ] No Android app, Tauri Mobile path, Android FFI crate, or networking-in-core is added.
- [ ] All final validation commands pass in an environment with Node 24 and Rust installed.
