# UI/UX Fixes — Iteration 6

Comprehensive task list derived from a full UI/UX review of the frontend. Tasks are grouped by screen/component, ordered P0 (broken) → P1 (significant friction) → P2 (polish). Each task references the exact file to touch.

---

## P0 — Bugs: broken or misleading behavior

### P0.1 — Port error renders on first load (`HostTournamentSetupScreen`)

**File:** `src/screens/HostTournamentSetupScreen.tsx`

The port validation error can appear before the user has touched the port field. The name field already uses blur-triggered validation correctly; the port field must do the same.

- [ ] Add a `portTouched: boolean` state variable, initialized to `false`.
- [ ] Set `portTouched = true` in the port `onBlur` handler.
- [ ] Only render the port error hint when `portTouched && portError`.
- [ ] Verify the error does not appear on initial render with default value.
- [ ] Verify the error does appear after the user blurs the port field with an invalid value.

---

### P0.2 — NPC add failure leaves host in an inconsistent state (`HostTournamentSetupScreen`)

**File:** `src/screens/HostTournamentSetupScreen.tsx`

When `startHostSession()` succeeds but `addNpcPlayers()` fails, the screen shows an `npcError` banner but `hostSession` is already set and the user can navigate to the lobby — without any NPCs. There is no retry path.

- [ ] When `npcError` is set and `hostSession` is set, show a dedicated banner that explains the session is live but NPCs were not added (distinct from a general NPC error).
- [ ] Add a "Retry adding bots" button to that banner that re-calls `addNpcPlayers()` with the current NPC count and style.
- [ ] Disable "Continue to lobby" while a retry is in flight.
- [ ] Clear `npcError` and re-enable "Continue to lobby" after a successful retry.
- [ ] Add a test: npc add failure followed by retry succeeds.

---

### P0.3 — `TournamentCompleteScreen` crashes when standings are empty

**File:** `src/screens/TournamentCompleteScreen.tsx`

The winner announcement accesses `standings[0]` without a guard. If the array is empty, it renders `"undefined wins …"`.

- [ ] Replace the winner reference with `standings[0]?.displayName ?? "—"` (or a suitable fallback).
- [ ] Add a guard: if `standings.length === 0`, render a neutral message (`"No result available"`) instead of the winner announcement.
- [ ] Add a test: `TournamentCompleteScreen` renders without crashing when `standings` is an empty array.

---

### P0.4 — Deep-link payload URL not cleaned up on join failure (`JoinTournamentScreen`)

**File:** `src/screens/JoinTournamentScreen.tsx`

If the app is launched with `?payload=pkr1_...` and the join fails, clicking "Clear and enter a different invite" wipes the textarea but leaves the query string in the URL. Navigating back re-imports the bad payload.

- [ ] Call `navigate(location.pathname, { replace: true })` when the user clicks "Clear and enter a different invite" to strip the query params from history.
- [ ] Also call this when the user manually clears the textarea after a deep-link load (detect that the payload came from `location.search`).
- [ ] Add a test: after clearing from a deep-link state, navigating back does not re-populate the textarea.

---

### P0.5 — Clipboard fallback textarea never dismisses (`HostTournamentSetupScreen`)

**File:** `src/screens/HostTournamentSetupScreen.tsx`

After a clipboard write failure, a fallback textarea appears for manual copying but has no way to be dismissed. It persists until the next copy attempt.

- [ ] Add a "Done" / close button to the fallback textarea block.
- [ ] Clicking it sets the fallback-visible state back to `false`.
- [ ] Alternatively, auto-dismiss after 30 seconds (a `useEffect` cleanup timer).

---

## P1 — Significant friction: confusing or error-prone UX

### P1.1 — Optimistic ready-state flip is confusing on failure (`TournamentLobbyScreen`)

**File:** `src/screens/TournamentLobbyScreen.tsx`

When the user clicks "I'm ready" the button immediately flips to "Undo ready" (optimistic update). If the API call fails, the button flips back with an error banner. Fast clickers may not know which state they are actually in.

- [ ] Lock the ready-toggle button (disabled + loading label) for the duration of the in-flight call instead of updating optimistically.
- [ ] On success: update to the confirmed server state.
- [ ] On failure: restore the pre-click state and show the error banner.
- [ ] Remove or simplify `optimisticReadyOverride` state if it is no longer needed.
- [ ] Add a test: button stays disabled while call is in flight.

---

### P1.2 — Two confirmation flows can be active simultaneously (`DeviceSettingsScreen`)

**File:** `src/screens/DeviceSettingsScreen.tsx`

`confirmReset` and `confirmClear` are independent boolean states. A user can open both confirmation dialogs at the same time, which is confusing.

- [ ] Replace `confirmReset: boolean` and `confirmClear: boolean` with a single `activeConfirmation: "reset" | "clear" | null`.
- [ ] Opening one confirmation automatically closes the other.
- [ ] Update all button renders and handlers to use the new state shape.
- [ ] Verify only one confirmation prompt is ever visible at a time.

---

### P1.3 — Settings and NPC Profiles screens have no back navigation

**Files:** `src/screens/DeviceSettingsScreen.tsx`, `src/screens/NpcProfilesScreen.tsx`

Both screens are dead-ends — no back button or breadcrumb in the header. Users must use the OS back gesture or browser back button.

- [ ] Add a `← Settings` / `← Back` link in the `ScreenShell` header props for `DeviceSettingsScreen`.
- [ ] Add a `← Settings` link in the `ScreenShell` header props for `NpcProfilesScreen` (since it is reached from Settings).
- [ ] Verify `AppFrame` renders the back target correctly for each route.

---

### P1.4 — Dense lobby seat grid reduces button label without aria-label (`TournamentLobbyScreen`)

**File:** `src/screens/TournamentLobbyScreen.tsx`

In dense layout (>8 seats) the "Take seat" button text is truncated to "Take" with no `aria-label`. Screen reader users and keyboard navigators receive no useful label for the action.

- [ ] Add `aria-label={`Take seat ${seat.index}`}` to every "Take seat" button in the dense layout.
- [ ] Also add it in the non-dense layout for consistency.
- [ ] Add a test: seat button has a descriptive `aria-label` attribute in both layout modes.

---

### P1.5 — Fallback to persisted history is silent in `HandHistoryScreen`

**File:** `src/screens/HandHistoryScreen.tsx`

If `getTableView()` fails on the history screen, the hand list may fall back to persisted data but the Standings and Events sections are silently empty. There is no indication whether the app is showing saved data or live data.

- [ ] Detect when the live table view failed and persisted history is being shown.
- [ ] Render a subtle inline banner: `"Showing saved history — live table not available"` above the hand list when in this state.
- [ ] Do not show the banner if the live table view loaded successfully (even if it is empty).

---

### P1.6 — Builtin profiles disable Delete silently without explanation (`NpcProfilesScreen`)

**File:** `src/screens/NpcProfilesScreen.tsx`

The Delete button renders as `disabled` for built-in profiles. Clicking it produces no feedback and there is no tooltip or label to explain why.

- [ ] Replace the disabled Delete button for builtins with either:
  - A button with `aria-describedby` pointing to a small note: `"Built-in profiles cannot be deleted."`, or
  - Hide the button entirely and show the note text in its place.
- [ ] Add a test: builtin profile detail card does not render a clickable Delete button.

---

### P1.7 — No unsaved-changes warning in NPC profile editor (`NpcProfilesScreen`)

**File:** `src/screens/NpcProfilesScreen.tsx`

If a user edits profile content and navigates away (route change or app close), changes are silently discarded.

- [ ] Track a `hasUnsavedChanges` flag that is set `true` when the textarea content differs from the originally loaded content and `false` after a successful save.
- [ ] Use React Router's `useBlocker` (or equivalent) to intercept navigation when `hasUnsavedChanges` is true and show a confirmation dialog: `"You have unsaved changes. Leave without saving?"`.
- [ ] Add a `window.addEventListener("beforeunload", ...)` guard for app-close events.
- [ ] Clear the flag after a successful `saveNpcProfile()` response.

---

### P1.8 — Profile ID input has no slug validation (`NpcProfilesScreen`)

**File:** `src/screens/NpcProfilesScreen.tsx`

The new-profile ID field accepts any string. The backend requires a valid filename stem. Invalid slugs produce a cryptic backend error.

- [ ] Add client-side validation on the ID field: allow only `[a-z0-9-]` (lowercase letters, digits, hyphens), 1–64 characters.
- [ ] Show an inline error hint beneath the field when the pattern is not matched: `"Use lowercase letters, numbers, and hyphens only (e.g. sharp-phil)"`.
- [ ] Disable the Save button when the ID field is invalid.
- [ ] Add a test: invalid slug shows error hint and disables Save.

---

### P1.9 — Provider key scope unclear when switching providers (`DeviceSettingsScreen`)

**File:** `src/screens/DeviceSettingsScreen.tsx`

When the user switches from Anthropic to OpenAI, the placeholder changes to `"Enter OpenAI API key"`, but the previous Anthropic key was stored in the keychain. A user might believe their key is still saved for the new provider.

- [ ] When provider selection changes away from the currently stored provider, show a one-line note beneath the key field: `"No key saved for this provider. Enter a new key to configure it."`.
- [ ] Only show `"Leave blank to keep existing key"` when the selected provider matches `hasExistingKeyForProvider`.
- [ ] Add a test: switching provider type clears the "keep existing key" hint.

---

### P1.10 — Empty lobby has no minimum-player hint (`TournamentLobbyScreen`)

**File:** `src/screens/TournamentLobbyScreen.tsx`

If the host is the only seated player, the Start Tournament button is disabled but nothing explains why. Users may not know that at least 2 players are required.

- [ ] When `participants.filter(p => p.seatIndex !== null).length < 2`, show a field hint beneath the Start button: `"At least 2 players must be seated to start."`.
- [ ] Remove the hint once 2+ players are seated.

---

## P2 — Polish: missing states, edge cases, and accessibility

### P2.1 — Confirmation dialogs do not move focus or manage return focus (`TournamentLobbyScreen`, `DeviceSettingsScreen`)

**Files:** `src/screens/TournamentLobbyScreen.tsx`, `src/screens/DeviceSettingsScreen.tsx`

When a confirmation card or leave-flow dialog appears, keyboard focus stays on the triggering button. The user must Tab to reach the confirmation controls.

- [ ] When a confirmation UI becomes visible, use `useEffect` + `ref.focus()` to move focus to the primary confirm button.
- [ ] When the confirmation is dismissed (via "Cancel" or "Stay here"), return focus to the original triggering button.
- [ ] Repeat this pattern for: leave-flow dialog in `TournamentLobbyScreen`, reset/clear confirmation in `DeviceSettingsScreen`, and raise confirmation card in `MainTableScreen`.

---

### P2.2 — No ARIA live regions for dynamic state changes

**Files:** `src/screens/TournamentLobbyScreen.tsx`, `src/screens/MainTableScreen.tsx`, `src/screens/HostTournamentSetupScreen.tsx`

Connection status changes, confirmation requests, and action-tray updates are silent to screen readers.

- [ ] Wrap the connection status banner in `<div role="status" aria-live="polite">` in `TournamentLobbyScreen` and `MainTableScreen`.
- [ ] Wrap the confirmation card (`actionConfirmation` block) in `<div role="alert" aria-live="assertive">` in `MainTableScreen` — it requires immediate attention.
- [ ] Wrap the LAN status pill update area in `<div role="status" aria-live="polite">` in `HostTournamentSetupScreen`.
- [ ] Verify changes are announced in a screen reader (VoiceOver / NVDA) before closing.

---

### P2.3 — No visible `:focus-visible` ring in CSS

**File:** `src/styles/01-base.css`

The base stylesheet does not define a `:focus-visible` outline that is distinct on the dark green/wood background. Keyboard users cannot see where focus is.

- [ ] Add a global `:focus-visible` rule with a high-contrast outline (e.g., `outline: 2px solid #f5d060; outline-offset: 2px;` — the gold accent).
- [ ] Suppress it only for elements that define their own visible focus style (e.g., primary buttons with active/hover states).
- [ ] Do not use `outline: none` without a substitute focus indicator anywhere in the stylesheet.
- [ ] Manually tab through Home → Host Setup → Lobby → Table to verify focus is always visible.

---

### P2.4 — Hand history list has no virtualization or cap (`HandHistoryScreen`)

**File:** `src/screens/HandHistoryScreen.tsx`

All settled hands are rendered into the DOM at once. A long tournament (200+ hands) causes layout lag on this screen.

- [ ] Cap the rendered list to the most recent 100 hands by default.
- [ ] Add a "Show all X hands" expand button below the list when there are more than 100.
- [ ] Alternatively, integrate `react-window` or `react-virtual` for full virtualization.
- [ ] Add a test: when `hands.length > 100`, only 100 items render without the expand being clicked.

---

### P2.5 — Leave-flow dialog remains open after an error (`TournamentLobbyScreen`)

**File:** `src/screens/TournamentLobbyScreen.tsx`

If the leave/close API call fails, the error banner appears inside the dialog, but the dialog stays open. The user must manually click "Stay here" to dismiss it. There is no way to retry from the dialog.

- [ ] Add a "Try again" button to the leave-flow dialog error state.
- [ ] Clicking "Try again" re-calls the leave/close action.
- [ ] Optionally auto-dismiss the dialog on error and show the error inline on the main screen (leave the dialog for clean retries only).

---

### P2.6 — No active-state styling for support nav links during tournament (`AppFrame`)

**File:** `src/components/layout/AppFrame.tsx`

The tournament topbar support links (Help, Settings) have no `aria-current` or active styling when the user is on those pages.

- [ ] Use `useLocation()` to detect the current path.
- [ ] Apply `aria-current="page"` and an `.active` CSS class to support nav links when their route is active.
- [ ] Add a CSS rule for `.nav-link.active` with a visible indicator (underline or accent color).

---

### P2.7 — Help screen scroll-to-section does not re-trigger on revisit (`RulesHelpScreen`)

**File:** `src/screens/RulesHelpScreen.tsx`

The smooth-scroll to a `context`-derived anchor only triggers on mount. If the user navigates back to the help screen, `location.state.context` is no longer present and the scroll does not fire.

- [ ] Move the scroll `useEffect` to depend on `[location.state?.context]` instead of `[]`.
- [ ] Pass `location.state` explicitly when navigating to `/rules` from error recovery links so the context is preserved.
- [ ] Consider using `scrollIntoView` with a short `setTimeout` to handle layout timing.

---

### P2.8 — Startup warnings section shows only first warning message (`HomeScreen`)

**File:** `src/screens/HomeScreen.tsx`

If multiple startup warnings exist, the resume-rail section shows a single "Some saved local data was unreadable" message. Users cannot tell how many files failed or which ones.

- [ ] Expand the startup warnings block to list each warning from `startupWarnings` (filename + error summary).
- [ ] If `startupWarnings.length > 3`, collapse the list with a "Show all" expand toggle.

---

### P2.9 — No indication of invite code validity period (`JoinTournamentScreen`)

**File:** `src/screens/JoinTournamentScreen.tsx`

After validating a join payload, there is no indication of how long the invite is valid or whether the host is still accepting connections. A stale invite shows as "valid" until the user tries to join.

- [ ] After a successful validation, if `payload.timestamp` (or equivalent) is available, show a subtle note: `"Invite decoded — host availability not confirmed until you join"`.
- [ ] If the backend `validateJoinPayloadInput` can return a host-reachability status, surface it in the preview card as a "Host reachable" / "Host unreachable" badge.

---

### P2.10 — Error scenario recovery path is one-way in `ErrorStateScreen`

**File:** `src/screens/ErrorStateScreen.tsx`

Scenario determination is computed once on mount. If the LAN error resolves after the screen loads (e.g., network interface comes up), the user is stuck on the error screen with no way to re-check short of navigating away and back.

- [ ] Add a "Re-check" button to the `invalid-lan-ip` scenario that re-runs the LAN resolution check without a full navigation cycle.
- [ ] If the check succeeds, navigate to the appropriate recovery route (e.g., `/host` if a host draft exists, or `/`).

---

### P2.11 — Confirmation card in `MainTableScreen` is not announced to screen readers

**File:** `src/screens/MainTableScreen.tsx`

The confirmation card (raise confirmation, all-in confirmation) uses `role="status"` but dynamically swaps content. Screen reader users may miss that a confirmation is required.

- [ ] Change `role="status"` to `role="alert"` on the confirmation card container (assertive: interrupts for required user action).
- [ ] Alternatively, use `aria-live="assertive"` on the container and keep `role="region"`.
- [ ] Add a visually-hidden label to the confirmation card: `"Action confirmation required"`.

---

### P2.12 — Observable seat-state transitions have no animation (`TournamentLobbyScreen`)

**File:** `src/screens/TournamentLobbyScreen.tsx`, `src/styles/03-panels.css`

When a seat badge transitions from "Waiting" to "Ready", it changes instantly with no animation. This is jarring when watching multiple players ready up.

- [ ] Add a CSS transition on the `.status-badge` class: `transition: background-color 0.25s ease, color 0.25s ease;`.
- [ ] Verify the transition fires for all tone changes (info → success, etc.) and does not interfere with the dense layout.

---

### P2.13 — History and Events sections show empty silently after live-view failure (`HandHistoryScreen`)

**File:** `src/screens/HandHistoryScreen.tsx`

See P1.5 above. Even after adding the persisted-history banner, the chip-order standings and events feed sections remain empty with no explanation when the live table view fails.

- [ ] When `tableViewError` is set, render a `"—"` placeholder row in the Standings and Events cards, accompanied by a small note: `"Live data unavailable"`.
- [ ] This is separate from the hand list fallback banner in P1.5 — both must be addressed.

---

### P2.14 — `NpcProfilesScreen` parsed-section detection is silent when headings mismatch

**File:** `src/screens/NpcProfilesScreen.tsx`

The collapsible "Parsed sections" panel only appears if the profile contains `## Opponent tendencies` or `## Tilt behaviour` exactly. A user who writes `## Opponent Tendencies` (capital T) or `## Tilt Behavior` (US spelling) sees nothing.

- [ ] Make the heading match case-insensitive and trim whitespace before comparing.
- [ ] Support `Tilt Behavior` as an alias for `Tilt behaviour`.
- [ ] If a profile has frontmatter but no recognized sections, show a small hint below the textarea: `"No parsed sections found. Add ## Opponent tendencies or ## Tilt behaviour headings to enable structured preview."`.

---

### P2.15 — `MainTableScreen` does not highlight recently eliminated players in the side panel

**File:** `src/screens/MainTableSidePanel.tsx`

When a player is eliminated, the standings list updates instantly with no visual emphasis. Users watching the side panel may miss who was just eliminated.

- [ ] When `standings` changes and a new entry has `eliminated: true` (or equivalent), apply an `.eliminated-flash` CSS class to that row for ~2 seconds.
- [ ] CSS: `@keyframes eliminated-flash { 0% { background: rgba(220, 38, 38, 0.3); } 100% { background: transparent; } }`.
- [ ] Use a `useRef`-tracked set to avoid re-flashing on subsequent renders.

---

## Accessibility checklist (cross-cutting)

These items apply across multiple screens and should be addressed in a single pass after the individual bugs are fixed.

- [ ] **Color-only state indication:** Audit every banner, badge, and pill for cases where state is communicated by color only. Each must also have a text or icon label that conveys the same meaning.
- [ ] **Icon-only buttons:** Audit every button that contains only a Lucide icon and no text. Each must have `aria-label` or a visually-hidden `<span>`. Known candidates: side panel toggle (`PanelRight`), any icon-only nav buttons.
- [ ] **Heading hierarchy:** Audit each screen for correct `h1`/`h2`/`h3` nesting. `ScreenShell` renders an `h2`; confirm no child components jump to `h4` or deeper without an intervening `h3`.
- [ ] **Keyboard-accessible seat grid:** Confirm that open seat buttons and filled seat cards are reachable by Tab in both dense and non-dense lobby layouts. Check `tabIndex` and focus order.
- [ ] **Keyboard-accessible recent invite pills:** Confirm that recent invite pills in `JoinTournamentScreen` are reachable by Tab and activatable by Enter/Space.
- [ ] **Modal focus trap:** The leave-flow dialog and raise-confirmation card must trap Tab focus within the dialog while open. Confirm focus does not escape to the background.
