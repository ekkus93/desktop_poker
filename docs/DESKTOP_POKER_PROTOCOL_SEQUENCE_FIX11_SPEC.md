# Desktop Poker Fix 11 Spec — Finish Sequenced Envelope Validation

## Purpose

Fix 11 is a narrow protocol-hardening cleanup after Fix 10. Fix 10 correctly moved live public-event `server_sequence` validation before stale-checking and before `last_seen_server_sequence` mutation, but review found one remaining sequencing-state problem in `src-tauri/src/networking/runtime/client.rs`: the `SNAPSHOT_EVENT` branch can still assign `envelope.server_sequence` directly into `last_seen_server_sequence`.

That direct assignment is not acceptable because a malformed snapshot event with `server_sequence == None` can silently clear the client's last-seen sequence state. The client must not convert malformed sequenced host input into “unknown sequence state.” Missing required sequence metadata must emit `ClientRuntimeEvent::ProtocolWarning`, drop the malformed frame, and preserve the previous `last_seen_server_sequence`.

Fix 11 must stay small. Do not add Android implementation, do not change poker rules, do not change networking architecture, and do not start another large refactor.

## Current architecture assumptions

- `crates/poker-core` owns platform-neutral poker rules, state transitions, projection, and serialization.
- `src-tauri` is the desktop adapter and owns Tauri commands, host/client runtime, provider/keychain integration, desktop session state, and desktop networking.
- Android will be native Kotlin/Compose + Rust bindings later. Kotlin owns Android networking/session transport. `poker-core` must not gain networking/platform dependencies.
- `ClientRuntime` receives host-originated sequenced envelopes and maintains `last_seen_server_sequence` for stale-event detection and resync.

## Problem statement

The client runtime has two categories of host messages relevant to sequence state:

1. Live public events, such as action-window, action-committed, street-revealed, elimination, hand-result, tournament-started, and tournament-complete events.
2. Snapshot events, such as resync or full table snapshot updates.

Both categories are host-originated sequenced input. If a required `server_sequence` is missing, the input is malformed. The client must not:

- default the sequence to `0`;
- default the sequence to the prior sequence;
- assign `None` into `last_seen_server_sequence`;
- run stale-sequence checks against a missing sequence as if that were a valid state;
- emit public/snapshot runtime events for the malformed frame.

Fix 10 addressed the live public-event branch, but the snapshot branch still needs the same fail-loud treatment.

## Required behavior

### Live public events

Fix 11 must preserve the Fix 10 behavior:

- Validate required `server_sequence` before stale checking.
- If missing, emit `ClientRuntimeEvent::ProtocolWarning`.
- Drop the malformed frame.
- Preserve previous `last_seen_server_sequence`.
- Do not emit `ClientRuntimeEvent::PublicEvent`.

### Snapshot events

Fix 11 must add equivalent protection for snapshot events:

- Validate required `server_sequence` before assigning to `last_seen_server_sequence`.
- If missing, emit `ClientRuntimeEvent::ProtocolWarning`.
- Drop the malformed snapshot frame.
- Preserve previous `last_seen_server_sequence`.
- Do not emit `ClientRuntimeEvent::Snapshot` for the malformed frame.
- Do not update reconnect token or host encryption public key from the malformed frame.

### ProtocolError messages

`ProtocolError` messages remain intentionally unsequenced. They should not update `last_seen_server_sequence` and should not trigger stale-sequence resync logic merely because they happen to carry a sequence field.

## Implementation direction

Prefer extracting a small production-used helper rather than adding another test-only helper. The helper should be private to `client.rs` unless there is a clear need to expose it.

A good shape is a generic required-sequence validator:

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

Then use this helper for both live public events and snapshot events.

If the existing `public_event_server_sequence_or_warn(...)` helper already exists, it may either remain as a thin wrapper around the generic helper or be replaced by the generic helper. Avoid duplicate logic.

## Snapshot branch target behavior

A snapshot event branch should look conceptually like this:

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

    let Some(snapshot_sequence) = required_server_sequence_or_warn(
        &sender,
        &mut protocol_warning_counts,
        &player_id,
        snapshot_envelope.server_sequence,
        "snapshot envelope missing server sequence",
    ) else {
        // Malformed sequenced host input. Do not mutate last_seen_server_sequence.
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

Adjust type names and field names to match the current code. The key invariant is ordering: validate sequence first, then mutate runtime state.

## Test requirements

Add tests that would catch the remaining Fix 10 gap.

At minimum, test the production-used helper that validates required sequence and preserves existing sequence state. Stronger is to add a near-runtime test for the snapshot branch.

Required test properties:

- Missing snapshot `server_sequence` emits `ProtocolWarning`.
- Missing snapshot `server_sequence` returns no validated sequence.
- Missing snapshot `server_sequence` does not mutate `last_seen_server_sequence`.
- Missing snapshot `server_sequence` does not emit `Snapshot`.
- Present snapshot `server_sequence` updates `last_seen_server_sequence` only after validation.

If a full runtime/read-loop test is too expensive, use a production-used helper and explain in a test comment why the helper is the seam.

## Non-goals

Fix 11 must not:

- add Android Gradle files;
- add Kotlin source;
- add Tauri Mobile config;
- add `poker-android-ffi`;
- move networking into `poker-core`;
- rewrite the protocol architecture;
- change poker rules;
- change table UI behavior;
- add broad new features.

## Acceptance criteria

Fix 11 is complete only when:

- no code directly assigns `last_seen_server_sequence = envelope.server_sequence`;
- live public events still validate required sequence before stale checking and before last-seen mutation;
- snapshot events validate required sequence before last-seen mutation;
- missing live public-event sequence emits `ProtocolWarning`, drops the event, and preserves prior last-seen sequence;
- missing snapshot-event sequence emits `ProtocolWarning`, drops the snapshot, and preserves prior last-seen sequence;
- no missing-sequence path defaults to `0` or to a prior sequence;
- tests cover the live public-event and snapshot-event sequence-preservation behavior or a documented production-used helper equivalent;
- previous hardening audits still pass;
- `poker-core` remains platform-neutral;
- `memory.md` accurately records the exact validation commands actually run;
- no Android implementation is started.
