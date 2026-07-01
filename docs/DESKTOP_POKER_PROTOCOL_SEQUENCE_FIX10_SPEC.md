# Desktop Poker Fix 10 Spec — Public Event Sequence Ordering Hardening

## Purpose

Fix 10 is a narrow stabilization pass. It exists to correct one remaining protocol-ordering bug found after Fix 9:

> `ClientRuntime` now warns and drops public events whose signed public-event envelope is missing `server_sequence`, but the validation happens too late. The malformed frame can still overwrite `last_seen_server_sequence` before being dropped.

That is not acceptable. A malformed public event must not mutate client protocol ordering state.

Fix 10 must not start Android implementation, add new UI features, or broaden the architecture. It should only harden the client public-event read path, add regression coverage, re-run prior audits, and update `memory.md` honestly.

## Current architecture baseline

The current intended architecture remains:

- `crates/poker-core` owns platform-neutral poker rules, state transitions, legal-action validation, dealing/shuffling, showdown/settlement, and public/private projection.
- `src-tauri` is the desktop adapter crate.
- Android will eventually be native Kotlin/Compose plus Rust bindings.
- Kotlin/desktop adapters own networking/session transport.
- Networking must not move into `poker-core`.

Fix 10 should preserve that boundary.

## Problem statement

In the client runtime signed public-envelope path, Fix 9 removed the direct defaulting behavior:

```rust
server_sequence: envelope.server_sequence.unwrap_or_default()
```

However, the read loop still has logic shaped like this:

```rust
if is_stale_server_sequence(last_seen_server_sequence, envelope.server_sequence) {
    // request resync
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

For a malformed signed public event with `server_sequence == None`, this does the wrong thing:

1. The stale-sequence check sees `None` and does not reject it.
2. `last_seen_server_sequence` is set to `None`.
3. The helper emits a warning and drops the frame.

The event is dropped, but the client has already lost its last known server sequence. That can let later stale events bypass ordering checks because the client no longer remembers the previous sequence.

## Required semantic

For host-originated live public events where the protocol requires `server_sequence`:

- Missing `server_sequence` is malformed protocol input.
- The client must emit `ClientRuntimeEvent::ProtocolWarning`.
- The malformed public event must be dropped.
- `last_seen_server_sequence` must remain unchanged.
- No `PublicEvent` must be emitted for that malformed frame.
- No public snapshot/state/order tracking should be mutated by that frame.

## Scope

### In scope

- `src-tauri/src/networking/runtime/client.rs` public-event sequence validation order.
- Tests for missing public-event `server_sequence` preserving prior sequence state.
- Focused audits proving no sequence defaulting remains.
- `memory.md` update with exact commands run.

### Out of scope

- Android implementation.
- Tauri Mobile.
- `poker-android-ffi` crate.
- Moving networking into `poker-core`.
- Broad rewrite of the client runtime.
- Changing public-event protocol format.
- Changing signing/encryption semantics except for validation order.

## Design requirement

Validate the required public-event sequence before any stale-sequence check or last-seen mutation.

The correct control flow is:

1. Decode and verify signed public envelope.
2. Determine whether the message type is a live host-originated public event that requires `server_sequence`.
3. If it requires `server_sequence`, validate that it exists.
4. If missing, emit warning and `continue` before stale checking and before mutating `last_seen_server_sequence`.
5. If present, use the validated `u64` for stale checking, last-seen update, and `PublicEvent` emission.

## Suggested helper boundary

Prefer a small helper so the ordering rule is testable without a full TCP runtime test.

Suggested helper:

```rust
fn public_event_server_sequence_or_warn(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    server_sequence: Option<u64>,
) -> Option<u64> {
    match server_sequence {
        Some(sequence) => Some(sequence),
        None => {
            emit_protocol_warning(
                sender,
                counts,
                player_id,
                "public event envelope missing server sequence",
            );
            None
        }
    }
}
```

Then add a higher-level helper or code pattern that preserves last-seen state:

```rust
fn required_public_event_sequence_for_dispatch(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    last_seen_server_sequence: u64,
    envelope_server_sequence: Option<u64>,
) -> Option<u64> {
    let Some(server_sequence) = public_event_server_sequence_or_warn(
        sender,
        counts,
        player_id,
        envelope_server_sequence,
    ) else {
        // Important: caller must not mutate last_seen_server_sequence on None.
        return None;
    };

    Some(server_sequence)
}
```

A helper that takes `&mut Option<u64>` is also acceptable if the implementation is explicit that the value is updated only after validation and stale checks.

## Required production code behavior

The public-event path should look conceptually like this:

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

Adjust field names and payload shape to match the existing code. The critical part is the order:

1. validate sequence;
2. stale-check using validated sequence;
3. then update `last_seen_server_sequence`;
4. then emit the public event.

## Message-type rule

Only require `server_sequence` for host-originated live public-event envelopes where the protocol expects ordering.

Do not accidentally require `server_sequence` for:

- local-only messages;
- resync request messages;
- client-to-host action messages;
- protocol error messages if they are intentionally unsequenced;
- any other message type that the protocol explicitly treats as unsequenced.

If the existing code already has a match branch for live public events, put validation inside that branch rather than adding a global requirement.

## Tests required

At minimum, add a focused test proving:

- missing public-event `server_sequence` emits `ClientRuntimeEvent::ProtocolWarning`;
- no `ClientRuntimeEvent::PublicEvent` is emitted;
- `last_seen_server_sequence` remains unchanged.

If a full read-loop test is too expensive, extract the sequencing decision into a small helper and test that helper. The helper test must cover prior-sequence preservation, not only warning emission.

Example unit-level intent:

```rust
#[test]
fn missing_public_event_sequence_warns_and_preserves_last_seen_sequence() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut warning_counts = std::collections::BTreeMap::new();
    let mut last_seen_server_sequence = Some(10);

    let result = validate_live_public_event_sequence_for_test(
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

    assert!(receiver.try_recv().is_err(), "no public event should be emitted");
}
```

Example helper shape:

```rust
fn validate_live_public_event_sequence_for_test(
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

Use this exact helper only if it matches the existing runtime design. The implementation may use a differently named helper, but the behavior must be the same.

## Acceptance criteria

Fix 10 is complete only when:

- No public-event path uses `server_sequence.unwrap_or_default()`.
- Missing required public-event `server_sequence` emits `ProtocolWarning`.
- The malformed public event is dropped.
- Missing public-event sequence does **not** clear or mutate `last_seen_server_sequence`.
- Tests cover warning emission and prior-sequence preservation.
- Previous hardening audits still pass.
- `poker-core` remains platform-neutral.
- `memory.md` records exact validation commands actually run.
- No Android implementation is started.

## Validation commands

Run from the repo root with Node 24 and Rust available:

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
- `src-tauri/src/tournament` may not exist after the `poker-core` extraction; that is okay.
- Desktop launch/package commands may still use `--manifest-path src-tauri/Cargo.toml` only when explicitly documented as launch/package commands, not validation.
