# Desktop Poker Fix 6 TODO — Complete Hardening and Extract Shared `poker-core`

This TODO is intentionally explicit for Claude Code. Do the tasks in priority order. Prefer small, test-backed patches. Do not introduce hidden fallback behavior to make tests pass.

Fix 5 was only partially completed. Do not mark Fix 6 complete until every acceptance item here is done and validated.

---

## P0.1 — Fix window persistence test noise for all tests

**Files:**

- `src/test/setup.ts`
- `src/app/persistence.ts`
- `src/app/persistence.test.ts`
- `vite.config.ts`

### Problem

`npm test` passes, but it still prints repeated errors like:

```text
Failed to initialize window state persistence.
TypeError: Cannot read properties of undefined (reading 'currentWindow')
```

Expected stderr noise is not acceptable. It hides real failures.

The likely current cause is test-global leakage:

- `persistence.test.ts` sets `window.__TAURI_INTERNALS__`.
- Single-threaded Vitest reuses the same jsdom worker.
- Later tests mount app providers that call `initializeWindowStatePersistence(...)`.
- The runtime guard sees a fake Tauri global and enters a half-mocked Tauri window path.

### Required change

Fix this at the root cause. Do **not** suppress `console.error` globally.

At minimum, clean up fake Tauri globals after every test in global setup.

Suggested addition to `src/test/setup.ts`:

```ts
import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach, vi } from "vitest";

(
  globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

type TauriGlobalWindow = Window & {
  __TAURI_INTERNALS__?: unknown;
  __TAURI__?: unknown;
};

function clearTauriTestGlobals() {
  const maybeWindow = window as TauriGlobalWindow;
  delete maybeWindow.__TAURI_INTERNALS__;
  delete maybeWindow.__TAURI__;
}

afterEach(() => {
  cleanup();
  clearTauriTestGlobals();
  vi.restoreAllMocks();
});

beforeEach(() => {
  clearTauriTestGlobals();
  localStorage.clear();
  Object.defineProperty(window.navigator, "clipboard", {
    configurable: true,
    value: {
      writeText: vi.fn().mockResolvedValue(undefined),
    },
  });
});
```

If `vi.restoreAllMocks()` breaks existing tests because they rely on a persistent mock, repair those tests instead of allowing global state leakage.

Also strengthen `persistence.ts` so non-Tauri/jsdom paths return quietly before dynamic import.

Suggested helper:

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

Then use it:

```ts
export function initializeWindowStatePersistence(storageNamespace: string) {
  if (!hasUsableTauriWindowRuntime()) {
    return () => {};
  }

  // existing Tauri path
}
```

If some tests intentionally exercise the Tauri window path, add a proper test-scoped mock for `@tauri-apps/api/window` in those tests. Do not leave fake Tauri globals behind.

Example local mock shape:

```ts
vi.mock("@tauri-apps/api/window", () => {
  const currentWindow = {
    innerSize: vi.fn().mockResolvedValue({ width: 1280, height: 720 }),
    outerPosition: vi.fn().mockResolvedValue({ x: 0, y: 0 }),
    isMaximized: vi.fn().mockResolvedValue(false),
    maximize: vi.fn().mockResolvedValue(undefined),
    setSize: vi.fn().mockResolvedValue(undefined),
    setPosition: vi.fn().mockResolvedValue(undefined),
    onMoved: vi.fn().mockResolvedValue(() => undefined),
    onResized: vi.fn().mockResolvedValue(() => undefined),
  };

  return {
    getCurrentWindow: () => currentWindow,
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

### Tests

Run:

```bash
npm test 2>&1 | tee /tmp/desktop-poker-npm-test.log
rg -n "Failed to initialize window state persistence|currentWindow" /tmp/desktop-poker-npm-test.log
```

The `rg` command must find nothing.

### Acceptance

- `npm test` passes.
- `npm test` emits no expected `Failed to initialize window state persistence.` error.
- No test suppresses `console.error` merely to hide this issue.
- Fake Tauri globals do not leak across tests.

---

## P0.2 — Remove layout probe/browser mock setup from production bundle

**Files:**

- `src/main.tsx`
- `src/app/runtimeGate.ts`
- `src/probe/LayoutProbeApp.tsx`
- frontend tests if needed

### Problem

Fix 5 removed the static import, but the production build still emitted a `LayoutProbeApp` chunk containing `window.__DESKTOP_POKER_BROWSER_MOCKS__`.

That fails the intent. Production builds should not ship mock setup code.

### Required change

Put the dynamic import behind a compile-time `import.meta.env.DEV` branch so Vite can fully remove it from production.

Suggested replacement for `src/main.tsx`:

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

Important details:

- Do not import `resolveLayoutProbeSurface` at top level if it exists only for dev/probe behavior.
- Do not import `LayoutProbeApp` at top level.
- Do not compute `layoutProbe` outside the `if (import.meta.env.DEV)` block.
- Remove comments claiming a module is absent from production unless the `dist` grep proves it.

### Tests / validation

Run:

```bash
npm run build
rg -n "__DESKTOP_POKER_BROWSER_MOCKS__|LayoutProbeApp" dist
```

Expected result: no hits in production JS payload.

If the only hits are in generated source maps and source maps are not shipped, either disable production source maps or document the exact packaging rule. The production JS must not contain the mock assignment.

### Acceptance

- `src/main.tsx` has no static `LayoutProbeApp` import.
- Production runtime cannot enter layout probe mode.
- Production build does not contain `window.__DESKTOP_POKER_BROWSER_MOCKS__` setup code.
- `npm run build` passes.

---

## P0.3 — Make NPC atomic add honest when snapshot sync fails

**Files:**

- `src-tauri/src/networking/runtime/host.rs`
- runtime tests around NPC add/remove if available
- app-state NPC tests if needed

### Problem

`HostServer::add_npc_participants_atomic(...)` mutates authoritative state and then calls `sync_snapshots_to_clients()?`.

If snapshot sync fails after the mutation, the method returns `Err` while the NPCs remain registered/seated/ready. That is a caller-visible lie: the command appears to have failed, but authoritative state changed.

The same pattern may exist in `remove_npc_participants_atomic`, `claim_seat`, and `set_ready_state`.

### Required change

Pick an honest semantic and implement it consistently.

Preferred semantic for lobby mutation methods:

- The authoritative state mutation is the command.
- Snapshot sync is post-mutation publication.
- If mutation succeeds but sync fails, record health/debug diagnostics and return success for the mutation.

This avoids pretending a successfully applied command failed.

Add a helper to record sync failure without turning the mutation into an error.

Suggested helper in `host.rs`:

```rust
impl HostServer {
    fn sync_snapshots_after_lobby_mutation(&self, operation: &str) {
        if let Err(error) = self.sync_snapshots_to_clients() {
            let message = format!("{operation}: lobby mutation applied but snapshot sync failed: {error}");
            update_health(&self.runtime_health, |health| {
                health.publish_error_count += 1;
                health.record_error(message);
            });
        }
    }
}
```

Then update methods like:

```rust
pub fn add_npc_participants_atomic(
    &self,
    assignments: Vec<super::NpcSeatAssignment>,
) -> Result<Vec<String>, NetworkingError> {
    let player_ids: Vec<String> = assignments.iter().map(|a| a.player_id.clone()).collect();

    self.update_lobby_state(|state| {
        // existing validation + mutation
        Ok(())
    })?;

    self.sync_snapshots_after_lobby_mutation("add_npc_participants_atomic");
    Ok(player_ids)
}
```

For `remove_npc_participants_atomic`:

```rust
pub fn remove_npc_participants_atomic(
    &self,
    player_ids: &[String],
) -> Result<(), NetworkingError> {
    self.update_lobby_state(|state| {
        // existing validation + removal
        Ok(())
    })?;

    self.sync_snapshots_after_lobby_mutation("remove_npc_participants_atomic");
    Ok(())
}
```

If the project strongly prefers snapshot sync failure to fail the command, then implement rollback before returning error. Do not leave the current halfway behavior.

### Tests

Add one focused test if practical:

- arrange a host server with a connected client whose stream write fails during sync;
- call `add_npc_participants_atomic`;
- assert the method returns `Ok(...)` if using preferred semantics;
- assert NPCs are present in authoritative state;
- assert `HostRuntimeHealth.publish_error_count > 0` or `last_error` mentions snapshot sync failure.

If simulating write failure is awkward, add a smaller unit test for the helper and document that full runtime coverage is pending.

### Acceptance

- No host lobby mutation method returns a plain error after applying authoritative state solely because post-mutation snapshot sync failed.
- Sync failure is visible through `HostRuntimeHealth` or equivalent diagnostics.
- The chosen semantics are documented in comments.

---

## P0.4 — Finish host/client runtime health and quiet-failure audit

**Files:**

- `src-tauri/src/networking/runtime/host.rs`
- `src-tauri/src/networking/runtime/host_session.rs`
- `src-tauri/src/networking/runtime/client_connect.rs`
- `src-tauri/src/networking/runtime/mod.rs`
- runtime tests if practical

### Problem

Fix 5 added more health counters, but important runtime failures still bypass health diagnostics or return paths.

Examples seen in the latest code review:

- `spawn_host_client_session` uses raw `thread::spawn`.
- `host_session.rs` has repeated `let _ = clients.lock().map(...)`.
- `host_session.rs` has repeated `let _ = mark_participant_reconnect_eligible(...)`.
- `client_connect.rs` ignores read/write timeout setup failures.
- host tick/publish code can use `unwrap_or_else(|_| state.clone())` for authoritative state lock failure.

### Required change

Every important host/client runtime failure must do one of these:

1. increment health and set `last_error`;
2. emit a structured runtime/debug event;
3. return a real error to the caller;
4. include a comment explaining why the ignored failure is safe during shutdown/test cleanup.

### Add helper functions for repeated disconnect cleanup

In `host_session.rs`, introduce explicit helpers instead of repeating silent cleanup blocks.

Suggested helper:

```rust
fn remove_client_or_record_health(
    clients: &Arc<Mutex<HashMap<String, ConnectedClient>>>,
    runtime_health: &Arc<Mutex<HostRuntimeHealth>>,
    player_id: &str,
) {
    match clients.lock() {
        Ok(mut connected_clients) => {
            connected_clients.remove(player_id);
        }
        Err(_) => update_health(runtime_health, |health| {
            health.record_client_registry_error();
        }),
    }
}

fn mark_reconnect_or_record_health(
    authoritative_state: &Arc<Mutex<TournamentState>>,
    runtime_health: &Arc<Mutex<HostRuntimeHealth>>,
    player_id: &str,
) {
    if let Err(error) = mark_participant_reconnect_eligible(authoritative_state, player_id) {
        update_health(runtime_health, |health| {
            health.reconnect_mark_error_count += 1;
            health.record_error(format!(
                "failed to mark participant {player_id} reconnect-eligible: {error}"
            ));
        });
    }
}

fn disconnect_client_or_record_health(
    clients: &Arc<Mutex<HashMap<String, ConnectedClient>>>,
    authoritative_state: &Arc<Mutex<TournamentState>>,
    runtime_health: &Arc<Mutex<HostRuntimeHealth>>,
    player_id: &str,
) {
    remove_client_or_record_health(clients, runtime_health, player_id);
    mark_reconnect_or_record_health(authoritative_state, runtime_health, player_id);
}
```

Use the helper everywhere the code currently does:

```rust
let _ = clients.lock().map(|mut connected_clients| {
    connected_clients.remove(&player_id);
});
let _ = mark_participant_reconnect_eligible(&authoritative_state, &player_id);
```

### Replace raw session thread spawn

Change `spawn_host_client_session` so thread spawn can fail and health records it.

Preferred signature:

```rust
pub(crate) fn spawn_host_client_session(
    // existing args...
    runtime_health: Arc<Mutex<HostRuntimeHealth>>,
) -> Result<(), NetworkingError> {
    thread::Builder::new()
        .name(format!("desktop-poker-client-{player_id}"))
        .spawn(move || {
            // existing loop
        })
        .map(|_| ())
        .map_err(|error| {
            update_health(&runtime_health, |health| {
                health.client_session_spawn_error_count += 1;
                health.record_error(format!("failed to spawn host client session: {error}"));
            });
            NetworkingError::new(format!("failed to spawn host client session: {error}"))
        })
}
```

Then update caller in `host.rs`:

```rust
if let Err(error) = spawn_host_client_session(
    player_id.clone(),
    stream,
    authoritative_state,
    tournament_runtime,
    clients,
    join_payload,
    server_sequence,
    host_signing_keys,
    host_encryption_keys,
    public_events,
    runtime_health_conn2,
) {
    update_health(&runtime_health_conn2, |health| {
        health.client_session_spawn_error_count += 1;
        health.record_error(error.to_string());
    });
}
```

Avoid double-counting if the callee already records the error.

### Fix client timeout setup

In `client_connect.rs`, replace ignored timeout setup:

```rust
let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
```

with:

```rust
stream
    .set_read_timeout(Some(Duration::from_secs(5)))
    .map_err(|error| NetworkingError::new(format!("failed to set client read timeout: {error}")))?;
stream
    .set_write_timeout(Some(Duration::from_secs(5)))
    .map_err(|error| NetworkingError::new(format!("failed to set client write timeout: {error}")))?;
```

A failed timeout setup can change behavior from bounded wait to indefinite blocking, so this should fail loudly.

### Fix authoritative-state lock fallback

Do not do this in host tick/publish code:

```rust
.unwrap_or_else(|_| state.clone())
```

For authoritative state lock poisoning, record health and skip/break instead.

Suggested pattern:

```rust
let previous_state = match authoritative_state.lock() {
    Ok(authoritative) => authoritative.clone(),
    Err(_) => {
        update_health(&runtime_health, |health| {
            health.state_lock_error_count += 1;
            health.record_error("authoritative state lock poisoned before runtime publish");
        });
        break;
    }
};

match authoritative_state.lock() {
    Ok(mut authoritative) => {
        *authoritative = state.clone();
    }
    Err(_) => {
        update_health(&runtime_health, |health| {
            health.state_lock_error_count += 1;
            health.record_error("authoritative state lock poisoned during runtime writeback");
        });
        break;
    }
}
```

### Audit command

Run:

```bash
rg -n "let _ =|thread::spawn|clients\.lock\(|mark_participant_reconnect|unwrap_or_else\(\|_\| state\.clone\(\)|continue;|return;" src-tauri/src/networking/runtime
```

For every hit in touched files:

- convert to structured handling; or
- add a short comment explaining why it is safe.

Examples of safe comments:

```rust
// Best-effort shutdown wake-up: the listener may already be closed.
let _ = TcpStream::connect(self.listener_addr);
```

```rust
// Best-effort shutdown join: dropping the server should not panic if the worker panicked.
let _ = join_handle.join();
```

### Acceptance

- Client timeout setup failures return errors.
- Host session spawn failure is not a raw panic path.
- Important client-registry/reconnect failures update `HostRuntimeHealth`.
- Authoritative-state lock poisoning is not converted into invented state.
- Remaining ignored results in runtime code are documented as safe best-effort cleanup.

---

## P0.5 — Correct `memory.md` before claiming Fix 6 completion

**Files:**

- `memory.md`

### Problem

The current `memory.md` overstates Fix 5 completion. It claims or implies some things are complete even though review showed they are not.

### Required change

Add a corrective entry near the top or in chronological order using the repo's timestamp convention. Do not fabricate the timestamp. Use:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

Suggested entry:

```md
- <timestamp> — Fix 6 correction: Fix 5 was only partially complete. Frontend lint/build/test passed, but npm test still printed expected window persistence errors; production dist still contained LayoutProbeApp / __DESKTOP_POKER_BROWSER_MOCKS__; no root Cargo workspace or crates/poker-core existed; Android architecture doc was missing; host runtime health still had quiet failure gaps. Fix 6 tracks completing those hardening gaps and actually extracting pure poker rules/state/projection into a shared Rust poker-core crate. Architecture decision: Android will be native Kotlin/Compose + Rust bindings, not Tauri Mobile; Kotlin owns networking/session transport, poker-core owns deterministic rules/state/projection only.
```

After all Fix 6 tasks pass validation, add a second completion entry listing the actual commands run.

Suggested completion entry:

```md
- <timestamp> — Fix 6 completed: npm ci, npm run lint, npm run build, npm test, cargo fmt --check, cargo clippy --workspace --all-targets --all-features -- -D warnings, cargo test --workspace --all-targets --all-features, cargo test -p poker-core, and cargo tree -p poker-core passed. npm test emitted no expected window-persistence errors. Production dist no longer contains LayoutProbeApp or __DESKTOP_POKER_BROWSER_MOCKS__. poker-core now owns pure domain/engine/tournament/projector logic with no Tauri/networking/platform/LLM/keychain dependencies. Android architecture doc records Kotlin-owned networking and Rust-owned rules/state/projection.
```

Only add the completion entry after the commands actually pass.

### Acceptance

- `memory.md` no longer says Fix 5 was fully complete.
- `memory.md` records the Android/Kotlin + Rust core architecture decision.
- Completion entry is only added after validation.

---

## P1.1 — Verify existing P0 hardening stays fixed

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- `src-tauri/src/npc/runner/action.rs`
- `src-tauri/src/npc/provider_storage.rs`
- tests in relevant modules

### Problem

Fix 5 fixed several P0 issues. Fix 6 should not regress them while doing broader refactors.

### Required checks

Run these audits:

```bash
rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated" src-tauri/src/networking
rg -n "hole_cards_by_player_id.*unwrap_or\(|unwrap_or\(&\[\]\)" src-tauri/src/npc src-tauri/src/tournament src-tauri/src/networking
rg -n "save_provider_config|save_provider_settings_only" src-tauri/src/npc/provider_storage.rs
```

Manual verification:

- Private-message AAD serialization failure emits `ClientRuntimeEvent::ProtocolWarning` and drops the frame.
- Acting NPC must have exactly two hole cards before rule-based or LLM-backed decision code runs.
- Full provider config save updates/deletes the secret before writing settings.
- Settings-only provider switch remains fail-loud and transactional.
- Runner-spawn failure rollback remains implemented and covered by tests.

### Optional stronger NPC tests

If current tests only hit `record_npc_internal_error(...)` directly, add at least one integration-style test that exercises the actual NPC action path with corrupted hole-card state.

Test intent:

```rust
#[test]
fn acting_npc_missing_hole_cards_submits_no_action() {
    // Arrange a host session with an acting NPC and a live action window.
    // Corrupt authoritative hand state by removing that NPC's hole-card entry.
    // Act: run one NPC action attempt.
    // Assert:
    // - outcome is non-success / RuntimeUnavailable equivalent;
    // - last_npc_action_error.reason == InternalError;
    // - message mentions missing hole cards;
    // - hand log/action count did not increase.
}
```

### Acceptance

- No prior P0 hardening fix regresses.
- Any missing direct integration coverage is added or explicitly documented as impractical.

---

## P2.1 — Add root Cargo workspace and `crates/poker-core` skeleton

**Files:**

- root `Cargo.toml` new
- `crates/poker-core/Cargo.toml` new
- `crates/poker-core/src/lib.rs` new
- `src-tauri/Cargo.toml`
- `CLAUDE.md`
- `.claude/skills/lint-n-test/SKILL.md` if present
- any docs/scripts that run cargo commands

### Problem

All Rust code currently lives under the Tauri crate. That blocks a clean Android Kotlin + Rust architecture.

### Required change

Create a root Cargo workspace.

Root `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/poker-core",
    "src-tauri",
]
resolver = "2"
```

Create `crates/poker-core/Cargo.toml`:

```toml
[package]
name = "poker-core"
version = "0.1.0"
edition = "2021"
description = "Shared deterministic poker engine core for desktop and Android adapters"
license = "MIT"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
```

Adjust the license if the repo uses a different license.

Create initial `crates/poker-core/src/lib.rs`:

```rust
//! Shared deterministic poker engine core.
//!
//! This crate intentionally contains no Tauri, networking, keychain, LLM,
//! Android UI, or desktop UI dependencies. Platform adapters own transport,
//! storage integration, and presentation.

pub mod domain;
pub mod engine;
pub mod tournament;

pub mod error;
pub mod facade;
```

Update `src-tauri/Cargo.toml`:

```toml
[dependencies]
poker-core = { path = "../crates/poker-core" }
# existing dependencies stay here
```

If `src-tauri/Cargo.toml` has a `[workspace]` section, remove it or consolidate it into root `Cargo.toml`.

Update cargo command docs. Search:

```bash
rg -n "cargo fmt|cargo clippy|cargo test|--manifest-path src-tauri/Cargo.toml|cd src-tauri" CLAUDE.md .claude docs README.md package.json
```

Replace old command guidance with workspace commands:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test -p poker-core
cargo tree -p poker-core
```

### Tests / validation

Run:

```bash
cargo metadata
cargo test -p poker-core
```

At this stage `poker-core` may have only empty modules or moved modules depending on task order. It must still compile.

### Acceptance

- Root `Cargo.toml` exists and is the workspace root.
- `crates/poker-core` exists.
- `src-tauri` is a workspace member.
- Cargo command docs are workspace-aware.
- No nested conflicting workspace remains in `src-tauri/Cargo.toml`.

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

Also move projector if pure:

- `src-tauri/src/domain/projector.rs` → `crates/poker-core/src/domain/projector.rs`

### Problem

The desktop crate still owns pure poker logic. Android cannot reuse it cleanly without pulling in desktop/Tauri concerns.

### Required change

Move pure modules into `poker-core`.

Inside `poker-core`, internal paths like `crate::domain`, `crate::engine`, and `crate::tournament` can usually remain the same.

In the desktop crate, update imports from:

```rust
use crate::domain::{ActionType, TournamentState};
use crate::tournament::TournamentController;
```

to:

```rust
use poker_core::domain::{ActionType, TournamentState};
use poker_core::tournament::TournamentController;
```

Remove these declarations from the Tauri crate once imports are updated:

```rust
mod domain;
mod engine;
mod tournament;
```

Prefer explicit imports such as `poker_core::domain::ActionType`, not broad crate-root glob re-exports.

If import churn becomes large, add a prelude rather than dumping every core type into the crate root:

```rust
pub mod prelude {
    pub use crate::domain::{ActionType, Card, TournamentConfig, TournamentState};
    pub use crate::tournament::TournamentController;
}
```

Do not add `pub use domain::*;` at crate root unless there is a concrete reason.

### Dependency rule

If a moved file imports any of these, stop and refactor before moving it:

- `tauri`
- `keyring`
- `dirs`
- `reqwest`
- `local-ip-address`
- `std::net`
- `TcpStream`, `TcpListener`, `UdpSocket`, socket types
- app-state modules
- networking modules
- LLM/provider modules
- OS window/UI code

### Projector guidance

Move projector only if it is pure projection over `TournamentState` and domain types. It should not depend on protocol envelope types, networking events, Tauri commands, or app-state debug types.

Good projector location:

```rust
// crates/poker-core/src/domain/mod.rs
pub mod projector;
```

or:

```rust
// crates/poker-core/src/lib.rs
pub mod projector;
```

Pick whichever causes less import churn. Keep semantics unchanged.

### Tests

Move existing domain/engine/tournament/projector tests with the modules.

Run:

```bash
cargo test -p poker-core
cargo test --workspace --all-targets --all-features
```

### Acceptance

- `poker-core` builds without Tauri.
- `poker-core` owns pure domain/engine/tournament/projector logic.
- Tauri crate imports moved types from `poker_core`.
- No duplicate stale copies remain under `src-tauri/src/domain`, `src-tauri/src/engine`, or `src-tauri/src/tournament`, except temporary documented shims.
- Existing behavior does not change.

---

## P2.3 — Add portable `PokerEngine` facade in `poker-core`

**Files:**

- `crates/poker-core/src/facade.rs`
- `crates/poker-core/src/error.rs`
- `crates/poker-core/src/lib.rs`
- facade tests

### Problem

The future Android app needs a small stable boundary. The desktop app can continue using existing controller types for now, but `poker-core` should expose a simple command/state facade to prove the boundary is portable.

### Required change

Add a minimal facade around existing tournament controller/state.

Suggested `crates/poker-core/src/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PokerCoreError {
    #[error("engine error: {0}")]
    Engine(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}
```

Suggested `crates/poker-core/src/facade.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::{
    domain::{ActionType, TournamentConfig, TournamentState},
    error::PokerCoreError,
    tournament::TournamentController,
};

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

Adjust method names to match the real `TournamentController` API. Do not invent a fake facade that compiles but is disconnected from the real engine.

Update `lib.rs`:

```rust
pub mod domain;
pub mod engine;
pub mod error;
pub mod facade;
pub mod tournament;
```

### Tests

Add tests in `facade.rs` or `tests/facade.rs`.

Suggested test shape:

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

    let events_a = a
        .submit_command(EngineCommand::AdvanceTime { now_ms: 1234 })
        .unwrap();
    let events_b = b
        .submit_command(EngineCommand::AdvanceTime { now_ms: 1234 })
        .unwrap();

    assert_eq!(events_a, events_b);
    assert_eq!(a.export_state_json().unwrap(), b.export_state_json().unwrap());
}
```

If `TournamentController::new(config)` uses random seeding, add a deterministic constructor or config-provided seed before writing this test. Do not write a flaky determinism test.

### Acceptance

- `poker-core` exposes `PokerEngine`, `EngineCommand`, `EngineEvent`, and `PokerCoreError`.
- Facade uses the real tournament controller/state.
- Facade has tests.
- Desktop behavior does not regress.

---

## P2.4 — Add Android/Kotlin architecture document

**Files:**

- `docs/ANDROID_ARCHITECTURE.md` new
- `README.md` optional

### Required change

Create `docs/ANDROID_ARCHITECTURE.md`.

Suggested content:

```md
# Android Architecture

The Android app will be a native Kotlin/Compose app. It will not use Tauri Mobile.

## Boundary

Kotlin owns:

- Compose UI
- ViewModel / Repository state flow
- Android lifecycle
- permissions
- storage integration
- networking / session transport
- platform notifications, haptics, and sound

Rust `poker-core` owns:

- poker rules
- tournament state transitions
- legal action validation
- dealing and shuffling
- blind and ante progression
- betting rounds
- showdown and settlement
- public/private projection
- deterministic state serialization

## Networking

Networking is platform/session adapter code. It is not part of `poker-core`.

Incoming Android network messages should be validated and routed by Kotlin, then converted into `EngineCommand`s or equivalent Rust-core commands. The core returns events/snapshots. Kotlin sends resulting network updates and updates Compose state.

Kotlin must not reimplement poker legality, pot settlement, showdown, blind progression, or hidden-card projection.

## Binding direction

Preferred future binding: UniFFI.

Hand-written JNI is acceptable only if UniFFI cannot represent the needed API cleanly.

A future binding crate may look like:

```text
crates/poker-android-ffi/
  depends on poker-core
  exposes UniFFI/JNI-safe DTOs
  no Tauri dependency
  no Android UI code inside Rust
  no networking inside poker-core
```

## Desktop relationship

The Tauri desktop app is one platform adapter. It may own desktop-specific session management, Tauri commands, keychain integration, and desktop networking. It must call into `poker-core` for poker rules/state/projection instead of duplicating rules in the UI layer.
```

### Acceptance

- Doc clearly says Android is Kotlin/Compose + Rust, not Tauri Mobile.
- Doc clearly says Kotlin owns networking/session transport.
- Doc clearly says `poker-core` owns rules/state/projection only.
- Doc warns Kotlin must not duplicate poker rules.

---

## P2.5 — Audit `poker-core` dependency purity

**Files:**

- `crates/poker-core/Cargo.toml`
- moved core files

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
- no Android/desktop UI dependency;
- no process spawning;
- no OS-specific app data path decisions.

If a hit is from comments or docs, decide whether it is useful. Code hits should be removed or moved back to the adapter crate.

### Acceptance

- `cargo tree -p poker-core` is small and platform-neutral.
- The `rg` audit has no code hits for platform/networking dependencies.
- Any remaining textual hits are comments/docs and are harmless.

---

## P3.1 — Do not implement Android app or FFI crate in Fix 6

**Files:**

- docs only, unless a tiny TODO marker is useful

### Required change

Do not create a half-built Android app in this pass.

Do not add Tauri Mobile.

Do not move networking into Rust core.

Do not add a full `poker-android-ffi` crate unless explicitly requested after Fix 6.

Optional TODO note is acceptable:

```text
Future:
crates/poker-android-ffi/
  depends on poker-core
  exposes UniFFI/JNI-safe DTOs
  no Tauri dependency
  no Android UI code inside Rust
```

### Acceptance

- No partial Android project is added.
- Future Android direction is documented clearly.

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

Focused audits:

```bash
npm test 2>&1 | tee /tmp/desktop-poker-npm-test.log
rg -n "Failed to initialize window state persistence|currentWindow" /tmp/desktop-poker-npm-test.log
rg -n "__DESKTOP_POKER_BROWSER_MOCKS__|LayoutProbeApp" dist || true
rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated" src-tauri/src/networking
rg -n "hole_cards_by_player_id.*unwrap_or\(|unwrap_or\(&\[\]\)" src-tauri/src/npc src-tauri/src/tournament src-tauri/src/networking
rg -n "let _ =|thread::spawn|clients\.lock\(|mark_participant_reconnect|unwrap_or_else\(\|_\| state\.clone\(\)" src-tauri/src/networking/runtime
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn|Command" crates/poker-core
```

## Definition of done

- [ ] `npm test` emits no expected window-persistence errors.
- [ ] Production bundle does not contain `LayoutProbeApp` / browser mock setup code.
- [ ] NPC atomic add cannot return a plain error after authoritative mutation solely because snapshot sync failed.
- [ ] Host/client runtime failures are recorded, returned, or documented as safe best-effort cleanup.
- [ ] Prior P0 hardening fixes remain intact: no empty-AAD fallback, no empty NPC hole-card fallback, provider config save ordering remains fail-loud, runner-spawn rollback remains intact.
- [ ] `memory.md` accurately says Fix 5 was partial and only claims Fix 6 completion after validation.
- [ ] Root Cargo workspace exists.
- [ ] `crates/poker-core` exists.
- [ ] Pure domain/engine/tournament/projector logic moved into `poker-core`.
- [ ] Tauri desktop crate imports pure poker logic from `poker_core`.
- [ ] `poker-core` has no Tauri/networking/platform/LLM/keychain dependencies.
- [ ] `poker-core` exposes a small portable facade with tests.
- [ ] Android architecture doc exists and says Kotlin owns networking while Rust owns rules/state/projection.
- [ ] No half-built Android app or Tauri Mobile path is added.
- [ ] All final validation commands pass.
