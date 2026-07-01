# Desktop Poker Fix 10 TODO — Preserve Client Sequence State on Malformed Public Events

This TODO is intentionally explicit for Claude Code. Do tasks in priority order. Prefer small, test-backed patches. Do not introduce hidden fallback behavior to make tests pass.

Fix 10 is a narrow protocol-hardening pass. Do not add features. Do not start Android implementation.

---

## P0.1 — Move public-event `server_sequence` validation before stale-check and last-seen mutation

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- client runtime/protocol tests under `src-tauri/src/networking/runtime/tests/**` if applicable

### Problem

Fix 9 removed the obvious fallback:

```rust
server_sequence: envelope.server_sequence.unwrap_or_default()
```

But the public-event read path still validates missing `server_sequence` too late. The code can do something equivalent to:

```rust
if is_stale_server_sequence(last_seen_server_sequence, envelope.server_sequence) {
    // resync
    continue;
}

last_seen_server_sequence = envelope.server_sequence;

let Some(server_sequence) = public_event_server_sequence_or_warn(
    &sender,
    &mut protocol_warning_counts,
    &player_id,
    envelope.server_sequence,
) else {
    continue;
};
```

For a malformed live public event with `server_sequence == None`, that clears `last_seen_server_sequence` before the event is dropped. That is a quiet protocol-ordering bug.

### Required behavior

For host-originated live public events that require sequencing:

- validate `server_sequence` before stale checking;
- emit `ClientRuntimeEvent::ProtocolWarning` if it is missing;
- drop the malformed event;
- do **not** mutate `last_seen_server_sequence`;
- do **not** emit `ClientRuntimeEvent::PublicEvent`.

### Required code change

Find the public-event branch in `src-tauri/src/networking/runtime/client.rs`.

Move this validation:

```rust
let Some(server_sequence) = public_event_server_sequence_or_warn(
    &sender,
    &mut protocol_warning_counts,
    &player_id,
    envelope.server_sequence,
) else {
    continue;
};
```

so it occurs before:

```rust
is_stale_server_sequence(...)
```

and before any assignment like:

```rust
last_seen_server_sequence = envelope.server_sequence;
```

Suggested target shape:

```rust
let Some(server_sequence) = public_event_server_sequence_or_warn(
    &sender,
    &mut protocol_warning_counts,
    &player_id,
    envelope.server_sequence,
) else {
    // Malformed live public event. Do not update last_seen_server_sequence.
    continue;
};

if is_stale_server_sequence(last_seen_server_sequence, Some(server_sequence)) {
    let _ = sender.send(ClientRuntimeEvent::ResyncRequested {
        player_id: player_id.clone(),
        last_seen_server_sequence: last_seen_server_sequence.unwrap_or(0),
    });

    match request_resync_snapshot(
        &crypto_provider,
        &mut stream,
        &join_payload,
        &player_id,
        &reconnect_identity_for_thread,
        last_seen_server_sequence.unwrap_or(0),
        &mut next_counter,
    ) {
        Ok(snapshot_envelope) => {
            reconnect_token = snapshot_envelope.payload.reconnect_token.clone();
            last_seen_server_sequence = snapshot_envelope.server_sequence;

            if let Some(next_host_encryption_public_key) =
                snapshot_envelope.payload.host_encryption_public_key.clone()
            {
                host_encryption_public_key = next_host_encryption_public_key;
            }

            let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(
                snapshot_envelope.payload,
            )));
        }
        Err(error) => {
            let _ = sender.send(ClientRuntimeEvent::SafeError {
                player_id: player_id.clone(),
                message: error.to_string(),
            });
            break;
        }
    }

    continue;
}

last_seen_server_sequence = Some(server_sequence);

let _ = sender.send(ClientRuntimeEvent::PublicEvent {
    player_id: player_id.clone(),
    message_type: envelope.message_type,
    server_sequence,
    payload: envelope.payload,
});
```

Adjust field names to match the current `ClientRuntimeEvent::PublicEvent` shape. The important part is the order.

### Message-type scope

Only require `server_sequence` for host-originated live public-event envelopes where the protocol expects one.

Do not require `server_sequence` for messages that are legitimately unsequenced, such as client-to-host messages, local-only messages, or protocol-error messages if the current protocol treats them as unsequenced.

If the current code has a `match envelope.message_type` branch for live public events, keep the requirement inside that branch.

### Acceptance

- Missing public-event `server_sequence` is validated before stale checking.
- Missing public-event `server_sequence` is validated before `last_seen_server_sequence` mutation.
- Missing public-event `server_sequence` does not clear/overwrite `last_seen_server_sequence`.
- Malformed public event is dropped with `ProtocolWarning`.
- No `PublicEvent` is emitted for the malformed frame.

---

## P0.2 — Add regression coverage that preserves prior sequence state

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- tests under `src-tauri/src/networking/runtime/tests/**` or inline `#[cfg(test)]` module if that is where current client tests live

### Problem

Fix 9 helper tests can prove that a warning is emitted, but they did not prove that the read-loop ordering state is preserved. That allowed the production bug where `last_seen_server_sequence` was cleared before the warning/drop path.

### Required test

Add a test that proves missing public-event sequence:

- emits `ProtocolWarning`;
- returns no validated sequence / drops the frame;
- preserves the previous `last_seen_server_sequence`.

If a full TCP/read-loop test is practical, use it. If not, extract a small helper that includes the last-seen update behavior and test that helper.

### Suggested helper

Use a helper like this if it fits the current code:

```rust
fn validate_live_public_event_sequence(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    last_seen_server_sequence: &mut Option<u64>,
    envelope_sequence: Option<u64>,
) -> Option<u64> {
    let sequence = public_event_server_sequence_or_warn(
        sender,
        counts,
        player_id,
        envelope_sequence,
    )?;

    if is_stale_server_sequence(*last_seen_server_sequence, Some(sequence)) {
        return None;
    }

    *last_seen_server_sequence = Some(sequence);
    Some(sequence)
}
```

If this helper is production-visible only because tests need it, keep it private and test it from an inline `#[cfg(test)]` module in `client.rs`.

If a stale sequence should trigger resync rather than simply return `None`, do not use this exact helper in production. Instead, extract only the missing-sequence validation and add a test-specific helper that mirrors production ordering. The required tested property is still: missing sequence does not mutate last-seen state.

### Suggested test

```rust
#[test]
fn missing_public_event_sequence_warns_and_preserves_last_seen_sequence() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut warning_counts = std::collections::BTreeMap::new();
    let mut last_seen_server_sequence = Some(10);

    let result = validate_live_public_event_sequence(
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

Also add a positive-path test if the helper is new:

```rust
#[test]
fn present_public_event_sequence_updates_last_seen_sequence() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut warning_counts = std::collections::BTreeMap::new();
    let mut last_seen_server_sequence = Some(10);

    let result = validate_live_public_event_sequence(
        &sender,
        &mut warning_counts,
        "player-1",
        &mut last_seen_server_sequence,
        Some(11),
    );

    assert_eq!(result, Some(11));
    assert_eq!(last_seen_server_sequence, Some(11));
    assert!(receiver.try_recv().is_err(), "valid sequence should not warn");
}
```

### Acceptance

- At least one test fails on the Fix 9 bug and passes after Fix 10.
- Test asserts the previous last-seen sequence is preserved when envelope sequence is missing.
- Test asserts `ProtocolWarning` is emitted.
- Test asserts no extra event is emitted by the missing-sequence helper path.

---

## P0.3 — Re-run previous hardening audits

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
- No public-event missing-sequence default to `0`.
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

## P1.2 — Update `memory.md` honestly after Fix 10 validation

**Files:**

- `memory.md`

### Problem

`memory.md` must not claim commands passed unless they actually passed in the current implementation environment.

### Required change

After completing Fix 10 and running validation, add a new entry using the project timestamp convention:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

Suggested entry:

```md
- <timestamp> — Fix 10 completed: client public-event handling now validates required `server_sequence` before stale-sequence checks and before mutating `last_seen_server_sequence`; missing public-event sequence emits `ProtocolWarning`, drops the frame, and preserves the previous last-seen sequence. Validation run in this environment: <exact commands actually run and pass/fail status>. Previous hardening remains intact: no test-noise regression, no production browser mock/probe bundle, no empty-AAD fallback, no empty NPC hole-card fallback, no raw client runtime `thread::spawn`. `poker-core` remains platform-neutral. No Android app, Tauri Mobile path, FFI crate, or networking-in-core was added.
```

### Acceptance

- `memory.md` lists exact commands actually run.
- No false completion claims.
- Android/core architecture decision remains recorded.

---

## P2.1 — Silent-failure audit for touched runtime code

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- tests touched by Fix 10

### Required audit

For code touched by Fix 10, run:

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

Do not replace the `server_sequence` bug with another fallback like:

```rust
let server_sequence = envelope.server_sequence.unwrap_or(last_seen_server_sequence.unwrap_or(0));
```

That is also wrong. Missing sequence is malformed input. Warn and drop.

### Acceptance

- No newly touched runtime code contains unexplained silent failure behavior.
- Any ignored error has a short comment explaining why it is safe.
- No new fallback invents authoritative protocol ordering state.

---

## P2.2 — Do not start Android implementation in Fix 10

**Files:**

- docs only, if needed

### Required rule

Do not add:

- Android Gradle project files;
- Kotlin source files;
- Tauri Mobile configuration;
- `poker-android-ffi` crate;
- networking inside `poker-core`.

Fix 10 is protocol hardening only.

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

- [ ] Public-event required `server_sequence` is validated before stale checking.
- [ ] Public-event required `server_sequence` is validated before `last_seen_server_sequence` mutation.
- [ ] Missing public-event sequence emits `ProtocolWarning`.
- [ ] Missing public-event sequence drops the malformed event.
- [ ] Missing public-event sequence preserves the previous `last_seen_server_sequence`.
- [ ] Tests cover warning emission and last-seen preservation.
- [ ] No missing-sequence path defaults to `0` or to prior sequence.
- [ ] Previous hardening remains fixed: no test noise, no production mock chunk, no empty-AAD fallback, no empty NPC hole-card fallback, no raw client runtime `thread::spawn`.
- [ ] `poker-core` purity audit remains clean.
- [ ] `memory.md` accurately records commands actually run.
- [ ] No new hidden fallbacks or silent failures are introduced.
- [ ] No Android app, Tauri Mobile path, Android FFI crate, or networking-in-core is added.
- [ ] All final validation commands pass in an environment with Node 24 and Rust installed.
