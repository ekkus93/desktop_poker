# DESKTOP_TODO.md

This file is the implementation plan for the desktop poker app defined by `DESKTOP_SPECS.md`.

It is intentionally written as a direct build plan for GitHub Copilot or another coding agent.
Do **not** reopen product decisions already fixed in `DESKTOP_SPECS.md`.
Implement the desktop app as a real Tauri + Rust LAN poker client/host, not as a mock UI shell.

## 1. Non-negotiable decisions

Before coding, treat these as fixed:

- The desktop app is a **real playable client/host**, not only a simulator.
- The desktop app uses **Tauri + Rust**.
- Rust owns networking, protocol, crypto, reconnect/resync, tournament/session logic, persistence, and authoritative state projection.
- The desktop app is **single-table Sit 'n Go No-Limit Texas Hold'em** only for MVP.
- Player count is **2 to 10**.
- The authority model is **host-authoritative** in v1.
- Transport is **raw TCP over local LAN**.
- Desktop production v1 join path is **direct join payload**.
- Room-code discovery is **not required** for production v1 and must not be exposed unless actually implemented.
- Eliminated players remain as **read-only observers**.
- Messages are **signed**.
- Private hole-card delivery is **encrypted to the recipient**.
- Reconnect/resync must work over real TCP.
- The desktop app must support **multiple instances on one machine**.
- Both debug and release desktop builds must default to the **real LAN runtime path**.
- Debug/internal simulator tools may exist, but must not be the default path.
- Desktop MVP must target **Android/Desktop protocol compatibility**.
- Desktop MVP manual QA must include:
  - desktop host + desktop client
  - desktop host + Android client
  - Android host + desktop client where possible

## 2. Milestone plan

### M0. Project skeleton and desktop architecture decisions
### M1. Core Rust domain and shared state model
### M2. Protocol and crypto compatibility layer
### M3. Real TCP LAN host/client runtime
### M4. Tournament coordinator and hand loop
### M5. Reconnect, resync, and sequence handling
### M6. Tauri frontend shell and screen flow
### M7. Main table UX and gameplay controls
### M8. Multi-instance local testing support
### M9. Interop testing with Android
### M10. Persistence, polish, and release readiness

---

## 3. M0 — Project skeleton and desktop architecture decisions

### 3.1 Create the Tauri workspace
- [x] Create a new Tauri application repository structure or desktop subproject structure
- [x] Create Rust backend crates/modules for:
  - [x] `domain`
  - [x] `engine`
  - [x] `tournament`
  - [x] `protocol`
  - [x] `networking`
  - [x] `crypto`
  - [x] `storage`
  - [x] `interop`
  - [x] `app_state`
- [x] Create frontend app structure for:
  - [x] screens
  - [x] shared components
  - [x] table rendering
  - [x] host/join flows
  - [x] debug/internal tools
- [x] Wire Tauri command/event bridge between frontend and Rust backend

### 3.2 Freeze implementation choices
- [x] Freeze frontend stack
  - [x] Use React + TypeScript unless there is a compelling reason not to
- [x] Freeze Rust serialization strategy
  - [x] `serde` for JSON models
  - [x] one canonical serializer path for signing bytes
- [x] Freeze Rust crypto stack
  - [x] `ed25519-dalek`
  - [x] `x25519-dalek`
  - [x] `chacha20poly1305`
- [x] Freeze TCP framing approach
  - [x] length-prefixed JSON envelopes
  - [x] or newline-delimited canonical envelope if and only if safe and fully specified
- [x] Freeze host default port
  - [x] match Android default if practical
- [x] Freeze per-instance storage strategy
  - [x] profile directory or instance namespace required

### 3.3 Document architecture boundaries
- [x] Create desktop architecture note summarizing:
  - [x] Rust-owned logic boundary
  - [x] frontend-owned rendering boundary
  - [x] protocol compatibility goals with Android
  - [x] multi-instance requirements
- [x] Explicitly document that frontend must not become source of truth for game state

### 3.4 Local development setup
- [x] Add README instructions for:
  - [x] Rust toolchain
  - [x] Node/package manager
  - [x] Tauri prerequisites on Linux
  - [x] running multiple instances locally
  - [x] passing join payload via CLI or env var
- [x] Add formatting/lint/test commands
  - [x] `cargo fmt`
  - [x] `cargo clippy`
  - [x] Rust tests
  - [x] frontend lint/test where applicable

---

## 4. M1 — Core Rust domain and shared state model

### 4.1 Port or recreate domain enums and value types
- [x] Tournament phase enums
- [x] Hand-cycle phase enums
- [x] Street phase enums
- [x] Seat occupancy state
- [x] Tournament seat state
- [x] Connection state
- [x] Hand participation state
- [x] Action type
- [x] Blind level model
- [x] Marker types (D/SB/BB)
- [x] Card / rank / suit model

### 4.2 Create immutable domain models
- [x] `TournamentConfig`
- [x] `BlindSchedule`
- [x] `TournamentState`
- [x] `HandState`
- [x] `BettingRoundState`
- [x] `SeatState`
- [x] `ParticipantRegistryEntry`
- [x] `PlayerIdentity`
- [x] `ActionWindow`
- [x] `HandResult`
- [x] `PotSummary`
- [x] `PlacementEntry`
- [x] `JoinPayload`
- [x] `SnapshotState`
- [x] `PublicState`
- [x] `PrivateState`
- [x] `ObserverProjection`

### 4.3 Add invariants and validators
- [x] Validate player count 2–10
- [x] Validate starting stack > 0
- [x] Validate blind schedule ordering
- [x] Validate no duplicate occupied seats
- [x] Validate no duplicate participant IDs
- [x] Validate no duplicate signing key bindings
- [x] Validate no illegal public/private state leakage

### 4.4 Seat and participant semantics
- [x] Create participant registry separate from seat map
- [x] Define participant states:
  - [x] admitted
  - [x] seated
  - [x] active
  - [x] reconnecting
  - [x] eliminated observer
  - [x] fully removed
- [x] Ensure seat occupancy and participant registry are distinct but linked
- [x] Implement one canonical capacity-counting function
- [x] Ensure eliminated observers do not count toward join capacity
- [x] Ensure admitted but unseated participants do count

### 4.5 Projection model
- [x] Create one projector that converts authoritative state into:
  - [x] public projection
  - [x] per-player private projection
  - [x] observer projection
- [x] Ensure hidden data never appears in public projection
- [x] Ensure eliminated observers never get private cards or action windows

---

## 5. M2 — Protocol and crypto compatibility layer

### 5.1 Implement canonical envelope models
- [x] `SignedEnvelope`
- [x] `EncryptedPrivateEnvelope`
- [x] message type enums matching the Android protocol where practical
- [x] body payload structs for:
  - [x] join
  - [x] reconnect
  - [x] seat claim
  - [x] ready state changes
  - [x] tournament start
  - [x] hand lifecycle events
  - [x] action submission
  - [x] action rejection
  - [x] elimination
  - [x] tournament completion
  - [x] snapshot
  - [x] resync
  - [x] protocol errors

### 5.2 Implement canonical JSON serialization for signature bytes
- [x] Sort object keys lexicographically at every nesting level
- [x] Use UTF-8 encoding
- [x] Emit no insignificant whitespace
- [x] Exclude `signature` field from signed bytes
- [x] Create test vectors for canonical serialization
- [x] Compare canonical bytes against expected fixtures

### 5.3 Implement Rust crypto provider
- [x] Generate Ed25519 keypair
- [x] Generate X25519 keypair
- [x] Sign signed envelopes
- [x] Verify signed envelopes
- [x] Encrypt private payloads
- [x] Decrypt private payloads
- [x] Expose key fingerprints / IDs
- [x] Keep provider behind internal trait/abstraction

### 5.4 Replay protection
- [x] Add message counters per sender
- [x] Add message IDs for dedupe
- [x] Reject stale counters
- [x] Reject duplicate message IDs
- [x] Reject stale session epoch
- [x] Reject mismatched table/session identifiers

### 5.5 Compatibility fixtures
- [x] Build protocol fixtures that match Android semantics
- [x] Add static tests for:
  - [x] envelope field ordering
  - [x] signature verification
  - [x] encrypted private payload round-trip
  - [x] sequence handling
- [x] Add comments noting any temporary intentional incompatibility

### 5.6 Join payload contract
- [x] Implement one canonical versioned join payload schema with fields:
  - [x] `payloadVersion`
  - [x] `hostAddress`
  - [x] `hostPort`
  - [x] `tableId`
  - [x] `sessionEpoch`
  - [x] `hostSigningPublicKey`
  - [x] `joinToken`
  - [x] `generatedAtMs`
  - [x] optional `tableName`
- [x] Implement strict validation for this payload
- [x] Use this exact schema everywhere:
  - [x] host generation
  - [x] UI display/copy
  - [x] join parsing
  - [x] CLI/deep link parsing
  - [x] Android interoperability testing

---

## 6. M3 — Real TCP LAN host/client runtime

### 6.1 Host TCP server
- [x] Implement real TCP listener
- [x] Bind to configured host port
- [x] Fail loudly if port unavailable
- [x] Accept multiple clients concurrently
- [x] Add graceful connection cleanup
- [x] Add framed message send/receive loop
- [x] Add host-side backpressure/error handling

### 6.2 Client TCP runtime
- [x] Implement real TCP client connect
- [x] Validate join payload before connect
- [x] Open connection to host
- [x] Perform signed join request flow
- [x] Handle accept/reject responses
- [x] Maintain live read loop for public events and private deliveries

### 6.3 LAN host IP resolution
- [x] Implement valid connectable LAN IP resolution
- [x] Reject loopback-only / wildcard / `0.0.0.0` for production host flow
- [x] Block production hosting if no valid LAN IP exists
- [x] Show explicit error to user
- [x] Do not generate join payload if host address is unusable

### 6.4 Host join payload generation
- [x] Generate canonical join payload on host startup
- [x] Allow copy-to-clipboard
- [x] Allow save/share as text
- [x] Optionally render QR code from same payload
- [x] Regenerate payload if host port changes

### 6.5 Room-code discovery stance
- [x] Do not expose unfinished room-code discovery in production UI
- [x] If discovery code exists, keep it debug-only/internal-only
- [x] Add explicit TODO comments if discovery is deferred

### 6.6 Public-event driven client update path
- [x] Process live gameplay from signed public events
- [x] Do not rely on snapshots as steady-state live-play updates
- [x] Handle event types for:
  - [x] action-window opened
  - [x] player action committed
  - [x] street revealed
  - [x] player eliminated
  - [x] tournament completed
  - [x] hand completed
  - [x] tournament starting
- [x] Reserve snapshots for:
  - [x] join
  - [x] reconnect
  - [x] explicit resync

### 6.7 Required tests
- [x] host can open listener
- [x] client can connect and join using canonical payload
- [x] public events flow from host to client over real TCP
- [x] private encrypted payload can be delivered and decrypted
- [x] invalid host IP blocks host startup
- [x] invalid payload blocks join before connect

---

## 7. M4 — Tournament coordinator and hand loop

### 7.1 Poker engine foundation
- [x] Implement standard 52-card deck
- [x] Implement shuffle
- [x] Implement dealing
- [x] Implement street reveals
- [x] Implement hand evaluation
- [x] Implement main pot and side-pot settlement
- [x] Implement odd-chip rule
- [x] Implement showdown tie handling

### 7.2 Tournament hand loop
- [x] Start tournament
- [x] Freeze roster
- [x] Assign equal starting stacks
- [x] Initialize blind schedule
- [x] Start first hand
- [x] Advance through full hand lifecycle
- [x] Settle hand
- [x] Process eliminations
- [x] Enter between-hands state
- [x] Advance blind level between hands only
- [x] Start next hand automatically after intermission
- [x] End tournament when one player remains

### 7.3 Action windows and turn ownership
- [x] Open exactly one action window at a time
- [x] Bind action window to acting participant
- [x] Compute legal actions from engine truth
- [x] Ensure UI can only act through authoritative action window
- [x] Ensure observer and non-acting players cannot act

### 7.4 Legal action generation
- [x] Make `legalActions()` match validator truth
- [x] Do not advertise `RAISE` if no legal full raise exists
- [x] Provide explicit `ALL_IN` path when only all-in is legal
- [x] Ensure short all-in is not mislabeled as a normal raise

### 7.5 Short all-in / reopen-action correctness
- [x] Short all-in must not reopen action unless it is a full raise
- [x] Preserve minimum full raise increment correctly
- [x] Reset acted-player tracking only after full raise
- [x] Add tests for edge cases

### 7.6 Timeout handling
- [x] Host clock is authoritative
- [x] Schedule timeout jobs for active action windows
- [x] Commit timeout actions through the same authoritative action path
- [x] `check if legal, else fold`
- [x] Late actions rejected as stale if timeout already fired

### 7.7 Required tests
- [x] hand progresses from start to completion
- [x] between-hands auto progression works
- [x] blind levels increase only between hands
- [x] short all-in does not reopen action
- [x] timeout commits work
- [x] tournament ends correctly

---

## 8. M5 — Reconnect, resync, and sequence handling

### 8.1 Reconnect identity rules
- [x] Reconnect requires:
  - [x] same `playerId`
  - [x] valid reconnect token
  - [x] valid signature from same original bound signing keypair
- [x] Regenerated keys after app restart do not qualify as reconnect in v1
- [x] Fail clearly if original ephemeral keypair is unavailable

### 8.2 Host-side disconnect handling
- [x] On unexpected client disconnect:
  - [x] mark participant reconnect-eligible
  - [x] preserve participant registry entry
  - [x] preserve seat ownership as appropriate
  - [x] do not silently drop session identity
- [x] Distinguish active reconnect-eligible participant from fully removed participant

### 8.3 Client-side reconnect flow
- [x] Detect transport loss
- [x] Enter reconnecting UI state
- [x] Reopen TCP connection to host
- [x] Send signed reconnect request
- [x] Handle accept/reject
- [x] On accept, replace local state with authoritative snapshot
- [x] On reject, exit to safe UI with explicit error

### 8.4 Explicit resync path
- [x] Implement real `RESYNC_REQUEST`
- [x] Trigger on sequence mismatch or state gap
- [x] Host responds with full snapshot
- [x] Client replaces local state with that snapshot

### 8.5 Sequence handling
- [x] Maintain authoritative host event sequence
- [x] Include sequence on signed public events
- [x] Advance sequence on authoritative event emission
- [x] Use one clear rule for snapshots vs events
- [x] Add sequence mismatch detection on client

### 8.6 Required tests
- [x] reconnect succeeds only with original keypair + valid token
- [x] reconnect fails with regenerated keypair
- [x] host marks disconnect as reconnect-eligible
- [x] resync replaces local state from authoritative snapshot
- [x] event sequence mismatch triggers resync

---

## 9. M6 — Tauri frontend shell and screen flow

### 9.1 Create screens
- [x] Home
- [x] Host Tournament Setup
- [x] Join Tournament
- [x] Tournament Lobby
- [x] Ready Room
- [x] Main Table
- [x] Hand History
- [x] Tournament Complete
- [x] Rules / Help
- [x] Reconnect / Error dialogs
- [x] optional debug/internal panel

### 9.2 Home screen
- [x] Host Tournament
- [x] Join Tournament
- [x] Rules
- [x] Settings
- [x] Debug/internal tools entry in debug builds only
- [x] Do not expose simulator mode in production UI

### 9.3 Host screen
- [x] Tournament name input
- [x] max players selection
- [x] starting stack selection
- [x] blind preset selection
- [x] turn timer selection
- [x] host port input or advanced settings
- [x] join payload display
- [x] copy payload button
- [x] optional show QR button

### 9.4 Join screen
- [x] paste payload text area/input
- [x] validate payload
- [x] connect button
- [x] error display
- [x] optional recent join payloads
- [x] support launch from CLI/deep-link payload

### 9.5 Lobby and ready room
- [x] seat map
- [x] participant list
- [x] ready state toggles
- [x] start tournament button for host
- [x] roster freeze explanation
- [x] leave table flow

### 9.6 Error and reconnect UI
- [x] explicit reconnecting state
- [x] reconnect success/failure banner/dialog
- [x] host-lost/table-closed dialog
- [x] invalid payload / invalid LAN IP / join failure messaging

---

## 10. M7 — Main table UX and gameplay controls

### 10.1 Main table rendering
- [ ] community cards centered
- [ ] pot totals visible
- [ ] action/turn ownership emphasized
- [ ] local player cards readable
- [ ] compact but clear opponent seats
- [ ] eliminated observer presentation
- [ ] standings access
- [ ] hand history access

### 10.2 Action tray
- [ ] Fold
- [ ] Check / Call
- [ ] Bet / Raise
- [ ] All-in
- [ ] raise slider
- [ ] quick buttons:
  - [ ] Min
  - [ ] 1/2 Pot
  - [ ] Pot
  - [ ] Max
- [ ] confirmation flow for Raise
- [ ] confirmation flow for All-in

### 10.3 Desktop-specific UX improvements
- [ ] optional side panel for history/event feed
- [ ] better use of wide desktop layouts
- [ ] resizable window support
- [ ] seat detail popovers
- [ ] more visible standings/elimination info

### 10.4 Observer mode
- [ ] no action tray
- [ ] public-only table
- [ ] standings visible
- [ ] hand history visible

### 10.5 Debug-only tools
- [ ] protocol log viewer
- [ ] current snapshot inspector
- [ ] sequence display
- [ ] action-window inspector
- [ ] launch additional client instance helper
- [ ] keep all of this out of production UI

### 10.6 Required tests / checks
- [ ] action tray only enabled for acting player
- [ ] observer mode cannot act
- [ ] table reflects public events correctly
- [ ] hand history and standings update after settlement

---

## 11. M8 — Multi-instance local testing support

### 11.1 Per-instance state isolation
- [ ] separate storage namespace per instance
- [ ] separate session identity per instance
- [ ] separate reconnect data per instance
- [ ] no cross-instance stomping of settings/state

### 11.2 Launching multiple instances
- [ ] allow multiple desktop app instances in debug and production where feasible
- [ ] no single-instance lock for development/testing builds
- [ ] document how to launch multiple clients locally

### 11.3 Local multi-instance join flow
- [ ] host on one instance
- [ ] copy payload
- [ ] join from another instance via paste or CLI arg
- [ ] ensure loopback/local LAN flows actually work

### 11.4 Optional tooling
- [ ] add a debug command/menu action to launch another instance with copied payload
- [ ] add a debug action to copy payload directly to clipboard
- [ ] add an “instance label” or profile ID in debug UI to avoid confusion

### 11.5 Required tests / checks
- [ ] two desktop instances can coexist
- [ ] they do not share identity/storage incorrectly
- [ ] they can host/join/play on one machine

---

## 12. M9 — Interop testing with Android

### 12.1 Protocol compatibility audit
- [ ] compare desktop envelope models to Android current protocol
- [ ] compare join payload semantics
- [ ] compare signature/canonical serialization semantics
- [ ] compare encrypted private envelope semantics
- [ ] compare reconnect/resync semantics

### 12.2 Interop runtime tests
- [ ] desktop host + Android client
- [ ] Android host + desktop client
- [ ] verify:
  - [ ] join
  - [ ] seat claim
  - [ ] ready/start
  - [ ] live public event handling
  - [ ] private hole-card handling
  - [ ] timeout behavior
  - [ ] reconnect/resync where possible
  - [ ] elimination / tournament complete

### 12.3 Compatibility documentation
- [ ] document any known temporary incompatibilities
- [ ] document required matching protocol version
- [ ] do not imply interop is complete if tests have not proven it

---

## 13. M10 — Persistence, polish, and release readiness

### 13.1 Persistence
- [ ] save local display name
- [ ] save last-used host settings
- [ ] save recent join payloads
- [ ] save window settings
- [ ] save hand-history summaries
- [ ] do not fake reconnect from local cache alone

### 13.2 Assets and visuals
- [ ] implement simple card rendering or generated treatments
- [ ] implement felt/background styling
- [ ] implement markers and status badges
- [ ] keep MVP asset pipeline simple and license-safe

### 13.3 Sound
- [ ] optional only
- [ ] if added, keep off by default unless intentionally chosen otherwise

### 13.4 Packaging
- [ ] build Linux desktop package
- [ ] ensure production build uses real LAN runtime by default
- [ ] ensure simulator/debug tools are hidden in production

### 13.5 Release notes / limitations
- [ ] clearly state whether Android/Desktop interop is proven
- [ ] clearly state whether room-code discovery is absent/deferred
- [ ] clearly state trusted-host model and LAN-only scope

---

## 14. Required automated test matrix

### 14.1 Domain/engine tests
- [ ] button/blind rotation
- [ ] heads-up rule
- [ ] betting legality
- [ ] short all-in / reopen-action
- [ ] pot settlement
- [ ] side pots
- [ ] odd chip
- [ ] elimination ordering
- [ ] tournament completion

### 14.2 Protocol/crypto tests
- [ ] canonical JSON bytes
- [ ] signature verification
- [ ] encrypted private payload round-trip
- [ ] replay rejection
- [ ] sequence handling
- [ ] reconnect validation
- [ ] payload parsing/validation

### 14.3 Networking tests
- [ ] host/client TCP connect
- [ ] framed envelope exchange
- [ ] disconnect handling
- [ ] reconnect flow
- [ ] resync flow

### 14.4 UI/frontend tests
- [ ] screen routing
- [ ] host flow
- [ ] join flow
- [ ] action tray enablement
- [ ] observer mode
- [ ] standings/history updates

---

## 15. Manual QA gates

Desktop MVP is **not done** until all of these succeed.

### 15.1 Desktop-only QA
- [ ] host from one desktop instance
- [ ] join from second desktop instance
- [ ] play a full hand
- [ ] play across multiple hands
- [ ] timeout commits work
- [ ] reconnect works
- [ ] elimination works
- [ ] tournament completes
- [ ] instance state isolation holds

### 15.2 Mixed desktop/Android QA
- [ ] desktop host + Android client
- [ ] Android host + desktop client
- [ ] real join via direct payload
- [ ] real live play across both platforms
- [ ] real timeout behavior across both platforms
- [ ] elimination/tournament completion visible on both

### 15.3 LAN truthfulness gate
- [ ] production build uses real LAN runtime by default
- [ ] no simulator default
- [ ] no fake/same-process-only path masquerading as production LAN
- [ ] no room-code discovery claims unless actually implemented

---

## 16. Suggested build order

Implement in this order:
1. project skeleton and module boundaries
2. domain/state model
3. protocol + crypto layer
4. real TCP host/client runtime
5. tournament hand loop
6. reconnect/resync
7. Tauri host/join flow
8. main table UI
9. multi-instance support
10. Android interoperability
11. persistence and polish
12. manual QA

---

## 17. Explicit future notes (not MVP blockers)

These are valid future tasks but not required to call desktop MVP done:
- [ ] room-code discovery / NSD equivalent
- [ ] camera QR scanning
- [ ] richer spectator tools
- [ ] detachable advanced debug views
- [ ] host migration
- [ ] between-hands host recovery
- [ ] internet connectivity
- [ ] trust-minimized dealing
