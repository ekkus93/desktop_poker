# Desktop Poker Fix 6 Spec — Finish Hardening and Actually Extract `poker-core`

## 1. Purpose

This spec defines the next implementation pass for the desktop poker app after the partial Fix 5 implementation.

Fix 5 improved several P0 hardening issues, but it was not complete. The next pass must do two things in this order:

1. **Finish the remaining hardening gaps** from the latest code review.
2. **Actually perform the shared Rust core extraction** so the poker rules/state/projection logic can later be reused from a native Android Kotlin app.

This pass is called **Fix 6**.

The most important instruction for Claude Code: **do not claim completion because tests pass if required behavior is still wrong, stderr is noisy, production bundles contain dev/mock code, or the shared core crate was not created.**

## 2. Current status summary

The latest reviewed code had these verified positives:

- `npm ci --ignore-scripts` passed, with a Node version warning because the repo declares Node `24.x` while the review sandbox used Node `22.x`.
- `npm run lint` passed.
- `npm run build` passed.
- `npm test` passed.
- NPC runner-spawn rollback was added.
- `associated_data_json().unwrap_or_default()` was removed from the client private-message decrypt path.
- Acting NPC missing/invalid hole cards now produce an internal error instead of using an empty-card fallback.
- Full provider config save now updates/deletes the secret before writing non-secret settings.
- Host runtime health was expanded but still incomplete.

The latest reviewed code still had these verified problems:

- `npm test` still prints repeated `Failed to initialize window state persistence.` errors.
- The production `dist` build still contains a `LayoutProbeApp` chunk and `window.__DESKTOP_POKER_BROWSER_MOCKS__` mock setup code.
- There is no root Cargo workspace.
- There is no `crates/poker-core` crate.
- `domain`, `engine`, `tournament`, and projector logic still live inside the Tauri crate.
- There is no portable `PokerEngine` facade.
- There is no Android/Kotlin architecture document.
- `memory.md` overstates Fix 5 completion.
- Some host/client runtime failures are still represented by ignored `let _ = ...`, raw `thread::spawn`, or quiet fallback behavior.

## 3. High-level architecture target

The long-term architecture should be:

```text
repo root
├── Cargo.toml                         # Cargo workspace
├── crates/
│   └── poker-core/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── domain/
│           ├── engine/
│           ├── tournament/
│           ├── projector.rs or domain/projector.rs
│           ├── facade.rs
│           └── error.rs optional
├── src-tauri/
│   ├── Cargo.toml                     # Tauri desktop adapter crate
│   └── src/
│       ├── app_state/
│       ├── networking/
│       ├── npc/
│       ├── protocol/
│       └── commands / Tauri integration
├── src/                               # React desktop UI
└── docs/
    └── ANDROID_ARCHITECTURE.md
```

The split must be clean:

```text
poker-core owns:
  - poker rules
  - tournament state transitions
  - table/domain types
  - legal action validation
  - dealing/shuffling mechanics
  - blind/ante progression
  - betting rounds
  - showdown/settlement
  - public/private projection logic
  - deterministic state serialization
  - deterministic tests

poker-core must NOT own:
  - TCP/WebRTC/Bluetooth/Wi-Fi networking
  - host/client session runtime
  - Tauri commands or Tauri app handles
  - React state or UI view models
  - Android lifecycle, Kotlin coroutines, or Compose state
  - keychain/keystore/provider secret storage
  - LLM client/provider code
  - local IP detection
  - OS window persistence
  - filesystem path selection for app data
```

Android will be a native Kotlin/Compose app using Rust bindings. It will **not** use Tauri Mobile.

Networking is platform/session adapter code. For Android, Kotlin should own networking and translate messages into core commands. Kotlin must not implement or duplicate poker legality, pot settlement, showdown logic, blind progression, or hidden-card projection.

## 4. Fix 6 requirements

### 4.1 Runtime honesty requirement

Any code touched by Fix 6 must not introduce hidden fallback behavior.

A failure is allowed to be best-effort only when all of these are true:

1. It is not authoritative game state.
2. It is not protocol/security/crypto-sensitive.
3. It cannot make the UI or caller believe a mutation succeeded when it did not.
4. It is documented with a short comment or recorded in health/debug diagnostics.

The following are not acceptable:

- swallowing lock poisoning and continuing with invented authoritative state;
- returning `Err` after applying an authoritative mutation unless the error explicitly describes that mutation succeeded but post-sync failed;
- fake UI-only rollback;
- decrypting with empty associated data after associated-data serialization fails;
- treating missing NPC hole cards as an empty hand;
- production builds containing test/probe browser mock setup;
- tests passing while expected stderr is printed.

### 4.2 Window persistence test noise requirement

`npm test` must emit no expected `Failed to initialize window state persistence.` messages.

Fix this at the root cause. Do not suppress `console.error` globally.

Likely root cause in current code:

- `src/app/persistence.test.ts` sets `window.__TAURI_INTERNALS__`.
- The global is not consistently cleaned up after every test.
- Later tests run in the same single-threaded jsdom worker and accidentally enter the Tauri window path.
- The dynamic import of `@tauri-apps/api/window` resolves to an API that cannot function in jsdom.

The fix may include:

- global `afterEach` cleanup of `window.__TAURI_INTERNALS__` and `window.__TAURI__`;
- a valid global Vitest mock for `@tauri-apps/api/window`;
- a stronger runtime guard in `persistence.ts` that only enters the Tauri path when a usable runtime is present.

### 4.3 Production probe/mock removal requirement

Browser mocks must be dev/test only. Production bundles must not contain the probe app or mock setup code.

The current partial fix dynamically imports `LayoutProbeApp`, but Vite still emits a production chunk. That is not acceptable.

The production build validation is:

```bash
npm run build
rg -n "__DESKTOP_POKER_BROWSER_MOCKS__|LayoutProbeApp" dist
```

The preferred result is **no hits**. If source maps are intentionally shipped and produce hits only in source maps, disable release source maps or document the release packaging decision. The production JS payload must not include mock setup code.

### 4.4 Authoritative mutation semantics requirement

`HostServer::add_npc_participants_atomic(...)` currently mutates state and then calls `sync_snapshots_to_clients()`. If sync fails after mutation, it returns `Err` even though authoritative state changed.

This is not honest transaction semantics.

Fix by choosing one explicit model:

- **Preferred:** mutation succeeds, snapshot sync failure is recorded as runtime health/debug warning, and the method returns `Ok(player_ids)` because the authoritative mutation succeeded.
- **Alternative:** rollback the authoritative mutation if snapshot sync fails, then return an error.
- **Do not:** return a plain error after mutation without rollback or mutation-success semantics.

For similar host methods (`claim_seat`, `set_ready_state`, `remove_npc_participants_atomic`), decide whether snapshot sync failure should be returned as a command failure or recorded as a publish/sync diagnostic. Be consistent and honest.

### 4.5 Host runtime health completion requirement

`HostRuntimeHealth` must cover remaining important host runtime failures.

At minimum:

- raw `thread::spawn` for host client sessions must be replaced with fallible `thread::Builder::spawn`, with failures recorded in health;
- important `clients.lock()` failures must record health instead of being ignored;
- important `mark_participant_reconnect_eligible(...)` failures must record health instead of being ignored;
- timeout setup failures in client connection code must return errors or diagnostics;
- authoritative-state lock poisoning in host tick/publish paths must be fail-loud or recorded, not converted to invented previous/current state.

Best-effort cleanup during shutdown may still ignore errors, but only with a short comment explaining why it is safe.

### 4.6 `memory.md` correctness requirement

`memory.md` must be corrected before the repo claims Fix 6 completion.

The entry must say Fix 5 was partial, not complete. It must not claim:

- tests are clean if stderr still contains expected errors;
- production bundle excludes `LayoutProbeApp` if `rg dist` still finds it;
- Rust workspace/core tests pass if no workspace/core crate exists or cargo validation was not run.

After Fix 6 is truly complete, add a second completion entry with the actual validation commands run.

### 4.7 Cargo workspace requirement

Create a root Cargo workspace.

Root `Cargo.toml` should be:

```toml
[workspace]
members = [
    "crates/poker-core",
    "src-tauri",
]
resolver = "2"
```

If `src-tauri/Cargo.toml` already contains a `[workspace]` section, remove or consolidate it so there is exactly one workspace root.

Update project docs/skills/CLAUDE instructions that reference old cargo commands. The standard command should become workspace-aware, for example:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test -p poker-core
cargo tree -p poker-core
```

### 4.8 `poker-core` extraction requirement

Create `crates/poker-core` and move pure poker modules out of `src-tauri`.

Move these first:

- `src-tauri/src/domain/**`
- `src-tauri/src/engine/**`
- `src-tauri/src/tournament/**`

Move projector if pure:

- `src-tauri/src/domain/projector.rs` → `crates/poker-core/src/domain/projector.rs` or `crates/poker-core/src/projector.rs`

Do not move anything that depends on:

- `tauri`
- `keyring`
- `dirs`
- `reqwest`
- `local-ip-address`
- `std::net`
- TCP/UDP/socket types
- app-state modules
- networking modules
- LLM/provider storage/client code
- OS window or UI concepts

The desktop Tauri crate should depend on `poker-core` via path dependency:

```toml
poker-core = { path = "../crates/poker-core" }
```

Update imports to use `poker_core::...`.

Prefer explicit module imports over broad glob re-exports. `poker-core/src/lib.rs` may expose modules:

```rust
pub mod domain;
pub mod engine;
pub mod tournament;
pub mod facade;
pub mod error;
```

A small optional `prelude` is acceptable, but avoid `pub use domain::*;` at crate root unless import churn becomes unmanageable.

### 4.9 Core facade requirement

Add a small portable facade in `poker-core`. This is a stepping stone, not the final Android FFI API.

The facade should prove that Android/Kotlin can later use a stable command/state boundary without depending on Tauri.

Minimum facade concepts:

- `PokerEngine`
- `EngineCommand`
- `EngineEvent`
- `PokerCoreError`
- `export_state_json()`
- deterministic tests

The desktop app does not need to be fully rewritten to use the facade in Fix 6. Existing desktop behavior should remain stable.

### 4.10 Android architecture documentation requirement

Add `docs/ANDROID_ARCHITECTURE.md`.

It must state:

- Android will be native Kotlin/Compose.
- The Android app will not use Tauri Mobile.
- Kotlin owns UI, lifecycle, permissions, storage integration, and networking/session transport.
- Rust `poker-core` owns poker rules/state/projection only.
- Kotlin networking messages are translated into `EngineCommand`s or equivalent core commands.
- Kotlin must not reimplement poker legality, pot settlement, showdown, blinds, or hidden-card projection.
- Future binding should probably use UniFFI unless there is a concrete reason to use handwritten JNI.

## 5. Acceptance criteria

Fix 6 is done only when all of these are true:

- `npm test` passes and emits no expected `Failed to initialize window state persistence.` errors.
- `npm run build` passes.
- `rg -n "__DESKTOP_POKER_BROWSER_MOCKS__|LayoutProbeApp" dist` has no production JS/mock setup hits.
- NPC add rollback after runner-spawn failure remains covered.
- Private-message empty-AAD fallback remains absent.
- Acting NPC missing/invalid hole cards remains fail-loud.
- Provider config save does not leave new settings behind after secret update failure.
- Host runtime health covers the remaining important host failures or documents safe ignored results.
- `memory.md` accurately records Fix 5 as partial and Fix 6 completion only after validation.
- Root Cargo workspace exists.
- `crates/poker-core` exists.
- Pure domain/engine/tournament/projector code lives in `poker-core`.
- `poker-core` has no Tauri/networking/platform/LLM/keychain dependencies.
- Desktop Tauri crate depends on and imports from `poker-core`.
- `poker-core` exposes a small portable facade with tests.
- Android architecture doc exists and says Kotlin owns networking while Rust owns rules/state/projection.
- All final validation commands pass in the developer environment.

## 6. Final validation commands

Run from repo root:

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

Focused audits:

```bash
rg -n "Failed to initialize window state persistence" <captured npm test output file if available>
rg -n "__DESKTOP_POKER_BROWSER_MOCKS__|LayoutProbeApp" dist || true
rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated" src-tauri/src/networking
rg -n "hole_cards_by_player_id.*unwrap_or\(|unwrap_or\(&\[\]\)" src-tauri/src/npc src-tauri/src/tournament src-tauri/src/networking
rg -n "let _ =|thread::spawn|clients\.lock\(|mark_participant_reconnect|unwrap_or_else\(\|_\| state\.clone\(\)" src-tauri/src/networking/runtime
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn|Command" crates/poker-core
```

If Node 24 is not active, switch to Node 24 first because the repo declares Node `24.x`.
