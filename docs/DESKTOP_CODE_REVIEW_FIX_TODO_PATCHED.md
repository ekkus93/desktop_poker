# DESKTOP_CODE_REVIEW_FIX_TODO.md

This file is intentionally written as a direct implementation order.  
Do exactly what is written.  
Do not reopen product decisions already made in `DESKTOP_SPECS.md` or `DESKTOP_CODE_REVIEW_FIX_SPEC.md`.

## 1. Non-negotiable decisions

Before coding, treat these as fixed:

- Clients must never receive other players' unrevealed hole cards in snapshots.
- Generic snapshots must be recipient-projected before serialization.
- Private hole cards must travel only through the allowed recipient-private path.
- Production workflows must not depend on shell/demo state.
- Ready Room must either be made real or removed from production flow.
- Desktop live gameplay must use the public-event-driven model, with snapshots only for join/reconnect/resync.
- Production UI must not expose fake event feeds or synthetic invite paths.
- Demo/probe/simulator helpers may remain only if they are explicitly debug/internal-only.
- README and user-facing docs must not overstate readiness or correctness.
- This cleanup pass is about correctness and truthfulness, not feature expansion.

---

## 2. M1 — Fix snapshot privacy immediately

### 2.1 Audit all snapshot creation paths
- [x] Find every code path that creates a snapshot or snapshot-like payload for a non-host recipient.
- [x] Enumerate all fields currently included.
- [x] Identify every path where `TournamentState` or equivalent authoritative state is cloned or serialized directly.

### 2.2 Remove direct full-state snapshot transmission
- [x] Stop sending full authoritative `TournamentState` to normal clients if it contains multi-player hidden state.
- [x] Remove any direct serialization path that includes full `hole_cards_by_player_id` for non-host recipients.
- [x] Ensure host-only state never crosses the network to normal clients.

### 2.3 Implement recipient-projected snapshot building
- [x] Create one canonical backend function to build a snapshot for a specific recipient.
- [ ] Input:
  - [x] authoritative state
  - [x] recipient identity / role
- [ ] Output:
  - [x] public state
  - [x] only that recipient's allowed private data
- [ ] Use this function for:
  - [x] join success snapshot
  - [x] reconnect snapshot
  - [x] resync snapshot

### 2.4 Enforce observer projection
- [x] Observers must receive no active players' private cards.
- [x] Eliminated observers must remain read-only.
- [x] Observer snapshots must contain public state only.

### 2.5 Required tests
- [x] join snapshot for active player contains only that player's hole cards
- [x] reconnect snapshot for player A does not contain player B's hole cards
- [x] observer snapshot contains no unrevealed active-player hole cards
- [x] host/internal state is not present in recipient-facing snapshot DTOs

---

## 3. M2 — Make private hole-card delivery real

### 3.1 Audit current private-hole-card flow
- [x] Identify whether encrypted private hole-card delivery is currently used in real production gameplay or only in tests / scaffolding.
- [x] Remove any reliance on generic snapshots to substitute for private delivery.

### 3.2 Implement one canonical private-card delivery path
- [ ] Choose the production path and use it consistently:
  - [x] recipient-projected snapshot for join/reconnect/resync
  - [x] encrypted private envelope for live hand private-card delivery
- [x] Ensure this path is actually invoked during live gameplay.

### 3.3 Enforce recipient-only semantics
- [x] Validate recipient identity before private-card delivery.
- [x] Ensure only the addressed client can receive/decode the payload.
- [x] Ensure frontend/private session state updates only from legitimate private delivery inputs.

### 3.4 Required tests
- [x] active player receives own private cards through the production path
- [x] non-recipient does not receive/decode that private payload
- [x] live gameplay does not require hidden multi-player card state in snapshots

---

## 4. M3 — Remove or quarantine fake/demo/stale code

### 4.1 Inventory fake/demo/probe code
- [ ] Audit the repo for:
  - [x] `demo_controller()`
  - [x] `DesktopTableRuntime`
  - [x] shell-generated participant builders
  - [x] synthetic invite helpers
  - [x] layout probe/test harness UIs
  - [x] fake seat/participant construction helpers
- [x] Classify each item:
  - [x] production
  - [x] debug/internal only
  - [x] delete now

### 4.2 Remove production-adjacent demo runtime usage
- [x] Remove any production path that still instantiates demo controller/runtime state.
- [x] Ensure normal Tauri app startup does not depend on demo runtime objects.

### 4.3 Rename and isolate surviving debug helpers
- [x] Any retained fake/demo helper must be renamed clearly:
  - [x] `debug_...`
  - [x] `internal_...`
  - [x] `demo_...`
  - [x] `probe_...`
- [x] Move them behind explicit debug/internal-only boundaries if possible.

### 4.4 Delete stale synthetic invite generation
- [x] Remove or rewrite any command/helper that generates invite payloads from synthetic/bootstrap values rather than a real live host session.
- [x] Production invite generation must always come from the live host session.

### 4.5 Required checks
- [x] production host/join/table path contains no fake/demo runtime dependency
- [x] synthetic invite generation is not reachable from production UI
- [x] retained debug/demo code is clearly labeled and gated

---

## 5. M4 — Fix or remove the Ready Room

### 5.1 Decide once: real or removed
- [x] Make an explicit implementation choice:
  - [ ] Ready Room becomes a real backend-driven production screen
  - [x] or Ready Room is removed from normal production flow

Do not leave it half-real.

### 5.2 If Ready Room is kept
Implement all of the following:
- [ ] load real session state from backend
- [ ] render real participants/seats/ready states
- [ ] call real backend start-tournament command
- [ ] call real leave/close behavior
- [ ] only transition to the table after authoritative start success
- [ ] remove shell-generated seat/participant data from the screen

### 5.3 If Ready Room is removed
- [x] remove the route from production navigation
- [x] remove links/transitions that depend on it
- [x] simplify flow so lobby transitions directly into the real table/runtime start path

### 5.4 Required tests / checks
- [x] production flow does not rely on shell-generated ready state
- [x] start tournament from UI uses real backend command
- [ ] leave/close from Ready Room (if kept) uses real backend action

---

## 6. M5 — Enforce one runtime truth model

### 6.1 Hard rule
- [x] Live gameplay updates must come from signed public events.
- [x] Snapshots must be limited to join, reconnect, and resync.
- [x] Private data must come from recipient-private delivery.

### 6.2 Audit current client runtime handling
- [x] Identify every incoming runtime event type currently emitted by the networking/runtime layer.
- [x] Identify which ones are ignored by the client session/app-state layer.
- [x] Remove dead-path event handling assumptions.

### 6.3 Make client session consume public events for live play
- [x] Ensure client session/app-state updates on:
  - [x] public gameplay events
  - [x] private hole-card deliveries
  - [x] reconnect/resync results
  - [x] protocol errors
- [x] Do not rely on continuous snapshot pushes as steady-state gameplay truth.

### 6.4 Remove muddled hybrid behavior
- [x] If the host is still calling snapshot sync after every action, reduce that to the allowed cases only unless explicitly required for recovery.
- [x] Do not keep a silent “snapshots do everything anyway” fallback in production live play.

### 6.5 Required tests
- [x] client live state advances from public events
- [x] reconnect/resync still restores state correctly via snapshot
- [x] private-card updates use private path
- [x] ordinary gameplay does not require full-state snapshot spam

---

## 7. M6 — Fix event-feed truthfulness

### 7.1 Audit current event-feed UI
- [x] Find every UI panel that claims to show live public events, history, or latest actions.
- [x] Trace each one back to actual backend/runtime data.

### 7.2 Make it real or remove it
For each production event/history panel:
- [x] wire it to real event data, or
- [ ] remove/hide it until real data exists

### 7.3 Required rule
- [x] Do not leave production UI surfaces that always receive `Vec::new()` or equivalent empty placeholder data while presenting themselves as live event feeds.

### 7.4 Required checks
- [x] production event-feed UI shows real public event data if visible
- [ ] or the panel is removed/hidden from production builds

---

## 8. M7 — Clean up UI truthfulness and shell scaffolding

### 8.1 Audit shell helpers
- [x] Audit `buildParticipantShell(...)` and any similar helpers.
- [x] Determine whether they are still used by production routes.
- [x] Remove them from production routes.

### 8.2 Remove misleading semantics
- [x] Fix any UI models that assign misleading participant kinds/states (for example, all seated participants styled as “host”).
- [x] Align labels with real session truth.

### 8.3 Remove prototype-only wording
- [ ] Audit visible strings for prototype/demo/debug wording that should not appear in production.
- [ ] Move debug wording behind debug-only panels.

### 8.4 Required checks
- [ ] production UI renders only real session data
- [ ] no production route depends on shell-generated participant lists
- [ ] visible copy no longer implies fake functionality

---

## 9. M8 — Clean up README and docs

### 9.1 Audit claims
- [x] Review README and desktop docs for claims about:
  - [x] LAN readiness
  - [x] reconnect/resync quality
  - [x] host/join completeness
  - [x] event-driven runtime
  - [x] production usability

### 9.2 Downgrade or update claims to match reality
- [x] If functionality is not truly production-ready yet, say so plainly.
- [x] Remove claims that are stronger than the code.

### 9.3 Document debug/internal boundaries
- [x] If demo/probe code remains, document it as internal-only.
- [x] Do not leave user-facing docs that blur production vs internal tooling.

### 9.4 Required checks
- [x] README does not overstate current readiness
- [x] docs do not treat debug/demo scaffolding as production behavior

---

## 10. M9 — Add explicit regression tests for the reviewed bugs

### 10.1 Snapshot privacy regression tests
- [x] client snapshot does not include full `hole_cards_by_player_id`
- [x] recipient projection strips non-local private cards
- [x] observer snapshot contains no hidden active-player cards

### 10.2 Invite-generation regression tests
- [x] production invite generation derives from a live host session
- [x] synthetic/bootstrap invite generation is not used by production commands

### 10.3 Ready Room regression tests
- [ ] Ready Room (if kept) renders live backend state
- [ ] start/leave operations invoke backend commands
- [x] no route enters table by fake-only navigation path

### 10.4 Runtime-model regression tests
- [x] client state advances from public events during normal live play
- [x] snapshots used only for join/reconnect/resync
- [x] event feed receives real production events if visible

---

## 11. Definition of done for this pass

This pass is **not done** until all items below are true.

### 11.1 Code and behavior gate
- [x] snapshots no longer leak other players' unrevealed hole cards
- [x] recipient-private hole-card flow is real and used correctly
- [x] production workflows do not depend on demo/shell state
- [x] Ready Room is either real or removed from production flow
- [x] client session consumes public events for live play
- [x] event-feed UI is real or removed
- [x] synthetic invite generation is removed or debug-only
- [x] demo/probe code is quarantined or deleted
- [x] docs/README accurately reflect implementation reality

### 11.2 Manual QA gate
These are required and cannot be skipped:
- [ ] host a real desktop session
- [ ] join from another desktop instance
- [ ] play at least one real hand
- [ ] confirm only the local player sees their own private cards
- [ ] confirm observer does not see hidden active-player cards
- [ ] confirm reconnect/resync restores only recipient-appropriate state
- [ ] confirm production routes do not hit fake/demo flows

### 11.3 Out-of-scope items may remain undone
The following are intentionally not required for this cleanup pass:
- [ ] internet play
- [ ] matchmaking
- [ ] host migration
- [ ] trust-minimized dealing
- [ ] major art polish
- [ ] new gameplay features unrelated to the reviewed issues


---

## 12. Copilot Clarification Addendum — Exact Implementation Targets

This addendum answers the follow-up questions. Follow it exactly.

### 12.1 Precedence

For protocol/Android compatibility:
1. current Android code/tests
2. `docs/ANDROID_DESKTOP_COMPAT_ANSWERS.md`

For this desktop cleanup pass:
1. `DESKTOP_CODE_REVIEW_FIX_SPEC.md`
2. `DESKTOP_CODE_REVIEW_FIX_TODO.md`

The cleanup docs do not override Android protocol truth.

### 12.2 TODO interpretation

This TODO is an ordered checklist.

Do:
- perform the audits
- inspect references before deleting code
- implement the milestones in order when practical
- add/adjust tests after each fix

Do not:
- treat audit items as optional
- keep fake/demo behavior because it is convenient
- substitute shell/demo state for real backend/runtime state
- change Android-compatible protocol behavior without an explicit protocol-versioned reason

### 12.3 Exact target: client public-event consumption

Fix:
- `src-tauri/src/app_state/mod.rs`
- `DesktopClientSession::apply_event(...)`

Required changes:
- [x] Stop ignoring `ClientRuntimeEvent::PublicEvent`.
- [x] Stop ignoring `ClientRuntimeEvent::PrivateHoleCards`.
- [x] Stop ignoring `ClientRuntimeEvent::ResyncRequested`.
- [x] Apply public events to the client's local session truth.
- [x] Apply private card events only to the intended recipient's private state.
- [x] Trigger real resync behavior when `ResyncRequested` is received.
- [x] Add tests proving public events change client table state without requiring a fresh snapshot.

### 12.4 Exact target: event-feed UI

Fix or remove:
- `src/screens/MainTableScreen.tsx`
  - the `Latest public events` panel
- `src/screens/HandHistoryScreen.tsx`
  - any `eventFeed` rendering

Backend targets:
- `src-tauri/src/app_state/mod.rs`
  - `DesktopHostSession::table_view(...)`
  - `DesktopClientSession::table_view(...)`
  - both currently pass `Vec::new()` into `build_table_view_snapshot(...)`
- `src-tauri/src/commands.rs`
  - fallback/no-session table view also uses `event_feed: vec![]`

Required decision for this pass:
- [x] If you can wire real production event data now, keep the UI and feed it real events.
- [ ] If not, remove/hide the production event-feed panels.
- [ ] Do not leave visible event-feed UI backed by placeholder empty lists.

### 12.5 Exact target: synthetic invite command

Fix:
- `src-tauri/src/commands.rs`
  - `create_host_invite(...)`
  - `create_host_invite_inner(...)`

Problem:
- it builds fake payload values from `DesktopBootstrapState`
- it does not use a real live host session

Correct production source:
- `src-tauri/src/app_state/mod.rs`
  - `DesktopHostSession::status(...)`
  - `HostSessionStatus.invite`
  - `self.host_server.encoded_join_payload()`

Required changes:
- [x] Remove `create_host_invite` from production command registration in `src-tauri/src/lib.rs`, or gate/rename it debug-only.
- [x] Remove production frontend usage of `createHostInvite(...)` in `src/api/desktop.ts` and related screens.
- [x] Ensure production host invite copying uses `HostSessionStatus.invite`.
- [x] Update tests that currently expect synthetic invite generation.

### 12.6 Exact target: Ready Room removal

Decision:
- Ready Room is removed from production flow.
- Ready-state behavior belongs in Tournament Lobby.

Required changes:
- [x] Remove production route `/ready-room` from `src/app/AppShell.tsx`.
- [x] Remove `/ready-room` from production tournament navigation logic in `src/components/layout/AppFrame.tsx`.
- [x] Remove links/buttons that route normal users to `/ready-room`.
- [x] Delete `src/screens/ReadyRoomScreen.tsx`, or move it to an explicitly debug/probe-only location.
- [x] Remove/replace `src/screens/ReadyRoomScreen.test.tsx`.
- [x] Convert any needed readiness assertions into Tournament Lobby tests.
- [x] Keep `src/probe/LayoutProbeApp.tsx` usage only if clearly probe/internal.

### 12.7 Exact target: shell/demo participant helper

Fix:
- `src/app/shell.ts`
  - `buildParticipantShell(...)`

Required changes:
- [x] Remove production route dependencies on `buildParticipantShell(...)`.
- [x] If it remains for probes/tests, rename or document it as demo/probe-only.
- [x] Do not let production screens use shell-generated participant/seat state.

### 12.8 Required final checks

Before marking this pass complete:
- [x] grep for `createHostInvite` and verify no production UI path uses it.
- [x] grep for `/ready-room` and verify no production route exposes it.
- [x] grep for `buildParticipantShell` and verify no production screen depends on it.
- [x] grep for `Vec::new()` event-feed table-view calls and either replace them with real event feed data or remove the visible UI.
- [x] verify `DesktopClientSession::apply_event(...)` handles public events, private hole cards, and resync requests.
