# DESKTOP_CODE_REVIEW2_FIX_SPEC.md

## 1. Purpose

This document defines the **second desktop repair pass** after the latest desktop code review.

The first desktop cleanup pass improved the code significantly:
- the fake Ready Room production route was removed
- stale synthetic invite generation was removed from production usage
- public event consumption was improved
- private hole-card delivery was integrated more seriously
- event-feed UI is now backed by real event data
- non-local hole cards were stripped from projected snapshots

However, the latest review found remaining confirmed issues that must be fixed before the desktop app can be treated as a trustworthy MVP candidate.

This document is the source of truth for this repair pass.

---

## 2. Scope

### 2.1 In scope
This pass fixes the remaining confirmed issues:

1. snapshot DTO/privacy hardening
2. join admission guards
3. secure random join-token generation
4. Android-compatible blind schedule
5. observer showdown-card visibility
6. hand-history board correctness
7. odd-chip client-side reconstruction
8. debug/probe release gating
9. README/docs truthfulness
10. regression tests for the above

### 2.2 Out of scope
Do **not** expand this pass into:
- new game variants
- internet play
- matchmaking
- bots
- trust-minimized dealing
- host migration
- major visual redesign
- room-code discovery
- large protocol redesign unless strictly needed to fix privacy/correctness

---

## 3. Canonical rules and precedence

### 3.1 Product and protocol truth
The desktop app must remain aligned with:
1. current Android runtime/protocol behavior
2. `ANDROID_DESKTOP_COMPAT_ANSWERS.md`
3. `DESKTOP_SPECS.md`
4. this repair spec

If older markdown files conflict with this pass, this pass wins for desktop cleanup work.

### 3.2 Android compatibility
Do not invent desktop-only tournament semantics when the Android-compatible behavior is already defined.

The desktop app must preserve:
- single-table Sit 'n Go
- 2 to 10 participants
- host-authoritative runtime
- raw TCP LAN model
- signed messages
- encrypted/private hole-card delivery
- direct join payload behavior
- reconnect/resync model
- observer mode

---

## 4. Confirmed remaining defects

Treat the following as confirmed defects to fix.

### 4.1 Snapshot DTO/privacy is still too broad
The previous pass stripped non-local hole cards from snapshots, but snapshots still embed broad `TournamentState` data. In particular, `TournamentState.participants` can include participant/session-private fields such as reconnect tokens.

This violates the rule that host-only/session-private state must not cross the network to ordinary clients.

### 4.2 Join admission guards are incomplete
The host join path must reject joins when:
- tournament is no longer in the pre-start lobby/joinable phase
- roster is frozen
- tournament is running, complete, or cancelled
- participant capacity is reached

Capacity must be participant-based, not open-seat-based.

### 4.3 Join token is deterministic
The join token must be an opaque random admission token. It must not be derived from deterministic bootstrap/session strings such as namespace and epoch.

### 4.4 Blind schedule is not Android-compatible
The desktop schedule currently uses a desktop-specific preset structure such as `standard`, `turbo`, or `deep-stack`.

The desktop app must use the Android-compatible fixed blind sequence and Fast/Normal/Slow durations.

### 4.5 Observer showdown visibility is wrong
Eliminated observers must see public showdown-revealed cards.

The UI must still hide unrevealed hole cards, but once showdown reveal events are public, observers should see those revealed cards just like other viewers.

### 4.6 Hand-history board cards are wrong
Historical hand rows must render the board from the completed hand, not from `state.current_hand.board_cards`.

A completed hand summary/history record must store the board at settlement time.

### 4.7 Client-side odd-chip reconstruction is incomplete
When a split pot has an odd chip, the client-side event application must account for `odd_chip_awarded_to`.

A client must not reconstruct stacks by simply dividing the pot evenly and ignoring odd-chip metadata.

### 4.8 Debug/probe code is too reachable
Debug/probe/internal tooling may remain, but it must not be reachable in release/production builds.

Known risk areas:
- layout probe access by query parameter
- debug state commands registered without release gating
- debug/demo controller paths reachable without explicit debug gate

### 4.9 README/docs still overstate readiness
Top-level docs must not claim the desktop app is complete or production-ready while manual QA is still incomplete and confirmed defects remain.

---

## 5. Hard decisions for this repair pass

### 5.1 Use safe recipient-facing snapshot DTOs
The preferred fix for snapshot privacy is to stop sending broad authoritative `TournamentState` to clients.

Create dedicated safe DTOs for recipient-facing snapshots.

A recipient-facing snapshot must contain:
- public table/tournament state
- local recipient identity
- local recipient private cards if allowed
- local recipient reconnect token if allowed
- host public keys if needed
- no other participant reconnect tokens
- no full deck
- no unrevealed non-local hole cards
- no host-only/private registry internals

If the implementation chooses to keep a `TournamentState`-like shape internally, it must be converted to a safe snapshot DTO before serialization.

### 5.2 Participant reconnect tokens are private
A participant reconnect token is private to that participant and the host.

A client snapshot may contain:
- that recipient's reconnect token

A client snapshot must not contain:
- any other participant's reconnect token

### 5.3 Join capacity is participant-based
Count toward capacity:
- admitted unseated participants
- seated participants
- disconnected but reconnect-eligible participants

Do not count:
- eliminated observers
- fully removed/left participants
- debug/probe-only participants not part of production session

Reject new joins when:
`count_capacity_participants() >= max_players`

Open seats do not override this rule.

### 5.4 Join allowed only before roster freeze
Reject join requests unless the tournament is in the pre-start joinable state.

Do not admit new participants after:
- ready/roster freeze if the implementation freezes there
- tournament start
- tournament running
- tournament complete
- table cancelled/closed

### 5.5 Join token must be secure random
Generate `joinToken` using secure random bytes:
- at least 24 random bytes
- base64url without padding or equivalent URL-safe opaque string
- unique per host session

Do not derive the token from:
- session epoch
- namespace
- table ID
- host name
- timestamp alone

### 5.6 Blind schedule must match Android-compatible v1 values
Use exactly this blind sequence:

1. 10 / 20
2. 15 / 30
3. 25 / 50
4. 50 / 100
5. 75 / 150
6. 100 / 200
7. 150 / 300
8. 200 / 400
9. 300 / 600
10. 400 / 800
11. 600 / 1200
12. 800 / 1600

Preset durations:
- Fast: 180 seconds
- Normal: 300 seconds
- Slow: 480 seconds

Remove or map old desktop-specific presets:
- `standard`
- `turbo`
- `deep-stack`

Do not expose old incompatible presets in production UI.

### 5.7 Observer sees public showdown cards
Observer mode must hide unrevealed private cards but show public showdown reveals.

At showdown/settlement, if a hand is publicly revealed, observers should see it.

### 5.8 Hand history owns its own board state
Hand-history entries must store:
- hand number
- board cards at completion/settlement
- winners
- pot summaries
- eliminations
- hand categories if available
- relevant final public summary

Do not render completed hand board cards from the current active hand.

### 5.9 Prefer final stack updates over reconstructing stacks when available
If a public hand-result/settlement event can include final stack values, the client should apply final stack values directly.

If reconstructing from pot summaries, apply:
- split shares
- odd chip award
- side-pot eligibility
- folded-player restrictions

At minimum, do not ignore `odd_chip_awarded_to`.

### 5.10 Release build must not expose debug/probe tools
In release/production:
- no layout probe via query string
- no debug state command should return sensitive/demo state
- no debug/demo runtime should be reachable from normal app routes
- no internal probe UI should be loadable by ordinary users

Debug/internal tools may remain only when gated by compile-time debug flags or explicit dev-only feature flags.

---

## 6. Required implementation outcomes

This pass is complete only when all outcomes below are true.

### 6.1 Privacy outcomes
- snapshots do not contain other participants' reconnect tokens
- snapshots do not contain non-local unrevealed hole cards
- snapshots do not contain full host-only state
- observer snapshots contain no private active-player state
- recipient snapshots contain only recipient-allowed private state

### 6.2 Join/admission outcomes
- joins are rejected after roster freeze / tournament start
- joins are rejected when participant capacity is reached
- capacity includes admitted unseated participants
- capacity includes reconnect-eligible disconnected participants
- eliminated observers do not block capacity

### 6.3 Token outcomes
- join token is random and opaque
- join token changes per host session
- token is not deterministic from predictable fields

### 6.4 Tournament compatibility outcomes
- desktop blind schedule matches Android-compatible Fast/Normal/Slow values
- production UI no longer exposes incompatible blind presets

### 6.5 UI/runtime outcomes
- observers see public showdown-revealed cards
- historical hand boards are correct
- odd chips are represented correctly in client-applied results
- debug/probe paths are release-gated
- README/docs are truthful about current readiness

---

## 7. Required tests

The implementation must add or update tests for:

### 7.1 Snapshot privacy
- recipient snapshot does not include other participants' reconnect tokens
- recipient snapshot does not include non-local private hole cards
- observer snapshot excludes all unrevealed private cards
- snapshot DTO cannot serialize host-only fields accidentally

### 7.2 Join admission
- join rejected after tournament start
- join rejected after roster freeze if applicable
- join rejected when participant capacity is reached
- admitted unseated participants count toward capacity
- reconnect-eligible disconnected participants count toward capacity
- eliminated observers do not count toward capacity

### 7.3 Join token
- join token is non-empty and URL-safe
- two host sessions do not reuse deterministic token values
- token is not the old deterministic namespace/epoch format

### 7.4 Blind schedule
- Fast/Normal/Slow produce the canonical 12-level sequence
- Fast duration = 180s
- Normal duration = 300s
- Slow duration = 480s
- old incompatible presets are not exposed in production

### 7.5 Observer/showdown
- observer does not see unrevealed cards pre-showdown
- observer sees showdown-revealed cards after public reveal

### 7.6 Hand history
- completed hand stores board at settlement time
- old hand history does not change when a new hand starts
- history does not read board from `current_hand`

### 7.7 Odd chip
- split pot with odd chip updates client-side display correctly
- `odd_chip_awarded_to` receives the extra chip

### 7.8 Debug/probe gating
- release build or release-mode configuration does not expose layout probe
- release-mode debug command returns error or is unavailable
- production routes cannot reach debug/probe runtime

---

## 8. Documentation requirements

Update README/docs to be honest.

Do not claim:
- complete production readiness
- fully verified LAN play
- fully completed reconnect/resync
- no known privacy issues
- manual QA completed

unless those statements are actually true.

Recommended language:
- "MVP in progress"
- "desktop host/join runtime under active validation"
- "manual multi-instance and Android interop QA still required"
- "debug/probe tools are internal-only"

---

## 9. Manual QA gate

This repair pass is not done until at least this manual QA has been performed or explicitly marked not yet complete:

1. run desktop host
2. join from a second desktop instance
3. play at least one hand
4. verify each client sees only its own private cards
5. verify eliminated observer sees public showdown cards but no unrevealed cards
6. verify hand history board remains correct after next hand starts
7. verify split pot odd-chip display is correct in a test scenario
8. verify release/prod build cannot open layout probe/debug routes
9. verify README reflects the real status

---

## 10. Definition of done

This pass is done only when:
- all confirmed defects in Section 4 are fixed
- tests in Section 7 are added or updated
- docs in Section 8 are updated
- manual QA status in Section 9 is recorded
- no known snapshot/reconnect-token leak remains
- no production-visible debug/probe path remains
