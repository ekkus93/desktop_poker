# Android/Desktop interoperability audit

This audit compares the desktop implementation against the current Android source at `ekkus93/android_poker@11666563e712b64004c46950966dd9b98230520f`.

## Evidence used

- Android source of truth:
  - `domain/src/main/kotlin/com/ekkus93/androidpoker/domain/JoinPayload.kt`
  - `crypto/src/main/kotlin/com/ekkus93/androidpoker/crypto/CanonicalJson.kt`
  - `crypto/src/main/kotlin/com/ekkus93/androidpoker/crypto/BouncyCastleCryptoProvider.kt`
  - `networking/src/main/java/com/ekkus93/androidpoker/networking/QrJoinPayloadCodec.kt`
  - `networking/src/main/java/com/ekkus93/androidpoker/networking/protocol/ProtocolModels.kt`
  - `networking/src/main/java/com/ekkus93/androidpoker/networking/protocol/ProtocolSecurity.kt`
  - `networking/src/main/java/com/ekkus93/androidpoker/networking/session/TournamentSessionFlows.kt`
- Android tests run during this audit:
  - `./gradlew :crypto:test :networking:test :tournament:test`

## Confirmed matches

| Area | Desktop status | Notes |
| --- | --- | --- |
| Required protocol version | Match | Both sides currently use protocol version `1`. |
| Join payload shape | Match | `payloadVersion`, `hostAddress`, `hostPort`, `tableId`, `sessionEpoch`, `hostSigningPublicKey`, `joinToken`, `generatedAtMs`, and optional `tableName` align. |
| Direct join payload codec | Match | Both sides use canonical JSON -> gzip -> base64url without padding with the `pkr1_` prefix, and both accept legacy raw JSON input. |
| Canonical JSON | Match | Both sides sort object keys recursively and omit explicit null fields. Signed bytes omit the top-level `signature` field. |
| Signed envelope core fields | Match | `protocolVersion`, `messageType`, `tableId`, `sessionEpoch`, `senderId`, `counter`, `messageId`, optional `serverSequence`, `payload`, and optional `signature` align. |
| Key encoding and fingerprints | Match | Public keys/signatures use base64url without padding. Key IDs are the first 8 bytes of SHA-256 rendered as 16 lowercase hex chars. |
| Replay/session guards | Match | Both sides reject wrong table/session, duplicate `messageId`, stale sender counters, and stale `serverSequence`. |
| Snapshot reconnect metadata | Match | Snapshot payloads include `reconnectToken`, `hostSigningPublicKey`, and `hostEncryptionPublicKey`. |
| Reconnect and resync counters | Match | Desktop now accepts/serializes Android-compatible optional `lastKnownServerSeq` and `lastSeenServerSequence`. |

## Fixed during this audit

- **Private-envelope AEAD key derivation:** Android derives the ChaCha20-Poly1305 key with `HKDF-SHA256(sharedSecret, salt="android_poker:v1", info="x25519-chacha20poly1305")`. Desktop now uses the same derivation instead of `SHA-256(sharedSecret)`.

## Known temporary incompatibilities

These still need live mixed-runtime verification before Android/Desktop interop can be called proven:

1. `ACTION_WINDOW_OPENED_EVENT` still has one intentional deferred shape difference. Android's model has `handNumber`, `handPhase`, `playerId`, `seatIndex`, `legalActions`, `callAmount`, optional raise bounds, and an optional deadline. Desktop now emits Android's street-specific phase enum (`PREFLOP_ACTION`, `FLOP_ACTION`, `TURN_ACTION`, or `RIVER_ACTION`), and the current Android coordinator supplies a deadline. Desktop also sends `actionWindowId`, which Android does not model; Android's wire decoder rejects unknown keys. Desktop uses this identifier to reject stale action submissions, while Android's `PlayerActionSubmission` has no corresponding field. Do not remove the guard or field until both platforms agree on a compatible stale-window token contract.
2. `ACTION_REJECTED_EVENT` now uses Android's `rejectedMessageId` and `reason` payload shape. Desktop currently has no production emitter for this event, so emission and correlation against an actual rejected request still need implementation and mixed-runtime verification.
3. The current desktop repo has not run a real Android client against the desktop host, or a real desktop client against the Android host, in this environment.

Desktop fixture tests pin the `actionWindowId` blocker and the aligned rejection payload. A runtime integration test also checks that action-window phase values use Android's enum names.

## What is **not** proven yet

Interop is **not** fully proven.

The following have **not** been validated end-to-end here with a live Android runtime or device/emulator session:

- desktop host + Android client
- Android host + desktop client
- live seat claim / ready / start flows across both platforms
- live public-event handling across both platforms
- private hole-card delivery across both platforms
- reconnect/resync across both platforms
- elimination and tournament completion visibility across both platforms

Until those mixed-runtime sessions are executed successfully, release notes and checklist status should continue to describe Android/Desktop interoperability as **audited but not fully proven**.
