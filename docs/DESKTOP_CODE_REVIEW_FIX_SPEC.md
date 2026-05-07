# DESKTOP_CODE_REVIEW_FIX_SPEC.md

## 1. Purpose

This document defines the **desktop cleanup and hardening pass** after code review.

It is **not** a new product spec for the desktop app.  
The product rules already exist in `DESKTOP_SPECS.md`.

This document exists to fix the currently confirmed implementation problems in the desktop poker app:
- hidden-information leakage through snapshots
- fake/demo/stale code remaining in the production codebase
- a fake/shell-driven Ready Room workflow
- muddled runtime truth between snapshots, public events, and private encrypted payloads
- stale/misleading invite-generation paths
- production UI/backend mismatch around event feeds and live session truth
- overly optimistic documentation and product claims

The goal of this pass is to make the desktop app:
- more truthful
- more secure
- more internally consistent
- easier to maintain
- safer to use as the desktop counterpart to the Android implementation

---

## 2. Source of truth and scope

### 2.1 Product rules are already defined elsewhere
This fix pass must preserve the existing desktop/Android-compatible product rules:
- single-table Sit 'n Go tournament
- 2 to 10 players
- host-authoritative runtime
- raw TCP over LAN
- signed protocol messages
- encrypted private hole-card delivery
- reconnect/resync model
- eliminated-player observer mode

This pass must **not** reopen those decisions.

### 2.2 This fix pass is implementation-focused
This pass is about:
- runtime correctness
- hidden-information safety
- removing stale/deceptive code paths
- aligning frontend workflows with backend truth
- making the production app stop depending on demo scaffolding

### 2.3 Canonical implementation rule
For this cleanup pass, the production runtime must be judged by:
- real host session behavior
- real join/client session behavior
- real Tauri command paths
- real Rust networking/protocol flows

Prototype/demo helpers must not be treated as acceptable substitutes for production workflows.

---

## 3. Confirmed defects to fix

Treat the following as confirmed issues to fix in this pass.

### 3.1 Snapshot hidden-information leak
Current snapshots are sending too much authoritative state to clients, including hidden card state that should never leave the host except through recipient-specific private delivery.

This is a **critical bug**.

### 3.2 Fake or stale production-adjacent code still exists
The repo still contains fake/demo/stale code that is too close to the production path, including:
- demo controller/runtime scaffolding
- shell-generated participant-seat models
- synthetic invite-generation helpers that do not reflect a real live host session
- shell-driven screens that look like production UI

These must either be:
- removed, or
- explicitly quarantined behind debug/internal-only boundaries

### 3.3 Ready Room is not a real production workflow
The current Ready Room flow is still shell-driven and not properly backed by the live host/client session state and commands.

This is a **broken workflow**, not just “unfinished polish.”

### 3.4 Runtime truth model is muddled
The codebase currently mixes:
- public event flow
- aggressive snapshot sync
- private-hole-card logic
- UI side panels that imply event-driven behavior

without one clean runtime truth model.

This creates confusion and helped cause the snapshot privacy leak.

### 3.5 Client public-event handling is incomplete
The runtime receives public events, but the client session/application layer is not using them fully as live gameplay truth.

### 3.6 Event-feed UI is misleading
The UI presents public-event/history surfaces that imply live event consumption, but production session code feeds them empty lists or underutilized data.

### 3.7 Documentation overstates readiness
The README and some adjacent docs imply a stronger and more complete product than the current implementation actually provides.

---

## 4. Hard decisions for this fix pass

These decisions are now fixed. Do not reopen them during implementation.

### 4.1 Hidden information must never be sent in generic snapshots
A recipient must never receive:
- other active players' unrevealed hole cards
- host-only hidden hand state
- any hidden information not explicitly intended for that exact recipient

This applies to:
- join snapshots
- reconnect snapshots
- resync snapshots
- any other snapshot-like transport payload

### 4.2 Private hole cards must travel only through a real private path
If a client needs its own hole cards, they must come from one of these:
- recipient-projected snapshot state that contains **only that player's own cards**
- a properly encrypted private envelope addressed to that recipient

No generic multi-player hidden card map may be sent to normal clients.

### 4.3 Production workflows must not depend on shell/demo state
Production host/join/lobby/ready/table flows must be backed by:
- real session state
- real Rust-side runtime state
- real commands/events

Shell helpers may remain only if:
- they are internal-only
- clearly marked debug/dev-only
- not reachable from normal production flows

### 4.4 Production UI must not imply unsupported behavior
If the backend does not really provide a feature, the production UI must not pretend it exists.

Examples:
- fake event feed
- fake ready-room state
- synthetic invite behavior presented as real
- demo/simulator state presented as live session truth

### 4.5 One runtime truth model must be enforced
The app must explicitly choose and implement one consistent model:

#### Required v1 desktop model
- signed public events are the normal live gameplay update mechanism
- snapshots are for join, reconnect, and resync
- private recipient-only data uses the private path
- UI panels that display event-driven history must be fed from real event data, not placeholders

Do not leave a mixed ambiguous model where snapshots silently do the work of live public events during ordinary gameplay.

### 4.6 Fake invite-generation code must not survive in production form
Any helper that constructs an invite from synthetic/bootstrap values instead of a real live host session must be:
- removed, or
- renamed and gated clearly as debug-only/internal-only,
and must not be used by production UI or production commands.

### 4.7 Demo code must be quarantined or deleted
`demo_controller()`, shell participant builders, layout probes, and similar demo/prototype artifacts are not forbidden in principle, but they must satisfy **all** of the following if they remain:
- clearly labeled debug/internal-only
- not reachable from production navigation flow
- not used by production commands
- not used to build production truth

If that cannot be guaranteed cleanly, delete them.

---

## 5. Snapshot privacy model

This section is non-negotiable.

### 5.1 Snapshot categories
There are only two acceptable snapshot shapes for non-host recipients:

#### Public snapshot
Contains only:
- tournament public metadata
- seat/public participant state
- public board cards
- blind level / timer info
- public stack / status info
- public action summaries
- placements / observer-visible results

#### Recipient-projected snapshot
Contains:
- the full public snapshot
- the local player's own recipient-only private data
- reconnect token and host key material if applicable
- no other player's unrevealed private information

### 5.2 Host-only state
The following must remain host-only:
- full hidden hand state
- unrevealed hole cards for all players
- deck order / remaining hidden deck state
- any host-only internal coordination state that is not meant for clients

### 5.3 Projection rule
Before any snapshot is serialized for a non-host recipient:
- the state must be projected for that exact recipient
- projection must happen before network serialization
- projection must happen before persistence if persisted in recipient-visible form
- projection must happen before any frontend-facing “session snapshot” is emitted

### 5.4 Tests required by the privacy model
The implementation must include explicit tests proving that:
- host sees full hidden state
- a normal active player sees only their own hole cards
- an observer sees no active players’ hole cards
- reconnect snapshot for player A does not contain player B’s unrevealed hole cards
- resync snapshot for player A does not contain player B’s unrevealed hole cards

---

## 6. Production vs debug/internal boundary

### 6.1 Production code
Production code means:
- real host session runtime
- real client session runtime
- Tauri commands used by normal app workflows
- frontend screens reachable from normal navigation
- host/join/table flows used by actual players

### 6.2 Debug/internal code
Debug/internal code may include:
- demo controllers
- layout probes
- synthetic shells
- simulator/probe panels
- stress harnesses

### 6.3 Required boundary
If debug/internal code remains:
- it must live in clearly named debug/internal modules or routes
- it must not be imported into production state flows unless behind an explicit debug gate
- it must not be the default path for any user-facing route

### 6.4 Naming requirement
Anything fake/simulated/debug should be named honestly:
- `debug_...`
- `internal_...`
- `demo_...`
- `probe_...`

Do not leave fake helpers with names that sound production-safe.

---

## 7. Ready Room requirements

### 7.1 Production requirement
The Ready Room must be one of these:

#### Option A: real production screen
If kept, it must:
- read live backend session state
- show real seated participants
- show real ready states
- call the real backend command to start the tournament
- call the real backend leave/close flows
- transition to the table only after authoritative start success

#### Option B: removed from production flow
If not made real in this pass:
- remove it from normal navigation
- do not route users through it
- do not leave it as a fake intermediary step

### 7.2 Forbidden Ready Room behavior
The Ready Room must not:
- build participant state from local shell helpers
- navigate to table with a frontend-only route transition while bypassing host start logic
- fake leave behavior by just changing routes
- present placeholder readiness not sourced from the backend

---

## 8. Event-driven runtime model

### 8.1 Required live-play model
For desktop v1:
- host emits signed public gameplay events
- clients consume those public events for normal live state changes
- snapshots are reserved for join/reconnect/resync
- private hole-card data is recipient-specific

### 8.2 Client session handling requirement
Client runtime/session handling must process:
- public gameplay events
- private hole-card deliveries
- reconnect/resync events
- protocol errors

These must actually affect the client-side session truth.

### 8.3 Event-feed truthfulness
If the UI shows an event feed, side log, or “latest public events” panel, then:
- production runtime must feed it with real event data
- or the panel must be removed/hidden until that is true

An empty placeholder event feed in production is not acceptable if the UI suggests otherwise.

### 8.4 Snapshot overuse is prohibited
Do not use “sync snapshots to everybody constantly” as the normal gameplay propagation mechanism if the app is claiming to be public-event driven.

If snapshots are used as the main live update channel, then the architecture/docs/UI must say so plainly.  
For this pass, that is **not** the chosen model.

---

## 9. Invite and join flow requirements

### 9.1 Production invite generation
Production invite/join payloads must be created from:
- a real live host session
- current real table/session values
- current real host keys
- current real join token/session metadata

### 9.2 Forbidden invite generation behavior
Do not generate production invites from:
- bootstrap placeholders
- synthetic table IDs
- fabricated signing keys
- debug namespace tokens
- anything not tied to a currently live host session

### 9.3 Stale command handling
If there are commands that only create synthetic/demo invites, they must be:
- removed, or
- renamed and marked debug-only/internal-only

---

## 10. UI truthfulness requirements

### 10.1 Production UI may only expose real flows
Production UI must only expose:
- host flow backed by real host commands
- join flow backed by real join commands
- table flow backed by real session/runtime state
- ready room only if real
- event feed only if real

### 10.2 Remove deceptive surfaces
Remove, hide, or clearly mark debug-only:
- fake ready-room flows
- fake session summaries
- fake event panels
- synthetic invite paths
- local shell participant builders driving production routes

### 10.3 README truthfulness
README and top-level docs must not claim:
- readiness
- reconnect quality
- LAN completeness
- gameplay correctness
beyond what the current code actually implements.

---

## 11. Canonical files and supersession rule

### 11.1 Canonical files for this pass
For the desktop cleanup pass, the canonical files are:
1. `DESKTOP_SPECS.md`
2. this file: `DESKTOP_CODE_REVIEW_FIX_SPEC.md`
3. the matching implementation plan: `DESKTOP_CODE_REVIEW_FIX_TODO.md`

### 11.2 Conflict rule
If older desktop review/spec/todo docs conflict with this file, this file wins for the cleanup pass.

### 11.3 Stale-file cleanup
As part of this pass, stale docs that still describe fake/demo flows as acceptable production behavior should be:
- removed
- archived
- or clearly marked obsolete

---

## 12. Required outcomes

This cleanup pass is complete only when all of the following are true:

1. snapshots no longer leak hidden information
2. private recipient-only card flow is real and safe
3. production host/join/table workflows do not depend on fake/demo shell state
4. Ready Room is either real or removed from production flow
5. public-event handling is actually used in the client session model
6. production event-feed UI is either real or removed
7. synthetic invite-generation code is removed or clearly debug-only
8. leftover demo/probe/shell code is quarantined or deleted
9. docs/README no longer overstate readiness
10. tests explicitly cover hidden-information protection and recipient projection

---

## 13. Out of scope for this pass

Do **not** expand this pass into unrelated feature work such as:
- internet play
- matchmaking
- bots
- trust-minimized dealing
- host migration
- major visual redesign
- new protocol versions unless strictly necessary for a security/privacy correction
