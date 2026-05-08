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
- [x] Find every snapshot type sent over the network.
- [x] Find every place a `TournamentState` or equivalent broad state object is embedded in a client-visible payload.
- [x] List every sensitive field currently reachable from that object, including:
  - [x] reconnect tokens
  - [x] hidden hole cards
  - [x] deck state
  - [x] host-only registries
  - [x] command/session internals

### 2.2 Create safe recipient-facing snapshot DTOs
- [x] Define a safe `RecipientSnapshot` / equivalent DTO.
- [x] Include only public table/tournament state.
- [x] Include only the recipient's own allowed private fields.
- [x] Include recipient reconnect token only for that recipient.
- [x] Exclude other participants' reconnect tokens.
- [x] Exclude non-local unrevealed hole cards.
- [x] Exclude host-only hidden state.

### 2.3 Replace broad snapshot transmission
- [x] Stop serializing broad `TournamentState` to normal clients.
- [x] Convert authoritative state to safe DTO before serialization.
- [x] Use safe DTO for:
  - [x] join snapshot
  - [x] reconnect snapshot
  - [x] resync snapshot

### 2.4 Preserve frontend compatibility deliberately
- [x] Update frontend/backend table-view mapping to consume the safe snapshot DTO.
- [x] Do not reintroduce broad state just to avoid frontend refactor.
- [x] Keep host-internal state available only inside host runtime.

### 2.5 Required tests
- [x] snapshot for player A does not contain player B reconnect token
- [x] snapshot for player A does not contain player B unrevealed hole cards
- [x] observer snapshot contains no active-player private cards
- [x] serialized snapshot JSON does not contain known other-player reconnect-token strings
- [x] serialized snapshot JSON does not contain known non-local hole-card strings

---

## 3. M2 — Fix join admission guards

### 3.1 Implement canonical capacity counting
- [x] Add one function such as `count_capacity_participants(...)`.
- [x] Count admitted unseated participants.
- [x] Count seated participants.
- [x] Count disconnected reconnect-eligible participants.
- [x] Exclude eliminated observers.
- [x] Exclude fully removed/left participants.
- [x] Use this function for all production join-capacity decisions.

### 3.2 Enforce max capacity on join
- [x] In the join request handler, reject if `count_capacity_participants >= max_players`.
- [x] Do not allow open unclaimed seats to override capacity rejection.
- [x] Return a clear join rejection reason.

### 3.3 Reject joins after roster freeze / start
- [x] Reject join if tournament is in ready-frozen state, if applicable.
- [x] Reject join if tournament is running.
- [x] Reject join if tournament is complete/cancelled/closed.
- [x] Accept join only in the explicit pre-start joinable phase.

### 3.4 Required tests
- [x] can admit up to max players
- [x] rejects max+1 participant even if seats are unclaimed
- [x] admitted unseated participants count toward capacity
- [x] disconnected reconnect-eligible participant still counts
- [x] eliminated observer does not count
- [x] join after tournament start is rejected
- [x] join after roster freeze is rejected if roster freeze exists before start

---

## 4. M3 — Replace deterministic join token with secure random token

### 4.1 Remove deterministic token generation
- [x] Find token generation code similar to `join-token:{namespace}:{epoch}`.
- [x] Delete that deterministic format from production.
- [x] Ensure no tests depend on deterministic token text.

### 4.2 Implement secure token generation
- [x] Generate at least 24 random bytes for each host session token.
- [x] Encode using base64url without padding or equivalent URL-safe opaque string.
- [x] Ensure token is unique per host session.
- [x] Store token in the live host session metadata.

### 4.3 Update join payload generation
- [x] Embed the random token into direct join payloads.
- [x] Validate incoming join token against the live host session token.
- [x] Reject missing or mismatched tokens.

### 4.4 Required tests
- [x] token is non-empty
- [x] token is URL-safe
- [x] two host sessions generate different tokens
- [x] token is not the old deterministic namespace/epoch format
- [x] join rejected with wrong token

---

## 5. M4 — Replace blind schedule with Android-compatible presets

### 5.1 Remove incompatible production presets
- [x] Remove or hide production presets such as:
  - [x] `standard`
  - [x] `turbo`
  - [x] `deep-stack`
- [x] Do not expose those names in production UI.

### 5.2 Implement canonical levels
- [x] Use exactly these levels:
  - [x] 10 / 20
  - [x] 15 / 30
  - [x] 25 / 50
  - [x] 50 / 100
  - [x] 75 / 150
  - [x] 100 / 200
  - [x] 150 / 300
  - [x] 200 / 400
  - [x] 300 / 600
  - [x] 400 / 800
  - [x] 600 / 1200
  - [x] 800 / 1600

### 5.3 Implement canonical durations
- [x] Fast = 180 seconds
- [x] Normal = 300 seconds
- [x] Slow = 480 seconds

### 5.4 Update UI labels
- [x] Host setup screen shows Fast / Normal / Slow.
- [x] Any old preset labels are removed from production UI.
- [x] Existing saved old preset values are migrated or rejected safely.

### 5.5 Required tests
- [x] Fast returns 12 levels with 180s duration
- [x] Normal returns 12 levels with 300s duration
- [x] Slow returns 12 levels with 480s duration
- [x] first level is 10/20
- [x] final level is 800/1600
- [x] old incompatible preset is not production-visible

---

## 6. M5 — Fix observer showdown visibility

### 6.1 Audit table-seat rendering/projection
- [x] Find code deciding whether a viewer sees another player's cards.
- [x] Identify branch that hides cards for `Observer` mode even during showdown.

### 6.2 Implement correct visibility rule
- [x] Before public showdown reveal: observers see no unrevealed private cards.
- [x] After public showdown reveal: observers see revealed showdown cards.
- [x] During settlement/history: observers see public revealed cards.
- [x] Local player still sees own cards when allowed.
- [x] Do not reveal folded/unrevealed cards unless the hand state says they are public.

### 6.3 Required tests
- [x] observer cannot see active private cards before showdown
- [x] observer sees public showdown-revealed cards
- [x] observer sees settlement public reveals
- [x] observer does not see unrevealed folded cards

---

## 7. M6 — Fix hand-history board correctness

### 7.1 Add board cards to completed hand summary
- [x] Store board cards at settlement/completion time.
- [x] Add `board_cards` or equivalent to hand-history record if missing.
- [x] Ensure hand result/history event carries completed board state.

### 7.2 Stop reading historical board from current hand
- [x] Find code using `state.current_hand.board_cards` to render every history row.
- [x] Replace it with the board stored on that hand's completed summary.
- [x] Ensure old hand rows do not change when a new hand starts.

### 7.3 Required tests
- [x] completed hand history records board at settlement
- [x] starting a new hand does not change previous hand board
- [x] history row uses stored hand board, not current hand board
- [x] no-current-hand state still shows completed hand board

---

## 8. M7 — Fix odd-chip client-side reconstruction

### 8.1 Audit client event application
- [x] Find code that applies `HandResultCommittedEvent` or equivalent to local client stacks.
- [x] Find any calculation that divides pot amount evenly among winners.
- [x] Identify whether `odd_chip_awarded_to` is ignored.

### 8.2 Implement correct odd-chip handling
Option A, preferred:
- [x] Include final stack values in the settlement/hand-result event.
- [x] Client applies final stack values directly.

Option B, acceptable if event shape cannot change now:
- [x] Divide pot equally among tied winners.
- [x] Add odd chip count to `odd_chip_awarded_to`.
- [x] Apply side pots independently.
- [x] Do not ignore odd-chip metadata.

### 8.3 Required tests
- [x] split pot with one odd chip awards correct extra chip
- [x] client-side reconstructed stacks match host stacks
- [x] side pot with odd chip applies correctly if supported

---

## 9. M8 — Gate debug/probe code from release/production

### 9.1 Gate layout probe
- [x] Find `LayoutProbeApp` query-parameter entry path.
- [x] Disable it in release/production builds.
- [x] Allow it only under explicit debug/dev build flag if retained.

### 9.2 Gate debug backend commands
- [x] Find `get_debug_state` and similar debug commands.
- [x] Ensure they return an error or are unavailable in release builds.
- [x] Do not return demo/probe/internal state in production.

### 9.3 Gate demo/debug runtime
- [x] Ensure `DebugTableRuntime` / `debug_demo_controller()` are debug/internal only.
- [x] Ensure normal production startup cannot instantiate debug runtime.
- [x] Ensure production routes cannot navigate to debug/probe surfaces.

### 9.4 Required tests / checks
- [x] release-mode config cannot open layout probe via query param
- [x] release-mode `get_debug_state` fails or is unavailable
- [x] production route list does not include debug/probe routes
- [x] debug runtime is not instantiated by normal production app state

---

## 10. M9 — Update README/docs truthfulness

### 10.1 Audit README claims
- [x] Review claims about:
  - [x] complete host/join flow
  - [x] local desktop-hosted poker usability
  - [x] reconnect/resync readiness
  - [x] production readiness
  - [x] manual QA status

### 10.2 Tone down overclaims
- [x] Do not say complete/production-ready unless true.
- [x] Say MVP is in progress if defects/manual QA remain.
- [x] Document known remaining validation needs.

### 10.3 Document manual QA status
- [x] Add a section indicating whether manual desktop multi-instance QA has been completed.
- [x] Add a section indicating whether Android/Desktop interoperability QA has been completed.

### 10.4 Required checks
- [x] README no longer overstates readiness
- [x] docs match current implementation status
- [x] debug/probe tools documented as internal-only if retained

---

## 11. M10 — Final regression and manual QA gate

### 11.1 Automated regression gate
Run or add tests for:
- [x] snapshot privacy
- [x] reconnect-token privacy
- [x] join admission capacity
- [x] post-start join rejection
- [x] random join token generation
- [x] canonical blind schedule
- [x] observer showdown visibility
- [x] hand-history board storage
- [x] odd-chip handling
- [x] debug/probe release gating

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
- [x] all M1-M9 implementation tasks are complete
- [x] automated tests pass
- [x] README/docs are updated
- [x] manual QA status is recorded
- [x] no known snapshot/reconnect-token leak remains
- [x] no production-visible debug/probe path remains

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
