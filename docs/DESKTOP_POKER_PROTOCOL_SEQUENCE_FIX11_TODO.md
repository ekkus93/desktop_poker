# Desktop Poker Fix 11 TODO — Finish Snapshot/Public Sequence Validation

This TODO is intentionally explicit for Claude Code. Do tasks in priority order. Prefer small, test-backed patches. Do not introduce hidden fallback behavior to make tests pass.

Fix 11 is a narrow protocol-hardening cleanup. Do not add features. Do not start Android implementation.

---

## P0.1 — Remove direct snapshot assignment from `envelope.server_sequence`

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- tests under `src-tauri/src/networking/runtime/tests/**` or inline tests in `client.rs`

### Problem

Fix 10 correctly fixed the live public-event branch, but review found this remaining pattern in the snapshot branch:

```rust
last_seen_server_sequence = envelope.server_sequence;
```

If a malformed `SNAPSHOT_EVENT` has `server_sequence == None`, this silently clears the client's last-seen sequence state. That is another protocol-ordering fallback. A missing required sequence on a host-originated snapshot is malformed input. The client must warn, drop the frame, and preserve the previous sequence.

### Required change

Validate snapshot event `server_sequence` before mutating `last_seen_server_sequence`.

Add or reuse a helper in `client.rs`:

```rust
fn required_server_sequence_or_warn(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    server_sequence: Option<u64>,
    reason: &'static str,
) -> Option<u64> {
    match server_sequence {
        Some(sequence) => Some(sequence),
        None => {
            emit_protocol_warning(sender, counts, player_id, reason);
            None
        }
    }
}
```

If `public_event_server_sequence_or_warn(...)` already exists, make it a wrapper or replace it:

```rust
fn public_event_server_sequence_or_warn(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    server_sequence: Option<u64>,
) -> Option<u64> {
    required_server_sequence_or_warn(
        sender,
        counts,
        player_id,
        server_sequence,
        "public event envelope missing server sequence",
    )
}
```

Then add a snapshot-specific wrapper if useful:

```rust
fn snapshot_server_sequence_or_warn(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    server_sequence: Option<u64>,
) -> Option<u64> {
    required_server_sequence_or_warn(
        sender,
        counts,
        player_id,
        server_sequence,
        "snapshot envelope missing server sequence",
    )
}
```

### Suggested snapshot branch shape

Find the `ProtocolMessageType::SnapshotEvent` branch or equivalent snapshot branch in `client.rs` and change it so the sequence is validated before state mutation.

Target shape:

```rust
ProtocolMessageType::SnapshotEvent => {
    let Ok(snapshot_envelope) = serde_json::from_value::<SnapshotEnvelope>(frame_value.clone()) else {
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
        snapshot_envelope.server_sequence,
    ) else {
        // Malformed sequenced snapshot. Do not update last_seen_server_sequence.
        continue;
    };

    reconnect_token = snapshot_envelope.payload.reconnect_token.clone();
    last_seen_server_sequence = Some(snapshot_sequence);

    if let Some(next_host_encryption_public_key) =
        snapshot_envelope.payload.host_encryption_public_key.clone()
    {
        host_encryption_public_key = next_host_encryption_public_key;
    }

    let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(snapshot_envelope.payload)));
}
```

Adjust type names and field names to match the current code. Do not force this exact shape if the current code uses a different envelope type. Preserve existing successful behavior.

### Important

Do not replace the bug with another fallback like:

```rust
let snapshot_sequence = envelope.server_sequence.unwrap_or(last_seen_server_sequence.unwrap_or(0));
```

That is also wrong. Missing required sequence is malformed input. Warn and drop.

### Acceptance

- No direct assignment remains:

```bash
rg -n "last_seen_server_sequence = envelope\.server_sequence" src-tauri/src/networking/runtime/client.rs
```

- Missing snapshot `server_sequence` emits `ProtocolWarning`.
- Missing snapshot `server_sequence` preserves previous `last_seen_server_sequence`.
- Missing snapshot `server_sequence` does not emit `ClientRuntimeEvent::Snapshot`.
- Present snapshot `server_sequence` updates `last_seen_server_sequence` after validation.

---

## P0.2 — Make sequence-preservation tests cover production-used logic

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- tests under `src-tauri/src/networking/runtime/tests/**` if current tests live there

### Problem

Fix 10 added helper-level tests for live public-event sequence preservation. That was better than no test, but the helper was test-only. The test did not prove the production read-loop uses the same ordering logic.

Fix 11 should make the sequence validation helpers production-used and test those helpers. If a near-runtime read-loop test is practical, add it too.

### Required change

Use production helpers for required sequence validation:

- `required_server_sequence_or_warn(...)`
- `public_event_server_sequence_or_warn(...)`
- `snapshot_server_sequence_or_warn(...)`

If a helper mutates `last_seen_server_sequence`, it must be production-used. If production requires stale-resync branching, do not oversimplify it into a helper that cannot represent resync. In that case, test the smaller production-used validators and add a test-specific helper only for the last-seen preservation property with a clear comment.

### Tests to add/update

Keep or update the existing live public-event sequence tests so they still verify:

```rust
#[test]
fn missing_public_event_sequence_warns_and_preserves_last_seen_sequence() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut warning_counts = std::collections::BTreeMap::new();
    let mut last_seen_server_sequence = Some(10);

    let result = validate_live_public_event_sequence_for_test_or_production_helper(
        &sender,
        &mut warning_counts,
        "player-1",
        &mut last_seen_server_sequence,
        None,
    );

    assert_eq!(result, None);
    assert_eq!(last_seen_server_sequence, Some(10));

    let warning = receiver.try_recv().expect("expected protocol warning");
    assert!(matches!(
        warning,
        ClientRuntimeEvent::ProtocolWarning { reason, .. }
            if reason.contains("missing server sequence")
    ));

    assert!(
        receiver.try_recv().is_err(),
        "malformed public event should not emit additional events"
    );
}
```

Add snapshot equivalent:

```rust
#[test]
fn missing_snapshot_sequence_warns_and_preserves_last_seen_sequence() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut warning_counts = std::collections::BTreeMap::new();
    let mut last_seen_server_sequence = Some(42);

    let sequence = snapshot_server_sequence_or_warn(
        &sender,
        &mut warning_counts,
        "player-1",
        None,
    );

    if let Some(sequence) = sequence {
        last_seen_server_sequence = Some(sequence);
    }

    assert_eq!(sequence, None);
    assert_eq!(last_seen_server_sequence, Some(42));

    let warning = receiver.try_recv().expect("expected protocol warning");
    assert!(matches!(
        warning,
        ClientRuntimeEvent::ProtocolWarning { reason, .. }
            if reason.contains("snapshot envelope missing server sequence")
    ));

    assert!(
        receiver.try_recv().is_err(),
        "malformed snapshot should not emit additional events"
    );
}
```

Add positive snapshot test:

```rust
#[test]
fn present_snapshot_sequence_updates_last_seen_sequence_after_validation() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut warning_counts = std::collections::BTreeMap::new();
    let mut last_seen_server_sequence = Some(42);

    let sequence = snapshot_server_sequence_or_warn(
        &sender,
        &mut warning_counts,
        "player-1",
        Some(43),
    );

    if let Some(sequence) = sequence {
        last_seen_server_sequence = Some(sequence);
    }

    assert_eq!(sequence, Some(43));
    assert_eq!(last_seen_server_sequence, Some(43));
    assert!(receiver.try_recv().is_err(), "valid sequence should not warn");
}
```

### Acceptance

- Tests cover missing live public-event sequence preservation.
- Tests cover missing snapshot sequence preservation.
- Tests cover present snapshot sequence update after validation.
- Tests use production-used sequence validators where practical.
- Tests do not rely on fallback-to-zero behavior.

---

## P0.3 — Re-run previous protocol and runtime hardening audits

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

## P1.2 — Update `memory.md` honestly after Fix 11 validation

**Files:**

- `memory.md`

### Problem

`memory.md` must not claim commands passed unless they actually passed in the current implementation environment.

### Required change

After completing Fix 11 and running validation, add a new entry using the project timestamp convention:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

Suggested entry:

```md
- <timestamp> — Fix 11 completed: client snapshot-event handling now validates required `server_sequence` before mutating `last_seen_server_sequence`; missing snapshot sequence emits `ProtocolWarning`, drops the malformed snapshot, and preserves the previous last-seen sequence. Live public-event sequence hardening from Fix 10 remains intact. Validation run in this environment: <exact commands actually run and pass/fail status>. Previous hardening remains intact: no test-noise regression, no production browser mock/probe bundle, no empty-AAD fallback, no empty NPC hole-card fallback, no raw client runtime `thread::spawn`, no direct assignment from `envelope.server_sequence` to `last_seen_server_sequence`. `poker-core` remains platform-neutral. No Android app, Tauri Mobile path, FFI crate, or networking-in-core was added.
```

### Acceptance

- `memory.md` lists exact commands actually run.
- No false completion claims.
- Android/core architecture decision remains recorded.

---

## P2.1 — Silent-failure audit for touched runtime code

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- tests touched by Fix 11

### Required audit

For code touched by Fix 11, run:

```bash
rg -n "let _ =|\.ok\(\)|unwrap_or\(|unwrap_or_else\(|unwrap_or_default\(|thread::spawn|continue;|return;" \
  src-tauri/src/networking/runtime/client.rs
```

For every hit in touched production code:

- convert real silent failures to structured errors/diagnostics;
- add a short comment for intentional best-effort cleanup;
- leave harmless presentation-only/test-only defaults alone;
- do not rewrite unrelated stable code just to reduce grep output.

### Specific warning

Do not replace the snapshot sequence bug with another fallback like:

```rust
let snapshot_sequence = envelope.server_sequence.unwrap_or(last_seen_server_sequence.unwrap_or(0));
```

That is wrong. Missing required sequence is malformed input. Warn and drop.

### Acceptance

- No newly touched runtime code contains unexplained silent failure behavior.
- Any ignored error has a short comment explaining why it is safe.
- No new fallback invents authoritative protocol ordering state.

---

## P2.2 — Do not start Android implementation in Fix 11

**Files:**

- docs only, if needed

### Required rule

Do not add:

- Android Gradle project files;
- Kotlin source files;
- Tauri Mobile configuration;
- `poker-android-ffi` crate;
- networking inside `poker-core`.

Fix 11 is protocol hardening only.

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

## Definition of done

- [ ] Snapshot-event required `server_sequence` is validated before `last_seen_server_sequence` mutation.
- [ ] Missing snapshot sequence emits `ProtocolWarning`.
- [ ] Missing snapshot sequence drops the malformed snapshot event.
- [ ] Missing snapshot sequence preserves the previous `last_seen_server_sequence`.
- [ ] Present snapshot sequence updates `last_seen_server_sequence` only after validation.
- [ ] Live public-event sequence hardening from Fix 10 remains intact.
- [ ] No direct assignment remains from `envelope.server_sequence` to `last_seen_server_sequence`.
- [ ] No missing-sequence path defaults to `0` or to prior sequence.
- [ ] Tests cover snapshot sequence warning and last-seen preservation.
- [ ] Previous hardening remains fixed: no test noise, no production mock chunk, no empty-AAD fallback, no empty NPC hole-card fallback, no raw client runtime `thread::spawn`.
- [ ] `poker-core` purity audit remains clean.
- [ ] `memory.md` accurately records commands actually run.
- [ ] No new hidden fallbacks or silent failures are introduced.
- [ ] No Android app, Tauri Mobile path, Android FFI crate, or networking-in-core is added.
- [ ] All final validation commands pass in an environment with Node 24 and Rust installed.
