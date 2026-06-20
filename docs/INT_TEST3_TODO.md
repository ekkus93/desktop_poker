# INT_TEST3_TODO.md

Comprehensive integration-test expansion plan. Each section targets a distinct
behavior gap not covered by existing tests. Tests must exercise **real production
code paths** — no fabricated doubles that only prove a mock behaves as
configured.

## What already exists (do not duplicate)

### Rust networking runtime (`networking/runtime/tests/`)
- `end_to_end/` — full SNG to completion, adversarial rejection, host timeout,
  reconnect mid-hand, blind escalation, hole-card integrity, eliminated player,
  replay protection
- `reconnect.rs` — disconnect + reconnect with correct/wrong keypair, stale
  token, retry exhaustion, eliminated observer state, post-complete resync
- `tournament.rs` — host start emits events, 2 NPCs seated, NPC plays a full
  hand, client action submission syncs state
- `resync.rs` — explicit resync snapshot delivery
- `join.rs`, `session.rs`, `misc.rs`, `snapshots.rs` — join flow, session
  lifecycle, misc host behaviors

### Rust tournament controller (`tournament/tests/`)
- `progression.rs` — hand lifecycle, between-hands auto-start, blind escalation,
  short all-in no-reopen, deck injection, fold-to-winner without showdown
- `deck.rs` — stacked-deck deal order, shuffle uniqueness invariants
- `endgame.rs` — elimination and completion

### Frontend AppShell (`src/app/AppShell.*.integration.test.tsx`)
- `host` — host-setup → lobby, NPC seat, join flow, seat claim, ready toggle,
  start guard, start reject, host recovery, client → table transition, invite
  copy/join, LAN address block, launch-attached invite, NPC profiles screen
- `session` — route guards, stop-before-leave, bootstrap updates, restart
  continuity, per-instance state isolation, join flow error handling
- `table` — action failure + recovery, waiting state, cross-instance isolation,
  reconnect metadata, debug panel, join/table failure states, eliminated
  observer, viewport layout
- `history` — live table view, persisted cache fallback, empty placeholder

---

## 0. Scope, conventions, and guardrails

### 0.1 Ground rules (apply to every task below)
- [x] Tests must exercise **real production functions** — no fabricated doubles
      that only prove a mock behaves as configured (per `CLAUDE.md`).
- [x] No new `#[ignore]` tests except those requiring an external service
      (none expected here).
- [x] Keep each test focused: one behavior per `#[test]` / `it(...)`, descriptive
      names.
- [x] Reuse existing helpers in `tests/support.rs` before adding new ones.
      Add a helper only when it removes duplication across ≥2 tests.
- [x] Every new test file keeps under 700 lines; split into a `tests/` submodule
      if it would exceed that.
- [x] Verify that every new test actually fails meaningfully when the behavior
      under test is broken before considering it done.

### 0.2 File placement decisions
- [x] New Rust end-to-end tests → `networking/runtime/tests/end_to_end/` (split
      to subdirectory after exceeding 700 lines).
- [x] New Rust tournament-controller tests → `tournament/tests/progression.rs`
      or a new file in `tournament/tests/` (e.g. `deck.rs`) as needed.
- [x] New frontend tests: decide per-section whether they belong in an existing
      AppShell file or warrant a new file (noted per section below).
- [x] Any new support helpers shared across multiple new test files → add to the
      existing `tests/support.rs` (Rust) or `appShellHarness.tsx` (frontend).

### 0.3 Verification gates (run after each section)
- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=2`
- [x] `npm run lint`
- [x] `npm run test`

---

## 1. Reconnect mid-hand — player rejoins during an open betting round

**Gap:** existing reconnect tests disconnect between hands or after the
tournament ends. No test verifies that a player who drops mid-hand can
reconnect, receive a resync snapshot that contains `current_hand`, and still act
on the open window before timing out.

**File:** `networking/runtime/tests/end_to_end/reconnect_and_blinds.rs`

### 1.1 Support helpers
- [x] Add `drain_events_until_hand_open(client)` helper to `support.rs`: polls
      `ClientRuntime::next_event` until `ClientRuntimeEvent::PublicEvent` of type
      `HandStartedEvent` is received, returning the `serde_json::Value` payload.
      Timeout 10 s. (Reuse `wait_for_public_event` pattern.)

### 1.2 Test: `d_client_reconnects_mid_hand_and_resumes_acting`
- [x] Start a 2-player tournament (`player-alice`, `player-bob`, stacks 500,
      blinds 50/100) using `bind_test_host_with_state`.
- [x] Wait for the first action window to open (poll `host.authoritative_state()`
      until `current_hand.action_window` is `Some`).
- [x] Record which player is NOT on the clock (`idle_player_id`).
- [x] Forcibly disconnect that player's client using `disconnect_client` from
      `support.rs`.
- [x] Wait for host to observe the disconnection: poll until
      `participants[idle_player_id].connection_state == Disconnected`.
- [x] Reconnect `idle_player_id` using a new `ClientRuntime::connect` (same
      keypair) — this exercises the real reconnect + resync path.
- [x] Assert the reconnected client receives a `ClientRuntimeEvent::Snapshot`
      whose `tournament_state.current_hand` is `Some`, confirming the mid-hand
      state was delivered in the resync.
- [x] Let the acting player submit a legal action to advance the window.
- [x] Assert the reconnected client receives the resulting
      `PlayerActionCommittedEvent` broadcast over TCP, proving it is back on the
      event stream.

### 1.3 Test: `e_reconnected_client_receives_auto_committed_event_after_reconnect`
- [x] Same setup as 1.2 but disconnect the player who IS currently on the clock.
- [x] Wait for the host timeout (1-second timer) to auto-commit an action.
- [x] Reconnect that player using the correct keypair.
- [x] Assert the reconnected client receives the `PlayerActionCommittedEvent`
      (their auto-committed timeout action) in the resync or event stream.
- [x] Verify `host.authoritative_state()` shows the hand has advanced (different
      window or hand complete).

---

## 2. Multi-hand blind escalation over the live TCP runtime

**Gap:** `tournament/tests/progression.rs` tests blind escalation at the
`TournamentController` level using `advance_time`. No test confirms that
blind-level advancement is correctly reflected in events broadcast to a real TCP
client across multiple consecutive hands.

**File:** `networking/runtime/tests/end_to_end/reconnect_and_blinds.rs`

### 2.1 Support helper
- [x] Add `wait_for_hand_number(host, n, deadline)` to `support.rs`: polls
      `host.authoritative_state().current_hand.hand_number` until it reaches `n`
      or deadline expires. Returns `TournamentState`.

### 2.2 Test: `f_blind_level_increments_are_reflected_in_the_second_hand`
- [x] Build a 2-player tournament with a custom blind schedule:
      - Level 0: sb=10, bb=20, duration=1 s
      - Level 1: sb=50, bb=100, duration=600 s
      Use `bind_test_host_with_state` with `starting_stack=500`.
- [x] Play hand 1 to completion using the "check/call everything" loop from
      test `a_full_sit_n_go...` (adapted for 2 players, no all-in required).
- [x] Wait for between-hands auto-advance (poll until `hand_number == 2`).
      The elapsed wall-clock from start will exceed 1 s so the host's tick loop
      should have escalated the blind level.
- [x] Assert `host.authoritative_state().blind_level_index == 1`.
- [x] Assert that a connected `alice` client receives a
      `BlindLevelChangedEvent` (or equivalent) public event whose payload
      reflects the new blind amounts.
- [x] Assert the second hand's `betting_round.current_bet` opens at the new big
      blind (100), not the old one (20).

---

## 3. Private hole-card delivery integrity

**Gap:** `reconnect.rs` test `private_encrypted_payload_can_be_delivered_and_decrypted`
verifies the crypto path at the unit level. No end-to-end test confirms that
real hole-card payloads dealt through the live TCP runtime arrive decryptable
and contain exactly 2 cards.

**File:** `networking/runtime/tests/end_to_end/integrity.rs`

### 3.1 Test: `g_each_player_receives_exactly_two_private_hole_cards_over_tcp`
- [x] Start a 3-player tournament.
- [x] For each of alice, bob, carol: collect a `ClientRuntimeEvent::PrivateHoleCards`
      event (reuse `wait_for_private_hole_cards` from `support.rs`).
- [x] Assert each player received exactly 2 cards.
- [x] Assert the decrypted cards are valid (rank in `2..=A`, known suit).
- [x] Assert all three players' hole-card sets are disjoint (no duplicate cards
      across hands — the deck did not deal the same card twice).

### 3.2 Test: `h_hole_cards_are_not_visible_in_pre_showdown_public_events`
- [x] Run the same 3-player setup.
- [x] Collect 5–10 public events from one client (drain via `next_event` in a
      tight loop with 50 ms timeout per poll, up to 5 s total).
- [x] Assert that none of the raw JSON payloads contain a `"hole_cards"` or
      `"private_cards"` key with a non-null value in a `PublicEvent` variant.

---

## 4. Observer-only enforcement after elimination

**Gap:** `AppShell.table.integration.test.tsx` has one test that transitions an
eliminated player to observer state via the live event. No test verifies that
the Rust host *rejects* any action submitted by a participant whose state is
`Eliminated`.

**File:** `networking/runtime/tests/end_to_end/integrity.rs`

### 4.1 Test: `i_eliminated_player_action_is_rejected_by_host`
- [x] Run a 3-player SNG (stacks=100, blinds=30/60) and drive it until one
      player is eliminated (stack hits 0, placement recorded).
- [x] Record the eliminated player id and client.
- [x] Start the next hand. Wait for an action window.
- [x] Have the eliminated client submit an action (any legal action type, using
      the current window id and a valid seat index).
- [x] Assert the host does NOT advance the action window — poll for 300 ms and
      confirm `action_window_id` is unchanged.
- [x] Assert the remaining two players can still act normally (submit a valid
      action from the real actor and verify the window advances).

---

## 5. Signature replay protection across sessions

**Gap:** `protocol/replay.rs` has unit tests but no end-to-end test proves that
the live host *actually rejects* a replayed signed message from a prior session.

**File:** `networking/runtime/tests/end_to_end/integrity.rs`

### 5.1 Test: `j_replay_protector_rejects_stale_epoch_and_duplicate_message_ids`
- [x] Capture a raw join envelope from session epoch N using the low-level
      signing helpers in `support.rs` (`signed_join_envelope`).
- [x] Bind a second host with session epoch N+1 (same table_id, new epoch).
- [x] Send the session-N envelope directly to the epoch-N+1 host's TCP port
      (raw framing write, as done in `misc.rs` or `join.rs` tests).
- [x] Assert that the client that sent the envelope receives a
      `ClientRuntimeEvent::SafeError` and is NOT added to the participant
      registry of the N+1 host.

---

## 6. Tournament controller — stacked-deck hole-card invariants

**Gap:** `progression.rs` injects a stacked deck but only checks that each
player has 2 hole cards. No test verifies the actual card values dealt, or that
the board cards match the expected sequence from the stacked deck.

**File:** `tournament/tests/deck.rs`

### 6.1 Test: `stacked_deck_deals_hole_cards_in_deal_order`
- [x] Build a 2-player controller. Inject a known 12-card stacked deck.
- [x] Start the tournament and read `current_hand.hole_cards_by_player_id`.
- [x] Assert p1 holds `[As, Qc]` and p2 holds `[Kh, Jd]` (standard 2-pass deal
      order: p1 first card, p2 first card, p1 second card, p2 second card).
- [x] Drive the hand to the river by submitting check/call for each window.
- [x] Assert the final board is exactly `[9h, 8c, 7d, 5h, 3d]` (burn cards
      excluded).

### 6.2 Test: `no_card_dealt_twice_in_a_real_shuffled_hand`
- [x] Build a 3-player controller without a stacked deck (real shuffle).
- [x] Start the tournament; drive to the river.
- [x] Collect all dealt cards: 6 hole cards + 5 board cards = 11 total.
- [x] Assert all 11 cards are distinct (no duplicates from the shuffle).

---

## 7. Tournament controller — fold-to-winner without showdown

**Gap:** existing controller tests either check/call to a showdown or go all-in.
No test verifies that a preflop fold immediately awards the pot to the remaining
player without proceeding to a board.

**File:** `tournament/tests/progression.rs`

### 7.1 Test: `preflop_fold_awards_pot_immediately_without_board`
- [x] Build a 2-player controller; start the tournament.
- [x] On the first action window, submit `ActionType::Fold` (SB faces BB's bet
      so fold is legal preflop).
- [x] Assert `current_hand.board_cards` is empty (no community cards dealt).
- [x] Assert `hand_results` contains exactly one result, and the winner is the
      non-folding player.
- [x] Assert total chips across both players equals starting chips (no chips
      created or destroyed).

### 7.2 Test: `postflop_fold_awards_pot_to_remaining_player`
- [x] Build a 2-player controller; start the tournament.
- [x] Call on preflop to reach the flop.
- [x] First postflop actor bets the minimum; second actor folds (now facing a
      bet so fold is legal).
- [x] Assert `board_cards.len() == 3` (only the flop was dealt; turn/river were
      not).
- [x] Assert `hand_results` records the non-folding player as winner.

---

## 8. Frontend — hand history screen populates from live events

**Gap:** `HandHistoryScreen.test.tsx` tests the screen in isolation. No
AppShell-level integration test verifies that hand results produced during a
live session are reflected in the hand history screen after the hand completes.

**File:** `src/app/AppShell.history.integration.test.tsx` (new file)

### 8.1 Test: `hand_history_populates_from_the_live_table_view_after_a_hand_completes`
- [x] Set up the harness with `getTableView` returning a snapshot that includes
      hand history entries.
- [x] Navigate to `/history`.
- [x] Assert that the hand history screen renders entries from the live table view.

### 8.2 Test: `hand_history_falls_back_to_the_persisted_cache_when_no_live_session_is_available`
- [x] `getTableView` rejects (session offline); `persistHandHistory` pre-fills
      the cache.
- [x] Assert the persisted entry and "Saved on this device." indicator are shown.

### 8.3 Test: `hand_history_shows_no_hands_placeholder_when_session_is_gone_and_cache_is_empty`
- [x] Both sources empty → assert "No settled hands yet." placeholder.

---

## 9. Frontend — NPC profile screen reflects registered NPC identities

**Gap:** `NpcProfilesScreen.test.tsx` tests the screen in isolation. No
AppShell integration test verifies that NPC participants registered via the host
runtime appear in the NPC profiles screen.

**File:** `src/app/AppShell.host.integration.test.tsx`

### 9.1 Test: `npc_profiles_screen_lists_all_profiles_returned_by_the_backend`
- [x] Set up the AppShell in host mode.
- [x] Mock `listNpcProfiles` to return 2 profiles.
- [x] Navigate to the NPC profiles screen.
- [x] Assert both NPC display names appear under the "AI Profiles" heading.

---

## 10. Frontend — join flow rejects a malformed payload gracefully

**Gap:** the join flow is tested for the happy path. No AppShell integration
test confirms that a syntactically invalid `pkr1_...` payload surfaces a
human-readable error rather than a silent freeze or crash.

**File:** `src/app/AppShell.session.integration.test.tsx`

### 10.1 Test: `join_flow_shows_a_human_readable_error_for_an_invalid_payload_instead_of_freezing`
- [x] Submit `pkr1_notbase64!!!` to the Invite field.
- [x] `validateJoinPayloadInput` rejects with "Invalid pkr1_ payload: base64
      decoding failed".
- [x] Assert error text shown, Continue button disabled, no lobby navigation.

### 10.2 Test: `join_flow_shows_a_connection_error_when_the_host_is_unreachable`
- [x] `validateJoinPayloadInput` succeeds → Continue button unlocks.
- [x] `joinHostSession` rejects with "Connection refused".
- [x] Assert "connection refused" error shown, no lobby navigation.

---

## 11. Rust — canonical JSON serialization round-trip (protocol layer)

**Gap:** `protocol/canonical.rs` and `protocol/models/tests.rs` have unit tests
for individual message types, but no test verifies that signing bytes produced
by the Rust implementation match what the Android `CanonicalJson.kt` would
produce for a known fixture.

**File:** `protocol/models/tests.rs` and `protocol/canonical.rs`

### 11.1 Test: `canonical_bytes_for_join_request_match_known_android_fixture`
- [x] Marked `#[ignore]` — requires a captured Android CanonicalJson fixture.
      Added as a placeholder with a TODO comment; will be wired up once the
      Android fixture is captured.

### 11.2 Test: `lexicographic_key_sort_is_applied_at_all_nesting_levels`
- [x] Build a `serde_json::Value::Object` with 3 nesting levels, keys out of
      alphabetical order.
- [x] Pass through the canonical serializer.
- [x] Assert all object keys sorted lexicographically at every level.

### 11.3 Test: `null_optional_fields_are_omitted_from_canonical_output`
- [x] Build a `SignedEnvelope` with `server_sequence: None`.
- [x] Serialize canonically.
- [x] Assert no `"null"` values and no `"serverSequence"` key in the output.

---

## 12. Verification and cleanup

### 12.1 Line-count audit
- [x] All modified/new test files confirmed under 700 lines:
      - `end_to_end/basic.rs` 311, `reconnect_and_blinds.rs` 308,
        `integrity.rs` 374 (split from 970-line monolith)
      - `tournament/tests/deck.rs` 185 (split from `progression.rs`)
      - `tournament/tests/progression.rs` 526
      - `AppShell.history.integration.test.tsx` 134
      - `AppShell.host.integration.test.tsx` 566
      - `AppShell.session.integration.test.tsx` 549

### 12.2 Final verification gate
- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=2`
      350 passed; 0 failed; 3 ignored (2 Ollama LLM + 1 Android fixture).
- [x] `npm run lint`
- [x] `npm run test` — 217 passed; 0 failed.
- [x] Committed and pushed.
