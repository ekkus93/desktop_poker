# DESKTOP_CODE_REVIEW2_FIX_TODO.md

This TODO implements `DESKTOP_CODE_REVIEW2_FIX_SPEC.md`.

Do exactly what is written.  
Do not reopen product decisions.  
This is a focused repair pass, not a feature expansion.

---

## 1. Non-negotiable decisions

Before coding, treat these as fixed:

- Do not send broad authoritative `TournamentState` snapshots to clients if it contains host-only/session-private data.
- Reconnect tokens are private to their owner and the host.
- Join admission is participant-capacity based, not open-seat based.
- Join requests are accepted only before roster freeze / tournament start.
- Join tokens must be secure random opaque values.
- Desktop blind schedule must match Android-compatible Fast/Normal/Slow presets.
- Eliminated observers must see public showdown-revealed cards.
- Hand history must store board cards per completed hand.
- Client-side odd-chip handling must be correct.
- Debug/probe tooling must not be reachable in release/production builds.
- README/docs must not overstate readiness.

---

## 2. M1 — Harden snapshot DTO/privacy

### 2.1 Audit current snapshot DTOs
- [ ] Find every snapshot type sent over the network.
- [ ] Find every place a `TournamentState` or equivalent broad state object is embedded in a client-visible payload.
- [ ] List every sensitive field currently reachable from that object, including:
  - [ ] reconnect tokens
  - [ ] hidden hole cards
  - [ ] deck state
  - [ ] host-only registries
  - [ ] command/session internals

### 2.2 Create safe recipient-facing snapshot DTOs
- [ ] Define a safe `RecipientSnapshot` / equivalent DTO.
- [ ] Include only public table/tournament state.
- [ ] Include only the recipient's own allowed private fields.
- [ ] Include recipient reconnect token only for that recipient.
- [ ] Exclude other participants' reconnect tokens.
- [ ] Exclude non-local unrevealed hole cards.
- [ ] Exclude host-only hidden state.

### 2.3 Replace broad snapshot transmission
- [ ] Stop serializing broad `TournamentState` to normal clients.
- [ ] Convert authoritative state to safe DTO before serialization.
- [ ] Use safe DTO for:
  - [ ] join snapshot
  - [ ] reconnect snapshot
  - [ ] resync snapshot

### 2.4 Preserve frontend compatibility deliberately
- [ ] Update frontend/backend table-view mapping to consume the safe snapshot DTO.
- [ ] Do not reintroduce broad state just to avoid frontend refactor.
- [ ] Keep host-internal state available only inside host runtime.

### 2.5 Required tests
- [ ] snapshot for player A does not contain player B reconnect token
- [ ] snapshot for player A does not contain player B unrevealed hole cards
- [ ] observer snapshot contains no active-player private cards
- [ ] serialized snapshot JSON does not contain known other-player reconnect-token strings
- [ ] serialized snapshot JSON does not contain known non-local hole-card strings

---

## 3. M2 — Fix join admission guards

### 3.1 Implement canonical capacity counting
- [ ] Add one function such as `count_capacity_participants(...)`.
- [ ] Count admitted unseated participants.
- [ ] Count seated participants.
- [ ] Count disconnected reconnect-eligible participants.
- [ ] Exclude eliminated observers.
- [ ] Exclude fully removed/left participants.
- [ ] Use this function for all production join-capacity decisions.

### 3.2 Enforce max capacity on join
- [ ] In the join request handler, reject if `count_capacity_participants >= max_players`.
- [ ] Do not allow open unclaimed seats to override capacity rejection.
- [ ] Return a clear join rejection reason.

### 3.3 Reject joins after roster freeze / start
- [ ] Reject join if tournament is in ready-frozen state, if applicable.
- [ ] Reject join if tournament is running.
- [ ] Reject join if tournament is complete/cancelled/closed.
- [ ] Accept join only in the explicit pre-start joinable phase.

### 3.4 Required tests
- [ ] can admit up to max players
- [ ] rejects max+1 participant even if seats are unclaimed
- [ ] admitted unseated participants count toward capacity
- [ ] disconnected reconnect-eligible participant still counts
- [ ] eliminated observer does not count
- [ ] join after tournament start is rejected
- [ ] join after roster freeze is rejected if roster freeze exists before start

---

## 4. M3 — Replace deterministic join token with secure random token

### 4.1 Remove deterministic token generation
- [ ] Find token generation code similar to `join-token:{namespace}:{epoch}`.
- [ ] Delete that deterministic format from production.
- [ ] Ensure no tests depend on deterministic token text.

### 4.2 Implement secure token generation
- [ ] Generate at least 24 random bytes for each host session token.
- [ ] Encode using base64url without padding or equivalent URL-safe opaque string.
- [ ] Ensure token is unique per host session.
- [ ] Store token in the live host session metadata.

### 4.3 Update join payload generation
- [ ] Embed the random token into direct join payloads.
- [ ] Validate incoming join token against the live host session token.
- [ ] Reject missing or mismatched tokens.

### 4.4 Required tests
- [ ] token is non-empty
- [ ] token is URL-safe
- [ ] two host sessions generate different tokens
- [ ] token is not the old deterministic namespace/epoch format
- [ ] join rejected with wrong token

---

## 5. M4 — Replace blind schedule with Android-compatible presets

### 5.1 Remove incompatible production presets
- [ ] Remove or hide production presets such as:
  - [ ] `standard`
  - [ ] `turbo`
  - [ ] `deep-stack`
- [ ] Do not expose those names in production UI.

### 5.2 Implement canonical levels
- [ ] Use exactly these levels:
  - [ ] 10 / 20
  - [ ] 15 / 30
  - [ ] 25 / 50
  - [ ] 50 / 100
  - [ ] 75 / 150
  - [ ] 100 / 200
  - [ ] 150 / 300
  - [ ] 200 / 400
  - [ ] 300 / 600
  - [ ] 400 / 800
  - [ ] 600 / 1200
  - [ ] 800 / 1600

### 5.3 Implement canonical durations
- [ ] Fast = 180 seconds
- [ ] Normal = 300 seconds
- [ ] Slow = 480 seconds

### 5.4 Update UI labels
- [ ] Host setup screen shows Fast / Normal / Slow.
- [ ] Any old preset labels are removed from production UI.
- [ ] Existing saved old preset values are migrated or rejected safely.

### 5.5 Required tests
- [ ] Fast returns 12 levels with 180s duration
- [ ] Normal returns 12 levels with 300s duration
- [ ] Slow returns 12 levels with 480s duration
- [ ] first level is 10/20
- [ ] final level is 800/1600
- [ ] old incompatible preset is not production-visible

---

## 6. M5 — Fix observer showdown visibility

### 6.1 Audit table-seat rendering/projection
- [ ] Find code deciding whether a viewer sees another player's cards.
- [ ] Identify branch that hides cards for `Observer` mode even during showdown.

### 6.2 Implement correct visibility rule
- [ ] Before public showdown reveal: observers see no unrevealed private cards.
- [ ] After public showdown reveal: observers see revealed showdown cards.
- [ ] During settlement/history: observers see public revealed cards.
- [ ] Local player still sees own cards when allowed.
- [ ] Do not reveal folded/unrevealed cards unless the hand state says they are public.

### 6.3 Required tests
- [ ] observer cannot see active private cards before showdown
- [ ] observer sees public showdown-revealed cards
- [ ] observer sees settlement public reveals
- [ ] observer does not see unrevealed folded cards

---

## 7. M6 — Fix hand-history board correctness

### 7.1 Add board cards to completed hand summary
- [ ] Store board cards at settlement/completion time.
- [ ] Add `board_cards` or equivalent to hand-history record if missing.
- [ ] Ensure hand result/history event carries completed board state.

### 7.2 Stop reading historical board from current hand
- [ ] Find code using `state.current_hand.board_cards` to render every history row.
- [ ] Replace it with the board stored on that hand's completed summary.
- [ ] Ensure old hand rows do not change when a new hand starts.

### 7.3 Required tests
- [ ] completed hand history records board at settlement
- [ ] starting a new hand does not change previous hand board
- [ ] history row uses stored hand board, not current hand board
- [ ] no-current-hand state still shows completed hand board

---

## 8. M7 — Fix odd-chip client-side reconstruction

### 8.1 Audit client event application
- [ ] Find code that applies `HandResultCommittedEvent` or equivalent to local client stacks.
- [ ] Find any calculation that divides pot amount evenly among winners.
- [ ] Identify whether `odd_chip_awarded_to` is ignored.

### 8.2 Implement correct odd-chip handling
Option A, preferred:
- [ ] Include final stack values in the settlement/hand-result event.
- [ ] Client applies final stack values directly.

Option B, acceptable if event shape cannot change now:
- [ ] Divide pot equally among tied winners.
- [ ] Add odd chip count to `odd_chip_awarded_to`.
- [ ] Apply side pots independently.
- [ ] Do not ignore odd-chip metadata.

### 8.3 Required tests
- [ ] split pot with one odd chip awards correct extra chip
- [ ] client-side reconstructed stacks match host stacks
- [ ] side pot with odd chip applies correctly if supported

---

## 9. M8 — Gate debug/probe code from release/production

### 9.1 Gate layout probe
- [ ] Find `LayoutProbeApp` query-parameter entry path.
- [ ] Disable it in release/production builds.
- [ ] Allow it only under explicit debug/dev build flag if retained.

### 9.2 Gate debug backend commands
- [ ] Find `get_debug_state` and similar debug commands.
- [ ] Ensure they return an error or are unavailable in release builds.
- [ ] Do not return demo/probe/internal state in production.

### 9.3 Gate demo/debug runtime
- [ ] Ensure `DebugTableRuntime` / `debug_demo_controller()` are debug/internal only.
- [ ] Ensure normal production startup cannot instantiate debug runtime.
- [ ] Ensure production routes cannot navigate to debug/probe surfaces.

### 9.4 Required tests / checks
- [ ] release-mode config cannot open layout probe via query param
- [ ] release-mode `get_debug_state` fails or is unavailable
- [ ] production route list does not include debug/probe routes
- [ ] debug runtime is not instantiated by normal production app state

---

## 10. M9 — Update README/docs truthfulness

### 10.1 Audit README claims
- [ ] Review claims about:
  - [ ] complete host/join flow
  - [ ] local desktop-hosted poker usability
  - [ ] reconnect/resync readiness
  - [ ] production readiness
  - [ ] manual QA status

### 10.2 Tone down overclaims
- [ ] Do not say complete/production-ready unless true.
- [ ] Say MVP is in progress if defects/manual QA remain.
- [ ] Document known remaining validation needs.

### 10.3 Document manual QA status
- [ ] Add a section indicating whether manual desktop multi-instance QA has been completed.
- [ ] Add a section indicating whether Android/Desktop interoperability QA has been completed.

### 10.4 Required checks
- [ ] README no longer overstates readiness
- [ ] docs match current implementation status
- [ ] debug/probe tools documented as internal-only if retained

---

## 11. M10 — Final regression and manual QA gate

### 11.1 Automated regression gate
Run or add tests for:
- [ ] snapshot privacy
- [ ] reconnect-token privacy
- [ ] join admission capacity
- [ ] post-start join rejection
- [ ] random join token generation
- [ ] canonical blind schedule
- [ ] observer showdown visibility
- [ ] hand-history board storage
- [ ] odd-chip handling
- [ ] debug/probe release gating

### 11.2 Manual desktop QA gate
Perform and record results:
- [ ] host desktop session
- [ ] join from second desktop instance
- [ ] play one hand
- [ ] verify private cards are local-only
- [ ] verify observer sees showdown public cards only
- [ ] verify hand history board remains stable after new hand
- [ ] verify split pot odd chip if practical
- [ ] verify debug/probe routes unavailable in production/release mode

### 11.3 Definition of done
This pass is done only when:
- [ ] all M1-M9 implementation tasks are complete
- [ ] automated tests pass
- [ ] README/docs are updated
- [ ] manual QA status is recorded
- [ ] no known snapshot/reconnect-token leak remains
- [ ] no production-visible debug/probe path remains

---

## 12. Notes for Copilot

Do not mark a task complete just because one narrow example works.

For every privacy task:
- inspect serialized payloads
- verify absence of forbidden fields
- add tests using distinct known values so leakage is obvious

For every UI truthfulness task:
- verify the route/screen is reachable in production
- if reachable, it must be real
- if not real, remove/hide it

For every compatibility task:
- prefer Android-compatible behavior unless this file explicitly says otherwise
