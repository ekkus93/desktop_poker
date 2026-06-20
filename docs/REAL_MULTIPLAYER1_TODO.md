# REAL_MULTIPLAYER1_TODO.md

This file tracks the work required to remove the remaining demo-backed desktop shell behavior and replace it with a real, working LAN multiplayer poker app.

The goal is not to make the UI look more realistic. The goal is to make the release app actually host, join, synchronize, and play real multiplayer poker across multiple desktop instances using the Rust networking/runtime stack as the source of truth.

Audit note:

- Items marked with **Approved by GPT-5.4 (2026-05-07)** were re-reviewed against the current codebase and the full validation gate (`npm run lint`, `npm run test`, `npm run test:geometry`, `npm run build`, `cargo fmt --check`, `cargo clippy`, and `cargo test`).
- Checked items without that note were not rejected automatically, but they were not fully re-approved in this pass.

## Product definition of done

The work in this file is done only when all of the following are true:

- [x] A host instance can create a real networked tournament session from the desktop UI _(Approved by GPT-5.4, 2026-05-07)_
- [x] A second desktop instance can join that session using a real compact `pkr1_` invite _(Approved by GPT-5.4, 2026-05-07)_
- [x] Host and client both see the same authoritative lobby state _(Approved by GPT-5.4, 2026-05-07)_
- [x] Seat assignment, ready state, tournament start, and table progression are synchronized across instances _(Approved by GPT-5.4, 2026-05-07)_
- [x] Real table actions flow through Rust-owned validation and authority rather than frontend/demo state _(Approved by GPT-5.4, 2026-05-07)_
- [ ] Disconnect, reconnect, and resync work through the live runtime path
- [x] Hand history, tournament completion, and restart-safe persistence reflect real session outcomes
- [x] The release app no longer boots or depends on demo-only controller/runtime code in the player flow _(Approved by GPT-5.4, 2026-05-07)_

## 1. Demo-code audit and removal map

### 1.1 Identify every demo-backed runtime entry point
- [x] Audit [src-tauri/src/app_state/mod.rs](/home/phil/work/desktop_poker/src-tauri/src/app_state/mod.rs) for `demo_controller()` usage and any helper paths that synthesize fake runtime state _(Approved by GPT-5.4, 2026-05-07; `demo_controller()` remains only in the debug inspector runtime path, not the release player flow.)_
- [x] Inventory every Tauri command that currently reads from or writes to demo-only app state rather than the real runtime _(Approved by GPT-5.4, 2026-05-07; live session commands are runtime-backed, while `get_debug_state` remains the debug-only demo surface.)_
- [x] Inventory every frontend route that depends on shell-local poker state instead of Rust-projected session state _(Approved by GPT-5.4, 2026-05-07; the live lobby fallback bug was fixed in this audit so the lobby no longer renders shell-local participant state.)_
- [x] Inventory any fake participant, seat, table, or history builders still reachable from release builds _(Approved by GPT-5.4, 2026-05-07; fake table/runtime builders remain in debug/probe/test support, not the normal release player flow.)_
- [x] Identify which demo helpers can remain as test-only fixtures and which must be deleted outright _(Approved by GPT-5.4, 2026-05-07; the remaining demo runtime helpers should be debug/test-only, while player-flow dependencies are no longer acceptable.)_

### 1.2 Classify demo code by final disposition
- [x] Mark each demo surface as one of: replace, move to tests, dev-only debug, or delete _(Approved by GPT-5.4, 2026-05-07; release player-flow use is replaced, while debug inspector/probe helpers remain dev-only until the later cleanup phase deletes or isolates them fully.)_
- [x] Separate harmless UI scaffolding from dangerous fake-authority code that must not remain in production flows _(Approved by GPT-5.4, 2026-05-07)_
- [x] Identify any docs or tests that currently encode demo behavior as if it were production behavior _(Approved by GPT-5.4, 2026-05-07; this audit updated lobby coverage to assert live-session state instead of shell-local fallback behavior.)_
- [x] Record explicit removal targets so the cleanup phase can prove nothing user-facing still depends on demo state _(Approved by GPT-5.4, 2026-05-07; remaining targets are the debug `DesktopTableRuntime`/`demo_controller()` path and any probe-only fake runtime builders.)_

### 1.3 Define cutover constraints
- [x] Preserve the Rust-owned authority boundary documented in [docs/DESKTOP_ARCHITECTURE.md](/home/phil/work/desktop_poker/docs/DESKTOP_ARCHITECTURE.md) _(Approved by GPT-5.4, 2026-05-07)_
- [x] Keep multi-instance local execution working on one machine via instance-scoped storage and explicit host ports _(Approved by GPT-5.4, 2026-05-07)_
- [x] Avoid moving authoritative game logic into React state while replacing the shell flow _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure the migration path does not temporarily ship a mixed model where some screens are live and others still mutate fake poker state _(Approved by GPT-5.4, 2026-05-07; the lobby fallback bug found in this audit was fixed before approval.)_

---

## 2. Replace the app-state runtime model

### 2.1 Redesign desktop app state around live sessions
- [x] Replace the current demo-backed `DesktopTableRuntime` boot model with a session model that can represent: idle, hosting, joining, connected lobby, active table, reconnecting, completed, and fatal error states
- [x] Define a Rust-side session container that can hold the active `HostServer`, active `ClientRuntime`, host/client identity metadata, and the current projected tournament state
- [x] Decide which session data is authoritative, which is cached projection, and which is UI convenience metadata
- [x] Make session state transitions explicit and validated so invalid route/state combinations cannot occur silently _(Approved by GPT-5.4, 2026-05-07)_

### 2.2 Remove fake bootstrap assumptions
- [x] Change bootstrap detection so release startup does not synthesize a live table via `demo_controller()` _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure bootstrap can represent a clean idle app with no active tournament session _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure launch-time join payload handling feeds the real join flow rather than only pre-populating a form _(Approved by GPT-5.4, 2026-05-07)_
- [x] Surface bootstrap-safe error states for malformed launch payloads, host bind failures, and persistence corruption _(Approved by GPT-5.4, 2026-05-07)_

### 2.3 Define session projection models for the UI
- [x] Replace ad hoc shell-local lobby/table derivation with Rust-projected view models for lobby participants, ready state, host controls, local seat, connection state, and table view
- [x] Define clear Rust-to-frontend data contracts for host-only actions, player actions, observer state, and reconnect state
- [x] Ensure projections distinguish local-player private state from shared public state
- [x] Add projection helpers for tournament completion, elimination, and post-game hand history access

---

## 3. Add real Tauri commands for session lifecycle

### 3.1 Host lifecycle commands
- [x] Add a command to start a real host session using `HostServer::bind` and the current host setup draft _(Approved by GPT-5.4, 2026-05-07)_
- [x] Validate host bind address, advertised LAN address, port selection, table identity, and join token generation in the host command path _(Approved by GPT-5.4, 2026-05-07)_
- [x] Return the live encoded invite from the running host session rather than a detached helper-generated payload _(Approved by GPT-5.4, 2026-05-07)_
- [x] Add a command to stop/tear down a host session cleanly _(Approved by GPT-5.4, 2026-05-07)_
- [x] Add a command to query the active host session status after startup, error, or reconnect events _(Approved by GPT-5.4, 2026-05-07)_

### 3.2 Client lifecycle commands
- [x] Add a command to join a real host session using `ClientRuntime::connect` _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure join commands generate and store a stable local player identity and reconnect material for that session _(Approved by GPT-5.4, 2026-05-07)_
- [x] Add a command to disconnect/leave a live client session safely _(Approved by GPT-5.4, 2026-05-07)_
- [x] Add a command to query active client session status and last known connection state _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure launch-time `--join-payload` and manual join use the same validated command path _(Approved by GPT-5.4, 2026-05-07)_

### 3.3 Session event and projection commands
- [x] Add commands to fetch the current authoritative/projection snapshot for the active session
- [x] Add commands to submit lobby actions such as seat selection, ready toggle, and host start request through Rust authority _(Approved by GPT-5.4, 2026-05-07)_
- [x] Add commands to submit live table actions through the real tournament/runtime path _(Approved by GPT-5.4, 2026-05-07)_
- [x] Add commands or event subscriptions for reconnecting, disconnected, fatal error, and resync-required states
- [x] Ensure command responses are typed and structured so the frontend can render exact causes instead of generic failure text

### 3.4 App-state mutation safety
- [x] Make all session-mutating commands reject invalid states clearly, such as starting when no host session exists or toggling ready before admission _(Approved by GPT-5.4, 2026-05-07)_
- [x] Prevent multiple simultaneous active host/client sessions inside the same instance unless intentionally supported _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure teardown clears only the correct session-scoped data and does not destroy unrelated instance persistence
- [x] Add locking/state-transition rules so Tauri commands cannot race each other into inconsistent session state

---

## 4. Wire the real host flow through the frontend

### 4.1 Host setup screen
- [x] Keep host draft editing local, but route host creation through the new Rust start-host command _(Approved by GPT-5.4, 2026-05-07)_
- [x] Replace any fake “continue to lobby” behavior with a real “host session started” transition _(Approved by GPT-5.4, 2026-05-07)_
- [x] Surface actual bind/listener/invite data from the running host session _(Approved by GPT-5.4, 2026-05-07)_
- [x] Show actionable errors for LAN IP resolution failure, port conflicts, or invalid host config _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure the copied invite always comes from the running host session and matches the actual listener state _(Approved by GPT-5.4, 2026-05-07)_

### 4.2 Host lobby behavior
- [x] Replace shell-local participant rendering with live participant/session projection data from Rust _(Approved by GPT-5.4, 2026-05-07; this audit fixed the remaining lobby fallback bug before approval.)_
- [x] Replace any fake ready/seat status with real participant admission, seat assignment, and ready state from the session _(Approved by GPT-5.4, 2026-05-07)_
- [x] Enable the host-only start control only when the authoritative runtime says starting is allowed _(Approved by GPT-5.4, 2026-05-07)_
- [x] Show host-visible connection state for joined players, including disconnect/reconnect eligibility _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure host lobby updates without requiring screen reloads or route resets _(Approved by GPT-5.4, 2026-05-07)_

### 4.3 Host shutdown and recovery UX
- [x] Add a clear host-side recovery path when the host session fails before play starts _(Approved by GPT-5.4, 2026-05-07)_
- [x] Add a clear host-side teardown path when the host chooses to cancel the session _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure leaving the host flow does not strand a live listener in the background _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure a restarted host instance must start a fresh session rather than silently reviving fake/demo state _(Approved by GPT-5.4, 2026-05-07)_

---

## 5. Wire the real join flow through the frontend

### 5.1 Join screen
- [x] Keep invite parsing and validation on the join screen, but route actual joining through the real Rust join-session command _(Approved by GPT-5.4, 2026-05-07)_
- [x] Distinguish between invite format errors, host unreachable errors, join rejection errors, and reconnect-related errors
- [x] Persist recent valid invites only after successful validation/join policy decisions _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure launch-payload boot flows can join directly when appropriate instead of stopping at review-only shell state _(Approved by GPT-5.4, 2026-05-07)_

### 5.2 Client lobby behavior
- [x] Replace shell-local client lobby state with real session projections from Rust _(Approved by GPT-5.4, 2026-05-07; this audit fixed the last shell-local lobby fallback before approval.)_
- [x] Show the client’s actual admission, seat, ready, and connection status _(Approved by GPT-5.4, 2026-05-07)_
- [x] Reflect host and peer participant changes in near real time _(Approved by GPT-5.4, 2026-05-07)_
- [x] Route to the table only when the live session transitions, not when local UI assumptions say it should _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure a failed join cannot leave partial client session state behind _(Approved by GPT-5.4, 2026-05-07)_

### 5.3 Client disconnect and retry UX
- [x] Surface explicit reconnecting state when the runtime emits reconnect events
- [x] Show safe retry messaging for recoverable network drops
- [x] Show terminal failure messaging when reconnect or resync cannot recover the session
- [x] Ensure a user can leave a broken client session and return to a clean home/join state

---

## 6. Implement real lobby actions and transitions

### 6.1 Admission and seat selection
- [x] Decide whether admission is automatic on join or requires explicit host approval in v1, then implement only that real rule _(Approved by GPT-5.4, 2026-05-07)_
- [x] Replace local seat toggles with Rust-owned seat selection/assignment commands _(Approved by GPT-5.4, 2026-05-07)_
- [x] Enforce seat conflicts, seat release, and host/client visibility through the authoritative state model
- [x] Ensure observers, eliminated players, and active participants are represented distinctly in the lobby and session state

### 6.2 Ready-state flow
- [x] Route ready toggles through Rust-owned validation and session mutation _(Approved by GPT-5.4, 2026-05-07)_
- [x] Broadcast ready-state changes to all participants via the live runtime path _(Approved by GPT-5.4, 2026-05-07)_
- [x] Remove the hardcoded nonfunctional lobby start path such as `canStart = false` _(Approved by GPT-5.4, 2026-05-07)_
- [x] Define and enforce the exact rules for when the host may start the tournament _(Approved by GPT-5.4, 2026-05-07)_

### 6.3 Tournament start cutover
- [x] Implement a real host start-tournament action that advances the authoritative session out of waiting/ready-check into live play _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure both host and clients transition from lobby to main table based on the same runtime event/snapshot _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure any late/duplicate start requests are rejected safely _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure start failure rolls back cleanly or leaves the lobby in a consistent state with a real error _(Approved by GPT-5.4, 2026-05-07)_

---

## 7. Drive the main table from the live runtime

### 7.1 Table projection replacement
- [x] Replace any demo/local table view generation in the player path with real projections sourced from the live authoritative state _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure the local player sees only allowed private information while all participants share the same public state _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure action ownership, turn timers if present, stacks, pot, board, and elimination state are all derived from Rust authority _(Approved by GPT-5.4, 2026-05-07)_
- [x] Remove any fake sample hand/feed/history data from release table routes _(Approved by GPT-5.4, 2026-05-07)_

### 7.2 Table action submission
- [x] Route fold, call, check, bet, raise, and any other supported actions through Rust-owned validation and mutation _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure invalid or stale actions are rejected by Rust and surfaced clearly in the UI _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure successful actions produce synchronized updates across host and all clients _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ensure the host does not get a special fake shortcut path that bypasses the same authority used by clients _(Approved by GPT-5.4, 2026-05-07)_

### 7.3 Event application and refresh model
- [x] Decide whether the frontend will poll snapshots, subscribe to Tauri events, or use a hybrid model for session updates
- [x] Implement one coherent update model for lobby and table state instead of ad hoc refresh behavior
- [x] Ensure public events, private hole cards, reconnect snapshots, and tournament completion all update the UI correctly
- [x] Prevent stale projections from surviving after reconnect/resync snapshots replace local state

---

## 8. Integrate reconnect, resync, and failure recovery into the app flow

### 8.1 Runtime event plumbing
- [x] Expose `ClientRuntimeEvent`-driven reconnect/resync/error states through app state and Tauri-facing APIs
- [ ] Decide how host-side disconnect observations should be surfaced in the lobby/table UI
- [x] Ensure reconnect-related state changes update the same live session container used by normal gameplay
- [x] Ensure fatal runtime errors transition the app into a recoverable screen state instead of silently freezing the old view

### 8.2 Reconnect continuity
- [x] Preserve the minimum reconnect identity and token material required for the current session only
- [x] Ensure reconnect can restore the client into the correct seat, role, and table state
- [x] Ensure reconnect does not duplicate participants or create ghost seats
- [ ] Ensure reconnect after elimination or tournament completion follows the intended product rules

### 8.3 Explicit resync handling
- [x] Surface resync-required states in logs/debug views without exposing confusing technical details in the normal UI
- [x] Ensure a resync snapshot fully replaces stale local projection state
- [x] Ensure resync can recover the lobby and active table, not just the initial join flow
- [x] Add guardrails so repeated resync failure exits cleanly into a safe error state

---

## 9. Make persistence match live multiplayer reality

### 9.1 Keep only valid local persistence
- [x] Preserve local-only preferences such as display name, window state, and host draft defaults where appropriate
- [x] Preserve recent join invites only as local convenience state, not as evidence of an active session
- [x] Remove any persistence assumptions that pretend a demo table is resumable as if it were a real network session

### 9.2 Define session restart behavior
- [x] Decide exactly what happens if the app restarts while hosting
- [x] Decide exactly what happens if the app restarts while joined as a client
- [x] Implement the chosen restart behavior explicitly rather than letting stale boot state simulate recovery
- [ ] Ensure the startup UI communicates whether the prior live session is gone, reconnectable, or intentionally not resumed

### 9.3 History and post-game persistence
- [x] Ensure persisted hand history comes from real completed or in-progress live sessions
- [x] Ensure tournament completion summaries match authoritative runtime results
- [x] Ensure observer/elimination history does not leak private data
- [x] Ensure history persistence remains instance-scoped and does not bleed across local profiles

---

## 10. Remove or isolate remaining demo surfaces

### 10.1 Rust cleanup
- [x] Delete demo-only runtime/controller code that is no longer needed in production paths
- [x] Move any useful deterministic builders into test support modules under clearly test-only boundaries
- [x] Remove unused projection helpers, sample participants, and fake tournament bootstraps from release code
- [x] Remove dead commands and state branches that existed only to support the demo table flow

### 10.2 Frontend cleanup
- [x] Remove shell-local poker state that duplicates authoritative runtime state
- [x] Remove fake lobby/table derivation helpers once the real projection path is in place
- [x] Remove release-route assumptions that a screen may proceed without a live host/client session behind it
- [x] Keep only purely local UI draft state in React context/providers

### 10.3 Debug and developer tools boundaries
- [x] Keep developer-facing debug panels only if they read real runtime/session data or clearly test-only fixtures
- [x] Ensure no debug or mock affordance leaks into normal release player flows
- [x] Gate any diagnostic-only surfaces behind explicit debug configuration where appropriate

---

## 11. Expand automated coverage for real multiplayer behavior

### 11.1 Rust runtime/session tests
- [x] Add or update Rust tests to cover the new app-state session container and state transitions _(Approved by GPT-5.4, 2026-05-07)_
- [x] Add host lifecycle tests for start, stop, bind failure, and host recovery
- [x] Add client lifecycle tests for join, disconnect, reconnect, and teardown
- [x] Add tests proving lobby mutations and start flow update the authoritative state correctly _(Approved by GPT-5.4, 2026-05-07)_

### 11.2 Frontend integration tests
- [x] Replace shell-demo assertions with real session-backed expectations in [src/app/AppShell.integration.test.tsx](/home/phil/work/desktop_poker/src/app/AppShell.integration.test.tsx) _(Approved by GPT-5.4, 2026-05-07)_
- [ ] Add tests that start from Home, host a real session, copy the real invite, and join from a second session context
- [x] Add tests for real lobby ready/start propagation across host and client UI projections _(Approved by GPT-5.4, 2026-05-07)_
- [x] Add tests for reconnecting, join rejection, host unavailable, and resync-required UI states
- [x] Add tests proving the UI cannot reach lobby/table routes without a valid live session state _(Approved by GPT-5.4, 2026-05-07)_

### 11.3 End-to-end multi-instance validation
- [ ] Add a reproducible manual checklist for two local release instances on one machine
- [ ] Add a reproducible manual checklist for two machines on the same LAN
- [ ] Verify host and client can complete at least one full tournament with synchronized outcomes
- [ ] Verify disconnect/reconnect during lobby and during live play
- [ ] Verify host shutdown behavior and client-visible failure handling

### 11.4 Regression coverage for demo removal
- [x] Add tests that fail if release bootstrap reintroduces `demo_controller()` or equivalent fake runtime boot paths _(Approved by GPT-5.4, 2026-05-07)_
- [x] Add tests that fail if lobby start becomes a UI-only toggle detached from Rust authority _(Approved by GPT-5.4, 2026-05-07)_
- [x] Add tests that fail if the table view can be constructed without an active session container _(Approved by GPT-5.4, 2026-05-07)_

---

## 12. Validate packaging, release behavior, and documentation

### 12.1 Release build validation
- [x] Run frontend lint, unit/integration tests, geometry tests, Rust tests, and clippy after each major cutover phase
- [ ] Run `npm run tauri build` after the runtime cutover is in place
- [ ] Verify release binaries can still be launched with separate `--instance-id` values on one machine
- [ ] Verify host port conflicts fail clearly in release builds

### 12.2 Product documentation
- [ ] Update [README.md](/home/phil/work/desktop_poker/README.md) so it describes the real multiplayer flow rather than shell/demo behavior
- [ ] Update any architecture docs that mention temporary demo behavior as if it were acceptable product behavior
- [ ] Add an explicit operator/developer flow for testing real host and client instances locally
- [ ] Document known limitations honestly if any multiplayer behaviors remain intentionally unsupported after this phase

### 12.3 Final acceptance checklist
- [x] Host can create a tournament from the UI and obtain a real invite from the running session _(Approved by GPT-5.4, 2026-05-07)_
- [x] Client can join from the UI using that invite and enter the same authoritative lobby _(Approved by GPT-5.4, 2026-05-07)_
- [x] Ready/start works across instances _(Approved by GPT-5.4, 2026-05-07)_
- [x] Main table play is synchronized across instances _(Approved by GPT-5.4, 2026-05-07)_
- [ ] Reconnect/resync works or fails safely according to documented behavior
- [x] Demo-backed release player flow has been removed

---

## Suggested implementation order

1. Replace the app-state runtime model and add real host/join lifecycle commands.
2. Cut the host and join screens over to those commands.
3. Replace the lobby with live session projections and Rust-owned ready/start actions.
4. Cut the main table over to the real session projection/update model.
5. Remove leftover demo runtime code and tighten automated regressions.
6. Run full multi-instance validation and update docs to match reality.
