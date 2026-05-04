# UI/UX Fixes 5 TODO

This backlog turns the latest UI/UX review into concrete implementation work.

## Goals

- Make every player-facing screen truthful about the current app state.
- Ensure critical actions are visible, usable, and correctly gated.
- Remove misleading multiplayer controls and state summaries.
- Make the desktop layout resilient without hiding important content.
- Expand test coverage so UI regressions are caught before release.

## Phase 1: Fix Incorrect Player-State UX

### 1.1 Fix local seat identification in the lobby

- [x] Audit how the local player is identified in the lobby.
- [x] Replace the current `seat.label === displayName` lookup with a reliable local-player signal.
- [x] Prefer a stable field such as `isLocal`, local seat index, or another explicit local-player flag.
- [x] Verify the top-level "You: Ready" badge reflects the actual local seat state.
- [x] Verify the table-ready summary reflects the correct count of ready players.

### 1.2 Prevent misleading readiness controls

- [x] Decide the intended behavior for lobby readiness buttons.
- [ ] If only the local player should control readiness:
	- [x] Hide readiness buttons for non-local occupied seats.
	- [x] Keep non-local seats read-only.
	- [x] Ensure only the local seat exposes a ready toggle.
- [x] If host-controlled simulation is intentionally supported:
	- [x] Visually mark the screen as simulation/debug behavior.
	- [x] Rename controls so they do not read like real player actions.
	- [x] Hide simulation-only controls outside debug mode.
- [x] Review seat labels and status chips so they communicate who controls what.

### 1.3 Verify participant-shell naming consistency

- [x] Review `buildParticipantShell` labels for host, local player, pending seats, and open seats.
- [x] Remove any UI logic that relies on display text for identity matching.
- [x] Ensure labels are presentation-only and not used as keys for behavior.

## Phase 2: Fix Broken or Misleading Flow Gating

### 2.1 Fix Join flow continuation state

- [x] Review all entry paths into Join Tournament:
	- [x] manual paste
	- [x] recent invite load
	- [x] launch payload
	- [x] deep-link payload
- [x] Ensure a valid invite always enables the correct next-step CTA.
- [x] Ensure invalid invite states always disable progression.
- [x] Remove any mismatch between preview validity and button visibility.
- [x] Confirm the preview, success banner, and continue action all derive from the same state model.

### 2.2 Fix Host flow continuation gating

- [x] Define the exact conditions required to proceed from Host setup to Lobby.
- [x] Disable or hide "Continue to lobby" when hosting is blocked.
- [x] Disable or hide "Continue to lobby" while LAN address resolution is still pending, if progression should be blocked.
- [x] Add visible explanation text for blocked progression.
- [x] Ensure the button state and the status banner never contradict each other.

### 2.3 Make invite-sharing actions trustworthy

- [x] Decide the primary share affordance on Host setup:
	- [x] clickable invite card
	- [x] explicit copy button
	- [x] both
- [x] Ensure the primary share affordance is visually obvious.
- [x] Add error handling for clipboard failures.
- [x] Show a failure banner if clipboard access is denied or unavailable.
- [x] Consider a fallback reveal area with selectable share text if copy fails.
- [x] Make success and failure states visually distinct.

## Phase 3: Remove Hidden-State UX Traps

### 3.1 Eliminate critical controls hidden behind legacy persisted state

- [x] Audit all persisted shell state that can affect initial screen visibility.
- [x] Identify any screen where old `localStorage` values can hide essential UI.
- [x] Decide which persisted fields are safe to honor and which should be migrated.
- [x] Replace one-off runtime recovery hacks with explicit migration or normalization where possible.
- [x] Add versioning or normalization logic for persisted UI state if needed.

### 3.2 Review all toggles that hide important controls

- [x] Review all collapsible UI sections in player-facing flows.
- [x] For each collapsible region, classify the hidden content as:
	- [x] critical setup
	- [x] optional detail
	- [x] debug/developer-only
- [x] Ensure critical setup controls are visible by default.
- [x] Keep optional detail collapsible only when it does not block successful flow completion.

## Phase 4: Improve Layout Resilience and Visibility

### 4.1 Audit the no-scroll shell strategy

- [x] Review every screen currently rendered inside the fixed-height shell.
- [x] Identify screens that rely on global `overflow: hidden` and may clip content.
- [x] Decide which screens should have a dedicated internal scroll container.
- [x] Replace brittle full-screen clipping with intentional, screen-level overflow behavior.
- [x] Keep critical actions and primary context visible without requiring accidental clipping workarounds.

### 4.2 Add per-screen viewport fit review

- [x] Review the following screens at common desktop sizes:
	- [x] Home
	- [x] Host Tournament Setup
	- [x] Join Tournament
	- [x] Lobby
	- [x] Main Table
	- [x] Hand History
	- [x] Rules
	- [x] Recovery
	- [x] Tournament Complete
- [x] Verify primary headings, main actions, and required controls are visible on initial load.
- [x] Verify no critical content is pushed below an inaccessible clipped region.
- [x] Verify card grids and control rails still work on shorter-height displays.

### 4.3 Improve density and hierarchy for short desktop heights

- [x] Review spacing tokens under `max-height` media queries.
- [x] Reduce vertical waste where panels stack too loosely.
- [x] Prioritize action visibility over decorative spacing.
- [x] Ensure button rows do not push required controls off-screen at shorter heights.
- [x] Review form control sizes and line lengths for compact desktop windows.

## Phase 5: Clean Up Information Architecture

### 5.1 Separate Help from Device Settings

- [ ] Review the current Rules screen content.
- [ ] Split pure gameplay/help content from device-level settings.
- [ ] Decide whether device settings should move to:
	- [ ] a separate Settings screen
	- [ ] a secondary support panel
	- [ ] a lower-priority device section with stronger separation
- [ ] Reduce context switching between rules explanation and configuration tasks.

### 5.2 Clarify support vs gameplay surfaces

- [ ] Review navigation labels for support screens.
- [ ] Ensure labels clearly distinguish:
	- [ ] active play
	- [ ] historical review
	- [ ] help/reference
	- [ ] device configuration
	- [ ] recovery/error handling
- [ ] Tighten any labels that feel generic or ambiguous.

## Phase 6: Tighten Action Design and Feedback

### 6.1 Improve state feedback for asynchronous actions

- [ ] Review async states for:
	- [ ] LAN IP resolution
	- [ ] invite validation
	- [ ] table loading
	- [ ] action submission
	- [ ] clipboard copy
- [ ] Ensure each async state has clear idle, loading, success, and failure feedback where relevant.
- [ ] Replace vague messages with direct outcome-oriented feedback.
- [ ] Prevent actions from appearing clickable when the result is not available yet.

### 6.2 Make disabled actions self-explanatory

- [ ] Audit disabled buttons across pregame and table flows.
- [ ] Add nearby explanatory text when an action is unavailable.
- [ ] Avoid leaving disabled controls without context.
- [ ] Ensure disabled actions remain visually legible and do not look broken.

## Phase 7: Strengthen Visual Consistency

### 7.1 Standardize primary and secondary CTA hierarchy

- [ ] Audit every button row on Home, Host, Join, Lobby, Table, and Complete screens.
- [ ] Ensure there is one obvious primary action per decision point.
- [ ] Downgrade secondary utilities so they do not visually compete with the main next step.
- [ ] Keep button copy short and specific.

### 7.2 Review badge and status color semantics

- [ ] Audit all `success`, `warning`, `info`, and `error` uses.
- [ ] Ensure similar states use the same tone across screens.
- [ ] Remove any cases where a state color is technically correct but semantically confusing.
- [ ] Verify contrast remains readable for all badge and banner states.

### 7.3 Review typography and supporting text density

- [ ] Audit helper text and low-priority copy across the app.
- [ ] Remove explanatory text that repeats what the controls already say.
- [ ] Keep support text focused on what the user needs to decide or do next.
- [ ] Ensure headings and subheadings are consistent in tone and length.

## Phase 8: Add Real Regression Coverage

### 8.1 Expand unit tests for initial visible state

- [ ] Add component tests for each major screen’s default visible controls.
- [ ] Verify critical controls exist on first render for fresh state.
- [ ] Verify critical controls still exist when persisted state is present.
- [ ] Add tests for blocked states and disabled-state explanations.

### 8.2 Add tests for persisted-state migrations and normalization

- [ ] Add tests that simulate old saved shell state.
- [ ] Verify outdated persisted values cannot hide or break critical UI.
- [ ] Add coverage for state normalization if persisted schema changes are introduced.

### 8.3 Add flow-level integration tests for CTA truthfulness

- [ ] Add integration tests covering:
	- [ ] host blocked but cannot continue
	- [ ] host ready and can continue
	- [ ] launch invite valid and can continue
	- [ ] invite invalid and cannot continue
	- [ ] local readiness badge matches actual local seat state
- [ ] Ensure each flow checks both content and enabled/disabled action state.

### 8.4 Add viewport-fit tests where practical

- [ ] Add tests that assert critical controls render within the intended scroll container or visible shell region.
- [ ] Add tests for shorter-height desktop layouts where regressions are likely.
- [ ] Prefer behavior-scoped layout assertions over brittle pixel snapshots.

## Phase 9: Final QA Pass

### 9.1 Manual screen review checklist

- [ ] Run the app in desktop dev mode.
- [ ] Review all major screens at typical desktop size.
- [ ] Review all major screens at shorter-height desktop size.
- [ ] Review fresh state and persisted-state scenarios.
- [ ] Review both normal and error paths.

### 9.2 Input and keyboard interaction review

- [ ] Verify keyboard access for buttons, toggles, sliders, and clickable cards.
- [ ] Verify focus order is sane on Host, Join, Lobby, and Main Table.
- [ ] Verify keyboard activation does not trigger surprising side effects.

### 9.3 Clipboard and error fallback review

- [ ] Verify invite copy works when clipboard permissions are available.
- [ ] Verify fallback messaging appears when clipboard access fails.
- [ ] Verify users can still retrieve invite details even if copy is unavailable.

## Deliverables

- [ ] Corrected lobby player-state logic
- [ ] Corrected host/join progression gating
- [ ] Safer persisted-state handling
- [ ] More resilient per-screen overflow behavior
- [ ] Cleaner support/settings information architecture
- [ ] Expanded regression test coverage for visible UI state and real user flows
- [ ] Final manual QA signoff for desktop viewport usability

