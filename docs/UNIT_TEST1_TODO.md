# UNIT_TEST1_TODO.md

This file tracks the next recommended unit-test expansion work for the desktop poker app.

It is focused on the highest-value gaps that remain after the current M0-M10 implementation work:

- protocol interop payload-shape coverage
- Tauri window-state persistence coverage
- hand-history persistence edge cases
- reconnect/resync edge cases
- controller-to-protocol serialization alignment
- CI/workflow smoke coverage

The goal is to tighten correctness around compatibility boundaries, persistence behavior, and recovery paths without changing production behavior.

## 1. Test infrastructure and fixture groundwork

### 1.1 Audit current test entry points
- [x] Inventory existing Rust test modules under `src-tauri/src/**`
- [x] Inventory existing frontend/Vitest suites under `src/**`
- [x] Identify which suggested gaps belong in Rust tests versus frontend tests
- [x] Document any helper overlap that should be reused instead of duplicated

### 1.2 Add shared test helpers where needed
- [x] Add protocol fixture helpers for canonical JSON serialization assertions
- [x] Add helper builders for reconnect/resync requests and snapshots
- [x] Add frontend storage helpers for seeding `localStorage`
- [x] Add frontend Tauri-window mocks for persistence tests
- [x] Add explicit invalid-data fixture helpers for corrupted persistence payloads

### 1.3 Define fixture conventions
- [x] Standardize fixture naming for protocol payloads
- [x] Standardize JSON fixture formatting for cross-platform comparisons
- [x] Keep fixtures small and purpose-specific
- [x] Document which fixtures are intended to mirror Android-compatible payload shapes

---

## 2. Protocol interop payload-shape tests

### 2.1 Add exact event-shape fixture tests for known interop risk areas
- [x] Add tests for `ACTION_WINDOW_OPENED_EVENT` serialization shape
  - [x] Assert field presence and absence exactly
  - [x] Assert naming and casing of serialized keys
  - [x] Assert optional fields are omitted when unset
  - [x] Add a fixture showing the current desktop payload contract
- [x] Add tests for `ACTION_REJECTED_EVENT` serialization shape
  - [x] Assert reason/error fields serialize consistently
  - [x] Assert no unexpected desktop-only fields leak into the payload
  - [x] Add a fixture showing the current desktop payload contract

### 2.2 Add coverage for join/reconnect/resync request shapes
- [x] Add fixture-based tests for join request serialization
- [x] Add fixture-based tests for reconnect request serialization with sequence present
- [x] Add fixture-based tests for reconnect request serialization with sequence omitted
- [x] Add fixture-based tests for resync request serialization with sequence present
- [x] Add fixture-based tests for resync request serialization with sequence omitted
- [x] Assert canonical omission of `null` fields in each case

### 2.3 Add negative-shape regression tests
- [x] Add tests that fail if deprecated or legacy keys reappear
- [x] Add tests that fail if optional fields start serializing as `null`
- [x] Add tests that fail if event type strings drift unexpectedly
- [x] Add tests that fail if canonical key ordering changes for signed payloads

### 2.4 Cross-platform contract documentation tasks
- [x] Link each new fixture test to the interop audit assumptions in comments only where needed
- [x] Record any still-unresolved Android/Desktop shape mismatches in `docs/ANDROID_INTEROP_AUDIT.md`
- [x] Update this TODO file if new interop risk surfaces are discovered while implementing the tests

---

## 3. Window-state persistence tests

### 3.1 Add happy-path persistence tests
- [x] Add tests for initial restore of saved window position
- [x] Add tests for initial restore of saved window size
- [x] Add tests for maximize/fullscreen restoration behavior if supported by current persistence logic
- [x] Add tests that window-state persistence only initializes in Tauri-capable environments

### 3.2 Add event-driven persistence tests
- [x] Add tests for persisting updated bounds after resize events
- [x] Add tests for persisting updated bounds after move events
- [x] Add tests for persisting maximize-state transitions
- [x] Add tests that duplicate events do not produce invalid stored state

### 3.3 Add invalid-data and fallback tests
- [x] Add tests for malformed JSON in stored window-state data
- [x] Add tests for partially missing window-state fields
- [x] Add tests for nonsensical numeric values
  - [x] negative width/height
  - [x] zero width/height
  - [x] non-numeric values if parsing allows them through
- [x] Add tests that invalid stored data is ignored without breaking shell startup

### 3.4 Add multi-instance persistence isolation tests
- [x] Add tests proving one instance profile does not overwrite another instance’s window-state data
- [x] Add tests for namespace/profile-key derivation used by persistence helpers
- [x] Add tests for reading the correct state after switching instance identity

---

## 4. Hand-history persistence edge-case tests

### 4.1 Expand cached-history happy-path coverage
- [x] Add tests that saved hand-history summaries render when the live fetch path fails
- [x] Add tests that live data replaces cached data when fetch succeeds
- [x] Add tests that newest saved summaries are surfaced first if ordering is expected

### 4.2 Add storage mutation tests
- [x] Add tests for writing the first cached hand-history entry
- [x] Add tests for appending additional summaries
- [x] Add tests for overwriting duplicate hand IDs instead of duplicating them
- [x] Add tests for clearing or replacing stale cached history when appropriate

### 4.3 Add empty and malformed state tests
- [x] Add tests for empty cached history arrays
- [x] Add tests for missing storage keys
- [x] Add tests for malformed cached history JSON
- [x] Add tests for partially malformed entry objects inside an otherwise valid array
- [x] Add tests that malformed cached data does not crash the screen

### 4.4 Add screen-level UX assertion tests
- [x] Add tests for the home screen saved-history count when cache exists
- [x] Add tests for the home screen saved-history count when cache is empty
- [x] Add tests for the hand-history screen empty state when neither live nor cached data is available
- [x] Add tests for transitions from cached data to refreshed live data

---

## 5. Reconnect and resync edge-case tests

### 5.1 Expand reconnect acceptance/rejection coverage
- [x] Add tests for reconnect requests using stale reconnect tokens
- [x] Add tests for reconnect requests after the player has already reconnected from another client
- [x] Add tests for reconnect requests after the seat/player is no longer eligible
- [x] Add tests for reconnect requests near tournament completion
- [x] Add tests for reconnect requests after tournament completion

### 5.2 Expand resync flow coverage
- [x] Add tests for repeated resync requests from the same client
- [x] Add tests for resync after multiple missed events
- [x] Add tests for resync when the client reports a future sequence
- [x] Add tests for resync when the client reports no sequence
- [x] Add tests that the authoritative snapshot fully replaces conflicting local state

### 5.3 Add ordering and race-style regression tests
- [x] Add tests for disconnect followed immediately by reconnect
- [x] Add tests for reconnect during an active action window
- [x] Add tests for reconnect between hands
- [x] Add tests for reconnect after elimination as observer-only
- [x] Add tests for stale reconnect attempts after a newer authoritative snapshot was issued

### 5.4 Add identity and key continuity tests
- [x] Add tests that original identity keys remain mandatory across reconnect attempts
- [x] Add tests that a valid token with the wrong keypair still fails
- [x] Add tests that sequence reconciliation does not bypass identity checks
- [x] Add tests for multi-client scenarios where one client’s recovery path cannot affect another client’s state

---

## 6. Controller-to-protocol serialization alignment tests

### 6.1 Map controller outputs to protocol contracts
- [x] Identify the controller/event outputs that are serialized into network payloads
- [x] Build focused fixtures for the most important table-state and action events
- [x] Reuse existing protocol model constructors where possible

### 6.2 Add serialization alignment tests for public events
- [x] Add tests for table-state update events
- [x] Add tests for player-joined / player-left style events if present
- [x] Add tests for betting/action window events
- [x] Add tests for showdown/result events
- [x] Add tests that public payloads never include private hole-card fields

### 6.3 Add serialization alignment tests for private events
- [x] Add tests for private player-view payload generation
- [x] Add tests for encrypted private payload wrapping after model serialization
- [x] Add tests that per-player private payloads differ where expected
- [x] Add tests that private payload fields stay structurally stable before encryption

### 6.4 Add regression guards for enum/string drift
- [x] Add tests that protocol event type strings remain stable
- [x] Add tests that action identifiers remain stable
- [x] Add tests that street/phase enum serialization stays unchanged
- [x] Add tests that observer/eliminated-player visibility rules remain intact

---

## 7. GitHub Actions and workflow smoke coverage

### 7.1 Add static workflow assertions
- [x] Add a lightweight test or script that validates the workflow file exists at `.github/workflows/ci.yml`
- [x] Add a check that required verify-job steps remain present
  - [x] Rust format check
  - [x] clippy
  - [x] Rust tests
  - [x] frontend lint
  - [x] frontend tests
  - [x] frontend build
- [x] Add a check that the release job remains tag-gated

### 7.2 Add regression checks for release behavior
- [x] Add a validation step that tagged-release asset publishing is only configured for `refs/tags/v*`
- [x] Add a regression check that normal pushes and PRs do not attempt asset publishing
- [x] Add a regression check that README badge URLs still point at the active workflow path

### 7.3 Decide scope of workflow validation
- [x] Decide whether workflow checks should live as:
  - [ ] Rust tests
  - [x] frontend tests
  - [ ] a repository validation script invoked by CI
- [x] Choose the least brittle option that still catches accidental workflow drift

---

## 8. Prioritization and rollout order

### 8.1 Implement highest-risk tests first
- [x] Phase 1: protocol interop payload-shape tests
- [x] Phase 2: reconnect/resync edge-case tests
- [x] Phase 3: window-state persistence tests
- [x] Phase 4: hand-history persistence tests
- [x] Phase 5: controller/protocol serialization alignment tests
- [x] Phase 6: workflow smoke coverage

### 8.2 Validate after each phase
- [x] Run Rust test suite after each Rust-test phase
- [x] Run frontend test suite after each frontend-test phase
- [x] Run lint/build checks if helpers or shared code were updated
- [x] Fix warnings or failures before moving to the next phase

### 8.3 Keep docs aligned
- [x] Update this file as tasks are completed
- [x] Update `docs/ANDROID_INTEROP_AUDIT.md` if protocol test findings reveal new interop constraints
- [x] Update `README.md` only if user-facing testing guidance materially changes

---

## 9. Completion criteria

- [x] All new tests are added in the correct Rust or frontend test suites
- [x] New fixtures are minimal, stable, and easy to review
- [x] No production behavior changes are introduced solely to make tests pass
- [x] Lint, build, and all test suites pass with the new coverage
- [x] Remaining known gaps, if any, are documented explicitly instead of being silently deferred
