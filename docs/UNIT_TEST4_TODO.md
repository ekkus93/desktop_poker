# Unit Test 4 TODO — Coverage Gaps

This TODO targets the production-code files identified as having no or partial direct unit-test
coverage. All work is Rust unless noted. Do not rewrite existing tests; do not add tests that
only exercise test doubles. Every test here must run at least one real production code path.

The tasks are grouped by module and ordered from highest-value to lowest. Wire up new test files
in their parent `mod.rs` before writing any tests.

---

## Group 1 — `crates/poker-core/src/tournament/controller_query.rs`

**New file:** `crates/poker-core/src/tournament/tests/query.rs`
**Wire up:** add `mod query;` to `crates/poker-core/src/tournament/tests/mod.rs`
**Import pattern:** mirror existing `tests/progression.rs` (use `super::super::*`, `crate::domain::*`, `crate::engine::Deck`, and the `support::*` helpers)

### T1.1 — `player_order_starting_with` wraps seat indices correctly

- Build a 3-seat active-player list at indices 1, 3, 5.
- Call `player_order_starting_with(3, &players)`.
- Assert result is `[p3, p5, p1]` (rotation from seat 3).
- Repeat starting from seat 1 → `[p1, p3, p5]`.
- Call with `first_seat_index` not in the list → falls back to slot 0 (first sorted entry).
- Call with empty list → returns empty vec.

### T1.2 — `next_active_seat_after` advances and wraps

- Three active seats at 0, 2, 4.
- `next_active_seat_after(0, &players)` → 2.
- `next_active_seat_after(2, &players)` → 4.
- `next_active_seat_after(4, &players)` → 0 (wrap).
- Single-seat list: `next_active_seat_after(0, &[(0, "p1")])` → 0 (sole entry).
- Empty list: returns `Err`.

### T1.3 — `next_dealer_seat_index` — first hand vs. subsequent hand

- With `dealer_button_seat_index = None` and players at seats 0 and 2: returns first active seat (0).
- With `dealer_button_seat_index = Some(0)` and players at seats 0 and 2: returns 2.
- With `dealer_button_seat_index = Some(2)` and players at seats 0 and 2: returns 0 (wrap).

### T1.4 — `advance_blind_levels_if_due` — before, at, and past deadline

- Set `next_blind_deadline_ms = Some(1000)`, `blind_level_index = 0`.
- Call with `now_ms = 999` → level unchanged, deadline unchanged.
- Call with `now_ms = 1000` → level advances to 1, deadline moves forward by the new level duration.
- Set up two expired deadlines; call once → advances two levels (while-loop).
- At the last blind level: deadline is cleared (`None`) once it expires.

### T1.5 — `player_needs_action` — contribution, acted, and participation variants

- Active player, `street_contribution == current_bet`, already in `acted_since_last_full_raise` → `Ok(false)`.
- Active player, `street_contribution < current_bet` → `Ok(true)` (behind, must act).
- Active player, `street_contribution == current_bet`, NOT yet in `acted_since_last_full_raise` → `Ok(true)`.
- AllIn player → `Ok(false)`.
- Folded player → `Ok(false)`.
- Player not in current hand → `Ok(false)`.

### T1.6 — `player_may_raise` — acted set gate

- Active player not in `acted_since_last_full_raise` → `true`.
- Active player already in `acted_since_last_full_raise` → `false`.
- Folded/AllIn player → `false`.

### T1.7 — `players_who_can_act` aggregates correctly

- Two players: one Active/unacted, one AllIn. → Result contains only the Active player.
- All players AllIn → empty vec.

### T1.8 — `remaining_contenders` includes Active and AllIn, excludes Folded/Out

- Build a hand with one Active, one AllIn, one Folded player.
- `remaining_contenders()` returns the Active and AllIn IDs, not the Folded one.

### T1.9 — `active_player_seats` excludes eliminated and zero-stack

- Seat occupied by EliminatedObserver → excluded.
- Seat occupied with `chip_count = Some(0)` → excluded.
- Seat occupied with `chip_count = Some(1500)`, Active participant → included.
- Empty seat → excluded.

### T1.10 — `competitor_count` counts only seated participants

- Participants map contains 3 seated and 1 unseated (admitted but no seat_index).
- `competitor_count()` → 3.

### T1.11 — `assign_markers` and `clear_markers`

- Start with all markers `None`.
- `assign_markers(0, 1, 2)` → seat 0 has Dealer, seat 1 has SmallBlind, seat 2 has BigBlind.
- `clear_markers()` → all markers `None`.
- Heads-up case: same seat can hold both Dealer and SmallBlind — use two separate seats in the test to check each marker is assigned at the right index.

### T1.12 — `sort_placements` orders by place ascending

- Insert placements in order 3, 1, 2.
- `sort_placements()` → places are 1, 2, 3 in order.

### T1.13 — `complete_tournament` sets phase, adds winner, assigns Dealer marker

- Start with one active player at seat 2 not yet in `placements`.
- Call `complete_tournament()`.
- Assert `state.phase == TournamentPhase::Complete`.
- Assert the player is now in `placements` with `place = 1`.
- Assert seat 2 now has `marker = Some(SeatMarker::Dealer)`.
- Call again with the player already in `placements` → placed only once (idempotent).

---

## Group 2 — `crates/poker-core/src/tournament/controller_hand.rs`

**New file:** `crates/poker-core/src/tournament/tests/hand.rs`
**Wire up:** add `mod hand;` to `crates/poker-core/src/tournament/tests/mod.rs`
**Setup helper:** write a `started_two_player_controller()` function in this file (build
`TournamentController` with 2 players, call `start_tournament(0)`; return the controller). Reuse
`support::player()` and `support::sample_config()`.

### T2.1 — `apply_action` — Fold sets participation

- Get the current action window, fold the acting player.
- Assert that player's participation is `Folded`.
- No error returned.

### T2.2 — `apply_action` — Check valid; Check when behind is an error

- Big blind checks when current_bet == 0 → `Ok`.
- Try to check when `street_contribution < current_bet` → `Err` with "cannot check when chips are required to call".

### T2.3 — `apply_action` — Call deducts stack; Call when stack ≤ to_call is an error

- Valid call: player stack > to_call → stack decreases by to_call, pot increases.
- Player with stack ≤ to_call attempts `ActionType::Call` → `Err("call is not legal when the player can only move all-in")`.

### T2.4 — `apply_action` — Raise below minimum is an error; valid raise clears acted set

- Raise to less than `min_full_raise_to` → `Err("raise must satisfy minimum full raise sizing")`.
- Raise exceeding remaining stack → `Err("raise exceeds remaining stack")`.
- Valid raise: `acted_since_last_full_raise` is cleared (all other players must re-act).

### T2.5 — `apply_action` — AllIn commits full stack; triggers full-raise if large enough

- Player goes all-in with stack > current_bet and all-in total ≥ min_full_raise_to → participation becomes AllIn, action reopens (acted set cleared).
- Player goes all-in with stack < min raise increment (short all-in) → participation becomes AllIn, action does NOT reopen.
- Player already at stack=0 attempts AllIn → `Err("player is already all-in")`.

### T2.6 — `commit_timeout` — Check when legal; Fold when check not legal

- With a Check action in the window: `commit_timeout(now_ms)` applies Check, no error.
- With only Fold and Call in the window (no Check): `commit_timeout(now_ms)` applies Fold.

### T2.7 — `process_eliminations` — busted players get EliminatedObserver and placements

- Settle a hand so one player ends up at stack=0.
- Assert that player's `participant.state == EliminatedObserver`.
- Assert their `seat.tournament_state == EliminatedObserver`.
- Assert a `PlacementEntry` was added with a non-None `busted_at_hand_number`.
- Multiple simultaneous eliminations: placement positions are distinct (not the same place twice).

### T2.8 — `handle_full_raise` — clears acted set, sets new increment

- After a full raise with increment 200: `full_raise_increment == 200`, `acted_since_last_full_raise` contains only the raiser.

---

## Group 3 — `crates/poker-core/src/tournament/controller_core.rs`

**Target file:** extend `crates/poker-core/src/tournament/tests/progression.rs` OR create
`crates/poker-core/src/tournament/tests/core.rs`. New file is preferred to keep the existing
progression tests readable.
**Wire up (if new):** add `mod core;` to `tests/mod.rs`.

### T3.1 — `start_tournament` — rejects duplicate start, unready seats, bad count

- Call `start_tournament` on a Running tournament → `Err("tournament has already started")`.
- Build a controller where one seat is not `is_ready` → `Err("tournament start requires every occupied seat to be ready")`.
- Build with only 1 player → constructor rejects it upstream; verify the check.

### T3.2 — `submit_action` — stale window rejected by timestamp

- Get the action window; advance `now_ms` past its `deadline_epoch_ms`.
- Call `submit_action` → `Err("stale action window rejected")` and the hand auto-times out.

### T3.3 — `submit_action` — stale window rejected by ID

- Get the action window, then call `apply_action` on the controller to consume it.
- Attempt to re-submit the original window ID → `Err("stale action window rejected")`.

### T3.4 — `submit_action` — wrong player rejected

- Get the action window for player A; submit with player B's ID → `Err("action rejected: player does not own the action window")`.

### T3.5 — `commit_total_wager` — deducts stack, accumulates pot and contributions

- Player at stack 1500 commits 200 (count_toward_street=true).
- Assert stack → 1300, pot_size += 200, `contributions_by_player_id[player] += 200`.
- Assert `street_contributions_by_player_id[player] == 200`.
- Assert `current_bet == 200` (since it's now the max street contribution).
- Commit 0 → no-op, no error.
- Commit more than stack → `Err`.

### T3.6 — `refresh_betting_round_bounds` — min/max raise reflect current state

- After blinds posted (SB=50, BB=100): `min_raise_to == 200`, `max_raise_to` equals the highest (stack + street_contribution) among active players.
- After all players go all-in: `max_raise_to` reflects all-in totals.

---

## Group 4 — `src-tauri/src/networking/runtime/reconnect.rs`

**Location:** inline `#[cfg(test)]` block at the bottom of `reconnect.rs` itself. These are pure
or near-pure functions; no sockets or runtime infrastructure needed.

### T4.1 — `is_stale_server_sequence` — all boundary cases

| last_seen  | next       | expected |
|------------|------------|----------|
| None       | None       | false    |
| None       | Some(1)    | false    |
| Some(5)    | None       | false    |
| Some(5)    | Some(6)    | false    |
| Some(5)    | Some(5)    | true     |
| Some(5)    | Some(4)    | true     |

Write as a single parameterised test or as individual named cases.

### T4.2 — `reconnect_window_ms` — all four branches

- `EliminatedObserver`, any phase, any hand → 300_000.
- Non-observer, non-Running phase, any hand → 120_000.
- Non-observer, Running, hand active → 30_000.
- Non-observer, Running, no hand → 120_000.

### T4.3 — `is_reconnectable_participant` — valid and invalid states

- States `Seated`, `Active`, `EliminatedObserver`, `Reconnecting`, `Admitted` → `true`.
- State `Removed` → `false`.

### T4.4 — `restore_participant_after_reconnect` — state transitions

- EliminatedObserver participant in any phase: `connection_state` → Connected, `reconnect_expiry_ms` → None, `state` stays EliminatedObserver.
- Seated participant in Running phase: `state` → Active.
- Seated participant in WaitingForPlayers phase: `state` → Seated.
- Participant with `seat_index = None` in Running phase: `state` → Admitted.

### T4.5 — `merge_networking_state` — only networking fields are merged

Build `authoritative_source` and `controller_state` with the same participant but different
values for:
- `reconnect_token`: only the source value should appear in the result.
- `reconnect_expiry_ms`: only the source value should appear.
- `connection_state`: only the source value should appear.
- `admitted_at_ms`: only the source value should appear.
- `state` (ParticipantState) and `seat_index`: must keep the controller-derived value, not overwritten.
- Participant present in source but absent in controller → silently skipped (no panic, no insertion).

### T4.6 — `issue_reconnect_token` — format and uniqueness

- Call once: result is a non-empty string, all characters are base64url-safe (no `+`, `/`, `=`).
- Call twice: both tokens have the same length (32 characters for 24 random bytes).
- Call twice: results are different (probabilistic; assert `token_a != token_b`).

---

## Group 5 — `src-tauri/src/app_state/projection.rs`

**New file:** `src-tauri/src/app_state/tests/projection.rs`
**Wire up:** add `mod projection;` to `src-tauri/src/app_state/tests/mod.rs`
**Note:** `format_phase`, `format_street`, and `format_marker` already have partial spot-checks in
`tests/units.rs`. The new tests must cover the remaining variants and the functions with no
existing coverage at all. Do not duplicate the variants already covered.

### T5.1 — `format_phase` — remaining variants not covered in units.rs

- `ReadyCheck` → "Ready check"
- `Complete` → "Complete"
- `Cancelled` → "Cancelled"

### T5.2 — `format_phase_value` — all five variants (none currently tested)

- WaitingForPlayers → "waitingForPlayers"
- ReadyCheck → "readyCheck"
- Running → "running"
- Complete → "complete"
- Cancelled → "cancelled"

### T5.3 — `format_street` — remaining variants not covered in units.rs

- `Flop` → "Flop"
- `Turn` → "Turn"
- `Showdown` → "Showdown"

### T5.4 — `format_marker` — SmallBlind not covered

- `SmallBlind` → "Small blind"

### T5.5 — `format_connection_state` — all three variants (none currently tested)

- Connected → "Connected"
- Disconnected → "Disconnected"
- Reconnecting → "Reconnecting"

### T5.6 — `format_tournament_seat_state` — all six variants

- Open → "Open"
- Lobby → "Lobby"
- Ready → "Ready"
- Active → "Active"
- EliminatedObserver → "Eliminated observer"
- Closed → "Closed"

### T5.7 — `format_action` — all six variants

- Fold → "Fold"
- Check → "Check"
- Call → "Call"
- Bet → "Bet"
- Raise → "Raise"
- AllIn → "All-in"

### T5.8 — `card_view` — representative rank and suit combinations

Test at least four cards covering all four suit tones and rank label variants:
- Ace of Spades → `label = "Ace of Spades"`, `compact_label = "A♠"`, `tone = "dark"`.
- Two of Hearts → `label = "Two of Hearts"`, `compact_label = "2♥"`, `tone = "red"`.
- Ten of Diamonds → `compact_label = "10♦"`, `tone = "red"`.
- Jack of Clubs → `compact_label = "J♣"`, `tone = "dark"`.

### T5.9 — `status_label_for_seat` — all participation states and None

- `Some(Folded)` → "Folded this hand"
- `Some(AllIn)` → "All-in"
- `Some(EliminatedObserver)` → "Eliminated observer"
- `Some(Out)` → "Out of this hand"
- `Some(Active)` → delegates to `format_tournament_seat_state` (assert non-empty)
- `Some(Waiting)` → delegates to `format_tournament_seat_state`
- `None` → delegates to `format_tournament_seat_state`

### T5.10 — `build_elimination_summary` — all branches

- Phase `WaitingForPlayers` → "Waiting for the first real hand to start."
- Phase `ReadyCheck` → "Waiting for every seated player to be ready."
- Phase `Running`, no hand results → "Table state is live."
- Phase `Running`, one hand result with winner "Alice" winning 200 → "Alice won 200 chip(s)."

### T5.11 — `build_table_standings_for_state` — ranking, sorting, reserved seat excluded

- Two players: Alice with 1200 chips and Bob with 800. Assert Alice is rank 1, Bob is rank 2.
- Tie in chips: sort alphabetically by display_name; lower-alpha name gets rank 1.
- Reserved-player-ID seat is excluded from standings.
- `is_local` flag is true only for the local player's seat.
- `is_observer` flag is true for EliminatedObserver tournament seat state.

### T5.12 — `build_table_history_for_state` — order and structure

- Empty `hand_results` → empty vec returned.
- Two results: most recent result appears first (reverse chronological).
- Result fields: `hand_number`, `pot_total` (sum of pot_summaries), `winning_players` (display names), `board_cards` (via card_view).

### T5.13 — `display_name_for_state` and `display_names_for_state`

- Known player ID → returns display name.
- Unknown player ID → `display_name_for_state` returns `None`; `display_names_for_state` falls back to the player ID string itself.
- Multiple IDs mix of known and unknown → correct mix of display names and raw IDs.

---

## Group 6 — `src-tauri/src/networking/runtime/snapshot.rs`

**New file:** `src-tauri/src/networking/runtime/tests/snapshot_utils.rs`
**Wire up:** add `mod snapshot_utils;` to `src-tauri/src/networking/runtime/tests/mod.rs`
**Imports:** use `super::super::{build_recipient_snapshot_state, public_revealed_hole_cards}` and
`super::support::*`. The existing `tests/snapshots.rs` file covers `build_snapshot_envelope`
integration; these tests target the pure helper functions in that module.

### T6.1 — `public_revealed_hole_cards` — pre-Showdown returns empty

- Build a hand at `HandCyclePhase::AwaitingAction` / `StreetPhase::Flop`.
- Two players with hole cards in `hole_cards_by_player_id`, both Active.
- `public_revealed_hole_cards(&hand)` → empty BTreeMap.

### T6.2 — `public_revealed_hole_cards` — Showdown includes Active/AllIn, excludes Folded/Out/EliminatedObserver

- Build a hand at `HandCyclePhase::Showdown`.
- Player A: Active with hole cards. Player B: Folded with hole cards. Player C: AllIn with hole cards.
- Result includes A and C; B is excluded.
- Repeat with `HandCyclePhase::Settlement` — same inclusion/exclusion logic applies.

### T6.3 — `build_recipient_snapshot_state` — private cards extracted for target, not others

- Build a `TournamentState` with two seated players (A and B) with hole cards in `current_hand`.
- Call `build_recipient_snapshot_state(&state, "player-a")`.
- Assert the returned `private_hole_cards` matches player A's cards.
- Assert the returned `RecipientSnapshotState.current_hand` does not contain player B's private cards in `public_hole_cards_by_player_id` (they are only revealed at Showdown/Settlement).

### T6.4 — `build_recipient_snapshot_state` — stale seat_index is normalized

- Add a participant whose `seat_index` points to a seat occupied by a different participant.
- Call `build_recipient_snapshot_state`.
- The stale participant's `seat_index` in the returned snapshot participants should be `None` (normalized away).

---

## Group 7 — `src-tauri/src/app_state/config.rs`

**New file:** `src-tauri/src/app_state/tests/config.rs`
**Wire up:** add `mod config;` to `src-tauri/src/app_state/tests/mod.rs`
**Note:** `blind_schedule_for_preset` ("fast"/"normal"/"slow") and `join_tokens_are_random_and_url_safe`
are already tested in `tests/units.rs`. These tests cover the missing aliases and the other
functions in this module.

### T7.1 — `blind_schedule_for_preset` — alias presets and unknown preset

- "turbo" → same structure as "fast" (180 s duration, 12 levels).
- "standard" → same structure as "normal" (300 s duration).
- "deep-stack" → same structure as "slow" (480 s duration).
- Unknown string → `Err` containing "unsupported blindPresetId".
- Whitespace-padded string (e.g., `" fast "`) → same result as "fast" (`.trim()` in production code).

### T7.2 — `format_connection_state_value` — all three variants

- Connected → "connected"
- Disconnected → "disconnected"
- Reconnecting → "reconnecting"

### T7.3 — `format_participant_state_value` — all six variants

- Admitted → "admitted"
- Seated → "seated"
- Active → "active"
- Reconnecting → "reconnecting"
- EliminatedObserver → "eliminatedObserver"
- Removed → "removed"

### T7.4 — `format_tournament_phase_value` — all five variants

- WaitingForPlayers → "waitingForPlayers"
- ReadyCheck → "readyCheck"
- Running → "running"
- Complete → "complete"
- Cancelled → "cancelled"

### T7.5 — `active_seat_count_for_state` — counts occupied seats only

- State with 3 occupied seats and 2 empty seats → returns 3.
- All empty → returns 0.

### T7.6 — `build_session_participants` — maps all participant fields

- One participant with known seat_index pointing to a `is_ready = true` seat.
- Assert returned view has `is_ready = true`, correct `connection_state` string, correct `participant_state` string.
- Participant with no seat_index → `is_ready = false`, `seat_index = None`.

### T7.7 — `client_snapshot_state_from_event` — private card merging and seat normalization

- Construct a `SnapshotEvent` with `private_hole_cards` non-empty and `local_player_id = "player-a"`.
- `current_hand.public_hole_cards_by_player_id` has only player B's cards.
- Call `client_snapshot_state_from_event`.
- Assert the returned `TournamentState.current_hand.hole_cards_by_player_id` contains player A's private cards.
- Assert player B's public cards are present unchanged.
- Repeat with empty `private_hole_cards` → player A does not appear in `hole_cards_by_player_id`.
- Verify `seat_index` is normalized: participant whose `seat_index` does not match the occupied
  seat link gets `seat_index = None` in the reconstructed state.

---

## Group 8 — `crates/poker-core/src/domain/projector.rs`

**Location:** inline `#[cfg(test)]` block at the bottom of `projector.rs`.
**Imports:** `use super::*;` and `use crate::domain::*;` are sufficient — all types are in scope.
**Setup helper:** a local `minimal_state(phase: TournamentPhase)` function that returns a valid
`TournamentState` with two participants, two occupied seats, and a matching config. The existing
`validate_tournament_state` is called inside `project()`, so the state must be valid.

### T8.1 — `StateProjector::project` — Removed participant excluded from private states

- Add a participant with `state = ParticipantState::Removed`.
- Call `StateProjector::project`.
- Assert the returned `private_states` BTreeMap does not contain the removed participant's ID.

### T8.2 — `StateProjector::project` — EliminatedObserver gets empty private hole cards

- Start a hand, set hole cards for all participants in `hole_cards_by_player_id`.
- Mark one participant as `EliminatedObserver`.
- Call `StateProjector::project`.
- Assert that participant's `PrivateState.private_hole_cards` is empty.
- Assert that active participants still have their cards.

### T8.3 — `StateProjector::project` — `can_act` flag set only for action window owner

- Build a state with an open action window for player A.
- Call `StateProjector::project`.
- Assert `private_states["player-a"].can_act == true` and `action_window_player_id == Some("player-a")`.
- Assert all other players have `can_act == false`.

### T8.4 — `StateProjector::project` — observer_projection always has empty cards and no action

- Call `StateProjector::project` on any valid state.
- Assert `observer_projection.private_hole_cards` is empty.
- Assert `observer_projection.can_act == false`.
- Assert `observer_projection.action_window_player_id == None`.
- Assert `observer_projection.is_observer == true`.

### T8.5 — `project_public_state` — public fields populated from tournament state

Test via the `public_state` field of the bundle returned by `StateProjector::project`:
- `tournament_name` matches `state.config.tournament_name`.
- `phase` matches `state.phase`.
- `board_cards` is empty when no current hand; non-empty when a hand has board cards.
- `action_window_player_id` is `None` with no current hand; `Some(player_id)` with an open window.
- `blind_level_label` is `Some(...)` when `blind_level_index` is in range; `None` when out of range.

---

## Group 9 — `src-tauri/src/app_state/live_events.rs`

**Target file:** add to `src-tauri/src/app_state/tests/units.rs` (existing file) or create
`tests/live_events.rs` if the additions would make `units.rs` too long.
**Imports:** `use super::super::{parse_protocol_street, apply_private_hole_cards_to_snapshot};`

### T9.1 — `parse_protocol_street` — all five valid values and case-insensitivity

- "PREFLOP" → `Some(StreetPhase::Preflop)`.
- "flop" → `Some(StreetPhase::Flop)` (lowercase; function upper-cases internally).
- "Turn" → `Some(StreetPhase::Turn)` (mixed case).
- "RIVER" → `Some(StreetPhase::River)`.
- "SHOWDOWN" → `Some(StreetPhase::Showdown)`.
- "" (empty) → `None`.
- "DEAL" (unknown) → `None`.
- " FLOP " (padded) → `Some(StreetPhase::Flop)` (trim check).

### T9.2 — `apply_private_hole_cards_to_snapshot` — inserts cards when hand is active

- Build a `TournamentState` with an active `current_hand` and no entry for the recipient in
  `hole_cards_by_player_id`.
- Create a `PrivateHoleCardsEvent` with `recipient_player_id = "player-a"` and two cards.
- Call `apply_private_hole_cards_to_snapshot(&mut state, &event)`.
- Assert `state.current_hand.unwrap().hole_cards_by_player_id["player-a"]` equals the two cards.

### T9.3 — `apply_private_hole_cards_to_snapshot` — no-op when no current hand

- `current_hand = None`.
- Call `apply_private_hole_cards_to_snapshot` — no panic, state unchanged.

---

## Group 10 — `src/screens/mainTableRaise.ts` (TypeScript / Vitest)

**New file:** `src/screens/mainTableRaise.test.ts`
**Imports:** `import { buildQuickSizes, clampRaiseAmount, defaultRaiseAmount, isWithinRaiseBounds } from "./mainTableRaise";`
**Helper:** write a local `sampleTray(overrides?)` that returns a minimal `actionTray` object with
`minRaiseTo = 200`, `maxRaiseTo = 1000`, `potTotal = 400`, `callAmount = 100`, `currentBet = 100`.

### T10.1 — `clampRaiseAmount` — clamps to min, max, and passes through in-range values

- Amount below minRaiseTo → returns minRaiseTo.
- Amount above maxRaiseTo → returns maxRaiseTo.
- Amount within bounds → returned unchanged.
- `minRaiseTo === null || maxRaiseTo === null` → returns amount unchanged (no clamping).

### T10.2 — `buildQuickSizes` — returns four labelled sizes in correct order

- With `minRaiseTo = 200`, `maxRaiseTo = 1000`, `potTotal = 400`:
  - Result has length 4.
  - First entry: `{ label: "Min", amount: 200 }`.
  - Second entry: `{ label: "1/2 Pot", amount: 200 }` (half-pot 200, clamped to min 200).
  - Third entry: `{ label: "Pot", amount: 400 }`.
  - Fourth entry: `{ label: "Max", amount: 1000 }`.
- `actionTray = undefined` → returns `[]`.
- `minRaiseTo === null` → returns `[]`.
- `maxRaiseTo === null` → returns `[]`.
- Half-pot exceeds max (e.g., `potTotal = 2400`, `maxRaiseTo = 1000`): "1/2 Pot" entry clamped to 1000.

### T10.3 — `defaultRaiseAmount` — preference order

- `minRaiseTo = 200`, `maxRaiseTo = 1000` → returns 200 (`minRaiseTo` preferred).
- `minRaiseTo = null`, `maxRaiseTo = 1000` → returns 1000.
- `minRaiseTo = null`, `maxRaiseTo = null`, `currentBet = 50` → returns 50.

### T10.4 — `isWithinRaiseBounds` — boundary conditions

- `amount = 200`, range `[200, 1000]` → `true` (inclusive lower bound).
- `amount = 1000`, range `[200, 1000]` → `true` (inclusive upper bound).
- `amount = 199`, range `[200, 1000]` → `false`.
- `amount = 1001`, range `[200, 1000]` → `false`.
- `minRaiseTo = null` → `false`.
- `maxRaiseTo = null` → `false`.

---

## Implementation notes

- All new test functions must use the naming convention `<what>_<condition>_<expected_outcome>`.
- Do not use `unwrap()` in test assertions where the failure message would be opaque — use
  `.expect("descriptive reason")` or `assert_eq!` / `assert!(...)` directly.
- Tests for `controller_query.rs` and `controller_hand.rs` can share the same setup helper;
  put it in a local `fn` at the top of the test file rather than duplicating.
- Tests for pure formatter functions (`format_phase`, `format_street`, etc.) may be grouped into
  a single test function per formatter since each variant is trivially small.
- Do not add production fallbacks or escape hatches just to make a test compile. If a function
  is hard to reach without a real `TcpStream`, note it and skip rather than adding a mock path.
- After writing all tests in a group, run `cargo test -p poker-core` (Groups 1–3) or
  `cargo test --workspace --all-targets --all-features` (Groups 4–7) to confirm all pass before
  moving to the next group.
