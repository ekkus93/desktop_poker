# INT_TEST2_TODO.md

This file tracks the next recommended integration-test expansion work after the first integration wave in `docs/INT_TEST1_TODO.md`.

It is focused on the highest-value remaining gaps that are still plausible to miss even with the current shell, runtime, and protocol coverage:

- bootstrap subscription updates that change live shell behavior after first render
- ready-room and start-gating flows across real shell routing
- action failure and recovery behavior after the table is already live
- reconnect-aware boot/restart behavior at the app-shell layer
- multi-instance/debug-launch flows that cross shell, bridge, and persistence boundaries
- targeted Rust end-to-end regressions exposed by real reconnect/action timing
- honest manual-only smoke checks for full Tauri process behavior where in-repo automation still stops short

The goal is to catch cross-layer regressions that sit between the current unit coverage and the already-landed first integration wave, without adding mock-only tests or duplicating weaker assertions that are already covered elsewhere.

## 1. Integration scope refresh and backlog setup

### 1.1 Re-inventory what INT_TEST1 already covers
- [x] Confirm which `AppShell` route/session flows are already covered directly in `src/app/AppShell.integration.test.tsx`
- [x] Confirm which reconnect/resync/runtime flows are already covered by Rust integration tests under `src-tauri/src/networking/runtime.rs`
- [x] Confirm which restart/history flows are already covered so the next wave does not restate them
- [x] Confirm which current “integration” candidates are actually better left as unit tests instead

### 1.2 Define the scope of the second-wave integration layer
- [x] Keep this wave focused on cross-layer regressions that require more than one abstraction to cooperate
- [x] Avoid duplicating static rendering checks already proven by screen-level tests
- [x] Avoid duplicating pure command/serialization checks already proven by unit tests
- [x] Explicitly mark process-level/manual-only gaps when Vitest or Rust tests cannot honestly prove them

### 1.3 Decide the concrete suites that should absorb the new work
- [x] Decide which new scenarios belong in `src/app/AppShell.integration.test.tsx`
- [x] Decide whether any new frontend integration scenarios deserve a separate neighboring suite instead of continuing to grow `AppShell.integration.test.tsx`
- [x] Decide which runtime regressions belong in `src-tauri/src/networking/runtime.rs`
- [x] Decide whether any repo/process smoke checks belong in `src/meta/**` versus manual docs only

---

## 2. Frontend shell bootstrap-update integration tests

### 2.1 Add live bootstrap subscription update coverage
- [x] Add an integration test that renders the shell from an initial bootstrap payload
- [x] Simulate a real `desktop://bootstrap` subscription update through the provider
- [x] Assert the updated bootstrap replaces the prior shell state instead of being ignored
- [x] Assert the updated bootstrap changes any affected route/screen availability immediately

### 2.2 Add route recovery coverage after a bootstrap surface change
- [x] Start on a route that is valid under the initial bootstrap
- [x] Publish a bootstrap update that removes or changes that route’s availability
- [x] Assert the shell reroutes to a safe fallback surface instead of stranding the user on a dead route
- [x] Assert the fallback route is deterministic and user-visible

### 2.3 Add launch-payload bootstrap error/update coverage
- [x] Start with a bootstrap that includes a launch-payload parse error
- [x] Assert the error-safe surface is shown clearly
- [x] Publish a later bootstrap update with a valid parsed launch payload
- [x] Assert the shell recovers into the valid join/lobby path without requiring a full remount

---

## 3. Ready-room and pre-start routing integration tests

### 3.1 Add full ready-room route coverage through the shell
- [x] Add an integration test that reaches the ready-room route through the real shell flow
- [x] Assert the ready-room badges and participant count reflect the same shell state used by lobby/setup
- [x] Assert the back/leave controls keep routing coherent when entered from the intended path

### 3.2 Add start-gating coverage for all-visible-participants-ready behavior
- [x] Add an integration test where not all visible participants are ready
- [x] Assert the shell keeps the tournament-start action disabled or unavailable
- [x] Transition all visible participants into ready state using the real shell controls
- [x] Assert the start action becomes available only after the final required readiness change

### 3.3 Add debug-mode host-controlled readiness integration coverage
- [x] Add an integration test with `debugToolsEnabled` on
- [x] Assert host-controlled remote readiness toggles appear only in debug mode
- [x] Assert toggling those controls changes the ready-room/start gate as expected
- [x] Assert the same controls remain hidden in non-debug mode for the equivalent flow

### 3.4 Add leave-flow integration coverage from ready room
- [x] Enter the leave-table confirmation flow from the ready room
- [x] Assert the dialog/surface is rendered without losing the current shell context
- [x] Cancel the leave flow and assert readiness state is preserved
- [x] Confirm the leave flow and assert routing returns to the home surface cleanly

---

## 4. Live-table action failure and recovery integration tests

### 4.1 Add `submitTableAction` failure non-corruption coverage
- [x] Render a live table with a valid action tray
- [x] Cause `submitTableAction` to reject on a real user action path
- [x] Assert the previous table snapshot remains visible instead of being blanked or replaced with unrelated state
- [x] Assert the action tray and navigation remain usable after the failure if the product intends retry behavior

### 4.2 Add “no open action window” shell-level regression coverage
- [x] Start from a table snapshot with no open local action window
- [x] Assert local action controls are absent or disabled truthfully
- [x] Attempt the nearest reachable user action path if any remains exposed
- [x] Assert the shell surfaces the stable error state without inventing a fake action-success transition

### 4.3 Add retry-after-failure table flow coverage
- [x] Trigger one failed table action
- [x] Keep the shell mounted on the same table route
- [x] Return a succeeding response on the next action attempt
- [x] Assert the shell recovers normally and renders the updated table snapshot/history

### 4.4 Add action-failure visibility boundary coverage
- [x] Assert action failures do not reveal observer-only or hidden-card state accidentally
- [x] Assert error messaging stays local to the acting shell instance
- [x] Assert shared public table data remains unchanged until a succeeding authoritative snapshot arrives

---

## 5. Reconnect-aware boot and restart integration tests

### 5.1 Add boot-time reconnect-state routing coverage
- [x] Seed restart/reconnect-related local state for a previously live table session
- [x] Render the shell from a table-oriented entry route
- [x] Assert the shell chooses the intended reconnect-safe or non-reconnect-safe surface for v1
- [x] Assert the user is not silently dropped into an unrelated fresh-join path

### 5.2 Add stale reconnect metadata rejection coverage
- [x] Seed reconnect metadata that is malformed, stale, or mismatched to the active bootstrap namespace
- [x] Render the shell through a restart-like boot path
- [x] Assert the invalid reconnect metadata is ignored safely
- [x] Assert the fallback surface explains or safely hides the invalid reconnect state without crashing

### 5.3 Add per-instance reconnect isolation integration coverage
- [x] Mount two shell instances with different storage namespaces
- [x] Seed reconnect-oriented state in only one namespace
- [x] Assert the second instance does not adopt or leak the first instance’s reconnect path
- [x] Assert restart-like boot behavior remains scoped per instance

### 5.4 Add post-completion restart routing coverage
- [x] Seed a completed-table or cached-history state and remount the shell
- [x] Assert the shell lands in the intended completion/history path rather than trying to reopen live gameplay
- [x] Assert reconnect metadata remains absent or ignored after tournament completion if that is still the product rule

---

## 6. Multi-instance and debug-launch integration tests

### 6.1 Add DebugPanel-to-launch flow coverage at the shell level
- [x] Enter the real debug-tools surface that can request an additional client launch
- [x] Trigger the launch flow with no join payload attached
- [x] Assert the visible shell/debug state reflects the launched child intent cleanly
- [x] Assert no current-instance shell state is corrupted by the launch request

### 6.2 Add join-payload-attached debug launch coverage
- [x] Trigger the additional-client launch flow with a real attached join payload
- [x] Assert the shell/debug surface reflects that the payload is already attached to the child launch
- [x] Assert the currently running instance remains on its current route and table state

### 6.3 Add child-instance namespace continuity regression coverage
- [x] Reuse or extend fixtures that simulate two independently bootstrapped instances
- [x] Assert child/parent profile namespaces remain distinct after the launch flow
- [x] Assert shell drafts, readiness state, and cached history do not bleed across the launched-instance boundary

### 6.4 Decide whether full process-spawn verification is automatable here
- [x] Determine whether a true spawned-child Tauri smoke test is realistic in-repo
- [x] If not, document the exact manual process smoke that still needs to be run outside Vitest
- [x] Keep any non-automated launch claims explicitly marked as unproven until manually executed

Manual process-boundary smoke still required outside Vitest:
- Launch the desktop app through a real Tauri build on a GUI-capable host.
- Open the routed `/debug` surface and trigger both `Launch extra client` and `Launch extra client with payload`.
- Verify the child process opens with a distinct profile/storage namespace and does not mutate the parent shell drafts, readiness state, or cached history.
- Verify the payload-attached child receives the copied `pkr1_` invite handoff without knocking the parent instance off its current route.

---

## 7. Focused Rust runtime integration regressions

### 7.1 Add reconnect cleanup timing regression coverage
- [x] Add a runtime integration test that exercises the transient “participant is already connected” reconnect race more directly
- [x] Assert reconnect eventually succeeds while the old connection is cleaning up
- [x] Assert retries stop once the authoritative reconnect succeeds
- [x] Assert the failure mode remains stable if cleanup never completes in time

### 7.2 Add reconnect exhaustion/error-shape coverage
- [x] Force the reconnect path to keep hitting the already-connected error beyond the retry window if that can be done honestly
- [x] Assert the returned error string remains the explicit retry-exhausted message
- [x] Assert no partial duplicate participant session survives after the failed reconnect

### 7.3 Add action-window truthfulness coverage across host/client integration
- [x] Start a live multi-client hand where only one participant may act
- [x] Assert the acting client sees the open action window while the other does not
- [x] Advance the action once and assert the prior actor loses the action window truthfully
- [x] Assert no client reports a stale open action window after authority advances

### 7.4 Add observer rejection regression coverage through the live runtime path
- [x] Drive a real elimination-to-observer transition in the runtime integration harness
- [x] Attempt an observer action through the nearest real boundary still available to tests
- [x] Assert the observer rejection remains explicit and does not mutate authoritative table state

---

## 8. Repository and process-level integration smoke gaps

### 8.1 Evaluate adding a Tauri bootstrap smoke test
- [x] Decide whether a lightweight repository smoke can validate that the desktop shell still boots far enough to expose bootstrap state
- [x] Avoid pretending that `npm run tauri dev` is a stable automated CI test if the environment cannot support it reliably
- [x] If a smoke test is added, keep it minimal and environment-aware

Decision: do not add an automated repository-level `tauri dev` bootstrap smoke in this wave. The GUI/window-manager dependency is too environment-sensitive for a truthful CI claim here, and the existing Rust + frontend automated suites already cover the bootstrap contract more reliably inside the repo.

### 8.2 Evaluate a process-boundary launch smoke test
- [x] Decide whether repo automation can validate `launch_additional_client_instance` against a real executable boundary
- [x] If not, write down the manual smoke checklist rather than fabricating weak assertions around mocks

Decision: keep real child-process launch validation manual. Vitest and the in-repo Rust/unit harness can validate launch intent, payload handoff, and namespace isolation, but they do not prove a real spawned Tauri child crosses the OS process boundary correctly.

### 8.3 Keep environment-blocked claims explicit
- [x] Record which second-wave integration scenarios remain blocked by GUI/process environment constraints
- [x] Leave blocked cases documented rather than implied by broader automated coverage

Environment-blocked/manual-only cases:
- Real desktop bootstrap to a visible Tauri window on a GUI-capable host.
- Real `launch_additional_client_instance` child-process spawning and OS-level window/profile isolation.
- End-to-end invite handoff into a spawned child window after launch.

---

## 9. Rollout and validation plan

### 9.1 Implement highest-value automated phases first
- [x] Phase 1: bootstrap-update and ready-room shell integration
- [x] Phase 2: live-table action failure and retry integration
- [x] Phase 3: reconnect-aware boot/restart shell integration
- [x] Phase 4: multi-instance/debug-launch integration
- [x] Phase 5: focused Rust reconnect/action-window regressions
- [x] Phase 6: repo/process smoke decisions and blocked-case documentation

### 9.2 Validate after each phase
- [x] Run the relevant Vitest suites after each frontend integration phase
- [x] Run the relevant Rust test filters after each Rust integration phase
- [x] Run `npm run lint` when shared frontend helpers or shell flows change
- [x] Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` when Rust runtime/helper code changes
- [x] Fix failures before moving to the next phase

### 9.3 Keep the backlog truthful as work lands
- [x] Mark completed tasks here as they land
- [x] Downgrade any scenario that proves redundant once stronger end-to-end coverage exists
- [x] Document intentionally deferred or environment-blocked process-level tests explicitly

---

## 10. Completion criteria

- [x] Live shell integration covers bootstrap updates after first render, not only initial boot
- [x] Ready-room/start gating is proven through the real routed shell flow
- [x] Table-action failures are proven not to corrupt the visible shell state
- [x] Reconnect-aware restart behavior is proven or explicitly documented as intentionally unsupported in v1
- [x] Multi-instance/debug-launch shell behavior is covered as far as the repo can honestly automate
- [x] Rust runtime integration protects the reconnect cleanup race and action-window truthfulness
- [x] Any process-level or GUI-environment gaps are documented explicitly instead of implied as covered
- [x] Lint, clippy, and all automated test suites pass after the new coverage lands
