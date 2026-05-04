# DESKTOP_SPECS.md

## 1. Overview

### 1.1 Product
A desktop companion app for the poker project, built with **Tauri** and **Rust**, that can host or join the same style of **single-table Sit 'n Go No-Limit Texas Hold'em tournament** as the Android app.

### 1.2 Primary Purpose
The desktop app exists for two reasons:

1. to make the poker app easier to test without needing many physical Android phones
2. to provide a real desktop client/host that can participate in the same local multiplayer ecosystem as the Android app

This is **not** just a debugging tool. It should be a real playable desktop client. But the spec should deliberately optimize for multi-instance local testing and protocol verification.

### 1.3 MVP Goal
Ship a **desktop LAN poker client/host** that is protocol-compatible with the Android app's current v1 architecture wherever practical, while making it easy to run multiple clients on one development machine.

### 1.4 Core MVP Decisions
- **Variant:** No-Limit Texas Hold'em
- **Mode:** single-table Sit 'n Go tournament
- **Players:** 2 to 10
- **Authority model:** host-authoritative in v1
- **Transport:** raw TCP over local LAN
- **Join path:** direct payload join first; desktop does not need camera QR scanning for MVP
- **Roster:** closed once tournament starts
- **Rebuy / top-up:** none
- **Spectating:** eliminated players remain as read-only observers
- **Security:** all messages signed; private hole-card delivery encrypted to recipient
- **Fairness model:** trusted host in v1
- **Framework:** Tauri + Rust
- **Frontend stance:** Tauri desktop UI with Rust backend; frontend technology may be chosen for ergonomics, but Rust backend owns networking, protocol, crypto, game/session orchestration, and persistence
- **Testing stance:** desktop MVP must support running multiple instances on one machine for development and QA

### 1.5 Non-Goals for MVP
- real money
- cash games
- rebuys
- multi-table tournaments
- centralized matchmaking server
- internet play
- blockchain oracle integration
- trustless / mental-poker dealing
- host migration in v1
- in-progress hand recovery after host death in v1
- production room-code LAN discovery unless explicitly implemented later
- native mobile packaging from this codebase
- bots / AI opponents in the desktop MVP

---

## 2. Relationship to the Android App

### 2.1 Desktop app role
The desktop app is a **companion implementation**, not a replacement for the Android app.

It should:
- preserve the same tournament semantics
- preserve the same protocol semantics
- preserve the same security model
- preserve the same host/client truth model

### 2.2 Compatibility goal
For MVP, the desktop app should be able to interoperate with the Android app at the protocol level, subject to the current Android v1 decisions:

- raw TCP over LAN
- signed protocol messages
- encrypted private hole-card delivery
- host-authoritative state
- snapshots for join / reconnect / resync
- signed public events for live play

### 2.3 Source of truth for product rules
The desktop app should inherit the poker and tournament rules already frozen for the Android project, including:

- Sit 'n Go tournament structure
- 2 to 10 players
- equal starting stacks
- fixed blind presets
- no rebuys
- all remaining showdown hands revealed in v1
- eliminated-player observer mode
- host-authoritative timeout handling
- no room-code discovery requirement in production v1 unless explicitly implemented

The desktop spec should not reopen those decisions unless the desktop UX or technical environment requires a clearly justified desktop-only deviation.

### 2.4 Desktop-first additions
The desktop app adds capabilities the Android app does not need as strongly:

- multi-instance local testing
- developer-friendly connection tooling
- copy/paste join payload flow
- optional debug/internal simulator tools, gated away from production behavior
- desktop window management and spectator-friendly layouts

---

## 3. Product Vision and Constraints

### 3.1 Product Vision
The desktop app should let a developer or player:

- host a LAN tournament from a desktop machine
- join a LAN tournament from a desktop machine
- run multiple desktop instances on one machine for local testing
- use the desktop app as a stable test harness for Android interoperability

### 3.2 Design Constraints
- must work without a centralized backend for v1
- must preserve hidden-information separation
- must keep host state authoritative
- must be robust enough for protocol-level interoperability testing
- must remain useful when multiple instances are launched on the same machine
- must not require physical QR scanning for normal desktop join flow
- should be Linux-first friendly, but not written in a way that blocks Windows/macOS later

### 3.3 Desktop-first priorities
For desktop MVP, optimize for:
- correctness over visual polish
- protocol clarity over fancy animation
- repeatable local testing over phone-like UX
- clear state visibility for debugging
- multi-window / multi-instance usability

---

## 4. Platform and Technology Decisions

### 4.1 Core stack
- **Tauri 2.x**
- **Rust** for backend logic
- Rust owns:
  - protocol
  - TCP transport
  - crypto
  - host/client session flow
  - tournament coordination
  - persistence
  - reconnect/resync logic
  - authoritative state projection
- Frontend may use a web UI inside Tauri, but the backend remains the core application brain

### 4.2 Frontend recommendation
For MVP, use:
- **TypeScript + React** in the Tauri frontend

Reason:
- fast iteration for desktop UI
- easy state inspection and developer tooling
- clean separation between Rust backend and UI presentation
- easier multi-view screen work than trying to force all UI logic into Rust

This is a recommendation, not the fundamental product requirement. The non-negotiable requirement is that **Rust owns the app logic**, not the frontend.

### 4.3 Target OS
MVP primary target:
- **Linux desktop**

Secondary target after MVP hardening:
- Windows
- macOS

### 4.4 Multi-instance requirement
The desktop app must support launching multiple instances on a single machine.

This is mandatory for MVP because it is one of the main reasons the desktop app exists.

That means:
- no single-instance lock for debug/development builds
- per-instance local profile/session state isolation
- per-instance storage namespace or profile ID
- no hardcoded singleton local ports for non-host clients
- host port collisions handled clearly

---

## 5. MVP Scope

### 5.1 Must-have MVP features
- host a tournament on desktop
- join a tournament on desktop using direct join payload
- play full tournament hands from lobby to completion
- host may also be a seated player
- reconnect/resync over TCP
- eliminated-player observer mode
- hand history
- multi-instance local testing support
- Android protocol interoperability target
- copy/paste join payload UX
- optional QR generation for sharing to phones

### 5.2 Nice-to-have but not MVP-blocking
- polished desktop-specific sound
- richer table themes
- detachable hand history panel
- developer traffic/protocol inspector pane
- read-only protocol log viewer
- room-code discovery
- QR scanner using desktop camera

### 5.3 Explicit v1 non-goals
- production cloud relay
- NAT traversal
- internet multiplayer
- trust-minimized dealing
- bot seats
- host migration
- account system
- ranking/leaderboards

---

## 6. Tournament Model

### 6.1 Tournament type
Single-table Sit 'n Go tournament.

### 6.2 Player count
2 to 10 total participants.

### 6.3 Tournament start
Tournament starts only when:
- 2 to 10 participants are seated
- all occupied seats are marked ready
- host starts the tournament
- cryptographic session identities are bound

### 6.4 Tournament progression
Once started:
- roster freezes
- no new joins
- no seat changes
- equal starting stacks assigned
- blind schedule becomes active
- tournament continues until one player remains not eliminated

### 6.5 Observer behavior
Eliminated players:
- cannot act
- cannot receive private cards
- cannot re-enter
- may remain as read-only observers until they leave or the tournament ends

---

## 7. Poker Rules and Shared Semantics

The desktop app must preserve the same poker rules as the Android app.

### 7.1 Core rules
- standard 52-card deck
- no-limit betting
- two hole cards per player
- five community cards max
- best 5-card hand from 7 available cards

### 7.2 Streets
- Preflop
- Flop
- Turn
- River

### 7.3 Showdown
- all players who reach showdown reveal in v1
- no mucking option in v1

### 7.4 Short all-in rule
Desktop implementation must preserve:
- short all-ins are legal
- short all-ins do not reopen action unless they constitute a full raise
- UI must not advertise normal raise when only short all-in sizing is available

### 7.5 Timeout rule
- host clock is authoritative
- if timeout fires first, timeout resolution wins
- timeout resolution is `check if legal, else fold`

### 7.6 Blind progression
Use the same blind preset values as the Android app for compatibility.

**Blind sequence:**
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

**Preset durations:**
- Fast: 3 min
- Normal: 5 min
- Slow: 8 min

Blind changes apply **between hands only**.

---

## 8. Networking Model

### 8.1 v1 networking
- raw TCP sockets over local LAN
- one host process owns authoritative table state
- clients connect directly to host
- no centralized backend
- no relay server

### 8.2 Supported join path for desktop MVP
Production v1 desktop join path is:

1. host generates one canonical direct join payload
2. joiner pastes payload or opens app with payload argument/link
3. client connects directly to host over TCP
4. signed join request is sent
5. host accepts/rejects

### 8.3 QR role on desktop
Desktop does **not** require camera-based QR scanning for MVP.

However, host desktop may optionally display a QR code that encodes the same canonical join payload so Android devices can join more easily.

### 8.4 Room-code discovery
Desktop MVP production stance:
- **not required**
- if unfinished, keep hidden
- do not imply it works unless fully implemented

### 8.5 Port behavior
Default host port should match the Android project where practical for compatibility, unless desktop-specific constraints force a change.

If the chosen port is unavailable:
- fail loudly
- allow host to choose another port
- regenerate/share a new join payload if port changes

---

## 9. Security and Crypto Model

### 9.1 Security goals
Desktop MVP must protect against:
- message spoofing
- forged actions
- forged seat claims
- simple replay attacks
- accidental stale state submissions

Desktop MVP does **not** claim:
- trustless fairness
- malicious host resistance
- advanced collusion resistance

### 9.2 Session identities
Each app instance generates ephemeral session keys:
- **Ed25519** signing keypair
- **X25519** key agreement keypair

### 9.3 Message security
- all messages signed with Ed25519
- private hole-card payloads encrypted to recipient using X25519 + ChaCha20-Poly1305
- host signs encrypted private envelopes too

### 9.4 Replay protection
Every signed message includes:
- protocolVersion
- messageType
- tableId
- sessionEpoch
- senderId
- counter
- messageId
- body

### 9.5 Canonical serialization
Desktop must preserve the same canonical envelope/signature rules as Android:
- canonical JSON
- UTF-8
- sorted object keys
- no insignificant whitespace

### 9.6 Rust crypto implementation
For desktop Rust implementation, choose one concrete Rust crypto stack and keep it wrapped behind an internal abstraction.

Recommended Rust baseline:
- `ed25519-dalek` for Ed25519
- `x25519-dalek` for X25519
- `chacha20poly1305` crate for AEAD
- `serde` + explicit canonical serializer for signing bytes

The important rule is not the crate names themselves; the important rule is:
- one fixed provider stack for v1
- hidden behind an internal crypto abstraction
- protocol-compatible with Android semantics

---

## 10. Protocol Compatibility Requirements

### 10.1 Compatibility goal
Desktop and Android should be able to interoperate as host/client pairs when they are on the same protocol revision.

### 10.2 Required compatibility domains
Desktop must match Android in:
- envelope fields
- message type semantics
- canonical serialization rules
- signature rules
- encrypted private message handling
- sequence handling
- join/reconnect/resync behavior
- tournament and hand state semantics

### 10.3 Compatibility testing
Desktop MVP must include explicit compatibility verification tasks:
- desktop host + desktop client
- desktop host + Android client
- Android host + desktop client

If full bidirectional compatibility is not complete in MVP, the spec must say so explicitly in code comments or release notes. Do not quietly assume compatibility.

---

## 11. Join Payload Contract

### 11.1 Canonical payload
Desktop direct join must use one canonical versioned payload schema.

Required fields:
- `payloadVersion`
- `hostAddress`
- `hostPort`
- `tableId`
- `sessionEpoch`
- `hostSigningPublicKey`
- `joinToken`
- `generatedAtMs`
- optional `tableName`

### 11.2 Validation
Reject payload if:
- unsupported version
- invalid/empty/non-connectable address
- invalid port
- missing `tableId`
- missing `hostSigningPublicKey`
- missing `joinToken`

### 11.3 Copy/paste flow
Desktop join UI must support:
- paste full payload string
- parse and validate before trying connection
- show clear errors before connect attempt if payload is invalid

### 11.4 Deep-link / CLI support
Strongly recommended:
- app accepts a join payload via CLI arg or custom URI
- makes it easy to launch multiple clients locally for testing

---

## 12. Reconnect and Resync Rules

### 12.1 Reconnect identity
Reconnect requires:
- same `playerId`
- valid reconnect token
- valid signature from the same signing keypair bound at original join

### 12.2 App restart rule
In v1:
- app restart with regenerated session keys does **not** qualify as reconnect
- if original ephemeral keypair is gone, reconnect fails

### 12.3 Mid-hand reconnect
Allowed:
- restore current public state
- restore own private cards if still live
- no replay of missed animations

### 12.4 Resync
Snapshots are used for:
- join
- reconnect
- explicit resync

Live gameplay updates are processed from signed public events.

---

## 13. Desktop UX and Screen Model

### 13.1 Desktop design goals
Desktop UX should be:
- usable for actual play
- better than Android for debugging and observation
- tolerant of multiple windows/instances
- easy to host and join without camera workflows

### 13.2 Core screens
- Home
- Host Tournament Setup
- Join Tournament
- Tournament Lobby
- Main Table
- Hand History
- Tournament Complete
- Rules / Settings
- Reconnect / Error states
- optional Internal Tools screen in debug builds only

Ready-state behavior is merged into the Tournament Lobby rather than exposed as a separate player-facing Ready Room.

### 13.3 Home screen
Primary actions:
- Host Tournament
- Join Tournament

Support actions:
- Rules / Settings
- Hand History

Optional developer action in debug builds only:
- Open Internal Tools

### 13.4 Host screen
Host screen should support:
- tournament name
- max players
- starting stack
- blind preset
- turn timer
- host port
- display share details for other players immediately
- copy share details button
- direct payload copy and QR sharing only when the host runtime has produced them

### 13.5 Join screen
Join screen should support:
- paste payload
- validate payload
- connect
- optional recent payload history
- optional join via CLI/deep link if app launched with payload

### 13.6 Main table
Desktop table should preserve the same information hierarchy as Android, but use the extra space better.

Always prioritize:
1. whose turn it is
2. community cards
3. current call / raise state
4. pot size
5. local player cards
6. stacks
7. blind level / next blind timer
8. eliminations / standings

### 13.7 Desktop-specific table enhancements
Desktop may include, if simple:
- collapsible right-side hand history / event feed
- player list / standings panel
- bigger seat widgets for 6 or fewer players
- resizable window support
- keyboard shortcuts for common actions in debug/internal mode only if safe

### 13.8 Observer mode
Eliminated players remain in a spectator view with:
- public table
- standings
- hand history
- no action tray

### 13.9 Debug/developer mode
Debug-only tools may include:
- protocol log panel
- current snapshot inspector
- active action window inspector
- sequence counter display
- reconnect token status view
- launch additional client instance shortcut

These surfaces should be presented as internal tools and must not pollute production UX.

---

## 14. Multi-Instance and Test Harness Requirements

### 14.1 Core requirement
The desktop app must make it easy to test 2 to 10 participants without needing 10 phones.

### 14.2 Supported testing modes
MVP should support:
- multiple desktop instances on one machine using loopback/local LAN
- desktop + Android mixed testing on same LAN
- developer-visible connection information

### 14.3 Profile isolation
Each desktop instance must have isolated local state:
- session identity
- reconnect data
- local settings
- hand-history cache if persisted
- no accidental sharing between instances unless deliberately designed

### 14.4 Internal-tools stance
Internal tools or synthetic participant helpers may exist, but:
- they are debug/internal only
- they are not the production default
- they must not hide broken real LAN behavior

### 14.5 Default runtime path
Both debug and release desktop builds should exercise the **real LAN runtime path by default**.

Do not default desktop debug builds to a fake simulator path.

---

## 15. Architecture and Module Layout

### 15.1 Rust backend modules
Recommended backend module layout:

- `domain` — shared immutable models and enums
- `engine` — pure poker rules and settlement
- `tournament` — lobby, ready-check, hand loop, placement flow
- `protocol` — envelope models, message types, canonical serialization
- `networking` — TCP server/client, join payload, session flow
- `crypto` — signing, verification, encryption, replay protection
- `storage` — local persistence
- `interop` — Android/Desktop compatibility helpers and protocol conformance tests
- `app_state` — backend application orchestration exposed to Tauri commands/events

### 15.2 Frontend layers
Recommended frontend separation:
- screen components
- presentation/view models
- table rendering
- host/join workflows
- debug/dev panels
- state subscriptions to Tauri backend events

### 15.3 State ownership
Rust backend owns:
- authoritative host runtime
- client runtime session truth
- snapshots
- protocol event processing
- reconnect logic
- storage

Frontend owns:
- rendering
- user interaction
- local transient UI state only

### 15.4 Event model
Desktop should preserve:
- signed public event consumption for live play
- snapshots only for join/reconnect/resync
- clear separation of public/private/host-only state

---

## 16. Persistence

### 16.1 Persist locally
- window settings
- last-used host settings
- recent join payloads
- local display name
- hand history summaries
- reconnect info for still-live session if useful

### 16.2 Do not persist as truth
- do not reconstruct gameplay state locally without authoritative reconnect/resync
- do not pretend reconnect succeeded from cached state alone

### 16.3 Per-instance storage
Storage must be namespaced so multiple app instances do not stomp each other.

---

## 17. Assets and Audio

### 17.1 Initial asset stance
Desktop should reuse the current Android asset philosophy where practical:
- generated in-app card faces/back treatments or permissive card art
- no licensing headaches for MVP
- minimal custom art dependency

### 17.2 Desktop-specific opportunities
Desktop can later improve:
- larger felt/table styling
- clearer markers
- sharper typography
- better win/elimination overlays

### 17.3 Sound
MVP sound remains optional and disabled by default unless intentionally implemented.

---

## 18. Testing Requirements

### 18.1 Required automated tests
- tournament start guards
- button/blind rotation
- heads-up behavior
- blind progression timing
- legal action generation
- short all-in / reopen-action behavior
- pot and side-pot settlement
- elimination ordering
- timeout resolution
- reconnect/resync validation
- invalid signature / replay rejection
- direct-join payload parsing/validation
- desktop backend protocol event consumption

### 18.2 Required multi-instance local tests
- one host + one desktop client on same machine
- one host + multiple desktop clients on same machine
- host/client on different desktop machines on same LAN if available

### 18.3 Required Android interop tests
- desktop host + Android client
- Android host + desktop client

### 18.4 Manual acceptance gate
Desktop MVP is not done until manual QA proves:
- host can create tournament
- client can join from copied payload
- real hand can be played
- timeout commits work
- reconnect works
- tournament advances across hands
- elimination and tournament complete are visible correctly
- multiple desktop instances can coexist without state collisions

---

## 19. Release and Debug Behavior

### 19.1 Production behavior
Production desktop build must:
- use real LAN runtime
- not expose internal simulator mode
- not claim room-code discovery works unless implemented
- fail loudly if no valid host LAN IP is available
- not advertise unusable host endpoints

### 19.2 Debug behavior
Debug build must still default to real LAN runtime.

Debug-only tools may exist, but they must be opt-in and clearly separated.

### 19.3 LAN IP failure
If no valid connectable LAN IP exists:
- block production hosting
- do not generate join payload
- show explicit error

---

## 20. v1 / v2 Boundary

### 20.1 v1
- trusted host
- local LAN tournament play
- direct payload join
- signed messages
- encrypted private cards
- reconnect and resync
- multi-instance desktop testing support
- desktop + Android compatibility target

### 20.2 v2 candidates
- room-code discovery if still missing
- richer spectator features
- detachable advanced debug panels
- deeper replay tools
- host migration
- between-hands host recovery
- internet connectivity
- trust-minimized dealing research

---

## 21. Definition of Done for Desktop MVP

Desktop MVP is complete only when all of the following are true:

1. a user can host a tournament from desktop
2. another desktop instance can join via direct join payload
3. the tournament can be played from lobby to completion
4. reconnect/resync works over real TCP
5. multiple desktop instances on one machine work without identity/storage collisions
6. Android/Desktop interoperability is at least intentionally tested and documented
7. production build uses real LAN runtime by default
8. simulator/debug tooling is not the default path
9. no hidden-information leaks occur in spectator/client views
10. manual QA confirms real multiplayer operation
