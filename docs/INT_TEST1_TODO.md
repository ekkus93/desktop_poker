# INT_TEST1_TODO.md

This file tracks the next recommended integration-test expansion work for the desktop poker app.

It focuses on the highest-value gaps that remain after the current unit-test expansion:

- real host/join/play flows through the shell
- reconnect/resync as full live session flows
- public/private event separation over real TCP
- tournament completion and observer transitions
- persistence across app restarts
- CI/tagged-release smoke behavior
- live desktop/Android interoperability

The goal is to prove multi-layer behavior across the Rust runtime, protocol layer, persistence layer, and UI shell without weakening production constraints or introducing simulator-only shortcuts into the default runtime path.

## 1. Integration test infrastructure and scope mapping

### 1.1 Inventory current integration-capable surfaces
- [x] Inventory existing Rust runtime tests that already exercise real TCP across host/client flows
- [x] Inventory existing frontend tests that could be promoted into broader route/session flows
- [x] Identify which new integration coverage should remain in Rust tests
- [x] Identify which new integration coverage should live in frontend/Vitest tests
- [x] Identify which scenarios require external/manual mixed-runtime execution instead of in-repo automation

### 1.2 Define integration test layers
- [x] Define a **Rust runtime integration** layer for real host/client and protocol flows
- [x] Define a **frontend shell integration** layer for multi-screen app behavior
- [x] Define a **repository integration** layer for CI/release smoke validation
- [x] Define a **manual/live interop** layer for Android/Desktop mixed-runtime testing

### 1.3 Establish shared integration fixtures
- [x] Add helper builders for live host runtime creation with deterministic state where appropriate
- [x] Add helper builders for multiple connected clients with explicit identities
- [x] Add helper builders for launch payload injection into frontend shell tests
- [x] Add helper builders for persisted app-state bootstrapping before “restart” tests
- [x] Add helper utilities for waiting on multi-step event sequences without brittle sleeps

### 1.4 Set fixture conventions
- [x] Standardize naming for integration fixtures and harness helpers
- [x] Keep deterministic fixtures separate from live/manual environment checklists
- [x] Document which helpers are safe for automated CI and which are manual-only

---

## 2. UI host/join/play integration tests

### 2.1 Add host creation flow coverage
- [x] Add a shell-level integration test that starts from the home screen and enters the host flow
- [x] Assert host draft defaults are surfaced correctly
- [x] Assert host settings changes persist through the shell flow
- [x] Assert the host flow renders the real runtime/share surface rather than a fake simulator path

### 2.2 Add join-payload flow coverage
- [x] Add an integration test that captures a real `pkr1_` join payload from the host runtime path
- [x] Inject that payload into a join flow for a second instance/session
- [x] Assert the join screen accepts the payload and routes into the live session path
- [x] Assert invalid payloads fail loudly without partially entering the session flow

### 2.3 Add ready/start table flow coverage
- [x] Add an integration test for host + joined client reaching the lobby/ready state
- [x] Assert ready toggles propagate correctly for both participants
- [x] Assert the tournament start flow transitions both sides into the main table
- [x] Assert the shell routing/history remains coherent across the start transition

### 2.4 Add live table behavior assertions across two instances
- [x] Assert both clients render the same public table state
- [x] Assert local-player indicators differ correctly by instance
- [x] Assert action ownership appears on the correct side
- [x] Assert public feed/history updates propagate to both sessions

### 2.5 Add failure-state integration coverage
- [x] Assert join rejection routes to an error-safe UI state
- [x] Assert lost host connection transitions to reconnect-safe UI messaging
- [x] Assert unrecoverable runtime errors do not silently strand the shell between routes

---

## 3. Reconnect and resync full-session integration tests

### 3.1 Add mid-hand disconnect/reconnect coverage
- [x] Start a live host/client hand with an active action window
- [x] Force one client connection to drop mid-hand
- [x] Assert reconnect UI/runtime behavior triggers using the original identity
- [x] Assert the reconnected client receives an authoritative snapshot
- [x] Assert action ownership after reconnect is correct

### 3.2 Add out-of-sequence/resync recovery coverage
- [x] Force a stale or out-of-sequence event path that triggers resync
- [x] Assert the client requests a resync instead of continuing on divergent local state
- [x] Assert the authoritative snapshot replaces local state fully
- [x] Assert post-resync gameplay can continue normally

### 3.3 Add between-hands and late-stage reconnect flows
- [x] Add reconnect coverage between hands
- [x] Add reconnect coverage near tournament completion
- [x] Add reconnect coverage after elimination in observer mode
- [x] Add reconnect coverage after tournament completion if reconnect is still allowed by product behavior

### 3.4 Add multi-client recovery isolation coverage
- [x] Assert one client’s reconnect path does not corrupt the other client’s session state
- [x] Assert one client’s resync path does not duplicate or reorder another client’s events
- [x] Assert reconnect tokens/identities remain scoped to the correct client

---

## 4. Public/private event separation integration tests

### 4.1 Add public-event ordering assertions
- [x] Start a live two-client session
- [x] Emit/broadcast several public events from the host runtime
- [x] Assert both clients receive the same ordered public event sequence
- [x] Assert both clients converge on the same authoritative public state

### 4.2 Add private-hole-card delivery assertions
- [x] Deal/send private hole cards to both players
- [x] Assert each client only receives its own private payload
- [x] Assert each client decrypts its own payload successfully
- [x] Assert the other client never sees the peer’s private hole cards in its event stream

### 4.3 Add mixed public/private sequencing coverage
- [x] Assert private delivery does not break subsequent public-event processing
- [x] Assert public events still land in order around private deliveries
- [x] Assert local/private views and observer/public views stay internally consistent

### 4.4 Add event-log integration assertions
- [x] Assert debug/runtime event logs show expected public/private boundaries where exposed
- [x] Assert no private payload details leak into public-facing history/feed surfaces

---

## 5. Tournament completion and observer-flow integration tests

### 5.1 Add deterministic short-game completion flow
- [x] Build or reuse a deterministic short-handed game path that reaches completion quickly
- [x] Run the game to completion across host/client integration harnesses
- [x] Assert placements/final standings are identical across clients
- [x] Assert the winner/completion state reaches the correct final UI surfaces

### 5.2 Add elimination-to-observer transition coverage
- [x] Eliminate one participant during a live session
- [x] Assert that participant transitions into observer mode
- [x] Assert the eliminated observer keeps public visibility only
- [x] Assert the observer cannot act after elimination

### 5.3 Add history and completion-surface assertions
- [x] Assert final hand summaries appear in hand history after completion
- [x] Assert elimination summaries surface in the expected UI/context
- [x] Assert final-screen or completion-route data matches tournament results

---

## 6. Persistence across app restarts integration tests

### 6.1 Add shell-draft restart coverage
- [x] Seed host draft state, join draft state, display name, and recent payloads
- [x] Simulate an app restart/re-bootstrap
- [x] Assert the shell restores the expected drafts and identity values
- [x] Assert per-instance namespaces remain isolated across restarts

### 6.2 Add cached history restart coverage
- [x] Persist hand-history summaries from a live session
- [x] Simulate restart with live fetch unavailable
- [x] Assert the restarted shell falls back to cached summaries correctly
- [x] Assert later live refresh replaces stale cached data when available

### 6.3 Add window-state restart coverage
- [x] Persist Tauri window bounds/maximize state
- [x] Simulate Tauri shell restart with restored environment
- [x] Assert the restored window state is applied correctly
- [x] Assert invalid stored state does not block restart

### 6.4 Add reconnect metadata restart considerations
- [x] Decide whether any reconnect metadata should survive restart in v1
- [x] If not, add an integration assertion that restart begins in a safe non-reconnect state
- [ ] If yes, add restart coverage proving identity continuity behaves correctly

---

## 7. CI and tagged-release integration smoke tests

### 7.1 Add repository-level verify-flow coverage
- [x] Add a smoke validation that the repository verify workflow still runs the required checks
- [x] Assert the workflow continues to cover Rust fmt/clippy/tests and frontend lint/tests/build
- [x] Assert workflow path/badge/references remain internally consistent

### 7.2 Add tagged-release behavior coverage
- [x] Add a smoke check that release publishing remains gated to version tags
- [x] Add a smoke check that normal pushes and PRs do not attempt asset publishing
- [x] Add a smoke check that tagged releases still route through the bundle-publish path

### 7.3 Decide automation scope
- [x] Decide whether CI integration smoke checks should live in:
  - [x] frontend tests
  - [ ] Rust tests
  - [ ] a repository validation script invoked by CI
- [x] Choose the least brittle option that still catches workflow drift

---

## 8. Desktop/Android live interop integration checklist

Blocked in this repository environment: automated desktop coverage is complete, but no live Android app/runtime session was executed here, so mixed-runtime interop remains manual-only and unproven.

### 8.1 Desktop host + Android client
- [ ] Run a real desktop host session
- [ ] Join from a real Android client using the direct payload path
- [ ] Assert join/ready/start succeeds across both sides
- [ ] Assert public event flow remains synchronized
- [ ] Assert private hole-card delivery works correctly

### 8.2 Android host + desktop client
- [ ] Run a real Android host session
- [ ] Join from a real desktop client using the direct payload path
- [ ] Assert join/ready/start succeeds across both sides
- [ ] Assert public event flow remains synchronized
- [ ] Assert private hole-card delivery works correctly

### 8.3 Mixed-runtime recovery flows
- [ ] Run disconnect/reconnect from desktop against Android host if supported
- [ ] Run disconnect/reconnect from Android against desktop host if supported
- [ ] Assert resync works correctly after induced divergence or dropped events

### 8.4 Record outcomes honestly
- [ ] Document exact runtime/build/device combinations used
- [ ] Record any remaining payload-shape or recovery mismatches in `docs/ANDROID_INTEROP_AUDIT.md`
- [x] Keep any unproven paths explicitly marked as unproven

---

## 9. Test orchestration and rollout plan

### 9.1 Implement highest-value automated integration phases first
- [x] Phase 1: UI host/join/play integration
- [x] Phase 2: reconnect/resync full-session integration
- [x] Phase 3: public/private event separation integration
- [x] Phase 4: tournament completion and observer-flow integration
- [x] Phase 5: persistence-across-restart integration
- [x] Phase 6: CI/tagged-release smoke integration
- [ ] Phase 7: live desktop/Android interop execution

### 9.2 Validate after each phase
- [x] Run the Rust test suite after each Rust integration phase
- [x] Run the frontend test suite after each frontend integration phase
- [x] Run lint/build checks if shared helpers or app/runtime code changed
- [x] Fix warnings and failures before moving to the next phase

### 9.3 Keep docs aligned
- [x] Update this file as phases and subtasks are completed
- [ ] Update `docs/ANDROID_INTEROP_AUDIT.md` when live interop evidence changes the repo’s claims
- [ ] Update `README.md` if user-facing testing or interop status materially changes

---

## 10. Completion criteria

- [x] Automated integration tests cover the major host/join/play/recovery flows
- [x] Public/private visibility boundaries are proven by integration behavior, not only unit fixtures
- [x] Restart/persistence behavior is proven end-to-end
- [x] CI/tagged-release smoke behavior is covered by regression checks
- [x] Mixed desktop/Android runtime results are documented honestly
- [x] Lint, build, and all automated test suites pass with the new coverage
- [x] Remaining manual-only or environment-blocked gaps, if any, are explicitly documented
