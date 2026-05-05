# UNIT_TEST2_TODO.md

This file tracks the next recommended unit-test expansion work after the recent audit that removed demo-coupled and mock-only coverage.

It is focused on the highest-value remaining gaps:

- frontend shell helpers with real branching and fallback behavior
- frontend Tauri bridge contract coverage
- hook-level provider and lifecycle edge cases
- Rust Tauri command-boundary coverage
- desktop app-state contract tests on the real runtime path
- low-level networking frame read/write behavior
- focused protocol helper and model-boundary regression tests
- lower-priority presentational component tests only where they protect meaningful behavior

The goal is to strengthen trust in the real production paths that connect the frontend, Tauri command layer, runtime state, and protocol boundaries.

## 1. Test infrastructure and backlog setup

### 1.1 Confirm test ownership and file placement
- [x] Decide the exact test files to add for each uncovered module
	- [x] `src/app/shell.test.ts`
	- [x] `src/api/desktop.test.ts`
	- [x] `src/app/useDesktopBootstrap.test.tsx`
	- [x] `src/app/useDesktopShell.test.tsx`
	- [x] `src-tauri/src/commands.rs` in-file test module or neighboring test module
	- [x] `src-tauri/src/networking/framing.rs` in-file test module
	- [x] `src-tauri/src/protocol/join_payload.rs` focused in-file tests
	- [x] `src-tauri/src/protocol/canonical.rs` focused in-file tests
	- [x] `src-tauri/src/protocol/models.rs` focused in-file tests
	- [x] `src-tauri/src/app_state/mod.rs` additional non-demo contract tests

### 1.2 Reuse existing helpers before adding new ones
- [x] Audit existing frontend fixture helpers under `src/test/**`
- [x] Audit existing Rust fixture helpers under `src-tauri/src/**`
- [x] Reuse real bootstrap/table snapshot builders where possible
- [x] Reuse existing tournament/controller fixtures instead of introducing synthetic demo data
- [x] Add new helpers only when they reduce duplication across at least two suites

### 1.3 Guardrails for every new test suite
- [x] Ensure each suite exercises real production logic in the unit under test
- [x] Keep external mocking limited to browser APIs, Tauri invoke/listen boundaries, storage, or socket transport edges
- [x] Avoid snapshot-only assertions when explicit field-level assertions are clearer
- [x] Prefer invalid-input and edge-case assertions over superficial rendering checks

---

## 2. Frontend shell helper coverage

### 2.1 Add direct tests for `getBlindPreset`
- [x] Add a test that returns the matching preset for each known preset id
- [x] Add a test that falls back to the first preset for an unknown id
- [x] Add a test that the fallback remains the current default preset rather than `undefined`

### 2.2 Add direct tests for `createDefaultHostDraft`
- [x] Add a test that the returned draft uses the current bootstrap instance label in the tournament name
- [x] Add a test that the returned draft uses the bootstrap default host port
- [x] Add a test that the default stack, blind preset, max players, and timer match the intended product defaults
- [x] Add a regression test that changes to `BLIND_PRESETS[0]` still propagate into the default draft

### 2.3 Add direct tests for `createDefaultDisplayName`
- [x] Add a test that the display name is derived from the bootstrap instance label
- [x] Add a regression test that unusual instance labels still preserve the expected `Player {label}` format

### 2.4 Add direct tests for `normalizeHostDraft`
- [x] Add a happy-path test that a fully valid stored draft is preserved exactly
- [x] Add a test that non-object input falls back completely
- [x] Add a test that blank `tournamentName` falls back while other valid fields survive
- [x] Add a test that unsupported `maxPlayers` values fall back
- [x] Add a test that unsupported `startingStack` values fall back
- [x] Add a test that unsupported `blindPresetId` values fall back
- [x] Add a test that unsupported `turnTimerSeconds` values fall back
- [x] Add a test that invalid `hostPort` values fall back
	- [x] `0`
	- [x] negative numbers
	- [x] values above `65535`
	- [x] non-integer numeric values
- [x] Add a mixed-validity test proving valid fields survive while invalid fields fall back independently

### 2.5 Add direct tests for `buildParticipantShell`
- [x] Add a test for the one-seat host-only case
- [x] Add a test that seat 1 is always the local host seat
- [x] Add a test that host ready state reflects `readySeats`
- [x] Add a test that seat 2 becomes `Reserved seat` when there is no join intent
- [x] Add a test that seat 2 becomes `Waiting for player` when `recentJoinPayloads` is non-empty
- [x] Add a test that a launch payload also counts as join intent even when `recentJoinPayloads` is empty
- [x] Add a test that `parsedLaunchJoinPayload` changes the seat detail to `Invite accepted`
- [x] Add a test that seat 2 ready state reflects `readySeats`
- [x] Add a test that seats 3+ remain open placeholders with no accidental local/ready flags

### 2.6 Add direct tests for `buildHostShareText`
- [x] Add a test for the LAN error path
	- [x] Assert the failure text is explicit about blocking hosting
	- [x] Assert the LAN error string is included verbatim
- [x] Add a test for the unresolved-LAN-IP loading path
- [x] Add a happy-path test that includes tournament name, endpoint, capacity, stack, blind preset, and timer
- [x] Add a test that attached launch payload text changes from `No invite is attached` to `An invite is already attached`
- [x] Add a regression test that the share text uses the resolved host IP and current draft port together

### 2.7 Add direct tests for storage helpers
- [x] Add tests for `storageKey`
	- [x] standard namespace + suffix
	- [x] empty suffix behavior if currently allowed
- [x] Add tests for `readStoredValue`
	- [x] `null` input returns fallback
	- [x] valid JSON returns parsed value
	- [x] invalid JSON returns fallback
	- [x] primitive JSON values parse correctly when the fallback is object-shaped or array-shaped

---

## 3. Frontend Tauri bridge contract coverage

### 3.1 Add direct tests for `fetchBootstrapState`
- [x] Add a test that browser mocks are preferred when present
- [x] Add a test that the Tauri command name is exactly `get_bootstrap_state` when mocks are absent
- [x] Add a test that the invoke result is returned unchanged

### 3.2 Add direct tests for `subscribeBootstrap`
- [x] Add a test that browser mocks are preferred when present
- [x] Add a test that the event name is exactly `desktop://bootstrap`
- [x] Add a test that event payloads are forwarded unchanged to the callback
- [x] Add a test that the listen unsubscribe function is returned to the caller

### 3.3 Add direct tests for join and host utility commands
- [x] Add a test that `validateJoinPayloadInput` invokes `validate_join_payload_input` with the raw payload string
- [x] Add a test that `resolveHostLanAddress` invokes `resolve_host_lan_address`
- [x] Add a test that browser mocks override each of those commands when present

### 3.4 Add direct tests for table-view commands
- [x] Add a test that `getTableView` invokes `get_table_view` with the provided viewer mode
- [x] Add a test that `getDebugState` invokes `get_debug_state` with the provided viewer mode
- [x] Add a test that browser mocks override each of those commands when present

### 3.5 Add direct tests for `submitTableAction`
- [x] Add a test that the command name is exactly `submit_table_action`
- [x] Add a test that `viewerMode` and `actionKind` are passed through unchanged
- [x] Add a test that an omitted raise amount becomes `null` instead of `undefined`
- [x] Add a test that an explicit raise amount is passed through as the provided number
- [x] Add a test that browser mocks override the Tauri path

### 3.6 Add direct tests for `launchAdditionalClientInstance`
- [x] Add a test that the command name is exactly `launch_additional_client_instance`
- [x] Add a test that omitted join payload becomes `null`
- [x] Add a test that a provided join payload is passed through exactly
- [x] Add a test that browser mocks override the Tauri path

### 3.7 Add negative and environment tests for the desktop bridge module
- [x] Add a test that server-side or non-browser execution safely behaves as `no browser mocks`
- [x] Add a test that unrelated keys on `window` do not affect mock resolution
- [x] Add a regression test that command names do not drift accidentally during refactors

---

## 4. Hook-level and provider-boundary frontend tests

### 4.1 Add direct tests for `useDesktopBootstrap`
- [x] Add a test that the hook throws or fails clearly when used outside its provider if that is the current contract
- [x] Add a test that initial loading state is exposed before bootstrap resolution
- [x] Add a test that bootstrap data is published after `fetchBootstrapState` resolves
- [x] Add a test that subscription updates replace prior bootstrap state
- [x] Add a test that the subscription cleanup runs on unmount
- [x] Add a test that fetch failures surface the expected error state

### 4.2 Add direct tests for `useDesktopShell`
- [x] Add a test that the hook exposes the current shell state from the provider
- [x] Add a test that local persistence-backed fields are restored on mount
- [x] Add a test that shell actions update the stored draft or derived shell state as expected
- [x] Add a test that invalid persisted host-draft data falls back cleanly through the real normalization path
- [x] Add a test that profile-scoped storage keys prevent state bleed across namespaces

### 4.3 Add focused screen-shell coverage where behavior matters
- [x] Add tests for `ScreenShell` only if it contains branching that is not already covered indirectly
	- [x] heading and status-region rendering
	- [x] action-slot rendering is not applicable in the current `ScreenShell` implementation
	- [x] back-navigation affordance visibility is not applicable in the current `ScreenShell` implementation
- [x] Skip purely presentational assertions that duplicate stronger screen-level tests

---

## 5. Rust Tauri command-boundary coverage

### 5.1 Add direct tests for pure command mapping in `commands.rs`
- [x] Add a test that `validate_join_payload_input` trims surrounding whitespace before decoding
- [x] Add a test that invalid join payloads become string errors rather than panics or opaque types
- [x] Add a test that `resolve_host_lan_address` stringifies the resolved IP exactly as expected

### 5.2 Add state-forwarding tests for command wrappers
- [x] Add a test that `get_bootstrap_state` returns the same bootstrap contract exposed by `DesktopAppState`
- [x] Add a test that `list_screen_catalog` returns the screen catalog from app state unchanged
- [x] Add a test that `get_table_view` forwards the requested viewer mode into app state
- [x] Add a test that `submit_table_action` forwards viewer mode, action kind, and optional raise amount unchanged
- [x] Add a test that `get_debug_state` forwards viewer mode unchanged
- [x] Add a test that `launch_additional_client_instance` forwards the optional join payload unchanged

### 5.3 Add command error-shape regression tests
- [x] Add a test that downstream app-state errors are surfaced as stable string messages
- [x] Add a test that observer-mode rejection or invalid action errors remain visible through the command layer
- [x] Add a regression test that command wrappers do not silently rewrite `None` optional arguments into incorrect values

---

## 6. Rust desktop app-state real-path contract tests

### 6.1 Expand `DesktopAppState` table-view coverage
- [x] Add a test that `table_view(TableViewerMode::Local)` returns the real local-player projection on a fresh runtime
- [x] Add a test that the pre-hand table view has no fake community cards and no fake hand number
- [x] Add a test that reserved or pending seats appear truthfully rather than as fabricated active players
- [x] Add a test that standings exclude reserved placeholder seats

### 6.2 Expand action-submission coverage without demo-only assumptions
- [x] Add a test that a legal local action advances the controller state and returns an updated table view
- [x] Add a test that an illegal action returns a string error without corrupting runtime state
- [x] Add a test that raise actions validate the supplied amount against the current action window
- [x] Add a test that observer-mode action submission is rejected if that is the current contract

### 6.3 Expand debug-state coverage on the real runtime path
- [x] Add a test that `debug_state` reports the current sequence and hand number from the live runtime
- [x] Add a test that the action-window summary appears only when a real action window exists
- [x] Add a test that protocol-log entries stay aligned with runtime event logging after one or more actions

### 6.4 Expand instance and launch behavior tests
- [x] Add a test that `launch_additional_client_instance` builds the child instance id within the active profile namespace
- [x] Add a test that a provided join payload is forwarded into the launched child command arguments
- [x] Add a test that invalid launch payload parsing is surfaced in bootstrap state without crashing detection

### 6.5 Add targeted pure-helper tests in `app_state/mod.rs`
- [x] Add tests for `resolve_action_request`
	- [x] fold path
	- [x] check/call path
	- [x] bet/raise path with amount
	- [x] bet/raise path without required amount
	- [x] all-in path
- [x] Add tests for `ensure_legal` against a real `ActionWindow`
	- [x] accepted legal action
	- [x] rejected illegal action
- [x] Add tests for formatting helpers only where they protect protocol-facing or user-visible contracts
	- [x] phase formatting
	- [x] street formatting
	- [x] seat-marker formatting

---

## 7. Rust networking frame read/write tests

### 7.1 Add happy-path framing tests
- [x] Add a test that `write_json_frame` writes a big-endian length prefix followed by the JSON body
- [x] Add a test that `read_json_frame` decodes a valid frame back into the requested type
- [x] Add a round-trip test that a value written by `write_json_frame` is read back unchanged by `read_json_frame`

### 7.2 Add short-read and truncated-frame tests
- [x] Add a test for truncated length-prefix reads
- [x] Add a test for a complete length prefix with a truncated body
- [x] Add a test that each failure surfaces the expected error context string

### 7.3 Add invalid-payload tests
- [x] Add a test for syntactically invalid JSON body bytes
- [x] Add a test for valid JSON that does not deserialize into the requested target type
- [x] Add a test that invalid JSON errors are wrapped as `invalid frame JSON: ...`

### 7.4 Add write-failure tests
- [x] Add a test for payloads that exceed `u32` frame length if that case can be simulated cleanly
- [x] Add a test for body or flush write failures using a controlled socket boundary
- [x] Add a test that write failures retain stage-specific context: length, body, or flush

### 7.5 Decide whether a non-`TcpStream` helper is needed for testability
- [x] If current APIs are too awkward to test directly, introduce the smallest internal helper that preserves production behavior
- [x] Do not widen the public API solely for convenience

---

## 8. Rust protocol helper coverage

### 8.1 Expand `join_payload.rs` validation tests
- [x] Add a test that unsupported `payloadVersion` is rejected
- [x] Add a test that blank `hostAddress` is rejected
- [x] Add a test that `0.0.0.0` is rejected
- [x] Add a test that `hostPort == 0` is rejected
- [x] Add a test that blank `hostSigningPublicKey` is rejected
- [x] Add a test that blank `joinToken` is rejected
- [x] Add a test that blank `tableId` is rejected
- [x] Add a test that `generatedAtMs == 0` is rejected

### 8.2 Expand join-payload encoding and decoding tests
- [x] Add a round-trip test for compact payload encoding and decoding
- [x] Add a test that compact encoded payloads keep the required `pkr1_` prefix
- [x] Add a test that legacy raw JSON decoding still works if legacy compatibility remains supported
- [x] Add a test that invalid base64 content returns the expected compact-payload error
- [x] Add a test that invalid gzip content returns the expected gzip decode error
- [x] Add a test that invalid decoded JSON returns the expected join-payload JSON error
- [x] Add a test that validly decoded but invalid semantic payloads still fail final validation

### 8.3 Expand `canonical.rs` tests
- [x] Add a test that object keys are sorted lexicographically at every nesting level
- [x] Add a test that `null` object fields are omitted
- [x] Add a test that array order is preserved
- [x] Add a test that primitive values are left unchanged
- [x] Add a test that `canonical_json_bytes_without_signature` removes only the top-level `signature` field
- [x] Add a regression test that nested `signature` keys inside payload objects are not incorrectly stripped

---

## 9. Rust protocol model-boundary regression tests

### 9.1 Add focused `SignedEnvelope` tests
- [x] Add a test that `sign` populates `signature`
- [x] Add a test that `verify` succeeds with the matching key and signed bytes
- [x] Add a test that `verify` fails when the signature is missing
- [x] Add a test that mutating a signed field invalidates verification
- [x] Add a test that omitted optional `serverSequence` stays omitted in canonical signing bytes

### 9.2 Add focused `EncryptedPrivateEnvelope` tests
- [x] Add a test that `associated_data_json` includes all expected authenticated metadata fields
- [x] Add a test that `from_encrypted_payload` copies nonce, ciphertext, recipient key id, and metadata fields exactly
- [x] Add a test that `sign` populates `signature`
- [x] Add a test that `verify` fails when `signature` is missing
- [x] Add a test that changing authenticated metadata invalidates verification

### 9.3 Add serialization-shape tests for high-risk protocol structs
- [x] Add tests for `ReconnectTournamentRequest`
	- [x] omission of `lastKnownServerSeq` when `None`
	- [x] correct field name when present
- [x] Add tests for `ActionWindowOpened`
	- [x] omission of `minRaiseTo` when `None`
	- [x] omission of `maxRaiseTo` when `None`
	- [x] stable `legalActions` serialization
- [x] Add tests for `PlayerActionCommitted`
	- [x] explicit `raiseToAmount` serialization when present
	- [x] omission behavior if `None` is intended by the current serde contract

### 9.4 Add enum and string-contract regression tests
- [x] Add a test that `ProtocolMessageType` serializes to the expected SCREAMING_SNAKE_CASE strings for high-risk variants
- [x] Add a test that no unintended rename drift occurs for message types used in compatibility-sensitive flows

---

## 10. Lower-priority frontend presentation coverage

### 10.1 Add tests only where small components protect meaningful logic
- [x] Evaluate whether `StatusBadge` has state-to-style or label branching worth locking down and defer direct tests because it only mirrors `tone` into a CSS class
- [x] Evaluate whether `SectionCard` has conditional region rendering worth locking down and defer direct tests because stronger screen-level coverage already executes the meaningful branch
- [x] Evaluate whether `TablePlaceholder` has meaningful empty-state branching not already covered by screens and defer direct tests because it is a static placeholder shell with no real state logic
- [x] Evaluate whether `ReadyRoomScreen` has conditional copy or action enablement not already covered indirectly

### 10.2 Skip low-value snapshot tests
- [x] Do not add tests that only assert static markup or CSS class presence unless they protect conditional behavior
- [x] Prefer screen-level tests when component-only assertions would just mirror props into text

---

## 11. Prioritization and rollout order

### 11.1 Implement highest-value gaps first
- [x] Phase 1: `src/app/shell.ts`
- [x] Phase 2: `src/api/desktop.ts`
- [x] Phase 3: `src-tauri/src/commands.rs`
- [x] Phase 4: `src-tauri/src/networking/framing.rs`
- [x] Phase 5: `src-tauri/src/app_state/mod.rs` non-demo contract tests
- [x] Phase 6: `src-tauri/src/protocol/join_payload.rs` and `src-tauri/src/protocol/canonical.rs`
- [x] Phase 7: `src-tauri/src/protocol/models.rs`
- [x] Phase 8: hook-level tests and any remaining low-priority frontend component coverage

### 11.2 Validate after each phase
- [x] Run the relevant Vitest suites after each frontend phase
- [x] Run the relevant Rust test filters after each Rust phase
- [x] Run `npm run lint` if shared frontend helpers change
- [x] Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` after Rust helper or API changes
- [x] Fix failures before widening scope to the next phase

### 11.3 Keep the backlog current
- [x] Mark completed tasks here as work lands
- [x] Add newly discovered edge cases as subtasks under the owning module instead of scattering them elsewhere
- [x] Remove or downgrade any task that proves redundant once stronger real-path coverage exists

---

## 12. Completion criteria

- [x] Every new test added here exercises meaningful production logic in the actual unit under test
- [x] High-value boundary modules have direct tests instead of relying only on indirect screen or integration coverage
- [x] No new demo-only or mock-only suites are introduced
- [x] Command names, protocol field names, and framing behavior are protected by explicit regression tests
- [x] Frontend storage and shell helpers are covered for valid, invalid, and mixed-validity inputs
- [x] Rust app-state tests cover truthful runtime behavior without going through fabricated demo assumptions
- [x] Lint, clippy, and the full test suites pass after the new coverage lands
- [x] Any intentionally deferred low-value coverage is left documented explicitly rather than forgotten
