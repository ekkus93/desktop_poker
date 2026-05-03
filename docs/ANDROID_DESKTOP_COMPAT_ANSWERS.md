# ANDROID_DESKTOP_COMPAT_ANSWERS.md

This file answers the current Android/Desktop compatibility questions using the **current Android codebase** as the source of truth.

## Critical rule for desktop implementation

When there is any mismatch between:
- older spec markdown files,
- older review docs,
- README wording,
- and the current Android code,

**the current Android code wins** for protocol compatibility.

For desktop interoperability, treat the following as canonical first:
1. `networking/protocol/ProtocolModels.kt`
2. `crypto/CanonicalJson.kt`
3. `networking/protocol/ProtocolSecurity.kt`
4. `networking/session/TournamentSessionFlows.kt`
5. `networking/QrJoinPayloadCodec.kt`
6. `domain/JoinPayload.kt`
7. protocol and session tests under `networking/src/test/...`

---

## 1. Exact protocol schema

### 1.1 Current protocol message types
Desktop should implement the exact current Android `ProtocolMessageType` enum:

- `JOIN_TOURNAMENT_REQUEST`
- `RECONNECT_TOURNAMENT_REQUEST`
- `SEAT_CLAIM_REQUEST`
- `READY_STATE_REQUEST`
- `TOURNAMENT_STARTED_EVENT`
- `HAND_STARTING_EVENT`
- `ACTION_WINDOW_OPENED_EVENT`
- `PLAYER_ACTION_COMMITTED_EVENT`
- `STREET_REVEALED_EVENT`
- `SHOWDOWN_STARTED_EVENT`
- `SHOWDOWN_HAND_REVEALED_EVENT`
- `HAND_RESULT_COMMITTED_EVENT`
- `HAND_LIFECYCLE_EVENT` (**legacy compatibility only; host no longer emits this**)
- `ACTION_SUBMISSION_REQUEST`
- `ACTION_REJECTED_EVENT`
- `ELIMINATION_EVENT`
- `TOURNAMENT_COMPLETE_EVENT`
- `SNAPSHOT_EVENT`
- `RESYNC_REQUEST`
- `PROTOCOL_ERROR`
- `PRIVATE_HOLE_CARDS_EVENT`

### 1.2 Signed public envelope shape
Android currently uses this signed public envelope shape:

- `protocolVersion: Int`
- `messageType: ProtocolMessageType`
- `tableId: String`
- `sessionEpoch: Long`
- `senderId: String`
- `counter: Long`
- `messageId: String`
- `serverSequence: Long?`
- `payload: JsonElement`
- `signature: String?`

### 1.3 Encrypted private envelope shape
Android currently uses this encrypted private envelope shape:

- `protocolVersion: Int`
- `messageType: ProtocolMessageType` (defaults to `PRIVATE_HOLE_CARDS_EVENT`)
- `tableId: String`
- `sessionEpoch: Long`
- `senderId: String`
- `counter: Long`
- `messageId: String`
- `serverSequence: Long`
- `recipientId: String`
- `recipientKeyId: String`
- `nonce: String`
- `ciphertext: String`
- `signature: String?`

### 1.4 Current payload data classes
Desktop should mirror the current Android payload data classes and field names exactly.

#### Client -> host commands
- `JoinTournamentRequest`
  - `displayName`
  - `joinToken`
  - `signingPublicKey`
  - `encryptionPublicKey`

- `ReconnectTournamentRequest`
  - `playerId`
  - `reconnectToken`
  - `lastKnownServerSeq`

- `SeatClaimRequest`
  - `seatIndex`

- `ReadyStateRequest`
  - `isReady`

- `PlayerActionSubmission`
  - `seatIndex`
  - `actionType`
  - `raiseToAmount`

- `ResyncRequest`
  - `lastSeenServerSequence`

#### Host -> clients public events
- `TournamentStartedEvent`
  - `tournamentName`
  - `startingStack`
  - `blindSchedulePreset`
  - `frozenPlayerIds`

- `HandStarting`
  - `handNumber`
  - `handPhase`
  - `dealerSeatIndex`
  - `smallBlindSeatIndex`
  - `bigBlindSeatIndex`
  - `boardCards`

- `ActionWindowOpened`
  - `handNumber`
  - `handPhase`
  - `playerId`
  - `seatIndex`
  - `legalActions`
  - `callAmount`
  - `minRaiseTo`
  - `maxRaiseTo`
  - `deadlineEpochMs`

- `PlayerActionCommitted`
  - `handNumber`
  - `seatIndex`
  - `playerId`
  - `actionType`
  - `raiseToAmount`

- `StreetRevealed`
  - `handNumber`
  - `street`
  - `boardCards`

- `ShowdownStarted`
  - `handNumber`
  - `boardCards`

- `ShowdownHandRevealed`
  - `handNumber`
  - `playerId`
  - `holeCards`

- `HandResultCommitted`
  - `handNumber`
  - `result`

- `EliminationEvent`
  - `playerId`
  - `place`

- `TournamentCompleteEvent`
  - `winnerPlayerId`
  - `placements`

- `SnapshotEvent`
  - `state`
  - `localPlayerId`
  - `reconnectToken`
  - `hostSigningPublicKey`
  - `hostEncryptionPublicKey`

- `ProtocolErrorMessage`
  - `code`
  - `message`
  - `rejectedMessageId`

#### Host -> client private payload
- `PrivateHoleCardsEvent`
  - `recipientPlayerId`
  - `holeCards`

### 1.5 Public events vs snapshots vs private encrypted payloads
This is the current Android split:

#### Public signed live-play events
- `TOURNAMENT_STARTED_EVENT`
- `HAND_STARTING_EVENT`
- `ACTION_WINDOW_OPENED_EVENT`
- `PLAYER_ACTION_COMMITTED_EVENT`
- `STREET_REVEALED_EVENT`
- `SHOWDOWN_STARTED_EVENT`
- `SHOWDOWN_HAND_REVEALED_EVENT`
- `HAND_RESULT_COMMITTED_EVENT`
- `ELIMINATION_EVENT`
- `TOURNAMENT_COMPLETE_EVENT`

#### Signed snapshots
- `SNAPSHOT_EVENT`

Snapshots are currently used for:
- initial join success
- reconnect success
- explicit resync

#### Signed private encrypted payloads
- `PRIVATE_HOLE_CARDS_EVENT`

### 1.6 Canonical examples / fixtures
For desktop compatibility, treat these tests as canonical fixtures:
- `networking/src/test/.../ProtocolCodecTest.kt`
- `networking/src/test/.../QrJoinPayloadCodecTest.kt`
- `networking/src/test/.../TournamentSessionFlowsTest.kt`
- `testing/src/main/.../TestData.kt`

---

## 2. Canonical serialization for signatures

### 2.1 Current Android algorithm
Android currently signs **canonical JSON UTF-8 bytes** produced by `CanonicalJson`.

The algorithm is:

1. serialize via `kotlinx.serialization`
2. `encodeDefaults = true`
3. `explicitNulls = false`
4. recursively sort object keys lexicographically at **every nesting level**
5. preserve array order
6. render compact JSON with:
   - no insignificant whitespace
   - normal JSON primitives via `JsonPrimitive.toString()`
7. encode resulting string as UTF-8 bytes

### 2.2 Null handling
Because Android uses `explicitNulls = false`:
- null optional fields are **omitted**, not emitted as `"field": null`

That matters for signatures.

Examples:
- `signature = null` means the `signature` field is omitted from the signed bytes
- `serverSequence = null` means that field is omitted

### 2.3 Arrays, numbers, booleans
- arrays keep their original order
- numbers and booleans are rendered by kotlinx.serialization/json primitives
- desktop should not invent alternate numeric formatting

### 2.4 Required desktop rule
Desktop must match Android **byte-for-byte** on canonical JSON output before signing or verification.

### 2.5 Current best test vectors
Android does not currently expose a separate standalone “golden byte vector” file, but these tests are the current best canonical checks:
- `ProtocolCodecTest`
- `QrJoinPayloadCodecTest`

Desktop should add explicit cross-platform golden test vectors as part of implementation.

---

## 3. Signature and encryption details

### 3.1 Signed public envelope coverage
For `SignedEnvelope`, Android signs the canonical bytes of:

- the entire envelope
- with `signature = null`

Because `explicitNulls = false`, the `signature` field is omitted from the signed bytes.

No other public-envelope fields are excluded.

### 3.2 Encrypted private envelope signature coverage
For `EncryptedPrivateEnvelope`, Android signs the canonical bytes of:

- the entire encrypted envelope
- with `signature = null`

So the signature covers:
- metadata fields
- `nonce`
- `ciphertext`
- recipient identifiers
- `serverSequence`

### 3.3 AEAD associated data for encrypted private payloads
Android also computes AEAD associated data using this exact field set:

- `protocolVersion`
- `messageType` = `PRIVATE_HOLE_CARDS_EVENT`
- `tableId`
- `sessionEpoch`
- `senderId`
- `counter`
- `messageId`
- `serverSequence`
- `recipientId`
- `recipientKeyId`

Desktop must match that associated-data shape exactly.

### 3.4 Public key encoding
Android currently encodes:
- signing public keys
- encryption public keys
- signatures
- nonces
- ciphertext

using **base64url without padding**.

### 3.5 Key IDs
Android key IDs are fingerprints of the public key:
- SHA-256 digest
- take first 8 bytes
- encode as lowercase hex
- resulting length = 16 hex chars

Desktop should match that if key IDs are displayed or transmitted.

---

## 4. Default networking values

### 4.1 Default host port
Current Android default host port:
- `43818`

### 4.2 Android-specific connection assumptions desktop should preserve
Desktop should preserve these current Android-side assumptions:
- raw TCP, not WebSocket
- host authoritative
- one table per host session
- join payload provides concrete endpoint tuple
- direct payload join is the actual current v1 path
- snapshots for join/reconnect/resync
- signed public events for steady-state live play

### 4.3 Host address validation currently enforced
Current Android runtime validation is weaker than the written spec in some places.

#### `JoinPayload` object validation
- `hostAddress` must be non-blank
- `hostPort` must be in `1..65535`

#### `TournamentSessionFlows.validateJoinPayload(...)`
Additional runtime rule:
- reject `hostAddress == "0.0.0.0"`

### 4.4 Important current implementation note
Current Android runtime does **not** fully enforce every stronger host-address rule that some markdown docs discussed earlier.
For desktop compatibility, match current runtime behavior first:
- non-blank address
- valid port
- reject `0.0.0.0`

You may enforce stronger validation later only if Android is updated to do the same.

---

## 5. Join payload contract

### 5.1 Canonical payload data model
Current Android `JoinPayload` fields are:

- `payloadVersion: Int`
- `hostAddress: String`
- `hostPort: Int`
- `tableId: String`
- `sessionEpoch: Long`
- `hostSigningPublicKey: String`
- `joinToken: String`
- `generatedAtMs: Long`
- `tableName: String? = null`

### 5.2 Serialized format currently used by Android
Android currently uses a **compact join string** format for QR/direct join:

- canonical JSON of `JoinPayload`
- gzip-compressed
- base64url encoded without padding
- prefixed with `pkr1_`

So the current direct payload is **not raw JSON** in normal use.

### 5.3 Backward compatibility behavior
Current Android decoder also accepts **legacy raw JSON payload strings**.

Desktop should:
- support the current compact `pkr1_...` format
- optionally support legacy raw JSON for compatibility

### 5.4 Versioning rule
Current payload version field:
- `payloadVersion = NetworkDefaults.PROTOCOL_VERSION`
- current Android value = `1`

Desktop should reject unsupported payload versions.

### 5.5 Join token source
Current Android host currently embeds:
- `joinToken = joinTokenForTable(tableId)` inside `LanTournamentPlatform.advertise(...)`

However, host-side join validation compares against:
- `hostSession.advertisedService.payload.joinToken`

This means desktop must match the actual currently advertised token behavior, not older markdown discussions about more elaborate token rotation.

### 5.6 Important implementation mismatch note
Current README says QR/direct payload join is the supported v1 path, and current `LanTournamentPlatform.discover()` returns `emptyList()`.
Treat **direct payload join** as canonical v1 behavior.
Do not treat NSD/room-code discovery as stable canonical behavior yet.

---

## 6. Reconnect identity rules

### 6.1 Reconnect token format
Current Android reconnect token:
- opaque base64url-without-padding random token
- 24 random bytes
- issued by `ReconnectManager.issueReconnectToken()`

### 6.2 When reconnect token is issued
Current Android issues reconnect token at **initial join acceptance** when creating the admitted participant identity.

### 6.3 Current reconnect identity binding
Reconnect acceptance currently depends on:
- `playerId`
- reconnect token
- signing public key matching the original participant record
- participant existing in registry
- participant state being reconnectable
- reconnect rights not expired
- `lastKnownServerSeq` not being ahead of authoritative host sequence

### 6.4 Current reconnectable participant states
Android currently accepts reconnect only if participant state is one of:
- `SEATED_LOBBY`
- `SEATED_READY`
- `ACTIVE_PLAYER`
- `ELIMINATED_OBSERVER`
- `RECONNECTING_ACTIVE`
- `RECONNECTING_OBSERVER`

### 6.5 Current reconnect expiry windows
Current Android reconnect windows:
- pre-start: 120s
- active hand: 30s
- between hands: 120s
- observer: 300s

### 6.6 Current reconnect acceptance / rejection conditions
Reconnect is accepted only if all of these are true:
- envelope signature verifies against original bound signing key
- `playerId` exists in participant registry
- reconnect token matches
- signing public key matches stored participant signing key
- participant state is reconnectable
- reconnect expiry not passed
- `lastKnownServerSeq` is not ahead of host authoritative sequence

Reconnect is rejected if any of those checks fail.

### 6.7 Same keypair rule
Current Android reconnect is tied to the **same original session signing keypair**.
Desktop should assume:
- regenerated keys after restart are **not** equivalent
- if desktop wants restart-safe reconnect later, that must be a deliberate protocol change on both platforms

---

## 7. Sequence and replay protection

### 7.1 Sender counters
Current Android tracks counters:
- per sender ID
- across the session
- not per connection

### 7.2 Authoritative host sequence
Current Android maintains one global host `serverSequence` domain for authoritative host envelopes.

### 7.3 What advances host sequence
Every emitted host envelope that carries `serverSequence` advances it:
- signed public events
- signed snapshots
- encrypted private envelopes

### 7.4 Duplicate / stale / out-of-order handling
Current Android replay protection enforces:
- reject wrong table/session => `STALE_SESSION`
- reject duplicate `messageId` => `DUPLICATE_MESSAGE_ID`
- reject non-increasing sender counter => `STALE_COUNTER`
- reject non-increasing host `serverSequence` => `STALE_SERVER_SEQUENCE`

### 7.5 Resync behavior
If client receives stale host sequence during live event handling, Android currently triggers a resync path:
- send `RESYNC_REQUEST`
- host responds with full signed snapshot
- client replaces local state

### 7.6 Desktop rule
Desktop should mirror this exact model:
- sender counters per sender
- one global authoritative host sequence
- same replay rejection categories

---

## 8. Tournament constants and rule truth

### 8.1 Blind levels and durations
Yes. The current Android code does use the blind levels and durations that were written into the current canonical spec:
- same fixed blind sequence
- `FAST = 180s`
- `NORMAL = 300s`
- `SLOW = 480s`

### 8.2 Starting stack / timer / capacity defaults
Current Android defaults:
- `maxSeats = 6`
- `startingStack = 1500`
- `turnTimerSeconds = 20`

Tournament config constraints:
- seats: `2..10`
- starting stack > 0
- turn timer > 0

### 8.3 Rule truth vs older docs
Current Android code should be treated as the rule truth over stale markdown if there is conflict.

### 8.4 Important current behavior notes for desktop
Desktop should preserve these actual Android behaviors:
- short all-in is a distinct legal-action concern
- timeout authority is host-clock based
- showdown reveals all remaining showdown hands in v1
- snapshots currently carry full `TournamentState` with recipient-projected hole cards
- direct payload join is the real current join path
- room-code discovery is not stable enough to be desktop-canonical

---

## 9. Snapshots and projections

### 9.1 Snapshot shape
Current Android `SnapshotEvent` fields:
- `state: TournamentState`
- `localPlayerId: String`
- `reconnectToken: String?`
- `hostSigningPublicKey: String?`
- `hostEncryptionPublicKey: String?`

### 9.2 Snapshot state contents
The snapshot currently contains a `TournamentState`, but Android projects that state **before** embedding it in the snapshot.

Specifically:
- if `currentHand` exists, Android filters `holeCardsByPlayerId` to only the recipient player
- other players’ hole cards are removed before snapshot send

### 9.3 Public projection model
Current Android `PublicTableSnapshot` fields:
- `tournamentName`
- `roomCode`
- `seats`
- `boardCards`
- `blindLevelLabel`
- `currentHandNumber`
- `placements`

### 9.4 Per-player private projection fields
Current Android `PlayerPrivateProjection` fields:
- `publicSnapshot`
- `localPlayerId`
- `privateHoleCards`
- `canAct`
- `isObserver`
- `actionWindowPlayerId`

### 9.5 Observer state
Observer/private behavior currently works like this:
- observer gets normal public snapshot
- observer gets no private hole cards
- observer `canAct = false`

### 9.6 Hidden-information leak protections
The current Android code has a direct projection rule that strips non-local hole cards before snapshot transmission.

The best current proof points are:
- `projectStateForRecipient(...)` in `TournamentSessionFlows`
- `PlayerViewProjector`
- related session/projection tests

Desktop should match that behavior exactly.

---

## 10. Interop readiness

### 10.1 Stable enough to treat as canonical now
Desktop can safely treat these Android pieces as canonical enough to target:

- `JoinPayload` data model
- compact `pkr1_` join payload encoding
- `ProtocolMessageType` enum and envelope shapes
- canonical JSON serialization rules
- Ed25519/X25519/ChaCha20-Poly1305 semantics
- sender counter / host sequence replay rules
- direct payload join flow
- snapshot shape and projection behavior
- reconnect identity semantics
- tournament defaults and blind presets

### 10.2 Known unstable / not-yet-canonical areas
Desktop should **not** treat these as stable protocol truth yet:
- room-code discovery / Android NSD join flow
- older markdown docs that still describe room-code discovery as a firm v1 path
- any stale review/spec files that conflict with README + current code
- anything only described in old TODO/spec files but not present in runtime code

### 10.3 Existing temporary hacks or compromises
Current Android code still has a few practical realities desktop should know:
- room-code discovery is effectively disabled in the production path (`discover()` returns empty)
- QR/direct join is the real implemented path
- snapshots still carry projected `TournamentState` instead of a slimmer dedicated DTO
- some older docs in the repo are stale and should not be used as desktop protocol truth

### 10.4 Test matrix / fixtures to reuse
Desktop should reuse or mirror these Android tests:
- `ProtocolCodecTest`
- `QrJoinPayloadCodecTest`
- `TournamentSessionFlowsTest`
- `ReconnectManagerTest`
- relevant engine/tournament rule tests
- `testing/TestData.joinPayload()`

### 10.5 Final desktop instruction
For desktop compatibility, do **not** implement from the oldest markdown spec.

Implement against:
1. current Android code
2. current README
3. current protocol/session tests

Then update Android and desktop together only through deliberate protocol-versioned changes.
