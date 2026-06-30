# Desktop Poker Fix 5 Spec — Finish Runtime Hardening and Extract Shared Rust Poker Core

## 1. Purpose

This spec defines the next implementation pass for the desktop poker app.

It has two related goals:

1. **Finish the remaining FIX4 runtime-hardening gaps** found in the latest review.
2. **Start the architectural migration toward a shared Rust poker engine core** that can be reused by both the current Tauri desktop app and a future native Android/Kotlin app.

The Android goal is explicit: **do not use Tauri Mobile**. Android should eventually be a normal Kotlin/Compose app that calls the shared Rust poker engine through UniFFI or JNI.

## 2. Key architecture decision

Networking must **not** live in the Rust poker engine core.

The Rust poker core is a deterministic, embeddable rules/state engine. It owns poker/tournament logic, snapshots, validation, and state transitions. Platform/session adapters own networking.

```text
poker-core
  owns:
    - poker domain types
    - deck/deal/shuffle abstractions
    - legal action calculation
    - action application
    - tournament state machine
    - blind/ante progression
    - hand progression
    - showdown/settlement
    - state serialization/deserialization
    - public/private table projections
    - deterministic seeded tests

  must not own:
    - sockets
    - TCP/WebRTC/Bluetooth/Wi-Fi
    - peer discovery
    - Tauri commands
    - Android lifecycle
    - Kotlin coroutines
    - UI state
    - OS window state
    - keychain/keystore access
    - LLM provider storage
    - platform file paths
```

The desired end state is:

```text
                 ┌──────────────────────────────┐
                 │         poker-core           │
                 │ Rust rules/state/projection  │
                 │ no Tauri, no networking, UI  │
                 └──────────────┬───────────────┘
                                │
           ┌────────────────────┴────────────────────┐
           │                                         │
┌──────────────────────────────┐       ┌──────────────────────────────┐
│ Tauri Desktop Adapter        │       │ Android Kotlin Adapter        │
│ React UI                     │       │ Kotlin/Compose UI             │
│ Tauri commands               │       │ ViewModel/Repository          │
│ desktop session manager      │       │ Android networking/lifecycle  │
│ desktop networking runtime   │       │ UniFFI/JNI bridge to Rust     │
│ calls poker-core             │       │ calls poker-core              │
└──────────────────────────────┘       └──────────────────────────────┘
```

## 3. Guiding principles

### 3.1 Deterministic core

The same command sequence against the same initial state and seed must produce the same engine state/events on desktop, Android, CLI tests, or future server code.

### 3.2 Command/event/snapshot boundary

The core should be interacted with through commands and snapshots, not by exposing mutable internals to UI/platform code.

Preferred conceptual API:

```rust
pub struct PokerEngine {
    state: TournamentState,
}

impl PokerEngine {
    pub fn new(config: TournamentConfig, seed: Option<u64>) -> Result<Self, EngineError>;

    pub fn submit_command(
        &mut self,
        command: EngineCommand,
    ) -> Result<Vec<EngineEvent>, EngineError>;

    pub fn snapshot_for(
        &self,
        viewer: Option<&PlayerId>,
    ) -> Result<TableSnapshot, EngineError>;

    pub fn export_state(&self) -> Result<Vec<u8>, EngineError>;

    pub fn import_state(bytes: &[u8]) -> Result<Self, EngineError>;
}
```

This exact API does not need to be completed in Fix 5, but the extraction must move in this direction.

### 3.3 Platform networking

Desktop and Android may have different networking implementations.

Desktop may keep the existing Rust/Tauri networking runtime for now. Android should later use Kotlin networking/coroutines if that is the most natural platform implementation.

The networking layer may carry `EngineCommand`s and broadcast `EngineEvent`s/snapshots, but it must not decide poker legality or mutate authoritative poker state directly.

### 3.4 Fail-loud over hidden fallback

Do not introduce silent fallbacks. In authoritative game logic, missing/corrupt state must become a structured error.

Examples of forbidden behavior:

- missing NPC hole cards become `[]`;
- failed private-message AAD serialization becomes empty AAD;
- NPC add reports an error while leaving registered/seated NPCs behind;
- test/runtime initialization errors are printed as expected noise;
- host runtime failures are ignored with `let _ = ...` and no diagnostic.

### 3.5 Small, test-backed migration

Do not attempt a massive rewrite. First make the repo a Cargo workspace and extract pure modules into `crates/poker-core`. Keep the desktop app working after each step.

## 4. Immediate hardening fixes required before or during extraction

The latest review found these remaining issues:

1. **NPC add is not all-or-nothing if runner spawn fails.**
   `HostServer::add_npc_participants_atomic` is good, but `DesktopAppState::add_npc_players` still mutates host state before `start_npc_runner`. If thread spawn fails, NPCs remain registered/seated/ready with no runner.

2. **Client private-message decrypt uses empty AAD on serialization failure.**
   `associated_data_json().unwrap_or_default()` in `client.rs` is not acceptable in crypto/protocol code.

3. **NPC decision treats missing hole cards as an empty hand.**
   An acting NPC without exactly two hole cards is corrupt authoritative state. It must record an internal NPC error and submit no action.

4. **Window persistence test noise remains.**
   `npm test` passes but still prints repeated `Failed to initialize window state persistence` errors. Expected stderr noise is not acceptable.

5. **Host runtime health is incomplete.**
   Health diagnostics exist, but some host runtime failures still bypass counters/errors.

6. **`save_provider_config(Some(...))` can leave settings and secrets inconsistent.**
   `save_provider_settings_only` was fixed, but the full config save path still writes the non-secret settings before keychain/file-secret writes.

7. **`LayoutProbeApp` is still statically imported into the production bundle.**
   Runtime mock gating exists, but probe code should not be in production bundles if it contains browser mock setup.

8. **`memory.md` overstates completion.**
   It must not claim FIX4 is complete or that no unsafe silent failures remain until these issues are fixed.

## 5. Proposed crate/workspace layout

Fix 5 should introduce the workspace shape and extract the first pure modules.

Target layout:

```text
.
├── Cargo.toml                         # new workspace root
├── crates/
│   └── poker-core/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── domain/
│           ├── engine/
│           ├── tournament/
│           └── projector.rs
└── src-tauri/
    ├── Cargo.toml                     # Tauri desktop adapter crate
    └── src/
        ├── app_state/
        ├── commands.rs
        ├── networking/                # remains desktop/session adapter, not core
        ├── npc/                       # may remain desktop-owned for now
        ├── protocol/                  # remains adapter/network protocol for now
        └── storage/
```

### 5.1 What to extract first

Extract only modules that are already close to pure poker logic:

- `src-tauri/src/domain/**` → `crates/poker-core/src/domain/**`
- `src-tauri/src/engine/**` → `crates/poker-core/src/engine/**`
- `src-tauri/src/tournament/**` → `crates/poker-core/src/tournament/**`
- `src-tauri/src/domain/projector.rs` → either `crates/poker-core/src/projector.rs` or `crates/poker-core/src/domain/projector.rs`

Keep existing tests with the moved modules.

### 5.2 What not to extract in Fix 5

Do **not** move these into `poker-core` in Fix 5:

- `src-tauri/src/networking/**`
- `src-tauri/src/commands.rs`
- `src-tauri/src/app_state/**`
- `src-tauri/src/storage/**`
- Tauri-specific config/state code
- keyring/keychain/secret stores
- LLM provider storage/client code
- socket/reconnect/replay runtime code

NPC code may be split later into `poker-npc`, but not as part of the initial core extraction unless it is required by tests. If moved later, `poker-npc` should depend on `poker-core`; `poker-core` must not depend on NPC code.

### 5.3 Dependency rules

`poker-core` dependencies should stay minimal and portable:

Allowed in `poker-core`:

- `serde`
- `serde_json` only if needed for state export/import tests or snapshot compatibility
- `thiserror`
- deterministic RNG dependency only if current core requires it

Forbidden in `poker-core`:

- `tauri`
- `keyring`
- `dirs`
- `reqwest`
- `local-ip-address`
- socket/networking types
- `std::net` in core modules
- platform file paths for app data
- LLM provider/client dependencies
- crypto/protocol dependencies unless purely needed for domain serialization; encryption/signing should not be core

## 6. Android direction

Fix 5 should not implement the Android app yet. It should make the Rust engine extractable and portable.

Add documentation for the eventual Android integration:

```text
Android Kotlin/Compose app
  ViewModel/Repository owns Android lifecycle and networking.
  Kotlin receives network messages and turns them into EngineCommand calls.
  Rust poker-core validates and applies commands.
  Rust returns EngineEvent/snapshot values.
  Kotlin updates UI and sends messages over Android-owned networking.
```

Preferred future binding layer: UniFFI. Hand-written JNI is acceptable only if UniFFI becomes impractical.

A future `poker-android-ffi` crate should depend on `poker-core` and expose Android-safe DTOs. It should not expose raw internal mutable state.

## 7. Testing expectations

Fix 5 is only done when:

- frontend lint/build/test pass;
- Rust fmt/clippy/tests pass for the whole workspace;
- `poker-core` tests run independently;
- Tauri desktop crate tests still pass;
- no hardening regression remains from Section 4;
- no new hidden fallback is added to make tests pass.

## 8. Definition of done

- [ ] NPC add rolls back/removes NPCs if runner spawn fails.
- [ ] Client private-message decrypt does not use empty AAD fallback.
- [ ] Acting NPC with missing/invalid hole cards records an internal error and submits no action.
- [ ] Window persistence tests emit no expected initialization errors.
- [ ] Host runtime health covers remaining important host failure paths.
- [ ] Full provider config save does not write settings before secret write/delete succeeds.
- [ ] Layout probe/mock setup is dev-only and not statically included in production entrypoint.
- [ ] `memory.md` accurately states remaining/fixed work.
- [ ] A new `poker-core` crate exists.
- [ ] Pure domain/engine/tournament/projector code lives in `poker-core`.
- [ ] `poker-core` has no Tauri/networking/platform/LLM/keychain dependencies.
- [ ] Desktop Tauri crate depends on `poker-core` and still works.
- [ ] Docs explain that Android networking is Kotlin/platform-owned and poker rules remain Rust-owned.
