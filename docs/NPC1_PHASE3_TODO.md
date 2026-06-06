# NPC Phase 3 — Advanced Profiles with Opponent Modelling TODO

This backlog implements Phase 3 of the NPC spec: giving LLM-driven NPCs memory of the
current session and awareness of each opponent's tendencies. The NPC runner accumulates
a per-session history as hands complete and injects that context into every LLM prompt.

See `docs/NPC1_SPEC.md` for the full design and `docs/NPC1_PHASE2_TODO.md` for Phase 2
(already complete).

Status legend:
- [ ] not started
- [~] in progress
- [x] done

---

## Part 1: Rust — per-hand action log

Phase 2's `GameStateSnapshot.street_history` is always empty because the domain
`HandState` does not expose per-action history within a betting street. This part
adds an action log that the runner maintains across the lifetime of a hand.

**Files:** `src-tauri/src/npc/hand_log.rs` (new), `src-tauri/src/npc/mod.rs`

### 1.1 Design the `HandActionRecord` type

- [x] Create `src-tauri/src/npc/hand_log.rs`.
- [x] Declare it in `src-tauri/src/npc/mod.rs` as `pub mod hand_log`.
- [x] Define:
  ```rust
  #[derive(Clone, Debug)]
  pub struct HandActionRecord {
      pub hand_number: u32,
      pub street: StreetPhase,
      pub player_id: String,
      pub action_type: ActionType,
      pub amount: Option<u32>,        // chips committed (None for fold/check)
      pub is_voluntary: bool,         // false only for forced blinds/antes
  }
  ```

### 1.2 Define `HandLog`

- [x] Define:
  ```rust
  pub struct HandLog {
      /// All actions in order across all streets for the current hand.
      pub actions: Vec<HandActionRecord>,
      pub hand_number: u32,
  }
  impl HandLog {
      pub fn new(hand_number: u32) -> Self
      /// Filter to actions for a given street.
      pub fn actions_on_street(&self, street: StreetPhase) -> Vec<&HandActionRecord>
      /// Filter to actions by a given player_id.
      pub fn actions_by(&self, player_id: &str) -> Vec<&HandActionRecord>
  }
  ```

### 1.3 Action log unit tests

- [x] Test `actions_on_street` filters correctly.
- [x] Test `actions_by` filters correctly.
- [x] Test that actions are appended in order.

---

## Part 2: Rust — session history tracker

A per-NPC struct that accumulates summarised events across hands throughout the
tournament session and can render a concise text block for LLM prompts.

**Files:** `src-tauri/src/npc/session_history.rs` (new), `src-tauri/src/npc/mod.rs`

### 2.1 Design `HandSummary`

- [x] Create `src-tauri/src/npc/session_history.rs`.
- [x] Declare it in `src-tauri/src/npc/mod.rs` as `pub mod session_history`.
- [x] Define:
  ```rust
  /// A condensed record of a completed hand relevant to a single NPC.
  #[derive(Clone, Debug)]
  pub struct HandSummary {
      pub hand_number: u32,
      pub npc_won: bool,
      pub pot_size: u32,               // total chips in the pot
      pub net_chips: i32,              // chips won (positive) or lost (negative)
      pub npc_went_to_showdown: bool,
      pub npc_bluffed: bool,           // NPC bet/raised with no pair and no draw
      pub npc_bluff_caught: bool,      // NPC bluffed and was called at showdown
      pub opponent_ids_in_hand: Vec<String>,
  }
  ```

### 2.2 Define `NpcSessionHistory`

- [x] Define:
  ```rust
  pub struct NpcSessionHistory {
      pub npc_player_id: String,
      summaries: Vec<HandSummary>,
  }
  impl NpcSessionHistory {
      pub fn new(npc_player_id: String) -> Self
      pub fn record_hand(&mut self, summary: HandSummary)
      /// Consecutive hands lost (ending streak) — for tilt detection.
      pub fn consecutive_losses(&self) -> u32
      /// Consecutive hands won (ending streak).
      pub fn consecutive_wins(&self) -> u32
      /// Total hands played.
      pub fn hands_played(&self) -> usize
      /// Net chip change across all hands.
      pub fn total_net_chips(&self) -> i32
      /// Render a human-readable summary block for LLM context (max ~400 tokens).
      pub fn render_context(&self) -> String
  }
  ```
- [x] `render_context` must:
  - State the session hand count and overall chip trajectory (up/down N chips).
  - Note the current loss or win streak if ≥ 2 hands.
  - List the most recent 5 key hands (big pot won/lost, bluff caught/succeeded).
  - Truncate gracefully when the session is long (keep most recent hands).

### 2.3 Session history unit tests

- [x] Test `consecutive_losses` after a losing streak.
- [x] Test `consecutive_wins` after a winning streak.
- [x] Test `consecutive_losses` resets to 0 after a win.
- [x] Test `render_context` output contains expected hand count and chip trajectory.
- [x] Test `render_context` truncates to ≤ 5 key hand descriptions when session is long.
- [x] Test that `render_context` mentions current streak when ≥ 2 consecutive losses.

---

## Part 3: Rust — opponent stats tracker

A per-NPC struct that aggregates observed opponent behaviours into statistics usable
for opponent modelling context in the LLM prompt.

**Files:** `src-tauri/src/npc/opponent_stats.rs` (new), `src-tauri/src/npc/mod.rs`

### 3.1 Define `OpponentStats`

- [x] Create `src-tauri/src/npc/opponent_stats.rs`.
- [x] Declare it in `src-tauri/src/npc/mod.rs` as `pub mod opponent_stats`.
- [x] Define:
  ```rust
  /// Aggregate stats observed for a single opponent over the current session.
  #[derive(Clone, Debug, Default)]
  pub struct OpponentStats {
      pub player_id: String,
      pub display_name: String,
      pub hands_observed: u32,
      /// Voluntarily Put In Pot: fraction of hands the player voluntarily entered.
      pub vpip_count: u32,             // hands where they put in a preflop voluntary action
      pub preflop_raise_count: u32,    // hands where they raised preflop
      pub aggression_bets: u32,        // post-flop bets + raises
      pub aggression_calls: u32,       // post-flop calls
      pub showdowns_seen: u32,
      pub showdowns_won: u32,
      pub times_bluffed_caught: u32,   // went to showdown with no pair, no draw, lost
  }
  impl OpponentStats {
      /// VPIP as a percentage (0–100).
      pub fn vpip_pct(&self) -> f32
      /// Preflop raise frequency as a percentage (0–100).
      pub fn pfr_pct(&self) -> f32
      /// Aggression factor: bets+raises / calls (∞ when calls == 0).
      pub fn aggression_factor(&self) -> f32
      /// Showdown win rate as a percentage (0–100).
      pub fn wtsd_win_pct(&self) -> f32
      /// One-line summary for prompt injection.
      pub fn render_line(&self) -> String
  }
  ```

### 3.2 Define `OpponentStatsTable`

- [x] Define:
  ```rust
  pub struct OpponentStatsTable {
      stats: BTreeMap<String, OpponentStats>,
  }
  impl OpponentStatsTable {
      pub fn new() -> Self
      pub fn update_from_hand(&mut self, hand_log: &HandLog, result: &HandResult,
                               display_names: &BTreeMap<String, String>)
      pub fn get(&self, player_id: &str) -> Option<&OpponentStats>
      /// Render a stats block for all tracked opponents (max ~300 tokens).
      pub fn render_context(&self) -> String
  }
  ```
- [x] `update_from_hand` must:
  - Increment `hands_observed` for every opponent in the hand.
  - Set `vpip_count` for opponents who voluntarily entered pre-flop.
  - Set `preflop_raise_count` for opponents who raised pre-flop.
  - Accumulate post-flop `aggression_bets` and `aggression_calls`.
  - Accumulate `showdowns_seen` and `showdowns_won` from `HandResult`.
  - Set `times_bluffed_caught` when revealed hand is no pair / no draw at showdown loss.
- [x] `render_context` must produce ≤ 5 lines for opponents with ≥ 3 hands observed.
  Opponents with fewer than 3 hands are omitted (insufficient sample).

### 3.3 Opponent stats unit tests

- [x] Test `vpip_pct` rounds correctly.
- [x] Test `aggression_factor` returns correct value and handles divide-by-zero.
- [x] Test `update_from_hand` increments counters correctly for a single hand.
- [x] Test `render_context` omits opponents with < 3 hands observed.
- [x] Test `render_context` output for an opponent with known stats contains their name and
  key metrics.

---

## Part 4: Rust — tilt state detection

Encapsulate the tilt logic so it can be queried at prompt-build time and referenced
from profiles.

**Files:** `src-tauri/src/npc/tilt.rs` (new), `src-tauri/src/npc/mod.rs`

### 4.1 Define `TiltState`

- [x] Create `src-tauri/src/npc/tilt.rs`.
- [x] Declare it in `src-tauri/src/npc/mod.rs` as `pub mod tilt`.
- [x] Define:
  ```rust
  #[derive(Clone, Debug, PartialEq, Eq)]
  pub enum TiltLevel {
      None,
      Mild,   // 2 consecutive losses
      Full,   // 3+ consecutive losses
  }

  pub struct TiltState {
      pub level: TiltLevel,
      pub consecutive_losses: u32,
      pub consecutive_wins: u32,
  }

  impl TiltState {
      pub fn from_history(history: &NpcSessionHistory) -> Self
      pub fn is_tilted(&self) -> bool
      /// Short phrase for prompt injection, e.g. "on a 3-hand losing streak".
      pub fn description(&self) -> Option<String>
  }
  ```

### 4.2 Tilt state unit tests

- [x] Test `TiltLevel::None` when no losses.
- [x] Test `TiltLevel::Mild` at exactly 2 consecutive losses.
- [x] Test `TiltLevel::Full` at 3+ consecutive losses.
- [x] Test `is_tilted` returns `false` for `TiltLevel::None`, `true` otherwise.
- [x] Test `description` returns `None` when not tilted, and a non-empty string when tilted.
- [x] Test that a win after 4 losses resets to `TiltLevel::None`.

---

## Part 5: Rust — extended profile format parsing

Parse the optional `## Opponent tendencies` and `## Tilt behaviour` sections from
profile Markdown. These are additive — existing profiles that lack these sections
continue to work unchanged.

**Files:** `src-tauri/src/npc/profile.rs`

### 5.1 Add optional section fields to `NpcProfile`

- [x] Add to `NpcProfile`:
  ```rust
  pub opponent_tendencies: Option<String>,  // content of ## Opponent tendencies section
  pub tilt_behaviour: Option<String>,       // content of ## Tilt behaviour section
  ```
  (These are `None` for profiles that do not include the sections.)

### 5.2 Update `parse_profile` to extract named sections

- [x] After extracting the `body`, scan it for `## Opponent tendencies` and
  `## Tilt behaviour` H2 headings.
- [x] Extract the content between the heading and the next `##` heading (or end of body).
- [x] Strip the extracted section from the general `description` body text.
- [x] If neither section is present, `description` is unchanged from Phase 2 behaviour.

### 5.3 Profile parsing unit tests

- [x] Test that a profile without either section parses with both fields as `None` and
  `description` unchanged.
- [x] Test that `## Opponent tendencies` section content is captured in
  `opponent_tendencies` and removed from `description`.
- [x] Test that `## Tilt behaviour` section content is captured in `tilt_behaviour` and
  removed from `description`.
- [x] Test that both sections can coexist in one profile.
- [x] Test that section extraction is whitespace-tolerant (leading/trailing blank lines).
- [x] Test that unrelated `##` headings in the body are preserved in `description`.

### 5.4 Update bundled starter profiles

- [x] Add an `## Opponent tendencies` section to `aggressive-alice.md` describing how
  Alice adjusts to tight vs. loose opponents.
- [x] Add a `## Tilt behaviour` section to `aggressive-alice.md` describing how Alice
  tilts after losses.
- [x] Add an `## Opponent tendencies` section to `conservative-carlos.md`.
- [x] Add a `## Tilt behaviour` section to `conservative-carlos.md` (Carlos barely tilts
  — tighten further rather than loosen).
- [x] Add an `## Opponent tendencies` section to `balanced-sam.md`.
- [x] Add a `## Tilt behaviour` section to `balanced-sam.md`.

---

## Part 6: Rust — updated prompt assembly

Extend `build_user_message` to inject session history context, opponent stats, and
tilt state when they are available.

**Files:** `src-tauri/src/npc/prompt.rs`

### 6.1 Extend `GameStateSnapshot` with session context

- [x] Add to `GameStateSnapshot`:
  ```rust
  pub session_context: Option<String>,    // rendered NpcSessionHistory::render_context()
  pub opponent_context: Option<String>,   // rendered OpponentStatsTable::render_context()
  pub tilt_description: Option<String>,   // TiltState::description()
  ```

### 6.2 Update `build_user_message`

- [x] Insert the session context block after the profile body and before the game state
  text, under a `## Session history` heading, when `session_context` is `Some`.
- [x] Insert the opponent context block under a `## Opponent tendencies observed` heading,
  when `opponent_context` is `Some`.
- [x] When the profile has an `opponent_tendencies` field set, append it under
  `## Your opponent tendencies` so the LLM can apply it to the observed data.
- [x] Insert `## Current tilt state` with `tilt_description` when the NPC is tilted.
- [x] When the profile has a `tilt_behaviour` field set, append it under
  `## Your tilt behaviour` so the LLM can apply its own tilt rules.

### 6.3 Prompt length guard

- [x] Measure the total approximate token count (characters / 4 as a rough estimate).
- [x] If the assembled message exceeds 6 000 tokens (24 000 chars), truncate the
  `session_context` to the most recent 3 hand summaries and drop the opponent context.
- [x] If still over limit, drop all context except `tilt_description`.
- [x] Write a helper `fn count_approx_tokens(s: &str) -> usize` returning `s.len() / 4`.

### 6.4 Prompt assembly unit tests

- [x] Test that `build_user_message` with all context fields `None` produces the same
  output as Phase 2.
- [x] Test that session context block appears in the output when `session_context` is set.
- [x] Test that opponent context block appears when `opponent_context` is set.
- [x] Test that tilt description appears when `tilt_description` is set.
- [x] Test that profile `opponent_tendencies` and `tilt_behaviour` sections are injected.
- [x] Test that a message exceeding the token limit is truncated and stays under the cap.
- [x] Test that truncation preserves tilt state even when session + opponent context are
  dropped.

---

## Part 7: Rust — integrate history tracking into the NPC runner

Wire the new structs into the runner so they are populated during play.

**Files:** `src-tauri/src/npc/runner.rs`

### 7.1 Add per-NPC tracker state to the runner loop

- [x] Introduce a `RunnerState` struct (local to the runner loop) holding:
  ```rust
  struct RunnerState {
      hand_log: Option<HandLog>,         // log for the hand currently in progress
      session_histories: Vec<NpcSessionHistory>,  // one per NPC in npc_configs
      opponent_stats: OpponentStatsTable,
      last_hand_number: Option<u32>,
  }
  ```
- [x] Initialise `RunnerState` at the start of `npc_runner_loop`, one
  `NpcSessionHistory` per entry in `npc_configs`.

### 7.2 Record each action into the hand log

- [x] After the runner submits an NPC action, append a `HandActionRecord` to
  `hand_log` for that player_id, street, action_type, and amount.
- [x] When a new hand begins (detected by a change in `hand_number`), reset
  `hand_log` to a fresh `HandLog::new(hand_number)`.

### 7.3 Record hand completion into session history

- [x] Detect hand completion by comparing
  `state.hand_results.len()` to `last_hand_number`.
- [x] When a new `HandResult` appears:
  - Build a `HandSummary` for each NPC.
  - Determine `npc_won` from `HandResult.winning_player_ids`.
  - Determine `net_chips` from `HandResult.final_stack_by_player_id` vs. the
    previous stack (track pre-hand stacks in `RunnerState`).
  - Determine `npc_went_to_showdown` from `HandResult.revealed_hands_by_player_id`.
  - Determine `npc_bluffed` and `npc_bluff_caught` from the hand log (NPC bet/raised
    with a weak hand on the river and the result shows they lost at showdown).
  - Call `session_histories[i].record_hand(summary)`.
  - Call `opponent_stats.update_from_hand(&hand_log, &result, &display_names)`.
  - Update `last_hand_number`.

### 7.4 Pass context into `GameStateSnapshot`

- [x] Before calling `choose_llm_action`, compute:
  ```rust
  let tilt = TiltState::from_history(&session_histories[npc_seat]);
  let session_ctx = session_histories[npc_seat].render_context();
  let opp_ctx = opponent_stats.render_context();
  ```
- [x] Populate `snapshot.session_context`, `snapshot.opponent_context`, and
  `snapshot.tilt_description` from the above.

### 7.5 Runner integration tests

- [x] Test that after one completed hand, `session_histories[0].hands_played()` is 1.
- [x] Test that `consecutive_losses` increments correctly across multiple hands.
- [x] Test that `opponent_stats` has entries for every non-NPC player after a hand.
- [x] Test that the `GameStateSnapshot` passed to `choose_llm_action` has non-`None`
  `session_context` after the first completed hand.

---

## Part 8: Rust — display name resolution for opponent stats

The `OpponentStatsTable` needs display names (not just player IDs) to render
readable opponent context for the LLM.

**Files:** `src-tauri/src/npc/runner.rs`, `src-tauri/src/npc/opponent_stats.rs`

### 8.1 Extract display names from `TournamentState`

- [x] In the runner loop, build a `BTreeMap<String, String>` of
  `player_id → display_name` from `state.seats` (using the `participant_id` and the
  matching `TournamentRegistryEntry` display name).
- [x] Pass this map to `opponent_stats.update_from_hand`.

### 8.2 Store display name in `OpponentStats`

- [x] When first creating or updating an `OpponentStats` entry, set `display_name`
  from the map if available; leave as empty string otherwise.

### 8.3 Unit test

- [x] Test that `render_line` uses the display name and not the raw player ID.

---

## Part 9: Rust — updated starter profiles

Update the three bundled starter profiles to exercise the new sections so they
provide richer LLM context out of the box.

**Files:**
- `src-tauri/src/npc/profiles/aggressive-alice.md`
- `src-tauri/src/npc/profiles/conservative-carlos.md`
- `src-tauri/src/npc/profiles/balanced-sam.md`

### 9.1 Aggressive Alice

- [x] Add:
  ```markdown
  ## Opponent tendencies
  - Tight players (VPIP < 20%): bluff river aggressively, they over-fold to pressure.
  - Loose players (VPIP > 50%): tighten value range, extract thin value, never bluff.
  - Aggressive players (AF > 3): trap with strong hands, let them bet into you.
  - Passive players: lead all streets, deny free cards.
  ```
- [x] Add:
  ```markdown
  ## Tilt behaviour
  After losing two or more hands in a row, Alice widens her opening range to any two
  broadway cards and any suited ace. She becomes more likely to triple-barrel bluffs and
  hero-call river bets with medium-strength hands.
  ```

### 9.2 Conservative Carlos

- [x] Add:
  ```markdown
  ## Opponent tendencies
  - Aggressive players: tighten pre-flop to premium hands only; trap with sets and
    two-pair; never bluff.
  - Loose passive players: value-bet relentlessly with two pair or better; never bluff.
  - Tight players: bluff rarely, only on paired boards where Carlos represents trips.
  ```
- [x] Add:
  ```markdown
  ## Tilt behaviour
  Carlos does not tilt in the traditional sense. After two consecutive losses he tightens
  further — playing only AA, KK, and AKs pre-flop — and never bluffs until he wins a
  hand.
  ```

### 9.3 Balanced Sam

- [x] Add:
  ```markdown
  ## Opponent tendencies
  - High VPIP opponents: tighten opening range, widen value range, bluff less.
  - High aggression opponents: call down more, trapping becomes more attractive.
  - Low VPIP opponents: bluff more on later streets, they fold too much.
  - Showdown-heavy opponents: reduce bluff frequency, increase value bet sizing.
  ```
- [x] Add:
  ```markdown
  ## Tilt behaviour
  Sam maintains discipline under pressure. After two consecutive losses Sam reduces
  bluffing frequency by half and focuses on solid value extraction. After a win the
  normal balanced strategy resumes.
  ```

---

## Part 10: Rust — lint, tests, and sign-off

### 10.1 Full test suite

- [x] Run `cargo test --all-targets` — all tests pass.

### 10.2 Clippy

- [x] Run `cargo clippy --all-targets --all-features -- -D warnings` — clean.

### 10.3 Format

- [x] Run `cargo fmt -- --check` — clean.

### 10.4 Commit and push

- [x] Commit all changes with a descriptive message.
- [x] Push to GitHub.

---

## Part 11: Frontend — profile editor UI for new sections

Surface the new profile sections in the `NpcProfilesScreen` editor so users can
see and edit opponent tendency and tilt behaviour text.

**Files:** `src/screens/NpcProfilesScreen.tsx`, `src/screens/NpcProfilesScreen.test.tsx`

### 11.1 Show parsed sections in profile detail view

- [x] Update `NpcProfile` type in `src/api/desktop.ts` to include:
  ```typescript
  opponentTendencies: string | null;
  tiltBehaviour: string | null;
  ```
- [x] In `NpcProfilesScreen`, when a profile is opened for editing, show the raw
  Markdown content in the textarea (unchanged from Phase 2 — the raw file is edited
  in one block).
- [x] Below the editor, show a read-only expandable "Parsed sections" summary that
  displays `opponentTendencies` and `tiltBehaviour` as formatted text when present.
  This helps the user verify that the section headings are correctly formatted.

### 11.2 Profile editor hint text

- [x] Add a collapsed `<details>` block under the profile textarea with the heading
  "Profile format help".
- [x] Inside the block, show a short template with the required frontmatter keys and
  the optional `## Opponent tendencies` and `## Tilt behaviour` section headings.

### 11.3 Frontend tests

- [x] Test that the "Parsed sections" summary appears when a profile with
  `opponentTendencies` set is opened.
- [x] Test that the "Parsed sections" summary is hidden when both optional fields are
  `null`.
- [x] Test that the "Profile format help" details block is present in the editor.

---

## Part 12: Frontend — tilt and session indicators at the table (optional)

The table view can optionally surface tilt state visually. This is cosmetic and
does not affect gameplay correctness.

> **Note:** This part is optional for the MVP of Phase 3. Implement it if the core
> parts (1–10) are complete and time permits.

**Files:** `src/screens/MainTableScreen.tsx` (or a child component)

### 12.1 Backend: expose tilt state in `DebugInspectorState`

- [x] Add `npc_tilt_levels: BTreeMap<String, String>` to `DebugInspectorState`
  in `src-tauri/src/app_state/mod.rs`.
- [x] Populate it from the runner's `RunnerState` when `debugToolsEnabled` is true.
  Keys are NPC player IDs, values are `"none"`, `"mild"`, or `"full"`.

### 12.2 Frontend: show tilt indicator in debug panel

- [x] In `DebugPanel.tsx`, when `npcTiltLevels` is non-empty, show a list of
  `playerID → tiltLevel` entries.
- [x] Only show the section when at least one NPC has `tiltLevel !== "none"`.

### 12.3 Frontend tests

- [x] Test that the tilt section appears in `DebugPanel` when at least one NPC is tilted.
- [x] Test that the tilt section is hidden when all NPCs are at `TiltLevel::None`.

---

## Part 13: Manual QA checklist

These items cannot be automated and must be verified by running the live app.

### 13.1 Session history accumulates correctly

- [ ] Start a session with one LLM NPC using Aggressive Alice.
- [ ] Play at least 5 hands.
- [ ] In a debug build, confirm that the NPC runner logs session history to stderr.
- [ ] Verify the history block in the 6th hand prompt includes recent hand summaries.

### 13.2 Opponent stats appear in prompts after 3 hands

- [ ] After 3 hands, confirm the opponent context block appears in LLM prompts
  (visible in debug stderr logs).
- [ ] Verify the VPIP and aggression numbers are plausible given actual play.

### 13.3 Tilt triggers after consecutive losses

- [ ] Deliberately lose 3 consecutive hands.
- [ ] Confirm the tilt description appears in the NPC's prompt.
- [ ] Win a hand and confirm the tilt state resets.

### 13.4 Extended profile sections parsed correctly

- [ ] Open the Aggressive Alice profile in the NpcProfilesScreen.
- [ ] Confirm the "Parsed sections" summary shows both `opponentTendencies` and
  `tiltBehaviour` text.

### 13.5 Prompt length guard does not truncate under normal play

- [ ] Play 20 hands.
- [ ] Confirm no truncation warnings appear in the debug log during normal play.

### 13.6 Fallback still works when context is unavailable

- [ ] Start a fresh session (no history).
- [ ] Confirm the NPC still makes valid moves on hand 1 when `session_context` is None.
