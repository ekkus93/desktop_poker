# Desktop Poker Fix 15 Spec — Final Event Delivery and Validation Closure

## Purpose

Fix 15 is a final optional hardening pass after Fix 14. The code is already in a good stabilization state. This pass should not introduce new architecture, new gameplay behavior, Android implementation, UI redesign, protocol changes, or broad refactors.

The goal is to make intentionally ignored runtime event-send failures explicit and auditable, then run the complete validation suite in the real project environment with Node 24 and Rust installed.

## Background

The latest review found no serious Fix 14 blocker. The remaining concern is that `src-tauri/src/networking/runtime/client.rs` still contains many `let _ = sender.send(...)` calls. Most are probably acceptable: runtime event delivery is best-effort when the receiver may have gone away during shutdown, fatal error handling, disconnect, or test teardown.

However, bare `let _ = ...` calls are hard to audit. They look similar to the silent failures that previously hid real bugs, such as accepting a reconnect snapshot without installing the command stream, defaulting missing protocol sequence numbers, or swallowing state validation defects.

Fix 15 should distinguish intentional best-effort event delivery from real ignored errors.

## Non-goals

Do not do any of the following:

- Do not add Android Gradle files, Kotlin source, Tauri Mobile configuration, or an Android FFI crate.
- Do not add networking to `poker-core`.
- Do not change the poker protocol message schema.
- Do not change public/snapshot sequence ordering behavior, except to preserve the existing hardening.
- Do not change reconnect semantics except preserving the existing strict command-stream install requirement.
- Do not convert event-channel disconnects into user-facing errors everywhere.
- Do not broadly refactor unrelated runtime modules.
- Do not silence lint/test failures.
- Do not add panics, `unwrap()`, or `expect()` around runtime failure paths.

## Required behavior

### Runtime event sending

`ClientRuntimeEvent` delivery from the client runtime thread should be explicitly best-effort where appropriate. If the frontend/runtime receiver is gone, the runtime cannot reliably deliver another event to report that delivery failed. In these cases, ignored `send` failures are acceptable only when the code makes that intent obvious.

The preferred approach is to add a small helper in `client.rs`:

```rust
fn send_runtime_event_best_effort(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    event: ClientRuntimeEvent,
) {
    // Best-effort event delivery: the receiver may be gone during shutdown,
    // fatal error handling, disconnect, or test teardown. There is no reliable
    // secondary channel to report failure to deliver this event.
    let _ = sender.send(event);
}
```

Use this helper for intentionally ignored `ClientRuntimeEvent` sends in `src-tauri/src/networking/runtime/client.rs`.

This helper is not a license to hide real errors. It should only replace event-channel sends where failure means the receiver is gone. It must not be used for:

- command-stream installation failure;
- protocol validation failure that should emit a warning before continuing;
- cryptographic validation failure;
- snapshot acceptance preconditions;
- host/client authoritative state mutation;
- file/network/socket operations that have a recoverable caller.

### Reconnect command stream invariant

Preserve the Fix 13/Fix 14 reconnect invariant:

- A reconnect snapshot must not be emitted unless command-stream clone succeeds.
- A reconnect snapshot must not be emitted unless command-connection lock succeeds.
- Clone or lock failure must emit `ClientRuntimeEvent::SafeError` and stop/break the runtime path.
- The code must not regress to nested silent `if let Ok(...)` stream-install behavior.

### Sequence handling invariants

Preserve the Fix 9–12 sequence hardening:

- Missing live public-event `server_sequence` warns and drops.
- Missing live snapshot `server_sequence` warns and drops.
- Stale live snapshot `server_sequence` warns and drops.
- No missing/stale sequence path defaults to `0` or the previous sequence.
- No direct assignment remains from `envelope.server_sequence` to `last_seen_server_sequence`.
- `ProtocolError` remains intentionally unsequenced unless a future protocol design says otherwise.

### `poker-core` boundary

`poker-core` must remain platform-neutral:

- no Tauri dependency;
- no Android dependency;
- no sockets/networking;
- no keychain/provider/LLM logic;
- no app-data-path logic;
- no process spawning;
- no thread spawning.

Expected broad-grep false positives such as `EngineCommand` must be recorded honestly rather than treated as failures or renamed away.

### Ledger honesty

`memory.md` must record exactly what was run in the implementation environment. If a command was not run, say so. If a grep returns expected false positives, say so. Do not claim a broad grep returned zero hits unless it actually did.

## Acceptance criteria

Fix 15 is done when:

- Intentional ignored `ClientRuntimeEvent` sends in `client.rs` use a clearly named best-effort helper or have equally clear comments.
- No real runtime failure is newly hidden behind the best-effort helper.
- Reconnect snapshot acceptance still requires successful command-stream clone and command-connection lock.
- Public/snapshot sequence hardening remains intact.
- Previous dangerous fallbacks remain absent.
- `poker-core` remains platform-neutral.
- `memory.md` records the exact commands actually run.
- No Android implementation is added.
- Full validation passes in an environment with Node 24 and Rust installed, or any command not run is explicitly recorded as not run.

