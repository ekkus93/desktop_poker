# Desktop Poker Fix 5 TODO — Finish Hardening and Extract Shared `poker-core`

This TODO is intentionally explicit for Claude Code. Do tasks in priority order. Prefer small, test-backed patches. Do not introduce hidden fallback behavior to make tests pass.

## P0.1 — Make `add_npc_players` rollback-safe after runner-spawn failure

**Files:**

- `src-tauri/src/app_state/app_npc.rs`
- `src-tauri/src/networking/runtime/host.rs`
- `src-tauri/src/networking/runtime/tests/tournament.rs`
- `src-tauri/src/app_state/tests/sessions_npc.rs` or equivalent

### Problem

`HostServer::add_npc_participants_atomic(...)` is all-or-nothing for the host-state mutation itself, but `DesktopAppState::add_npc_players(...)` still does this:

```rust
session.host_server.add_npc_participants_atomic(assignments)?;
let runner_handle = start_npc_runner(...)?;
session.npc_runner = Some(...);
```

If `start_npc_runner(...)` fails, NPCs remain registered/seated/ready, but no runner exists. The caller receives an error while authoritative state is mutated. That violates all-or-nothing behavior.

### Required change

Add an authoritative removal/rollback method for the exact NPC player IDs just added. Use it if runner spawn fails.

### Suggested host method

Add this near `add_npc_participants_atomic` in `src-tauri/src/networking/runtime/host.rs`.

```rust
pub fn remove_npc_participants_atomic(
    &self,
    player_ids: &[String],
) -> Result<(), NetworkingError> {
    let targets: std::collections::HashSet<&str> =
        player_ids.iter().map(String::as_str).collect();

    self.update_lobby_state(|state| {
        ensure_lobby_phase(state.phase)?;
        ensure_lobby_seat_map(state);

        for player_id in &targets {
            let Some(participant) = state.participants.get(*player_id) else {
                continue;
            };

            if participant.is_host {
                return Err(NetworkingError::new(format!(
                    "refusing to remove host participant {player_id} during NPC rollback"
                )));
            }

            if !crate::npc::NpcConfig::is_npc_player_id(player_id) {
                return Err(NetworkingError::new(format!(
                    "refusing to remove non-NPC participant {player_id} during NPC rollback"
                )));
            }
        }

        for seat in &mut state.seats {
            if seat
                .participant_id
                .as_deref()
                .is_some_and(|id| targets.contains(id))
            {
                seat.occupancy = crate::domain::SeatOccupancyState::Empty;
                seat.participant_id = None;
                seat.display_name = None;
                seat.chip_count = None;
                seat.tournament_state = crate::domain::TournamentSeatState::Empty;
            }
        }

        for player_id in &targets {
            state.ready_players.remove(*player_id);
            state.participants.remove(*player_id);
        }

        Ok(())
    })
}
```

Adjust field names if the exact `TournamentSeatState`/seat fields differ. The key rule: rollback must mutate **authoritative host state**, not just UI or local app state.

### Suggested caller change

In `DesktopAppState::add_npc_players(...)`, capture player IDs and rollback on runner-spawn failure.

```rust
let added_player_ids: Vec<String> = npc_configs
    .iter()
    .map(|config| config.player_id.clone())
    .collect();

session
    .host_server
    .add_npc_participants_atomic(assignments)
    .map_err(|e| e.to_string())?;

let runner_handle = match crate::npc::runner::start_npc_runner(
    Arc::clone(&session.host_server),
    npc_configs,
    Arc::clone(&stop),
    Arc::clone(&self.llm_provider),
    Arc::clone(&tilt_levels),
    Arc::clone(&last_llm_fallback),
    Arc::clone(&last_npc_action_error),
) {
    Ok(handle) => handle,
    Err(error) => {
        let rollback = session
            .host_server
            .remove_npc_participants_atomic(&added_player_ids)
            .map_err(|rollback_error| rollback_error.to_string());

        return Err(match rollback {
            Ok(()) => error,
            Err(rollback_error) => format!(
                "{error}; additionally failed to rollback NPC add after runner spawn failure: {rollback_error}"
            ),
        });
    }
};
```

This snippet consumes `npc_configs`; if the current code needs `npc_configs` after the call, clone only what is needed. Prefer a small helper if the function becomes too long.

### Tests

Add or extend tests so all cases are covered:

1. `remove_npc_participants_atomic` removes only NPC participants and frees their seats.
2. Removing an unknown NPC ID is harmless or returns a clearly documented error. Pick one behavior and test it.
3. Removing a host or human player is rejected.
4. `add_npc_players` rolls back if `start_npc_runner` fails.

The fourth test may require injecting the spawner into the app-state path. If direct injection is awkward, add an app-state helper that accepts a runner-start function.

Example test helper direction:

```rust
type RunnerStarter = dyn Fn(
    Arc<crate::networking::HostServer>,
    Vec<crate::npc::NpcConfig>,
    Arc<std::sync::atomic::AtomicBool>,
    Arc<Mutex<Option<crate::npc::LlmProviderConfig>>>,
    Arc<Mutex<std::collections::BTreeMap<String, String>>>,
    Arc<Mutex<Option<String>>>,
    Arc<Mutex<Option<crate::app_state::NpcActionErrorDebug>>>,
) -> Result<std::thread::JoinHandle<()>, String>;
```

Do not use OS thread exhaustion as a test technique.

### Acceptance

- If runner spawn fails, no requested NPC remains registered, seated, or ready.
- Rollback failure is included in the returned error message.
- No fake UI-only rollback.

---

## P0.2 — Remove private-message empty-AAD fallback

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- `src-tauri/src/networking/runtime/tests/protocol_warning.rs`

### Problem

Private hole-card decrypt currently uses something equivalent to:

```rust
envelope.associated_data_json().unwrap_or_default().as_slice()
```

This is a bad protocol/crypto fallback. If AAD serialization fails, the client must drop the frame and emit a protocol warning. It must not decrypt with empty AAD.

### Required change

Replace the fallback with explicit warning + `continue`.

```rust
let associated_data = match envelope.associated_data_json() {
    Ok(associated_data) => associated_data,
    Err(error) => {
        emit_protocol_warning(
            &sender,
            &mut protocol_warning_counts,
            &player_id,
            format!("private hole-card associated-data serialization failed: {error}"),
        );
        continue;
    }
};

let Ok(plaintext) = crypto_provider.decrypt(
    encryption_keys,
    &host_encryption_public_key,
    &encrypted_payload,
    associated_data.as_slice(),
) else {
    emit_protocol_warning(
        &sender,
        &mut protocol_warning_counts,
        &player_id,
        "private hole-card payload decrypt failed",
    );
    continue;
};
```

If `associated_data_json()` is currently infallible in practice, keep the branch anyway. The return type is `Result`, so the code must treat failure honestly.

### Tests

Add a focused unit test if possible. If constructing an envelope that makes `associated_data_json()` fail is impractical, at minimum:

- add a regression test around the helper if it can be made injectable;
- add an `rg`-style test or CI guard is not necessary, but manually verify no `associated_data_json().unwrap_or_default()` remains.

### Acceptance

- `rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated" src-tauri/src/networking` finds nothing.
- AAD failure path emits `ClientRuntimeEvent::ProtocolWarning`.
- No decrypt attempt is made with empty AAD after AAD serialization failure.

---

## P0.3 — Treat missing/invalid acting NPC hole cards as an internal error

**Files:**

- `src-tauri/src/npc/runner/action.rs`
- `src-tauri/src/npc/runner/tests.rs`
- possibly `src-tauri/src/npc/runner/decision.rs`

### Problem

The acting NPC's hole cards are currently fetched with a fallback to an empty slice. That silently converts corrupt authoritative state into a weak/no-card decision.

An acting Hold'em player must have exactly two hole cards. If not, record an internal NPC error and submit no action.

### Required change

Replace the fallback with validation before building `RuleDecisionContext` or `GameStateSnapshot`.

```rust
let hole_cards = match fresh_hand
    .hole_cards_by_player_id
    .get(&fresh_window.player_id)
{
    Some(cards) if cards.len() == 2 => cards.as_slice(),
    Some(cards) => {
        let msg = format!(
            "[npc-runner] NPC {} has invalid hole-card count {}; expected 2; no action submitted",
            fresh_window.player_id,
            cards.len()
        );
        return record_npc_internal_error(
            runner_state,
            fresh_window.player_id.clone(),
            Some(fresh_hand.hand_number),
            NpcActionErrorReason::InternalError,
            msg,
        );
    }
    None => {
        let msg = format!(
            "[npc-runner] NPC {} is missing hole cards; no action submitted",
            fresh_window.player_id
        );
        return record_npc_internal_error(
            runner_state,
            fresh_window.player_id.clone(),
            Some(fresh_hand.hand_number),
            NpcActionErrorReason::InternalError,
            msg,
        );
    }
};
```

### Tests

Add tests:

```rust
#[test]
fn npc_decision_fails_when_hole_cards_are_missing() {
    // Arrange an acting NPC with an action window but remove its hole_cards entry.
    // Act: try_npc_action(...)
    // Assert: outcome is RuntimeUnavailable or equivalent non-success,
    // last_npc_action_error.reason == InternalError,
    // error message mentions missing hole cards,
    // and no action was submitted/recorded.
}

#[test]
fn npc_decision_fails_when_hole_card_count_is_not_two() {
    // Arrange acting NPC with one or three cards.
    // Assert same no-action/internal-error behavior.
}
```

### Acceptance

- No production NPC decision path uses empty hole cards for an acting Hold'em NPC.
- Corrupt hole-card state is visible through `last_npc_action_error`.
- Rule-based and LLM-backed NPC paths both use the validated cards.

---

## P0.4 — Make full provider config save fail before writing settings if secret update fails

**Files:**

- `src-tauri/src/npc/provider_storage.rs`
- provider storage tests in the same file

### Problem

`save_provider_settings_only(...)` was made transactional, but `save_provider_config(Some(...))` still writes the non-secret settings file before writing/deleting the secret. If the secret write/delete fails, the settings file may now point at a provider/key state that was not actually saved.

### Required change

For `Some(config)`:

1. Create the settings parent directory if needed.
2. Write/delete the provider secret first.
3. Only after the secret update succeeds, write the non-secret settings file.

Suggested replacement for the `Some(cfg)` branch:

```rust
Some(cfg) => {
    if let Some(parent) = sp.parent() {
        fs::create_dir_all(parent)?;
    }

    let key = cfg.api_key.as_deref().unwrap_or("").trim().to_string();
    let provider_str = cfg.settings.provider.as_str();

    if key.is_empty() {
        store.delete_key(provider_str).map_err(|error| {
            std::io::Error::other(format!(
                "could not delete provider key for {provider_str}: {error}"
            ))
        })?;
    } else {
        store.write_key(provider_str, &key).map_err(|error| {
            std::io::Error::other(format!(
                "could not write provider key for {provider_str}: {error}"
            ))
        })?;
    }

    let settings_json = serde_json::to_string_pretty(&cfg.settings)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    fs::write(&sp, settings_json)?;
}
```

For `None`, the current behavior of deleting all known provider accounts before deleting settings is acceptable, provided delete failures are returned honestly.

### Tests

Add tests:

```rust
#[test]
fn save_provider_config_does_not_write_settings_if_secret_write_fails() {
    let dir = tempfile::tempdir().unwrap();
    let store = FailingSecretStore::new().fail_write();
    let cfg = openai_config("sk-openai-new");

    let result = save_provider_config(dir.path(), Some(&cfg), &store);

    assert!(result.is_err());
    assert!(!settings_path(dir.path()).exists());
}

#[test]
fn save_provider_config_preserves_existing_settings_if_secret_write_fails() {
    let dir = tempfile::tempdir().unwrap();
    let good_store = InMemorySecretStore::default();
    save_provider_config(dir.path(), Some(&anthropic_config("sk-ant-old")), &good_store).unwrap();

    let failing_store = FailingSecretStore::new().fail_write();
    let result = save_provider_config(dir.path(), Some(&openai_config("sk-openai-new")), &failing_store);

    assert!(result.is_err());
    let persisted = fs::read_to_string(settings_path(dir.path())).unwrap();
    let current: LlmProviderSettings = serde_json::from_str(&persisted).unwrap();
    assert_eq!(current.provider, LlmProviderType::Anthropic);
}
```

Adjust helper names to match existing tests.

### Acceptance

- Secret write/delete failure cannot leave newly written provider settings behind.
- Tests cover empty-key delete failure and non-empty-key write failure.

---

## P0.5 — Fix window persistence test noise for all tests

**Files:**

- `src/app/persistence.ts`
- `src/app/persistence.test.ts`
- `src/test/setup.ts` or equivalent Vitest setup file
- `vite.config.ts`

### Problem

`npm test` now finishes, but test output still includes repeated expected errors like:

```text
Failed to initialize window state persistence.
TypeError: Cannot read properties of undefined (reading 'currentWindow')
```

Expected stderr noise is not acceptable. It hides real failures.

### Required change

Fix either the runtime guard or the global test mock so tests do not enter a half-mocked Tauri window path.

Preferred: add a global Vitest mock for `@tauri-apps/api/window` if any tests intentionally set `window.__TAURI_INTERNALS__`.

Example `src/test/setup.ts`:

```ts
import { vi } from "vitest";

vi.mock("@tauri-apps/api/window", () => {
  const currentWindow = {
    innerSize: vi.fn().mockResolvedValue({ width: 1280, height: 720 }),
    outerPosition: vi.fn().mockResolvedValue({ x: 0, y: 0 }),
    setSize: vi.fn().mockResolvedValue(undefined),
    setPosition: vi.fn().mockResolvedValue(undefined),
    listen: vi.fn().mockResolvedValue(() => undefined),
  };

  return {
    getCurrentWindow: () => currentWindow,
    Window: {
      getCurrent: () => currentWindow,
    },
    LogicalSize: class LogicalSize {
      constructor(
        public width: number,
        public height: number,
      ) {}
    },
    LogicalPosition: class LogicalPosition {
      constructor(
        public x: number,
        public y: number,
      ) {}
    },
  };
});
```

Register it in `vite.config.ts` if not already present:

```ts
test: {
  setupFiles: ["src/test/setup.ts"],
  pool: "threads",
  poolOptions: {
    threads: {
      singleThread: true,
    },
  },
}
```

Also strengthen `persistence.ts` so non-Tauri/jsdom paths return quietly before dynamic import.

```ts
function hasUsableTauriWindowRuntime(): boolean {
  if (typeof window === "undefined") {
    return false;
  }

  const maybeWindow = window as Window & {
    __TAURI_INTERNALS__?: unknown;
    __TAURI__?: unknown;
  };

  return Boolean(maybeWindow.__TAURI_INTERNALS__ || maybeWindow.__TAURI__);
}
```

Do not just suppress `console.error` in tests. That hides real failures.

### Acceptance

- `npm test` emits no expected `Failed to initialize window state persistence` errors.
- Real Tauri window API failures still produce actionable errors in Tauri-like environments.

---

## P1.1 — Finish host runtime health coverage

**Files:**

- `src-tauri/src/networking/runtime/host.rs`
- `src-tauri/src/networking/runtime/host_session.rs`
- `src-tauri/src/networking/runtime/host_broadcast.rs`
- `src-tauri/src/networking/runtime/events.rs`
- `src-tauri/src/networking/runtime/mod.rs`
- runtime tests

### Problem

`HostRuntimeHealth` exists, but several important host runtime failures still bypass it.

Audit current host runtime code for:

```bash
rg -n "let _ =|try_clone\(|clients\.lock\(|thread::spawn|return;|continue;" \
  src-tauri/src/networking/runtime
```

### Required change

Every important host runtime failure must do one of these:

1. increment a health counter and set `last_error`;
2. emit a structured runtime/debug event;
3. return a real error to caller;
4. include a comment explaining why the ignored failure is safe during shutdown or test cleanup.

### Suggested health fields

If the current struct lacks specific counters, add these:

```rust
pub client_registry_error_count: u64,
pub stream_clone_error_count: u64,
pub client_session_spawn_error_count: u64,
pub reconnect_mark_error_count: u64,
pub best_effort_write_error_count: u64,
```

### Suggested helper methods

```rust
impl HostRuntimeHealth {
    pub fn record_stream_clone_error(&mut self, error: impl std::fmt::Display) {
        self.stream_clone_error_count += 1;
        self.last_error = Some(format!("failed to clone client stream: {error}"));
    }

    pub fn record_client_registry_error(&mut self) {
        self.client_registry_error_count += 1;
        self.last_error = Some("connected-client registry lock poisoned".to_string());
    }
}
```

### Example replacement

Replace silent clone failure:

```rust
let stream_handle = match stream.try_clone() {
    Ok(cloned_stream) => Arc::new(Mutex::new(cloned_stream)),
    Err(error) => {
        update_health(&runtime_health_conn, |health| {
            health.record_stream_clone_error(error);
        });
        return;
    }
};
```

Replace silent client registry lock skip:

```rust
match clients.lock() {
    Ok(mut connected_clients) => {
        connected_clients.insert(player_id.clone(), ConnectedClient { /* ... */ });
    }
    Err(_) => {
        update_health(&runtime_health_conn, |health| {
            health.record_client_registry_error();
        });
        return;
    }
}
```

### Acceptance

- No critical host runtime failure is represented only by `let _ = ...`, `return`, or `continue`.
- Any remaining ignored results have comments explaining why they are safe.
- Debug inspector can show the added counters.

---

## P1.2 — Remove layout probe/browser mock code from production bundle

**Files:**

- `src/main.tsx`
- `src/app/runtimeGate.ts`
- `src/probe/LayoutProbeApp.tsx`
- frontend tests if any

### Problem

Browser mocks are runtime-gated in `src/api/desktop.ts`, but `LayoutProbeApp` is still statically imported in `src/main.tsx`. The probe module assigns `window.__DESKTOP_POKER_BROWSER_MOCKS__`; it should not be included in production bundles.

### Required change

Use a dev-only dynamic import. Production entrypoint should not statically import `LayoutProbeApp`.

Suggested `src/main.tsx` direction:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

async function bootstrap() {
  const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);

  if (import.meta.env.DEV) {
    const { resolveLayoutProbeSurface } = await import("./app/runtimeGate");
    const layoutProbe = resolveLayoutProbeSurface(window.location.search, true);

    if (layoutProbe) {
      const { LayoutProbeApp } = await import("./probe/LayoutProbeApp");
      root.render(
        <React.StrictMode>
          <LayoutProbeApp surface={layoutProbe} />
        </React.StrictMode>,
      );
      return;
    }
  }

  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

void bootstrap();
```

### Tests / validation

Run:

```bash
npm run build
rg -n "__DESKTOP_POKER_BROWSER_MOCKS__|LayoutProbeApp" dist
```

The ideal result is no production bundle hit for `__DESKTOP_POKER_BROWSER_MOCKS__`. If Vite still preserves a string in source maps, document why and ensure source maps are not shipped in release if that matters.

### Acceptance

- `src/main.tsx` has no static `LayoutProbeApp` import.
- Production runtime cannot enter layout probe mode.
- Production bundle does not contain mock setup code.

---

## P1.3 — Correct `memory.md`

**Files:**

- `memory.md`

### Problem

`memory.md` currently overstates FIX4 completion. It must not say no unsafe silent failures remain while this TODO still exists.

### Required change

Add a corrective entry using the project’s existing timestamp convention. Do not fabricate a timestamp. Use the repo’s documented command if present; otherwise use:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

Suggested entry content:

```md
- <timestamp> — FIX5 follow-up review correction: FIX4 improved provider settings, NPC fallback handling, host health, protocol warnings, and test reliability, but it was not complete. Remaining issues tracked in DESKTOP_POKER_CORE_EXTRACTION_FIX5_TODO.md: NPC add must rollback after runner-spawn failure; private-message AAD must not fall back to empty bytes; acting NPCs must fail loudly when hole cards are missing/invalid; persistence tests must stop printing expected initialization errors; host health coverage needs to cover remaining lock/clone/client-registry failures. Architecture decision: shared Rust poker-core will own deterministic poker rules/state/projection only; desktop and Android adapters own networking. Android will be Kotlin + Rust bindings, not Tauri Mobile.
```

After all tasks are actually complete, add a second completion entry.

### Acceptance

- `memory.md` no longer misleadingly states FIX4 is complete with no remaining unsafe silent failures.
- The Android/core architecture decision is recorded.

---

## P2.1 — Add Cargo workspace and `crates/poker-core` skeleton

**Files:**

- root `Cargo.toml` new
- `crates/poker-core/Cargo.toml` new
- `crates/poker-core/src/lib.rs` new
- `src-tauri/Cargo.toml`

### Problem

All Rust code currently lives under the Tauri crate. That makes it harder to reuse poker logic from Android/Kotlin without dragging in Tauri/networking/platform dependencies.

### Required change

Create a Cargo workspace and a new `poker-core` crate.

Root `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/poker-core",
    "src-tauri",
]
resolver = "2"
```

`crates/poker-core/Cargo.toml`:

```toml
[package]
name = "poker-core"
version = "0.1.0"
edition = "2021"
description = "Shared deterministic poker engine core for desktop and Android adapters"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
```

Update `src-tauri/Cargo.toml`:

```toml
[dependencies]
poker-core = { path = "../crates/poker-core" }
# existing deps remain here; do not move Tauri/platform deps into poker-core
```

### Acceptance

- `cargo metadata` works from repo root.
- `cargo test -p poker-core` works once modules are moved.
- `src-tauri` still builds/tests through the workspace.

---

## P2.2 — Move pure poker modules into `poker-core`

**Files:**

Move from:

- `src-tauri/src/domain/**`
- `src-tauri/src/engine/**`
- `src-tauri/src/tournament/**`

To:

- `crates/poker-core/src/domain/**`
- `crates/poker-core/src/engine/**`
- `crates/poker-core/src/tournament/**`

Also move projector if it is pure:

- `src-tauri/src/domain/projector.rs` → `crates/poker-core/src/projector.rs` or `crates/poker-core/src/domain/projector.rs`

### Required change

Add `crates/poker-core/src/lib.rs`:

```rust
pub mod domain;
pub mod engine;
pub mod tournament;

// If moved outside domain:
// pub mod projector;

pub use domain::*;
pub use engine::*;
pub use tournament::*;
```

Then update Tauri imports.

Example replacements:

```rust
// before
use crate::domain::{ActionType, TournamentState};
use crate::tournament::TournamentController;

// after
use poker_core::domain::{ActionType, TournamentState};
use poker_core::tournament::TournamentController;
```

Inside moved core modules, replace `crate::domain`/`crate::engine` paths as needed. Since they now live together in `poker-core`, many internal paths can stay `crate::domain`, `crate::engine`, etc.

In `src-tauri/src/lib.rs` or `src-tauri/src/main.rs`, remove `mod domain; mod engine; mod tournament;` declarations after consumers import from `poker_core`.

### Dependency rule

If a moved file pulls in `tauri`, `keyring`, `dirs`, `reqwest`, `local-ip-address`, `std::net`, or app-state/networking modules, stop and refactor. That code is not pure enough for `poker-core`.

### Acceptance

- `poker-core` builds without Tauri.
- `poker-core` has no networking/platform/LLM/keychain dependencies.
- Existing engine/tournament/domain tests moved with the modules and pass under `cargo test -p poker-core`.
- Desktop crate imports the moved types from `poker_core`.

---

## P2.3 — Add portable core facade types without changing desktop behavior

**Files:**

- `crates/poker-core/src/lib.rs`
- `crates/poker-core/src/facade.rs` new
- `crates/poker-core/src/error.rs` optional
- tests under `crates/poker-core/src/tests` or module tests

### Problem

The future Android app needs a stable boundary. The desktop app can continue using existing controller types internally for now, but the core should start exposing a simple command/snapshot facade.

### Required change

Add a minimal facade that wraps the existing tournament controller/state. Keep it small. Do not rewrite all desktop code to use it yet unless easy.

Suggested starting point:

```rust
use serde::{Deserialize, Serialize};

use crate::domain::{ActionType, TournamentConfig, TournamentState};
use crate::tournament::TournamentController;

#[derive(Debug, thiserror::Error)]
pub enum PokerCoreError {
    #[error("engine error: {0}")]
    Engine(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineCommand {
    SubmitAction {
        player_id: String,
        action_window_id: String,
        action_type: ActionType,
        raise_to_amount: Option<u32>,
    },
    AdvanceTime {
        now_ms: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineEvent {
    StateChanged,
    Noop,
}

pub struct PokerEngine {
    controller: TournamentController,
}

impl PokerEngine {
    pub fn new(config: TournamentConfig) -> Result<Self, PokerCoreError> {
        Ok(Self {
            controller: TournamentController::new(config)
                .map_err(|error| PokerCoreError::Engine(error.to_string()))?,
        })
    }

    pub fn state(&self) -> &TournamentState {
        self.controller.state()
    }

    pub fn submit_command(
        &mut self,
        command: EngineCommand,
    ) -> Result<Vec<EngineEvent>, PokerCoreError> {
        match command {
            EngineCommand::SubmitAction {
                player_id,
                action_window_id,
                action_type,
                raise_to_amount,
            } => {
                self.controller
                    .submit_action(&player_id, action_window_id, action_type, raise_to_amount)
                    .map_err(|error| PokerCoreError::Engine(error.to_string()))?;
                Ok(vec![EngineEvent::StateChanged])
            }
            EngineCommand::AdvanceTime { now_ms } => {
                self.controller
                    .advance_time(now_ms)
                    .map_err(|error| PokerCoreError::Engine(error.to_string()))?;
                Ok(vec![EngineEvent::StateChanged])
            }
        }
    }

    pub fn export_state_json(&self) -> Result<String, PokerCoreError> {
        serde_json::to_string(self.state())
            .map_err(|error| PokerCoreError::Serialization(error.to_string()))
    }
}
```

Adjust method names to match the actual `TournamentController` API.

### Important

This facade is not the final Android API. It is a stepping stone that proves `poker-core` can expose a stable command/state boundary.

### Tests

Add tests:

```rust
#[test]
fn poker_engine_exports_state_json() {
    let config = test_config();
    let engine = PokerEngine::new(config).unwrap();
    let json = engine.export_state_json().unwrap();
    assert!(json.contains("phase"));
}

#[test]
fn poker_engine_advance_time_command_is_deterministic() {
    let config = test_config();
    let mut a = PokerEngine::new(config.clone()).unwrap();
    let mut b = PokerEngine::new(config).unwrap();

    let events_a = a.submit_command(EngineCommand::AdvanceTime { now_ms: 1234 }).unwrap();
    let events_b = b.submit_command(EngineCommand::AdvanceTime { now_ms: 1234 }).unwrap();

    assert_eq!(events_a, events_b);
    assert_eq!(a.export_state_json().unwrap(), b.export_state_json().unwrap());
}
```

### Acceptance

- `poker-core` exposes a small facade with no platform dependencies.
- The facade is covered by tests.
- Existing desktop behavior does not regress.

---

## P2.4 — Document Android/Kotlin integration boundary

**Files:**

- `docs/ANDROID_ARCHITECTURE.md` new or update existing docs
- `README.md` optional

### Required change

Create a short architecture doc explaining:

- Android will be native Kotlin/Compose, not Tauri Mobile.
- Kotlin owns Android UI, lifecycle, permissions, storage integration, and networking.
- Rust `poker-core` owns rules/state/projection only.
- Kotlin networking messages are translated into core commands.
- Kotlin must not reimplement poker legality, pot settlement, showdown, blinds, or hidden-card projection.
- Future Rust binding layer should probably use UniFFI unless there is a concrete reason to use hand-written JNI.

Suggested doc skeleton:

```md
# Android Architecture

The Android app will be a native Kotlin/Compose app. It will not use Tauri Mobile.

## Boundary

Kotlin owns:
- Compose UI
- ViewModel/Repository state flow
- Android lifecycle
- permissions
- storage integration
- networking/session transport

Rust `poker-core` owns:
- poker rules
- tournament state transitions
- legal action validation
- dealing/shuffling
- showdown/settlement
- public/private projection
- deterministic state serialization

## Networking

Networking is platform/session adapter code. It is not part of `poker-core`.

Incoming network messages should be validated/routed by Kotlin and converted into `EngineCommand`s. The core returns events/snapshots. Kotlin sends resulting updates over Android-owned networking and updates Compose state.

## Binding direction

Preferred future binding: UniFFI. Hand-written JNI is acceptable only if UniFFI cannot represent the needed API cleanly.
```

### Acceptance

- Docs clearly state networking is not part of `poker-core`.
- Docs clearly state Android is Kotlin + Rust, not Tauri Mobile.

---

## P2.5 — Audit `poker-core` dependency purity

**Files:**

- `crates/poker-core/Cargo.toml`
- moved core files

### Required validation

Run:

```bash
cargo tree -p poker-core
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|Mutex<.*Tcp|thread::spawn|Command" crates/poker-core src-tauri/src/domain src-tauri/src/engine src-tauri/src/tournament
```

After the move, `src-tauri/src/domain`, `src-tauri/src/engine`, and `src-tauri/src/tournament` should ideally not exist except maybe temporary re-export shims. If shims remain, document why and remove them soon.

### Acceptance

- `poker-core` has no Tauri dependency.
- `poker-core` has no socket/networking dependency.
- `poker-core` has no LLM/provider/keychain dependency.
- `poker-core` has no Android/desktop UI dependency.

---

## P3.1 — Prepare, but do not implement, future Android FFI crate

**Files:**

- optional `docs/ANDROID_ARCHITECTURE.md`
- do not add a full Android app in this task

### Required change

Do not implement the Android app yet. Do not add Tauri Mobile. Do not move networking into Rust core.

Optionally add a TODO note for future crates:

```text
crates/poker-android-ffi/
  depends on poker-core
  exposes UniFFI/JNI-safe DTOs
  no Tauri dependency
  no Android UI code inside Rust
```

### Acceptance

- No half-built Android project is added during Fix 5.
- The future direction is documented clearly enough for the next pass.

---

## Final validation commands

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

If Node 24 is not active, switch to Node 24 first because `package.json` declares Node `24.x`.

Also run these focused audits:

```bash
rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated" src-tauri/src/networking
rg -n "hole_cards_by_player_id.*unwrap_or\(|unwrap_or\(&\[\]\)" src-tauri/src/npc src-tauri/src/tournament src-tauri/src/networking
rg -n "__DESKTOP_POKER_BROWSER_MOCKS__|LayoutProbeApp" dist || true
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn" crates/poker-core
```

## Definition of done

- [ ] NPC add rolls back/removes NPCs if runner spawn fails.
- [ ] Client private-message AAD failure emits warning and drops frame; no empty-AAD fallback.
- [ ] Acting NPC with missing/invalid hole cards records error and submits no action.
- [ ] Full provider config save cannot leave new settings behind after secret write/delete failure.
- [ ] Window persistence tests emit no expected errors.
- [ ] Host runtime health covers remaining important host failures or documents safe ignored results.
- [ ] Layout probe/mock setup is dev-only and not statically imported by production entrypoint.
- [ ] `memory.md` is corrected.
- [ ] Cargo workspace exists.
- [ ] `poker-core` crate exists and owns pure domain/engine/tournament/projector logic.
- [ ] `poker-core` has no networking/Tauri/platform/LLM/keychain dependencies.
- [ ] Desktop Tauri crate depends on `poker-core` and still passes tests.
- [ ] Android architecture doc says Kotlin owns networking and `poker-core` owns poker rules/state/projection.
