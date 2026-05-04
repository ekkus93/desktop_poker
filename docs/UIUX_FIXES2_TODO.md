# UI/UX Fixes 2 TODO

This is the replacement execution backlog for turning Desktop Poker into a real poker game app rather than a polished QA harness.

The earlier backlog captured the intended direction, but it was too willing to mark work complete when the app still looked and behaved like a desktop workflow tool. This file is the stricter follow-up plan.

Status legend:

- [ ] not started
- [~] in progress
- [x] done

## 0. Non-negotiable product bar

- [ ] The app must feel like a poker game first and a desktop app second.
- [ ] A normal user must be able to host or join without seeing developer, QA, routing, or internal-tool language.
- [ ] The app must stop looking like a sidebar-driven admin shell before any more polish is treated as complete.
- [ ] No item in this file should be marked done until both code and live UI behavior match the stated outcome.

## 1. Current reality check

These are the problems this backlog exists to fix.

- [ ] The pre-tournament experience still reads like app navigation instead of game entry.
- [ ] The home screen is still card-and-utility driven instead of being a strong game landing surface.
- [ ] Host and Join still read like forms inside a product shell, not like intentional full-screen flows.
- [ ] History and Rules are still too present in the pre-game experience.
- [ ] Debug/internal access still influences the information architecture too much, even when visually reduced.
- [ ] The table art direction is stronger than the entry flow direction, so the app identity does not start strong enough.
- [ ] The repo needs a more honest definition of done for visual/product work.

## 2. Immediate code-health checkpoint

Before deeper UI work continues, stabilize the current in-progress shell rewrite so the repo is not carrying silent drift.

- [x] Run full validation after the latest shell rewrite.
	- [x] `npm run lint`
	- [x] `npm run test`
	- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
	- [x] `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] Fix any compile/lint drift introduced by the latest shell changes before additional UI work.
- [x] Confirm the pre-tournament shell still renders correctly in dev and in the test harness.

## 3. Replace the pre-tournament shell for real

Owning code:

- `src/components/layout/AppFrame.tsx`
- `src/app/AppShell.tsx`
- `src/App.css`

Goals:

- [x] Stop using a persistent app-navigation mentality for the landing flow.
- [~] Make pre-tournament screens feel like entering a game, not navigating an admin console.
- [~] Keep support surfaces reachable without visually competing with the main poker path.

Tasks:

- [~] Decide the true pre-tournament shell model.
	- [ ] Option A: no persistent navigation at all on Home.
	- [x] Option B: only a minimal top brand bar plus a single quiet support affordance.
	- [x] Reject any design that still feels like `Play / Support` app chrome.
- [x] Remove any remaining shell text that sounds like a product tour or workflow explanation.
- [x] Remove any visible debug/internal promotion from the normal shell, even in dev builds.
- [x] Decide whether History and Rules belong in the shell at all before a game starts.
	- [ ] If yes, demote them harder.
	- [x] If no, move them behind a single secondary menu or footer affordance.
- [~] Keep player identity visible only if it helps the next action.
	- [x] If the player label is retained, make it feel like a table identity, not a dev profile tag.

- [~] Re-check shell behavior by route.
	- [x] Home
	- [x] Host
	- [x] Join
	- [ ] Rules
	- [ ] History
	- [x] Lobby
	- [ ] Table
	- [ ] Complete
	- [ ] Errors

Acceptance bar:

- [ ] The user’s first impression is “this is a poker game app.”
- [ ] The shell does not feel like software navigation before it feels like a game surface.

## 4. Rebuild the home screen as a game landing page

Owning code:

- `src/screens/HomeScreen.tsx`
- `src/screens/HomeScreen.test.tsx`
- `src/App.css`

Goals:

- [~] The home screen should feel like the front door to a poker table.
- [ ] The user should see exactly two main paths: host a game or join a game.
- [~] The home screen should stop reading like a dashboard with support cards.

Tasks:

- [~] Reframe the home hero.
	- [x] Stronger title than the current generic product wording.
	- [x] One short line that sounds like poker, not workflow management.
	- [x] Make Host and Join feel like the only decisions that matter.
- [~] Rework the visual layout.
	- [x] Host and Join should dominate the fold.
	- [x] Remove or heavily demote utility-style cards.
	- [ ] Make the landing space feel centered and table-oriented.
- [~] Rethink the resume area.
	- [x] Keep it hidden unless there is actually something useful to resume.
	- [x] Avoid showing generic empty-state furniture by default.
	- [~] Make saved-history and saved-invite recovery feel like secondary recovery options, not equal-weight actions.
- [x] Decide whether Rules/Help belongs on Home at all.
	- [x] If it stays, reduce it to a low-emphasis support action.
- [~] Tighten home copy again.
	- [~] Remove any remaining “saved tables,” “review invite,” or utility-language tone where a more game-oriented phrasing would work.

Acceptance bar:

- [ ] A user can glance at Home for two seconds and know whether to host or join.
- [ ] No part of the Home screen feels like a debug panel, control center, or settings page.

## 5. Turn Host into a real host flow

Owning code:

- `src/screens/HostTournamentSetupScreen.tsx`
- `src/screens/HostTournamentSetupScreen.test.tsx`
- `src/app/shell.ts`
- `src/App.css`

Goals:

- [~] Hosting should feel like starting a table, not configuring a piece of software.
- [x] The share/invite step should feel productized, not like copying structured output.

Tasks:

- [x] Rework the visual hierarchy of Host.
	- [x] One clear setup area.
	- [x] One clear share/invite area.
	- [x] One clear continue action.
- [x] Improve the share surface.
	- [x] Make the invite feel like an invite card, not a text area dump.
	- [x] Show the table name and join instructions more like a host handoff.
	- [x] Keep copy action obvious but not technical.
- [~] Reconsider the LAN-readiness panel.
	- [x] It should reassure, not expose implementation detail.
	- [~] Failure states should stay plain-language and local.
- [x] Keep advanced settings truly advanced.
	- [x] They should not look like part of the main task.

Acceptance bar:

- [ ] Hosting feels like “start a table and invite players.”
- [ ] It no longer feels like network setup.

## 6. Turn Join into a real invite flow

Owning code:

- `src/screens/JoinTournamentScreen.tsx`
- `src/screens/JoinTournamentScreen.test.tsx`
- `src/App.css`

Goals:

- [~] Joining should feel like accepting an invite to a table.
- [~] The screen should not feel like validating a token or pasting structured app data.

Tasks:

- [x] Rework the screen to make the invite input feel like the whole screen’s purpose.
- [x] Make the review step feel confident and product-like.
	- [x] Host destination
	- [x] Table name
	- [x] Ready-to-continue state
- [~] Rework recent invites so they feel like useful shortcuts, not persisted state management.
- [~] Remove any remaining “review tool” or “parser” feeling from the page.

Acceptance bar:

- [ ] A user pastes an invite and immediately understands whether they can continue.

## 7. Tighten the lobby into a waiting room

Owning code:

- `src/screens/TournamentLobbyScreen.tsx`
- `src/screens/TournamentLobbyScreen.test.tsx`
- `src/App.css`

Goals:

- [~] The lobby should feel like people gathering at a table.
- [~] It should not feel like roster management.

Tasks:

- [x] Push readiness and startability higher than all other detail.
- [x] Make the local player’s state more immediate and less verbose.
- [~] Reduce card/chrome density in the seat map.
- [x] Keep “Leave table” available but far less competitive with “Start tournament.”
- [x] Revisit copy so it feels more human and less status-board-like.

Acceptance bar:

- [ ] The lobby reads as a waiting room, not a control console.

## 8. Keep pushing the table toward premium gameplay

Owning code:

- `src/screens/MainTableScreen.tsx`
- `src/screens/MainTableScreen.test.tsx`
- `src/App.css`

Goals:

- [ ] The table must remain the strongest surface in the app.
- [ ] Supporting panels must never pull more attention than turn state or action state.

Tasks:

- [ ] Review the balance between felt, cards, action tray, and side panels again after the entry-flow rewrite.
- [ ] Make sure action controls are still the strongest interactive cluster.
- [ ] Reassess side-panel visibility and weight.
- [ ] Improve card and seat composition further if needed to keep the table premium.

Acceptance bar:

- [ ] The app reaches its strongest visual identity once the user gets to the table.

## 9. Demote support surfaces harder

Owning code:

- `src/screens/RulesHelpScreen.tsx`
- `src/screens/HandHistoryScreen.tsx`
- `src/screens/TournamentCompleteScreen.tsx`
- `src/screens/ErrorStateScreen.tsx`
- `src/App.css`

Goals:

- [ ] Support surfaces should feel helpful, not co-equal with the core play flow.

Tasks:

- [ ] Decide whether Rules should be renamed or repositioned.
	- [ ] `Game Help`
	- [ ] `How to Play`
	- [ ] hidden in a quieter help affordance
- [ ] Keep Hand History clearly secondary until the player intentionally opens it.
- [ ] Keep Tournament Complete emotionally satisfying and concise.
- [ ] Keep Error/Recovery direct and singular.

Acceptance bar:

- [ ] Support screens no longer shape the product identity more than the poker flow does.

## 10. Hide internal tooling properly

Owning code:

- `src/app/AppShell.tsx`
- `src/components/debug/DebugPanel.tsx`
- `src/components/layout/AppFrame.tsx`
- related tests

Goals:

- [ ] Internal tools should exist for development without being part of the normal app architecture.

Tasks:

- [ ] Remove any remaining shell or copy logic that assumes internal tools are a normal destination.
- [ ] Decide the real dev-only entry mechanism.
	- [ ] explicit hidden route only
	- [ ] explicit dev flag affordance outside player flow
	- [ ] keyboard shortcut or separate launcher path
- [ ] Ensure no normal player screen references internal tooling.

Acceptance bar:

- [ ] Debug capability exists, but the product does not advertise it.

## 11. Rewrite the visual hierarchy honestly

This section is about not lying to ourselves again.

- [ ] Re-audit the actual first impression after each major shell change.
- [ ] Do not mark a screen complete because tests pass if the visual hierarchy still reads like a tool.
- [ ] Require a visual judgment checkpoint after each phase.
- [ ] Update the README and any UX docs only after the live UI really matches the claims.

## 12. Validation requirements for every phase

- [ ] Focused tests for touched screens.
- [ ] `npm run lint`
- [ ] `npm run test`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] Live UI review in the running app.
- [ ] Only then commit and push.

## 13. Suggested execution order

- [x] Phase A: stabilize current shell rewrite and remove any lint/test drift.
- [~] Phase B: finish the pre-tournament shell replacement.
- [~] Phase C: rebuild Home as a true landing page.
- [~] Phase D: rebuild Host and Join as full-screen product flows.
- [~] Phase E: retune Lobby and support surfaces.
- [ ] Phase F: final art-direction and hierarchy pass across the whole app.
- [ ] Phase G: final validation, documentation truth pass, and push.

## 14. Definition of done

Do not mark this effort complete until all of these are true.

- [ ] The first impression is “poker game,” not “desktop tool.”
- [ ] Home feels like entering a table flow, not using an application menu.
- [ ] Host feels like opening a table, not configuring software.
- [ ] Join feels like accepting an invite, not validating a payload.
- [ ] Lobby feels like waiting for a game to begin, not managing a roster.
- [ ] Table remains the visual and emotional center of the product.
- [ ] Support screens are present but subordinate.
- [ ] Internal tools exist without affecting the normal app identity.
- [ ] README claims match the actual shipped UI.