# UI/UX Fixes 4 TODO

This is the next execution backlog for turning Desktop Poker into a self-evident poker app.

The previous backlog fixed product identity, player flow, and viewport fit. The remaining problem is more specific: some screens still explain themselves with extra text instead of making the next action obvious through layout, labels, and state.

This file is the backlog for fixing that.

Status legend:

- [ ] not started
- [~] in progress
- [x] done

## 0. Non-negotiable clarity bar

- [x] Primary actions should be obvious without helper paragraphs.
- [x] Status text should communicate state, not teach the player how buttons work.
- [x] Screens should be understandable by scanning labels, hierarchy, and affordances.
- [x] Explanatory copy should exist only when the player would otherwise make the wrong decision.
- [x] Every screen in this file should be reviewed for both visual noise and duplicated meaning.

## 1. Current clarity problems

- [x] Home used redundant narration around actions and recovery.
- [x] Host used helper text to explain invite sharing and LAN readiness.
- [x] Join used helper text to explain invite review and preview.
- [x] Lobby used repeated readiness and seat-state wording.
- [x] Main Table used support-side wording that read like instruction text.
- [x] History, Rules, Complete, and recovery states used verbose summaries where labels were enough.
- [x] Shared shell copy has been reduced and ScreenShell usage has been fully audited.

## 2. Copy policy before any additional screen edits

Owning code:

- `src/screens/ScreenShell.tsx`
- `src/components/layout/AppFrame.tsx`
- shared labels inside screen components

Goals:

- [x] Define a clear copy threshold for the product.
- [x] Stop reintroducing helper text by habit.

Tasks:

- [x] Treat the following as suspicious by default:
	- [x] lead paragraphs that restate the page title
	- [x] paragraphs below buttons that describe the same action
	- [x] card summaries that merely restate a visible status label
	- [x] empty-state text that explains obvious controls
- [x] Keep only copy that does one of these jobs:
	- [x] clarifies a destructive or irreversible action
	- [x] communicates an error or blocked state
	- [x] names state the UI cannot otherwise make clear
	- [x] provides domain information the player cannot infer
- [x] Prefer shorter labels over helper paragraphs.
- [x] Prefer tighter information hierarchy over explanatory body copy.

Acceptance bar:

- [x] The repo has a consistent standard for when explanatory text is allowed.

## 3. Audit shared shell text

Owning code:

- `src/components/layout/AppFrame.tsx`
- `src/screens/ScreenShell.tsx`

Goals:

- [x] Remove shell copy that does not help the current decision.
- [x] Keep route context visible without turning the shell into documentation.

Tasks:

- [x] Audit `ScreenShell` title + lead usage across all screens.
	- [x] Remove leads that restate the screen purpose.
	- [x] Keep leads only where state or risk truly needs explanation.
- [x] Audit landing-frame topbar text.
	- [x] Ensure back navigation and player identity remain clear.
	- [x] Remove decorative or repetitive phrasing.
- [ ] Audit in-tournament sidebar copy.
	- [x] Remove any phrase that tells the player to do what the current layout already shows.
	- [x] Keep only minimal route and player identity context.

Acceptance bar:

- [x] Shared shell text feels structural, not narrative.

## 4. Finish Home as a self-evident landing screen

Owning code:

- `src/screens/HomeScreen.tsx`
- `src/screens/HomeScreen.test.tsx`

Goals:

- [x] Home should center the two primary choices without explanation.
- [x] Recovery should read like available state, not prose.
- [x] Remaining labels should be challenged for brevity and clarity.

Tasks:

- [x] Re-check whether `Pull up a chair` is the right title or whether an even clearer product-first title is better.
- [x] Re-check whether `Play` is the strongest hero title or whether the action buttons are sufficient without it.
- [x] Re-check whether `Rules` should stay visible on the main action card or move into less prominent support navigation.
- [x] Re-check whether the recovery rail can become even more action-first.
	- [x] Prefer noun/action labels over descriptive phrases.
	- [x] Ensure launch-payload error text appears only on actual failure.

Acceptance bar:

- [x] A first-time player can land on Home and choose an action immediately.

## 5. Finish Host as a clear host control station

Owning code:

- `src/screens/HostTournamentSetupScreen.tsx`
- `src/screens/HostTournamentSetupScreen.test.tsx`

Goals:

- [x] Host should show setup, readiness, sharing, and continue actions without helper narration.
- [x] Field labels and section labels should now do all the work.

Tasks:

- [x] Audit field labels for brevity.
	- [x] Keep labels specific but short.
	- [x] Remove duplicated units where the control already implies them.
- [x] Audit invite-card labels.
	- [x] Keep only the stats a player needs to share confidently.
	- [x] Remove decorative or duplicate stat labels.
- [x] Audit advanced settings copy.
	- [x] Keep only real diagnostics.
	- [x] Remove general advice phrasing.
- [x] Audit action labels.
	- [x] Confirm `Copy share details` is the clearest button text.
	- [x] Confirm `Continue to lobby` is the clearest progression text.

Acceptance bar:

- [x] A host can understand the screen from labels and state alone.

## 6. Finish Join as a clear invite acceptance flow

Owning code:

- `src/screens/JoinTournamentScreen.tsx`
- `src/screens/JoinTournamentScreen.test.tsx`

Goals:

- [x] Join should let the player paste, review, and continue without prose.
- [x] Validation, preview, and recent-invite affordances should now read as obvious state.

Tasks:

- [x] Audit the join textarea label and button labels.
	- [x] Confirm the player does not need helper text to understand the input.
- [x] Audit preview copy.
	- [x] Keep only state that affects trust in the invite.
	- [x] Remove labels that repeat visible values.
- [x] Audit success/error banners.
	- [x] Keep error copy precise.
	- [x] Keep success copy short and actionable.
- [x] Audit recent invites.
	- [x] Ensure the empty state is as short as possible.
	- [x] Ensure the clear action is proportional and not overexplained.

Acceptance bar:

- [x] A player can confirm where they are going without reading extra explanation.

## 7. Reduce repeated meaning in the Lobby

Owning code:

- `src/screens/TournamentLobbyScreen.tsx`
- `src/screens/TournamentLobbyScreen.test.tsx`

Goals:

- [x] The lobby should show table state, readiness, and next actions without repeated wording.

Tasks:

- [x] Audit the screen title and any lead copy.
- [x] Audit readiness summary text.
	- [x] Remove repeated statements of the same ready/not-ready condition.
	- [x] Prefer concise counts and badges.
- [x] Audit seat cards.
	- [x] Shorten seat state labels.
	- [x] Remove explanatory subtext that duplicates icon, badge, or button meaning.
- [x] Audit start/leave actions.
	- [x] Ensure button labels are direct and require no supporting sentence.

Acceptance bar:

- [x] The lobby reads like a ready room, not like a checklist explanation.

## 8. Reduce explanatory text on the Main Table

Owning code:

- `src/screens/MainTableScreen.tsx`
- `src/screens/MainTableScreen.test.tsx`

Goals:

- [x] The table should read primarily through state, chips, cards, pot, and action controls.

Tasks:

- [x] Audit the screen header and any lead copy.
- [x] Audit side-panel headings and support labels.
	- [x] Remove wording that explains visible game state.
	- [x] Keep only labels that disambiguate data.
- [x] Audit action tray copy.
	- [x] Keep betting controls explicit.
	- [x] Remove any helper phrasing around obvious actions.
- [x] Audit observer/reconnect notices.
	- [x] Keep only meaningful state and blocking information.

Acceptance bar:

- [x] The table feels like a game surface, not a guided walkthrough.

## 9. Reduce support-surface narration

Owning code:

- `src/screens/HandHistoryScreen.tsx`
- `src/screens/RulesHelpScreen.tsx`
- `src/screens/TournamentCompleteScreen.tsx`
- `src/screens/ErrorStateScreen.tsx`

Goals:

- [x] Support screens should remain readable without sounding like documentation.

Tasks:

- [x] Audit History headings, summaries, and empty states.
- [x] Audit Rules headings and intro copy.
	- [x] Keep rules content informative.
	- [x] Remove wrapper copy that merely introduces the rules section.
- [x] Audit Tournament Complete summary copy.
	- [x] Keep outcome clarity.
	- [x] Remove celebratory or explanatory filler if it slows scanning.
- [x] Audit recovery/error screens.
	- [x] Keep error cause and recovery action.
	- [x] Remove any extra narrative sentence that does not change action.

Acceptance bar:

- [x] Secondary screens still read clearly, but faster.

## 10. Validation and review pass

Goals:

- [x] Verify that reduced copy improves clarity instead of hiding meaning.

Tasks:

- [x] Re-run focused tests after each screen batch.
- [x] Run the full frontend test suite once the copy pass is complete.
- [x] Run `npm run lint` after the copy pass.
- [ ] Run a real visual pass in the desktop shell if available.
	- [ ] Check Home for instant comprehension.
	- [ ] Check Host for scan-first hosting.
	- [ ] Check Join for trust and clarity.
	- [ ] Check Lobby for obvious readiness state.
	- [ ] Check Main Table for zero instructional feel.
- [x] Reject any change that makes the UI shorter but more ambiguous.

Note:

- Direct desktop visual inspection is not available from the current CLI agent environment, so the visual pass items remain open even though automated validation is complete.

Acceptance bar:

- [x] The app no longer needs helper prose to explain normal flows.