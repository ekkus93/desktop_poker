# Desktop Poker Fix 16 Spec — Final Event Send Consistency and Ledger Closure

## Purpose

Fix 16 is a tiny final cleanup pass after Fix 15. The goal is to remove the last known inconsistency in `ClientRuntime` event delivery and ensure `memory.md` accurately describes what was changed and validated.

Do **not** add features. Do **not** start Android implementation. Do **not** refactor the networking runtime beyond the explicitly listed cleanup.

## Background

Fix 15 introduced `send_runtime_event_best_effort(...)` in `src-tauri/src/networking/runtime/client.rs` so intentional `ClientRuntimeEvent` send failures are explicit instead of appearing as unexplained `let _ = sender.send(...)` silent failures.

Review of Fix 15 found one remaining raw event send in the live `SNAPSHOT_EVENT` branch:

```rust
// Best-effort event delivery: receiver may be gone during shutdown.
let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(envelope.payload)));
```

This is not a severe runtime bug because it has an explanatory comment, but it is inconsistent with the new helper and with the Fix 15 ledger wording that claimed all ignored `sender.send(...)` call sites were replaced.

Fix 16 should close that gap.

## Non-goals

- Do not change poker rules, tournament rules, NPC decision logic, or UI behavior.
- Do not alter protocol semantics beyond replacing the raw event-channel send with the existing helper.
- Do not change reconnect command-stream installation behavior except to verify it remains strict.
- Do not add Android Gradle files, Kotlin files, Tauri Mobile config, `poker-android-ffi`, or networking inside `poker-core`.
- Do not perform broad grep-driven rewrites of unrelated runtime code.

## Required behavior

After Fix 16:

1. Intentional ignored `ClientRuntimeEvent` sends in `client.rs` should use `send_runtime_event_best_effort(...)`, except for cases with an equally explicit comment where conversion would make code materially worse.
2. The live snapshot branch should use `send_runtime_event_best_effort(...)` for `ClientRuntimeEvent::Snapshot` delivery.
3. `memory.md` should not claim all raw sends were replaced unless that is actually true.
4. Reconnect snapshot acceptance must still require successful command-stream clone and command-connection lock before emitting the accepted snapshot.
5. Public/snapshot sequence hardening must remain intact.
6. `poker-core` must remain platform-neutral.
7. The final validation commands must be run in the implementation environment and recorded honestly.

## Expected implementation summary

The likely production code change is tiny. Replace this live snapshot send shape:

```rust
// Best-effort event delivery: receiver may be gone during shutdown.
let _ = sender.send(ClientRuntimeEvent::Snapshot(Box::new(envelope.payload)));
```

with:

```rust
send_runtime_event_best_effort(
    &sender,
    ClientRuntimeEvent::Snapshot(Box::new(envelope.payload)),
);
```

Then run:

```bash
rg -n "let _ = sender\.send" src-tauri/src/networking/runtime/client.rs
```

Expected: the only hit should be inside `send_runtime_event_best_effort(...)` itself. If another raw event send remains, either convert it or document why it is intentionally not converted.

## Safety requirements

This helper only documents best-effort event-channel delivery. It must not be used to hide real runtime precondition failures.

Do not use the helper as a substitute for handling:

- stream clone failures;
- command-connection lock failures on required reconnect command-stream installation;
- crypto/decryption failures;
- malformed required protocol fields;
- stale or missing required sequence handling;
- host/client state mutation failures;
- file/network/socket operations that should be surfaced as `Result`, `SafeError`, `ProtocolWarning`, or another structured diagnostic.

## Validation expectation

Run the full validation set in an environment with Node 24 and Rust installed:

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

Also run the focused hardening audits listed in the TODO.

If a command is not run, `memory.md` must say it was not run. Do not imply it passed.
