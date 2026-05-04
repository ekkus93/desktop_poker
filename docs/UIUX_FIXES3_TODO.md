# UI/UX Fixes 3 TODO

This is the next execution backlog for turning Desktop Poker into a viewport-first poker app.

The previous backlog fixed product identity, player-facing language, shell structure, and support-surface demotion. The remaining problem is now more specific: the app still behaves too much like a vertically stacked document. A poker app should keep the current task, current state, and current action visible without making the player scroll to find them.

This file is the backlog for fixing that.

Status legend:

- [ ] not started
- [~] in progress
- [x] done

## 0. Non-negotiable viewport bar

- [x] The current decision and current table state should fit in one viewport on normal desktop sizes.
- [x] Scrolling should be treated as a failure for primary play surfaces unless the content is explicitly secondary.
- [x] A poker screen should be laid out like a game surface, not like a document page.
- [x] Text volume must be reduced until the player can parse the screen by scanning, not reading paragraphs.
- [ ] No screen in this file should be marked done until it has been checked at realistic desktop heights, not just wide monitors.

## 1. Current reality check

These are the problems this backlog exists to fix.

- [x] Home still stacks multiple large cards vertically and can overflow moderate-height windows.
- [x] Host still uses document-style sections instead of one integrated host surface.
- [x] Join still uses vertical stacked cards instead of a compact invite-acceptance layout.
- [x] Lobby still spends too much height on stacked sections and repeated labels.
- [x] Main Table still treats details as page content instead of a height-budgeted game layout.
- [x] Support surfaces still inherit content-page spacing that wastes viewport height.
- [x] The global shell still uses spacing and min-height rules designed for content readability instead of fit-to-screen game views.
- [x] The app has no explicit screen-height budget, no viewport-fit rules, and no definition of when a screen is allowed to scroll.

## 2. Layout policy before any screen rewrite

Owning code:

- `src/App.css`
- shared shell/layout components

Goals:

- [x] Define a real viewport-first layout policy for the whole app.
- [x] Stop treating every screen like a natural-height document.
- [x] Create explicit rules for what can scroll and what cannot.

Tasks:

- [x] Define target desktop viewport sizes to optimize for.
	- [x] Primary baseline height budget, e.g. around 768px tall windows.
	- [x] Secondary compact-height budget for smaller laptop windows.
	- [x] Explicit statement of what is allowed to collapse below those sizes.
- [x] Introduce viewport-aware height tokens in CSS.
	- [x] App frame height budget.
	- [x] Header/topbar height budget.
	- [x] Content area height budget.
	- [x] Inner panel height budget.
- [x] Define a surface policy.
	- [x] Primary surfaces should fit without page scrolling.
	- [x] Secondary drawers/panels may scroll internally.
	- [x] Modal dialogs may scroll internally if needed.
	- [x] Long historical/reference content may scroll, but only inside bounded containers.
- [x] Reduce global spacing defaults.
	- [x] Tighten outer page padding.
	- [x] Tighten inter-section gaps.
	- [x] Tighten card padding where safe.
	- [x] Tighten heading-to-body spacing.
- [x] Stop using natural vertical growth as the default layout strategy.
	- [x] Prefer grid/flex layouts with allocated rows.
	- [x] Prefer `minmax(0, 1fr)` patterns over content-led growth.
	- [x] Prefer internal overflow regions over page overflow for secondary details.

Acceptance bar:

- [x] The repo has a clear, reusable layout policy for viewport-first game screens.
- [x] The CSS makes it hard to accidentally reintroduce document-style screen growth.

## 3. Audit the shell for height waste

Owning code:

- `src/components/layout/AppFrame.tsx`
- `src/App.css`

Goals:

- [x] Make the shell consume less vertical space.
- [x] Keep the shell informative without stealing height from the actual screen.

Tasks:

- [x] Audit topbar height.
	- [x] Reduce vertical padding.
	- [x] Reduce brand block height.
	- [x] Make route/context pills more compact.
- [x] Audit in-tournament sidebar height and density.
	- [x] Remove any non-essential copy.
	- [x] Tighten nav item height.
	- [x] Tighten section spacing.
- [x] Audit content padding inside `.content` and `.screen-shell`.
	- [x] Reduce default page padding.
	- [x] Reduce header-to-body gap.
	- [x] Reduce body-to-actions gap.
- [x] Decide whether the topbar/sidebar should become shorter in compact-height mode.
	- [x] Add height-based media queries, not just width-based ones.

Acceptance bar:

- [x] The shell reads clearly while consuming materially less height.

## 4. Rebuild Home as a one-viewport landing screen

Owning code:

- `src/screens/HomeScreen.tsx`
- `src/screens/HomeScreen.test.tsx`
- `src/App.css`

Goals:

- [x] Home should fit in one viewport on normal desktop heights.
- [x] The player should see only the current choice and an optional compact recovery strip.
- [x] No scrolling should be needed on Home for the normal case.

Tasks:

- [x] Collapse Home into one dominant hero region.
	- [x] One heading.
	- [x] One lead line.
	- [x] Two dominant actions.
	- [x] Minimal support affordance.
- [x] Compress or merge the host/join explanation cards.
	- [x] Decide whether both cards are needed at all.
	- [x] If retained, shorten them to one compact row each.
	- [x] Prefer visual emphasis over explanatory text.
- [x] Convert saved progress into a compact recovery rail.
	- [x] Keep it hidden unless there is real recovery state.
	- [x] Present recovery as short actions, not mini-panels.
	- [x] Avoid repeated headings and helper copy.
- [x] Add explicit height budgeting.
	- [x] Ensure Home centers vertically within the viewport.
	- [x] Prevent Home from growing naturally taller due to card stacking.
	- [x] Introduce compact-height layout behavior.

Acceptance bar:

- [x] Home fits without scrolling at the target desktop height.
- [x] A player can understand the screen in one glance.

## 5. Rebuild Host as a single-screen host station

Owning code:

- `src/screens/HostTournamentSetupScreen.tsx`
- `src/screens/HostTournamentSetupScreen.test.tsx`
- `src/App.css`

Goals:

- [x] Host should fit as one integrated hosting surface.
- [x] The player should see setup, invite, and continue action without vertical page scrolling.

Tasks:

- [x] Replace vertical section stacking with a viewport-aware grid.
	- [x] Setup column or zone.
	- [x] Invite/share column or zone.
	- [x] LAN readiness integrated into the share area.
- [x] Compress form density.
	- [x] Reduce label/help spacing.
	- [x] Shorten field labels where possible.
	- [x] Move explanatory copy into fewer, tighter lines.
- [x] Keep advanced settings out of the normal layout budget.
	- [x] Collapsed by default.
	- [x] Opens in a bounded area or modal.
	- [x] Does not push the main host surface off-screen.
- [x] Keep the continue action visible in the default viewport.
	- [x] No vertical hunt to reach the lobby action.

Acceptance bar:

- [x] A host can configure, share, and continue without page scrolling.

## 6. Rebuild Join as a compact invite acceptance screen

Owning code:

- `src/screens/JoinTournamentScreen.tsx`
- `src/screens/JoinTournamentScreen.test.tsx`
- `src/App.css`

Goals:

- [x] Join should fit in one viewport as a compact accept-invite flow.
- [x] The invite input, preview, and continue action should all be visible together.

Tasks:

- [x] Replace stacked Step 1 / Step 2 / Step 3 cards with one integrated layout.
	- [x] Invite input area.
	- [x] Invite preview area.
	- [x] Continue action area.
- [x] Compress recent invites.
	- [x] Render as a compact shortcut row/list.
	- [x] Avoid full-height list-panel presentation where possible.
	- [x] Keep them visually secondary.
- [x] Tighten post-validation layout.
	- [x] Continue action should appear without pushing content down.
	- [x] Preview should stay compact and glanceable.
- [x] Add compact-height behavior.
	- [x] Secondary invite shortcuts can collapse or scroll internally.

Acceptance bar:

- [x] A player can paste, confirm, and continue without scrolling the page.

## 7. Rebuild Lobby as a one-screen ready room

Owning code:

- `src/screens/TournamentLobbyScreen.tsx`
- `src/screens/TournamentLobbyScreen.test.tsx`
- `src/App.css`

Goals:

- [x] The lobby should fit as a ready room in one viewport.
- [x] Readiness, startability, and seat presence should all be visible together.

Tasks:

- [x] Merge the current stacked sections into a viewport-aware lobby layout.
	- [x] One compact readiness summary.
	- [x] One seat area.
	- [x] One start/leave action area.
- [x] Reduce seat card height.
	- [x] Shorten labels.
	- [x] Remove repeated state wording.
	- [x] Collapse secondary seat detail.
- [x] Decide whether seat actions should be icon+label pills instead of full buttons.
- [x] Keep “Start tournament” always visible.
- [x] Ensure leave flow appears as a modal/overlay without expanding the page.

Acceptance bar:

- [x] The lobby fits in one viewport at the target desktop height.
- [x] The player never needs to scroll to see who is ready and whether the table can start.

## 8. Rebuild the Main Table as a true fixed game surface

Owning code:

- `src/screens/MainTableScreen.tsx`
- `src/screens/MainTableScreen.test.tsx`
- `src/App.css`

Goals:

- [x] The main table should be a fixed-height game surface.
- [x] The player should not need page scrolling during live play.
- [x] Secondary detail should live in bounded side panels, drawers, overlays, or tabs.

Tasks:

- [x] Define the table viewport layout.
	- [x] Toolbar row height budget.
	- [x] headline/status row height budget.
	- [x] board area height budget.
	- [x] seat ring/grid height budget.
	- [x] action tray height budget.
- [x] Prevent the table page itself from growing taller than the viewport.
	- [x] Convert outer table layout to explicit rows.
	- [x] Keep scrolling internal to side/detail containers only.
- [x] Rework the detail panel.
	- [x] Keep it closed by default if necessary.
	- [x] Use tabs/segmented controls for standings / feed / history if vertical stacking is too tall.
	- [x] Bound internal scroll regions.
- [x] Reduce seat surface height.
	- [x] Compress markers, chip lines, and buttons.
	- [x] Move seat notes into overlays or popovers only.
	- [x] Avoid making every seat a mini document.
- [x] Keep action controls visually and spatially dominant.
	- [x] Ensure fold/check/raise/all-in never fall below the fold.
	- [x] Ensure raise controls do not force extra page growth.
	- [x] Consider compact quick-action variants.
- [x] Re-check observer mode.
	- [x] Observer preview must stay within the same viewport budget.
	- [x] Hidden debug-only features must not change player-layout assumptions.

Acceptance bar:

- [x] During normal play, the player does not scroll the page to see the board, seats, and actions.
- [x] Secondary detail is available without displacing the main play surface.

## 9. Refit support surfaces to bounded, secondary layouts

Owning code:

- `src/screens/HandHistoryScreen.tsx`
- `src/screens/RulesHelpScreen.tsx`
- `src/screens/TournamentCompleteScreen.tsx`
- `src/screens/ErrorStateScreen.tsx`
- `src/App.css`

Goals:

- [x] Secondary screens should be compact and bounded.
- [x] Long content should scroll inside a clear content region, not by making the whole page feel endless.

Tasks:

- [x] Hand History
	- [x] Keep the primary history list in a bounded scroll area.
	- [x] Compress standings and feed presentation.
	- [x] Avoid three large equal-weight columns if height is the actual bottleneck.
- [x] Game Help
	- [x] Reduce vertical padding and list spacing.
	- [x] Keep settings/actions compact.
	- [x] Ensure the help screen feels short and skimmable.
- [x] Tournament Complete
	- [x] Keep result, next action, and final hands within a bounded layout.
	- [x] Avoid long vertical stacks.
- [x] Recovery / Error
	- [x] Keep the primary explanation and next action visible immediately.
	- [x] Hide debug-only review affordances from normal layout assumptions.

Acceptance bar:

- [x] Support screens feel secondary and spatially disciplined.

## 10. Add explicit compact-height responsiveness

Owning code:

- `src/App.css`
- affected screens

Goals:

- [x] The app should respond to short windows, not just narrow windows.

Tasks:

- [x] Add height-based media queries for compact laptop heights.
- [x] Define what shrinks first.
	- [x] padding
	- [x] gaps
	- [x] heading sizes
	- [x] badge rows
	- [x] secondary helper copy
- [x] Define what collapses second.
	- [x] secondary cards
	- [x] detail rails
	- [x] side panels
	- [x] history/feed regions
- [x] Define what never disappears.
	- [x] current action
	- [x] current state
	- [x] primary continue/start buttons
	- [x] essential game status

Acceptance bar:

- [x] Short-height windows degrade gracefully without turning the app into a scroll hunt.

## 11. Reduce text volume screen by screen

Owning code:

- all player-facing screens

Goals:

- [x] Replace explanatory text with clearer structure and stronger visual emphasis.
- [x] Keep only the words needed for the current decision.

Tasks:

- [x] Audit every heading, lead, helper line, and card description.
- [x] Remove repeated phrasing that restates what the layout already implies.
- [x] Shorten button labels where the destination is obvious.
- [x] Replace multi-line helper copy with single-line status when possible.
- [x] Use text removal, not typography alone, to recover height.

Acceptance bar:

- [x] No primary screen feels wordy for the task it is asking the player to perform.

## 12. Redefine validation for viewport-fit work

Goals:

- [~] Add a real viewport-fit definition of done for layout work.

Tasks:

- [x] Keep the existing executable validation.
	- [x] `npm run lint`
	- [x] `npm run test`
	- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
	- [x] `cargo test --manifest-path src-tauri/Cargo.toml`
- [~] Add visual validation requirements.
	- [ ] Check Home at the target desktop height.
	- [ ] Check Host at the target desktop height.
	- [ ] Check Join at the target desktop height.
	- [ ] Check Lobby at the target desktop height.
	- [ ] Check Table at the target desktop height during live play.
- [~] For each primary screen, explicitly record:
	- [ ] page scroll present or absent
	- [ ] internal scroll regions present or absent
	- [ ] current action visible without scrolling
	- [ ] current state visible without scrolling

Acceptance bar:

- [ ] A screen is not marked done merely because tests pass; it must also fit the viewport policy.

Note:

- [~] Automated validation is complete. Browser-based visual verification is blocked in the current workspace because the web preview crashes without the Tauri desktop bridge (`transformCallback` is undefined), so the remaining checks must be run in a real desktop session.

## 13. Suggested execution order

- [x] Phase A: define the viewport-fit layout policy and tighten the shell.
- [x] Phase B: rebuild Home, Host, and Join for one-viewport fit.
- [x] Phase C: rebuild Lobby for one-viewport fit.
- [x] Phase D: rebuild Main Table as a fixed game surface.
- [x] Phase E: refit support surfaces and compact-height responsiveness.
- [~] Phase F: final visual audit, validation, commit, and push.

## 14. Definition of done

Do not mark this effort complete until all of these are true.

- [ ] Home fits in one viewport on normal desktop heights.
- [ ] Host fits in one viewport on normal desktop heights.
- [ ] Join fits in one viewport on normal desktop heights.
- [ ] Lobby fits in one viewport on normal desktop heights.
- [ ] Main Table keeps board, seats, and actions visible without page scrolling during live play.
- [x] Support screens use bounded scroll regions instead of endless page growth.
- [ ] The player never has to scroll to find the current action they are supposed to take.
- [ ] The player never has to scroll to find the current game state they need right now.
- [ ] The app feels like a game surface, not a vertically stacked document.
