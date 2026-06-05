# UI/UX Improvements 3 TODO

This backlog captures bugs, workflow issues, and UX improvements identified during the
June 2026 UI/UX review. Items are grouped by type and ordered by impact within each group.

Status legend:
- [ ] not started
- [~] in progress
- [x] done

---

## Part 1: Bug Fixes

These are correctness issues that produce wrong output or broken UI state.

### B1. Eliminated players show "0 chips" in standings

**File:** `src/screens/MainTableScreen.tsx:453`  
**Problem:** `{entry.chipCount ?? 0} chips` prints "0 chips" for eliminated players because
`chipCount` is null after elimination. `TableStandingView` already carries `isEliminated` and
`statusLabel`; use those instead.

- [x] Replace `{entry.chipCount ?? 0} chips · {entry.statusLabel}` with a conditional:
  - If `entry.isEliminated`: render "Out" (or use `entry.statusLabel` directly).
  - Otherwise: render `{entry.chipCount} chips`.
- [x] Add a unit test asserting that an eliminated standing entry renders "Out" and does not
  render a chip count.

---

### B2. Only the first startup warning is shown on Home

**File:** `src/screens/HomeScreen.tsx:119`  
**Problem:** `startupWarnings[0]` is hardcoded. If multiple storage reads fail on startup, the
user only sees the first warning message.

- [x] Replace the single `startupWarnings[0]` render with a list that renders all entries in
  `startupWarnings`.
- [x] If the list is long (>3), collapse extras behind a "Show more" toggle.
- [x] Add a unit test with two startup warnings and assert both messages are rendered.

---

### B3. Event feed is unbounded — grows forever during long games

**File:** `src/screens/MainTableScreen.tsx:464`  
**Problem:** `tableView.eventFeed.map(...)` renders the entire feed with no cap. Long sessions
produce an ever-growing DOM list that degrades performance.

- [x] Render only the most recent N events (suggested cap: 50).
- [x] When the feed is at the cap, show a "Showing last 50 events" note below the list.
- [x] Add a unit test asserting that only the last 50 events render when the feed has >50 entries.

---

### B4. Raise slider min/max uses `?? 0` — broken display when bounds are null

**File:** `src/screens/MainTableScreen.tsx:387–388`  
**Problem:** `max={tableView.actionTray.maxRaiseTo ?? 0}` and
`min={tableView.actionTray.minRaiseTo ?? 0}` set the slider bounds to 0 when null, making
the slider render in a broken state. The quick-size buttons (Min / Max) also get 0 when bounds
are null. `buildQuickSizeButtons` at line 536 already has a null guard that returns `[]` — but
the slider itself does not.

- [x] Wrap the raise slider and quick-size button row in a null guard: do not render them when
  `actionTray.minRaiseTo === null || actionTray.maxRaiseTo === null`.
- [x] Instead, render a disabled placeholder ("Raise unavailable") in the same region so the
  action tray layout does not jump.
- [x] Remove the `?? 0` fallbacks from the slider `min`/`max` props — once the guard is in
  place they are not needed and mask bugs.
- [x] Add a unit test: when `minRaiseTo` and `maxRaiseTo` are null, the raise slider is absent
  and no NaN labels appear.

---

### B5. Host recovery error state has no action path

**File:** `src/screens/TournamentLobbyScreen.tsx`  
**Problem:** When the host stops before the tournament starts, the lobby shows "Host stopped –
Recovery required" but provides no button, navigation link, or explanation of what "recovery"
means. The user is stuck.

- [x] Define the intended recovery action for this state (return to Home, return to Host setup,
  or show `ErrorStateScreen` with the appropriate scenario).
- [x] Add an action button to the recovery error banner that navigates to the chosen destination.
- [x] Update the banner copy so it names the next step: e.g., "Host stopped. Return home to
  start a new table."
- [x] Add a unit test asserting the recovery button is rendered and links to the expected route.

---

### B6. Polling never backs off or shows connection-lost state

**Files:** `src/screens/TournamentLobbyScreen.tsx:133`,
`src/screens/MainTableScreen.tsx:65`  
**Problem:** The 800 ms `setInterval` poll continues at full rate regardless of how many
consecutive errors are returned. After a network hiccup the poll hammers the backend and gives
the user no indication that the connection is degraded.

- [x] Track a consecutive-error counter in each polling hook.
- [x] After 3 consecutive errors, increase the poll interval to 3 s and display a small inline
  banner: "Connection slow — retrying…"
- [x] After 10 consecutive errors, stop polling and navigate to `ErrorStateScreen` with the
  appropriate scenario (host-lost or reconnecting).
- [x] On a successful response, reset the error counter and restore the normal interval.
- [ ] Add unit tests for the backoff thresholds and the connection-lost navigation trigger.

---

### B7. Leave dialog stays open after a backend failure

**File:** `src/screens/TournamentLobbyScreen.tsx`  
**Problem:** If `stopHostSession()` or `leaveClientSession()` rejects, the error is displayed
but the confirmation dialog remains open with no way to retry.

- [x] On backend failure, keep the dialog open and show an inline error message inside the
  dialog (e.g., "Failed to leave — try again?").
- [x] Add a "Retry" button inside the dialog that re-attempts the leave call.
- [x] Add a "Cancel" option that closes the dialog without leaving, so the user is not stuck.
- [x] Add a unit test: when the leave command rejects, the dialog stays open and shows the
  retry option.

---

### B8. Auto-join on launch can leave the Join screen stuck

**File:** `src/screens/JoinTournamentScreen.tsx`  
**Problem:** When `bootstrap.launchJoinPayload` is invalid, an error is displayed but the only
recovery path is clicking "Continue to lobby" — which is blocked because the payload is
invalid. The user cannot exit this state without navigating away manually.

- [x] When auto-join validation fails, show a clear error banner with a "Clear and enter a
  different invite" action that resets the payload input.
- [x] Ensure "Continue to lobby" remains disabled (correct) but add a secondary escape action
  ("Start over" / "Return home") so the screen is not a dead end.
- [x] Add a unit test: when `launchJoinPayload` fails validation, the escape action is visible
  and navigates to Home.

---

## Part 2: High-Impact UX Changes

These affect how responsive and alive the app feels during play.

### H1. Replace polling with Tauri event subscriptions for table and lobby updates [x]

**Files:** `src/screens/TournamentLobbyScreen.tsx`, `src/screens/MainTableScreen.tsx`,
`src/api/desktop.ts`, `src-tauri/src/lib.rs`, `src-tauri/src/app_state/mod.rs`  
**Problem:** Both the lobby and the main table poll every 800 ms via `setInterval`. This means
every user action — claim seat, fold, raise — has up to 800 ms of perceived lag before the UI
reflects it. The Rust backend already emits `desktop://bootstrap` via `app.emit()` in
`lib.rs:32`, so the pattern is established.

- [x] **Rust side:** add a `desktop://session-update` event emitted from all session mutation
  commands (`start_host_session`, `stop_host_session`, `host_claim_lobby_seat`,
  `host_set_lobby_ready_state`, `host_start_tournament`, `join_host_session`,
  `leave_client_session`, `client_claim_lobby_seat`, `client_set_lobby_ready_state`).
  - [x] The event payload is empty (signal only) — the frontend re-fetches on receipt.
- [x] **Rust side:** add a `desktop://table-update` event emitted from `submit_table_action`.
  - [x] Again, the payload is a signal only.
- [x] **Frontend API bridge (`src/api/desktop.ts`):** added `onSessionUpdate(cb)` and
  `onTableUpdate(cb)` wrapper functions mirroring the existing `subscribeBootstrap` pattern.
  - [x] Added browser-mock stubs for both listeners so tests still work.
- [x] **Lobby screen:** subscribes to `desktop://session-update` on mount; retains 5 s
  fallback poll.
  - [x] Listener and fallback poll both cancelled on unmount.
- [x] **Main table screen:** subscribes to `desktop://table-update` on mount; retains 5 s
  fallback poll.
  - [x] Both cancelled on unmount.
- [x] Fallback poll slowed from 800 ms to 5 s (events drive fast updates).
- [x] Integration tests updated: event callback captured and fired manually to trigger refresh.

---

### H2. Add optimistic updates for user actions

**Files:** `src/screens/TournamentLobbyScreen.tsx`, `src/screens/MainTableScreen.tsx`  
**Problem:** After the user clicks Fold, Check, Raise, or Ready, nothing visual changes until
the next poll (or the next event). The app feels unresponsive during the latency window.

- [x] **Lobby — ready toggle:** immediately flips the local seat's ready badge via
  `optimisticReadyOverride` state before the server response arrives. Reverts on error.
- [x] **Table — action submission:** action tray already disabled via `submitting` state as
  soon as user clicks; a "Sending your action…" banner already shows immediately.
- [ ] Add unit tests for each optimistic state: assert the UI reflects the optimistic value
  immediately after the action is triggered, before the mock async call resolves.

---

## Part 3: Form Validation and Input Correctness

### F1. Tournament name cannot be empty

**File:** `src/screens/HostTournamentSetupScreen.tsx`  
**Problem:** There is no minimum-length validation on the tournament name field. A host can
submit with an empty name, producing a table with no readable label in the lobby or history.

- [x] Add client-side validation: tournament name must be at least 1 non-whitespace character.
- [x] Show an inline field error ("Name is required") when the field is blurred while empty.
- [x] Disable "Start hosting" when the name is blank.
- [x] Add a unit test asserting the button is disabled and the error is shown for an empty name.

---

### F2. Port clamping is silent

**File:** `src/screens/HostTournamentSetupScreen.tsx`  
**Problem:** When a user types an out-of-range port number the value silently clamps to the
nearest valid value. The user has no idea why their input changed.

- [x] Show an inline field error ("Port must be between 1 and 65535.") in `onChange` when the
  raw typed value is out of range.
- [x] Clear the hint when a valid port is entered.
- [x] Add a unit test: entering `99999` shows the hint, entering `43818` clears it.

---

### F3. "Check invite" validation has no timeout

**File:** `src/screens/JoinTournamentScreen.tsx`  
**Problem:** When the user clicks "Check invite" and the backend is slow or unresponsive,
the button shows the validating state indefinitely. The user cannot tell if the app is frozen.

- [x] Add a 10-second timeout to the `validateJoinPayloadInput` call using `Promise.race`.
- [x] On timeout, transition to the `invalid` state with an error message: "Validation timed
  out — check your connection and try again."
- [x] Add a unit test asserting the timeout error message appears when validation hangs past
  the threshold.

---

## Part 4: Workflow and Navigation Issues

### W1. Fix the table ↔ lobby navigation dead end

**File:** `src/app/AppShell.tsx` (route guard logic)  
**Problem:** When a player is on `/table` and no hand is running, the message says "Return to
Lobby to confirm the table and start the first hand." But navigating to `/lobby` may trigger
the live-route guard to immediately redirect back to `/table` because the session phase is
`running`. The user is looped with no exit.

- [x] Audited route guard: `/lobby` has no phase-based redirect (only the lobby SCREEN
  auto-navigates to `/table` when phase becomes "running"). The loop does not exist.
- [x] Updated the in-table "before first hand" message to remove the misleading suggestion
  to return to lobby. New message: "The tournament is live but the first hand has not started
  yet. The host will deal once all players are seated and ready."
- [ ] Add an integration test that navigates from `/table` (pre-hand) to `/lobby` and asserts
  the redirect does not loop back to `/table`.

---

### W2. Pasting host share text on the Join screen gives a confusing error

**File:** `src/screens/JoinTournamentScreen.tsx:191–196`  
**Problem:** `extractCompactInvite` already looks for a `pkr1_...` token in pasted text. If
the host shares the human-readable "host share details" card text (which includes the IP,
port, and name but embeds the `pkr1_` token), the regex should match. However if the share
text does not embed the token, the error message blames the user for pasting "the wrong
thing" when they did exactly what the host told them.

- [x] Updated error copy to: "That looks like host details — ask the host to share the invite
  code (starts with pkr1_) instead."
- [x] Removed blame-framing from the error message.
- [x] Existing test updated to match new copy.

---

### W3. "Host another table" always shows on Tournament Complete regardless of role

**File:** `src/screens/TournamentCompleteScreen.tsx`  
**Problem:** The "Host another table" button is shown to all players, including clients who
joined someone else's table. A client cannot host — this button is misleading for them.

- [x] Added `wasHost: boolean` and `setWasHost` to `DesktopShellProvider`.
- [x] `HostTournamentSetupScreen` calls `setWasHost(true)` when hosting starts successfully.
- [x] `TournamentCompleteScreen` reads `wasHost` and shows "Host another table" only for hosts;
  clients see "Join another table" linking to `/join`.
- [ ] Add a unit test for each role: host sees "Host another table," client does not.

---

### W4. Destructive settings actions execute immediately without confirmation

**File:** `src/screens/DeviceSettingsScreen.tsx`  
**Problem:** "Clear saved invites" and "Reset host setup" execute immediately on click with no
confirmation or undo. While low-risk, these are surprising to users who click by accident.

- [x] "Clear saved invites" now shows "Confirm clear" / "Cancel" on first click.
- [x] "Reset host setup" now shows "Confirm reset" / "Cancel" on first click.
- [x] Both show a success flash after the action completes.
- [ ] Add unit tests for both confirmation flows.

---

### W5. Side panel open/close preference is lost on navigation

**File:** `src/screens/MainTableScreen.tsx`  
**Problem:** The side panel toggle state is local component state and resets whenever the
player navigates away and returns to the table (e.g., to view hand history then back).

- [x] Moved side panel state (`tableSidePanelOpen` / `setTableSidePanelOpen`) to
  `DesktopShellProvider` so it persists across navigation.
- [x] `MainTableScreen` reads and writes the persisted shell state instead of local state.
- [ ] Add a unit test: toggling the panel, unmounting, and remounting reads the persisted state.

---

## Part 5: Information and Display Correctness

### I1. Recent invite pills show raw payload strings instead of decoded metadata

**File:** `src/screens/JoinTournamentScreen.tsx:383–387`  
**Problem:** Recent invites are displayed as raw `pkr1_...` tokens. A user with multiple
recent invites cannot distinguish them. The `JoinPayload` type already has `hostAddress`,
`hostPort`, and `tableName` (nullable).

- [ ] When rendering each recent invite pill, decode the stored payload using
  `validateJoinPayloadInput` (or a lighter decode helper) to extract `tableName` and
  `hostAddress`.
  - [ ] If decoding succeeds: display `{tableName ?? hostAddress}:{hostPort}` as the pill
    label.
  - [ ] If decoding fails (stale/invalid): display the truncated raw token as a fallback and
    mark it with a "possibly invalid" visual treatment.
- [ ] Cache the decoded metadata in shell state alongside the raw payload to avoid re-decoding
  on every render.
- [ ] Add a unit test asserting that a pill for a valid payload shows the table name/host
  address, not the raw token.

---

### I2. Community card placeholder text is confusing

**File:** `src/screens/MainTableScreen.tsx` (board card rendering)  
**Problem:** Empty community card slots labelled "Board 1", "Board 2", etc. imply named
positions rather than empty deal slots, which is unfamiliar poker terminology.

- [ ] Replace "Board 1" / "Board 2" labels (or any similar text) on empty card slots with
  blank card outlines (CSS-only empty card face) or a suit-symbol placeholder.
- [ ] If the label is needed for accessibility (`aria-label`), use "Community card 1" or
  "Undealt card 1" — only in the accessible attribute, not rendered as visible text.
- [ ] Add a unit test confirming empty card slots do not render visible "Board N" text.

---

### I3. Add total hands played and player count to Tournament Complete

**File:** `src/screens/TournamentCompleteScreen.tsx`  
**Problem:** The completion screen shows final standings but no summary stats — no total hands
played, no player count, no duration indication. These are natural "how did it go?" data
points that make the end state feel complete.

- [ ] If `TableViewSnapshot` carries `handNumber` or equivalent, display "X hands played" on
  the complete screen.
- [ ] Display the total player count ("4-player tournament").
- [ ] If duration data is not available from the backend, omit it rather than adding a
  placeholder — do not fake it.
- [ ] Add a unit test asserting the hand count and player count render correctly when the data
  is present.

---

### I4. Add hand number context for eliminated players at the table

**File:** `src/screens/MainTableScreen.tsx`  
**Problem:** Eliminated players see "Eliminated from this hand" but no indication of which
hand number they are watching. In a long game this context matters.

- [ ] Display the current `handNumber` from `TableViewSnapshot` in the observer/eliminated
  state banner: "Watching hand #N."
- [ ] Position this inline with the existing observer notice, not as a separate card.
- [ ] Add a unit test asserting the hand number appears in the observer notice when `isObserver`
  is true.

---

## Part 6: Help and Error Messaging

### E1. Help screen is fully static and context-unaware

**File:** `src/screens/RulesHelpScreen.tsx`  
**Problem:** The rules screen shows identical content regardless of how the user got there.
A player who tapped Help after a join failure or LAN error sees game rules — not anything
useful for their situation.

- [ ] Add an optional `context` query param or router state that callers can pass when
  navigating to `/rules`.
- [ ] When `context` is `join-failure`, scroll to or highlight the "Join flow" section
  automatically on mount.
- [ ] When `context` is `lan-error`, scroll to or highlight the "Host flow / LAN" section.
- [ ] Update callers (error state screen, host setup LAN error) to pass the appropriate context.
- [ ] Add unit tests for each context: assert the correct section is highlighted.

---

### E2. Error screen scenario copy uses internal labels as user-facing text

**File:** `src/screens/ErrorStateScreen.tsx`  
**Problem:** Scenario keys like `reconnect-failed` and `host-lost` read as internal debug
identifiers, not user-friendly descriptions.

- [ ] Audit all scenario labels exposed through the error screen's visible copy.
- [ ] Replace any scenario label text that appears verbatim in the rendered UI with plain
  language: e.g., "reconnect-failed" → "Connection lost", "host-lost" → "Host closed the
  table."
- [ ] Add unit tests that render each scenario and assert the internal key string does not
  appear in the output.

---

### E3. "Start hosting" button has no loading indicator

**File:** `src/screens/HostTournamentSetupScreen.tsx`  
**Problem:** Clicking "Start hosting" invokes a Rust command with no spinner or disabled state.
If the backend is slow to bind the socket, the button appears frozen.

- [ ] Add a local `starting` boolean state, set to `true` when the button is clicked and
  `false` when the promise resolves or rejects.
- [ ] While `starting` is true: disable the button and show a spinner or "Starting…" label.
- [ ] On error: clear `starting`, leave the button re-enabled, and display the error message.
- [ ] Add a unit test: assert the button is disabled while the mock `startHostSession` is
  pending.

---

## Part 7: Accessibility

### A1. Add ARIA labels to icon-only or ambiguous interactive elements

**Files:** all screen components, `src/components/`  
**Problem:** Several buttons and clickable cards rely on visual layout or adjacent text for
meaning but lack `aria-label` attributes, making them opaque to screen readers.

- [ ] Audit every `<button>` and `role="button"` element across all screens.
- [ ] For any button whose visible label is an icon (or a very short label like "Take"), add
  an `aria-label` with a full description.
- [ ] For the raise slider, ensure `aria-label="Raise amount"` and `aria-valuemin`,
  `aria-valuemax`, `aria-valuenow` are set correctly.
- [ ] Add a unit test per screen asserting that every interactive element has an accessible
  label (use `getByRole` with `name` to assert).

---

### A2. Status and error states use color alone

**Files:** `src/components/shared/StatusBadge.tsx`, all screens  
**Problem:** Success/warning/error states are conveyed by color only (green/amber/red).
Users with color-vision deficiencies may not be able to distinguish them.

- [ ] Add a small icon to each `StatusBadge` tone: checkmark for success, exclamation for
  warning, X for error, info-circle for info.
- [ ] Keep existing color classes — the icon is an additive signal, not a replacement.
- [ ] Ensure the icon has `aria-hidden="true"` (the badge label already carries the meaning).
- [ ] Add a unit test asserting the correct icon is rendered for each tone.

---

## Part 8: Test Coverage for New Behavior

Each task above that introduces a new behavior should have a corresponding unit or integration
test. This section lists the additional coverage needed at the suite level.

### T1. Add regression tests for all fixed bugs (B1–B8)

- [x] Eliminated player standings → renders statusLabel not "0 chips" (B1)
- [x] Multiple startup warnings → all rendered (B2)
- [x] Event feed cap → only last 50 events rendered (B3)
- [x] Null raise bounds → slider absent, no NaN (B4)
- [x] Host recovery banner → includes navigation action (B5)
- [ ] Poll backoff → error counter threshold triggers interval increase (B6)
- [x] Leave dialog → stays open with retry on backend failure (B7)
- [x] Auto-join failure → escape action visible and functional (B8)

### T2. Add integration tests for H1 (event subscriptions)

- [ ] Mock `desktop://session-update` event fires → lobby refresh is triggered.
- [ ] Mock `desktop://table-update` event fires → table refresh is triggered.
- [ ] Fallback poll at 5 s → fires when no events received.

### T3. Add integration test for W1 (nav loop fix)

- [ ] Navigate from `/table` (pre-hand, phase = running) to `/lobby` → stays on `/lobby`,
  does not redirect back to `/table`.

### T4. Run full test suite after each phase

- [ ] Run `npm run test` after Part 1 (bugs) is complete.
- [ ] Run `npm run test` after Part 2 (H1/H2) is complete.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml` after Rust changes in H1.
- [ ] Run `npm run lint` and `cargo clippy -D warnings` before final sign-off.

---

## Deliverables

- [ ] All Part 1 bugs fixed with regression tests
- [ ] Tauri event subscriptions replacing tight polling (H1)
- [ ] Optimistic action updates in lobby and table (H2)
- [ ] Form validation for tournament name and port (F1, F2)
- [ ] Invite validation timeout (F3)
- [ ] Table↔lobby navigation loop resolved (W1)
- [ ] Improved join error copy for host share text paste (W2)
- [ ] Conditional "Host another table" button (W3)
- [ ] Destructive action confirmations in settings (W4)
- [ ] Side panel preference persistence (W5)
- [ ] Decoded metadata on recent invite pills (I1)
- [ ] Community card placeholders cleaned up (I2)
- [ ] Tournament Complete stats added (I3)
- [ ] Observer hand number context (I4)
- [ ] Context-aware help screen (E1)
- [ ] Error scenario copy cleaned up (E2)
- [ ] "Start hosting" loading state (E3)
- [ ] ARIA labels on interactive elements (A1)
- [ ] Icons added to status badges (A2)
- [ ] Full test suite green
- [ ] Manual visual QA pass on desktop shell
