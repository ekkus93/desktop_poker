# Desktop Poker Fix 7 Spec — Finalize Core Extraction Cleanup and Runtime Diagnostics

## Purpose

Fix 6 substantially improved the project: the production browser mock/probe issue was fixed, test stderr noise was eliminated, `poker-core` was extracted, and the Android/Kotlin architecture direction was documented. Fix 7 is a focused cleanup pass before treating the extracted `poker-core` baseline as stable.

This pass must **not** start the Android app, add Tauri Mobile, or move networking into `poker-core`.

The goals are:

1. Finish the remaining runtime diagnostic gaps from Fix 6.
2. Make validation docs and developer instructions match the new Cargo workspace.
3. Ensure host runtime health counters are visible to the desktop frontend.
4. Replace remaining raw thread-spawn paths that can panic.
5. Add stronger regression coverage for acting NPC missing/invalid hole-card state.
6. Re-validate that `poker-core` is pure and reusable by future Android/Kotlin bindings.

## Current known state from the Fix 6 review

The following items were already good in the reviewed Fix 6 snapshot:

- `npm test` passed cleanly with no window-persistence stderr noise.
- Production `dist` no longer contained `LayoutProbeApp` or `__DESKTOP_POKER_BROWSER_MOCKS__`.
- `associated_data_json().unwrap_or_default()` was gone from networking code.
- Acting NPC hole cards were validated in production code before rule-based or LLM-backed decisions.
- Full provider config save updated/deleted the secret before writing non-secret settings.
- `crates/poker-core` existed and contained pure domain/engine/tournament/projector logic.
- `docs/ANDROID_ARCHITECTURE.md` existed and correctly documented Kotlin-owned networking and Rust-owned rules/state/projection.
- No partial Android app or Tauri Mobile path had been added.

The following items still need work:

- `README.md` still contains old Rust validation commands using `--manifest-path src-tauri/Cargo.toml`.
- Frontend `HostRuntimeHealth` TypeScript types and debug UI do not expose all new Rust health counters.
- Some host-session authoritative-state lock failures still reject/disconnect/break without incrementing `HostRuntimeHealth.state_lock_error_count`.
- `ClientRuntime::connect` still uses raw `thread::spawn`, which can panic on thread-spawn failure.
- NPC missing-hole-card tests appear to exercise helper-level error recording more than the real NPC action path.
- Rust validation was not independently verified in the review environment because `cargo`/`rustc` were unavailable there.

## Architecture rules

### `poker-core`

`poker-core` must remain a platform-neutral Rust crate.

It may contain:

- domain types;
- engine/rules logic;
- tournament controller/state transitions;
- public/private projection;
- deterministic serialization;
- minimal facade types such as `PokerEngine`, `EngineCommand`, `EngineEvent`, and `PokerCoreError`;
- test helpers that do not rely on Tauri, networking, local OS app paths, keychains, LLM providers, or Android UI.

It must not contain:

- Tauri commands or Tauri runtime types;
- desktop window/persistence code;
- TCP/UDP/WebRTC/Bluetooth networking;
- `std::net` socket use;
- Android lifecycle/UI code;
- keychain/keystore/provider secret code;
- LLM clients;
- process spawning;
- platform app-data path decisions.

### Desktop Tauri crate

The Tauri crate is the desktop platform adapter. It may contain:

- Tauri commands;
- desktop app state;
- desktop networking/session runtime;
- desktop keychain/provider storage;
- NPC runtime orchestration;
- frontend bridge DTOs;
- debug inspector mapping.

It must import poker rules/state/projection from `poker_core` rather than duplicating or retaining stale copies.

### Android future direction

Android will be native Kotlin/Compose + Rust bindings. Kotlin owns networking/session transport. `poker-core` owns poker rules/state/projection only. Fix 7 must not implement Android or add a binding crate unless explicitly requested later.

## Runtime diagnostic policy

Important runtime failures must be visible, not silently swallowed.

For host/client runtime code, every important failure must do one of the following:

1. Return a real error to the caller.
2. Increment `HostRuntimeHealth` or another structured diagnostic.
3. Emit a structured runtime/debug event.
4. Be documented as safe best-effort cleanup with a short comment.

Examples of acceptable ignored errors:

```rust
// Best-effort shutdown wake-up: the listener may already be closed.
let _ = TcpStream::connect(self.listener_addr);

// Best-effort shutdown join: dropping the server should not panic if the worker panicked.
let _ = join_handle.join();
```

Examples of unacceptable quiet failures:

```rust
let _ = clients.lock().map(|mut clients| clients.remove(&player_id));
let _ = mark_participant_reconnect_eligible(&authoritative_state, &player_id);

let previous_state = authoritative_state
    .lock()
    .map(|s| s.clone())
    .unwrap_or_else(|_| state.clone());

thread::spawn(move || { ... });
```

## Host runtime health surface

If Rust adds a health counter, the TypeScript API model and debug UI should surface it unless there is a deliberate reason not to.

Required Rust health fields expected after Fix 6/Fix 7:

- `accept_error_count`
- `stream_timeout_error_count`
- `tick_advance_error_count`
- `publish_error_count`
- `state_lock_error_count`
- `stream_clone_error_count`
- `client_registry_error_count`
- `reconnect_mark_error_count`
- `snapshot_sync_error_count`
- `last_error`
- `last_successful_tick_ms`
- `last_successful_publish_ms`

The frontend should expose these as camelCase fields and show non-zero counts in the debug inspector.

## NPC decision integrity

An acting Texas Hold'em NPC must have exactly two hole cards before any rule-based or LLM-backed decision logic runs.

Missing or invalid acting NPC hole-card state is authoritative-state corruption. It must produce:

- no submitted action;
- `last_npc_action_error.reason == InternalError` or the nearest existing internal-error reason;
- an error message mentioning missing or invalid hole cards;
- no hand-log action entry.

Helper-level tests are not enough. At least one test should exercise the actual NPC action path or a near-production extraction of that path.

## Developer docs and validation

The repo is now a Cargo workspace. Required Rust validation docs should use workspace-wide commands.

Preferred validation commands:

```bash
npm ci
npm run lint
npm run build
npm test
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test -p poker-core
cargo tree -p poker-core
```

Using `--manifest-path src-tauri/Cargo.toml` for desktop launch is still acceptable where appropriate. It is not acceptable as the only validation path because it can skip `poker-core`-specific checks.

## Definition of done

Fix 7 is done only when:

- README and dev docs use workspace-wide Rust validation commands.
- New Rust `HostRuntimeHealth` counters are represented in TypeScript and visible in the debug UI.
- Host-session authoritative-state lock failures increment `state_lock_error_count` or return structured errors.
- Client runtime thread spawn is fallible and returns `NetworkingError` instead of panicking.
- At least one real NPC action-path test covers missing or invalid acting NPC hole cards.
- `poker-core` purity audits are clean.
- No new hidden fallbacks or silent failures are introduced.
- All final validation commands pass in an environment with Node 24 and Rust installed.
- `memory.md` records only the validation commands that actually passed.
