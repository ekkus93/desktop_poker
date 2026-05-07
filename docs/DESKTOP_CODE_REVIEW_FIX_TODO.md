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
- [ ] Find every code path that creates a snapshot or snapshot-like payload for a non-host recipient.
- [ ] Enumerate all fields currently included.
- [ ] Identify every path where `TournamentState` or equivalent authoritative state is cloned or serialized directly.

### 2.2 Remove direct full-state snapshot transmission
- [ ] Stop sending full authoritative `TournamentState` to normal clients if it contains multi-player hidden state.
- [ ] Remove any direct serialization path that includes full `hole_cards_by_player_id` for non-host recipients.
- [ ] Ensure host-only state never crosses the network to normal clients.

### 2.3 Implement recipient-projected snapshot building
- [ ] Create one canonical backend function to build a snapshot for a specific recipient.
- [ ] Input:
  - [ ] authoritative state
  - [ ] recipient identity / role
- [ ] Output:
  - [ ] public state
  - [ ] only that recipient's allowed private data
- [ ] Use this function for:
  - [ ] join success snapshot
  - [ ] reconnect snapshot
  - [ ] resync snapshot

### 2.4 Enforce observer projection
- [ ] Observers must receive no active players' private cards.
- [ ] Eliminated observers must remain read-only.
- [ ] Observer snapshots must contain public state only.

### 2.5 Required tests
- [ ] join snapshot for active player contains only that player's hole cards
- [ ] reconnect snapshot for player A does not contain player B's hole cards
- [ ] observer snapshot contains no unrevealed active-player hole cards
- [ ] host/internal state is not present in recipient-facing snapshot DTOs

---

## 3. M2 — Make private hole-card delivery real

### 3.1 Audit current private-hole-card flow
- [ ] Identify whether encrypted private hole-card delivery is currently used in real production gameplay or only in tests / scaffolding.
- [ ] Remove any reliance on generic snapshots to substitute for private delivery.

### 3.2 Implement one canonical private-card delivery path
- [ ] Choose the production path and use it consistently:
  - [ ] recipient-projected snapshot for join/reconnect/resync
  - [ ] encrypted private envelope for live hand private-card delivery
- [ ] Ensure this path is actually invoked during live gameplay.

### 3.3 Enforce recipient-only semantics
- [ ] Validate recipient identity before private-card delivery.
- [ ] Ensure only the addressed client can receive/decode the payload.
- [ ] Ensure frontend/private session state updates only from legitimate private delivery inputs.

### 3.4 Required tests
- [ ] active player receives own private cards through the production path
- [ ] non-recipient does not receive/decode that private payload
- [ ] live gameplay does not require hidden multi-player card state in snapshots

---

## 4. M3 — Remove or quarantine fake/demo/stale code

### 4.1 Inventory fake/demo/probe code
- [ ] Audit the repo for:
  - [ ] `demo_controller()`
  - [ ] `DesktopTableRuntime`
  - [ ] shell-generated participant builders
  - [ ] synthetic invite helpers
  - [ ] layout probe/test harness UIs
  - [ ] fake seat/participant construction helpers
- [ ] Classify each item:
  - [ ] production
  - [ ] debug/internal only
  - [ ] delete now

### 4.2 Remove production-adjacent demo runtime usage
- [ ] Remove any production path that still instantiates demo controller/runtime state.
- [ ] Ensure normal Tauri app startup does not depend on demo runtime objects.

### 4.3 Rename and isolate surviving debug helpers
- [ ] Any retained fake/demo helper must be renamed clearly:
  - [ ] `debug_...`
  - [ ] `internal_...`
  - [ ] `demo_...`
  - [ ] `probe_...`
- [ ] Move them behind explicit debug/internal-only boundaries if possible.

### 4.4 Delete stale synthetic invite generation
- [ ] Remove or rewrite any command/helper that generates invite payloads from synthetic/bootstrap values rather than a real live host session.
- [ ] Production invite generation must always come from the live host session.

### 4.5 Required checks
- [ ] production host/join/table path contains no fake/demo runtime dependency
- [ ] synthetic invite generation is not reachable from production UI
- [ ] retained debug/demo code is clearly labeled and gated

---

## 5. M4 — Fix or remove the Ready Room

### 5.1 Decide once: real or removed
- [ ] Make an explicit implementation choice:
  - [ ] Ready Room becomes a real backend-driven production screen
  - [ ] or Ready Room is removed from normal production flow

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
- [ ] remove the route from production navigation
- [ ] remove links/transitions that depend on it
- [ ] simplify flow so lobby transitions directly into the real table/runtime start path

### 5.4 Required tests / checks
- [ ] production flow does not rely on shell-generated ready state
- [ ] start tournament from UI uses real backend command
- [ ] leave/close from Ready Room (if kept) uses real backend action

---

## 6. M5 — Enforce one runtime truth model

### 6.1 Hard rule
- [ ] Live gameplay updates must come from signed public events.
- [ ] Snapshots must be limited to join, reconnect, and resync.
- [ ] Private data must come from recipient-private delivery.

### 6.2 Audit current client runtime handling
- [ ] Identify every incoming runtime event type currently emitted by the networking/runtime layer.
- [ ] Identify which ones are ignored by the client session/app-state layer.
- [ ] Remove dead-path event handling assumptions.

### 6.3 Make client session consume public events for live play
- [ ] Ensure client session/app-state updates on:
  - [ ] public gameplay events
  - [ ] private hole-card deliveries
  - [ ] reconnect/resync results
  - [ ] protocol errors
- [ ] Do not rely on continuous snapshot pushes as steady-state gameplay truth.

### 6.4 Remove muddled hybrid behavior
- [ ] If the host is still calling snapshot sync after every action, reduce that to the allowed cases only unless explicitly required for recovery.
- [ ] Do not keep a silent “snapshots do everything anyway” fallback in production live play.

### 6.5 Required tests
- [ ] client live state advances from public events
- [ ] reconnect/resync still restores state correctly via snapshot
- [ ] private-card updates use private path
- [ ] ordinary gameplay does not require full-state snapshot spam

---

## 7. M6 — Fix event-feed truthfulness

### 7.1 Audit current event-feed UI
- [ ] Find every UI panel that claims to show live public events, history, or latest actions.
- [ ] Trace each one back to actual backend/runtime data.

### 7.2 Make it real or remove it
For each production event/history panel:
- [ ] wire it to real event data, or
- [ ] remove/hide it until real data exists

### 7.3 Required rule
- [ ] Do not leave production UI surfaces that always receive `Vec::new()` or equivalent empty placeholder data while presenting themselves as live event feeds.

### 7.4 Required checks
- [ ] production event-feed UI shows real public event data if visible
- [ ] or the panel is removed/hidden from production builds

---

## 8. M7 — Clean up UI truthfulness and shell scaffolding

### 8.1 Audit shell helpers
- [ ] Audit `buildParticipantShell(...)` and any similar helpers.
- [ ] Determine whether they are still used by production routes.
- [ ] Remove them from production routes.

### 8.2 Remove misleading semantics
- [ ] Fix any UI models that assign misleading participant kinds/states (for example, all seated participants styled as “host”).
- [ ] Align labels with real session truth.

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
- [ ] Review README and desktop docs for claims about:
  - [ ] LAN readiness
  - [ ] reconnect/resync quality
  - [ ] host/join completeness
  - [ ] event-driven runtime
  - [ ] production usability

### 9.2 Downgrade or update claims to match reality
- [ ] If functionality is not truly production-ready yet, say so plainly.
- [ ] Remove claims that are stronger than the code.

### 9.3 Document debug/internal boundaries
- [ ] If demo/probe code remains, document it as internal-only.
- [ ] Do not leave user-facing docs that blur production vs internal tooling.

### 9.4 Required checks
- [ ] README does not overstate current readiness
- [ ] docs do not treat debug/demo scaffolding as production behavior

---

## 10. M9 — Add explicit regression tests for the reviewed bugs

### 10.1 Snapshot privacy regression tests
- [ ] client snapshot does not include full `hole_cards_by_player_id`
- [ ] recipient projection strips non-local private cards
- [ ] observer snapshot contains no hidden active-player cards

### 10.2 Invite-generation regression tests
- [ ] production invite generation derives from a live host session
- [ ] synthetic/bootstrap invite generation is not used by production commands

### 10.3 Ready Room regression tests
- [ ] Ready Room (if kept) renders live backend state
- [ ] start/leave operations invoke backend commands
- [ ] no route enters table by fake-only navigation path

### 10.4 Runtime-model regression tests
- [ ] client state advances from public events during normal live play
- [ ] snapshots used only for join/reconnect/resync
- [ ] event feed receives real production events if visible

---

## 11. Definition of done for this pass

This pass is **not done** until all items below are true.

### 11.1 Code and behavior gate
- [ ] snapshots no longer leak other players' unrevealed hole cards
- [ ] recipient-private hole-card flow is real and used correctly
- [ ] production workflows do not depend on demo/shell state
- [ ] Ready Room is either real or removed from production flow
- [ ] client session consumes public events for live play
- [ ] event-feed UI is real or removed
- [ ] synthetic invite generation is removed or debug-only
- [ ] demo/probe code is quarantined or deleted
- [ ] docs/README accurately reflect implementation reality

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
