# NPC Phase 1 — Rule-Based AI Players TODO

This backlog implements Phase 1 of the NPC spec: rule-based Aggressive and Conservative
NPC players that run entirely inside the host process. See `docs/NPC1_SPEC.md` for the
full strategy tables and design decisions.

Status legend:
- [ ] not started
- [~] in progress
- [x] done

---

## Part 1: Rust — NPC data model

### 1.1 Create `src-tauri/src/npc/` module

- [ ] Create `src-tauri/src/npc/mod.rs` and declare it in `lib.rs`.
- [ ] Add `pub mod npc;` to `src-tauri/src/lib.rs`.

### 1.2 Define `NpcStyle` enum

**File:** `src-tauri/src/npc/mod.rs`

- [ ] Define:
  ```rust
  #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
  #[serde(rename_all = "camelCase")]
  pub enum NpcStyle {
      Aggressive,
      Conservative,
  }
  ```

### 1.3 Define `NpcConfig` struct

**File:** `src-tauri/src/npc/mod.rs`

- [ ] Define:
  ```rust
  #[derive(Clone, Debug, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct NpcConfig {
      pub display_name: String,
      pub style: NpcStyle,
  }
  ```
- [ ] Implement a helper `NpcConfig::player_id(seat_index: u8) -> String` that returns
  a deterministic player ID like `"npc-seat-{seat_index}"`.

### 1.4 Define `AddNpcPlayersRequest` for the Tauri command

**File:** `src-tauri/src/app_state/mod.rs` (alongside other request types)

- [ ] Define:
  ```rust
  #[derive(Clone, Debug, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct AddNpcPlayersRequest {
      pub npcs: Vec<NpcConfig>,
  }
  ```

### 1.5 Add NPC seat tracking to the host session

**File:** `src-tauri/src/app_state/mod.rs`

- [ ] Add a field `npc_configs: Vec<NpcConfig>` to the host session state struct
  (whichever struct holds the mutable session for a running host).
- [ ] Add a helper `fn is_npc_player_id(player_id: &str) -> bool` that checks whether a
  given player ID matches the NPC ID pattern.
- [ ] Ensure `npc_configs` is cleared when the host session stops.

---

## Part 2: Rust — Pre-flop hand strength

**File:** `src-tauri/src/npc/preflop.rs`

### 2.1 Create `preflop.rs`

- [ ] Create the file and expose `pub fn preflop_hand_tier(hole_cards: &[Card]) -> PreflopTier`.

### 2.2 Define `PreflopTier` enum

- [ ] Define:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
  pub enum PreflopTier {
      Fold,
      Marginal,
      Playable,
      Strong,
      Premium,
  }
  ```

### 2.3 Implement `preflop_hand_tier`

Input: exactly 2 `Card` values. Output: `PreflopTier`.

- [ ] Extract the two ranks and suits; determine whether the hand is suited or offsuit.
- [ ] Map to `Premium`:
  - Pairs: AA, KK, QQ, JJ, TT
  - AK (suited or offsuit), AQs, AQo
- [ ] Map to `Strong`:
  - Pairs: 99, 88, 77
  - AJs, ATs, KQs, KQo
- [ ] Map to `Playable`:
  - Pairs: 66, 55, 44, 33, 22
  - KJs, QJs, JTs, T9s, 98s, 87s, 76s, 65s
  - A9s–A2s (any ace suited below ATs)
  - KTs, QTs, JTs (note: JTs is in Playable, not Strong)
- [ ] Map everything else to `Marginal` if it contains a broadway card (T+) offsuit without
  being in the above categories.
- [ ] Map everything else to `Fold`.

### 2.4 Test `preflop_hand_tier`

- [ ] Test: AA → Premium, KK → Premium.
- [ ] Test: 99 → Strong, AJs → Strong.
- [ ] Test: 65s → Playable, A2s → Playable.
- [ ] Test: 72o → Fold.
- [ ] Test: both suited and offsuit variants for borderline hands.

---

## Part 3: Rust — Post-flop hand strength

**File:** `src-tauri/src/npc/postflop.rs`

### 3.1 Create `postflop.rs`

- [ ] Create the file. Import `Card`, `evaluate_best_holdem_hand`, `HandCategory` from the
  engine module.
- [ ] Expose `pub fn postflop_hand_category(hole_cards: &[Card], board: &[Card]) -> PostflopCategory`.

### 3.2 Define `PostflopCategory` enum

- [ ] Define:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
  pub enum PostflopCategory {
      Weak,
      Draw,
      Medium,
      Strong,
      Monster,
  }
  ```

### 3.3 Implement `postflop_hand_category`

- [ ] Call `evaluate_best_holdem_hand(hole_cards, board)` to get the best made hand.
- [ ] Map `HandCategory` to `PostflopCategory`:
  - `HighCard` / `OnePair` (non-top pair) → `Weak`
  - `OnePair` (top pair) → `Medium`; with top kicker → `Strong`
  - `TwoPair` → `Strong`
  - `ThreeOfAKind` → `Strong`
  - `Straight`, `Flush`, `FullHouse`, `FourOfAKind`, `StraightFlush` → `Monster`
- [ ] Detect flush draws: if hole cards share a suit with 2+ board cards of the same suit,
  and board has fewer than 4 of that suit already complete, flag as draw.
- [ ] Detect open-ended straight draws: if the 5-card set has 4 consecutive ranks with an
  open end, flag as draw.
- [ ] If the made hand is `Weak` but a draw is detected, return `Draw`.
- [ ] If the made hand is `Medium` and a draw is detected, keep `Medium` (made hand wins).

### 3.4 Test `postflop_hand_category`

- [ ] Test: pocket aces on a dry board → `Monster` (if full house/set) or `Strong` (overpair).
- [ ] Test: 72o on unrelated board → `Weak`.
- [ ] Test: suited hole cards with two matching board cards → `Draw`.
- [ ] Test: two pair → `Strong`.
- [ ] Test: set → `Strong`.
- [ ] Test: flush → `Monster`.

---

## Part 4: Rust — NPC action selection engine

**File:** `src-tauri/src/npc/strategy.rs`

### 4.1 Create `strategy.rs`

- [ ] Create the file. Import `NpcStyle`, `PreflopTier`, `PostflopCategory`.
- [ ] Define the public interface:
  ```rust
  pub fn choose_preflop_action(
      style: &NpcStyle,
      tier: PreflopTier,
      position: Position,
      facing_raise: bool,
      raise_count: u8,
      min_raise_to: Option<u32>,
      max_raise_to: Option<u32>,
      call_amount: u32,
      pot_total: u32,
      stack: u32,
      seed: u64,
  ) -> NpcAction
  
  pub fn choose_postflop_action(
      style: &NpcStyle,
      category: PostflopCategory,
      is_aggressor: bool,
      facing_bet: bool,
      facing_bet_fraction: f32,
      min_raise_to: Option<u32>,
      max_raise_to: Option<u32>,
      call_amount: u32,
      pot_total: u32,
      stack: u32,
      seed: u64,
  ) -> NpcAction
  ```

### 4.2 Define `Position` enum

- [ ] Define:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum Position {
      Early,   // Blinds + 1–2 spots after
      Middle,  // 3–5 spots before button
      Late,    // Button and cutoff
  }
  ```
- [ ] Add a helper to derive position from `seat_index`, dealer seat index, and player count.

### 4.3 Define `NpcAction` enum

- [ ] Define:
  ```rust
  #[derive(Clone, Debug)]
  pub enum NpcAction {
      Fold,
      CheckOrCall,
      Raise(u32),  // raise-to amount
  }
  ```

### 4.4 Implement `choose_preflop_action`

Use a seeded PRNG (`SmallRng::seed_from_u64(seed)`) for deviation rolls.

- [ ] **Conservative:**
  - `Premium` → raise 2.5x big blind (or 3-bet if facing raise); 4-bet only with AA/KK.
  - `Strong` → call from late position, fold from early.
  - `Playable` or lower → fold.
  - Deviation (8% chance): treat `Strong` as `Premium`.

- [ ] **Aggressive:**
  - `Premium` → raise 2.5–3x; 3-bet/4-bet aggressively.
  - `Strong` → raise from any position; call 3-bets.
  - `Playable` → open-raise or call from late position; fold to 3-bet unless strong.
  - `Marginal` → fold from early, occasional open-limp or fold from late.
  - `Fold` → fold (occasional bluff-raise 5% of the time from late position).
  - Deviation (15% chance): fold one tier higher than normal.

- [ ] Clamp all raise amounts to `[min_raise_to, max_raise_to]`. If `max_raise_to` is `None`,
  use `CheckOrCall` instead.
- [ ] If the computed raise amount ≥ stack, return the all-in amount (use `max_raise_to`).

### 4.5 Implement `choose_postflop_action`

- [ ] Compute bet sizing as a fraction of `pot_total`:
  - `Monster` → 80% of pot (value, possible slow-play 10% of time)
  - `Strong` → 65% of pot
  - `Medium` → 40% of pot
  - `Draw` → 50% of pot semi-bluff (Aggressive) or check (Conservative)
  - `Weak` → check or fold

- [ ] When `facing_bet`:
  - `Monster` → raise (2.5x the facing bet)
  - `Strong` → call (Aggressive may raise)
  - `Medium` → call if `facing_bet_fraction ≤ 0.25` (Conservative) or `≤ 0.50` (Aggressive)
  - `Draw` → call if `facing_bet_fraction ≤ 0.35`; fold otherwise
  - `Weak` → fold if `facing_bet_fraction > 0.10`, check-call otherwise

- [ ] Apply deviation rolls (same percentages as pre-flop).
- [ ] Clamp raise amounts to legal bounds.

### 4.6 Test `choose_preflop_action` and `choose_postflop_action`

- [ ] Conservative Premium facing no raise → returns `Raise`.
- [ ] Conservative Playable → returns `Fold`.
- [ ] Aggressive Marginal from late, no prior raise → non-zero chance of `Raise`.
- [ ] Both styles facing a bet larger than pot → fold with weak hand.
- [ ] Raise amounts are always within `[min_raise_to, max_raise_to]`.
- [ ] With `max_raise_to: None` → never returns `Raise`.

---

## Part 5: Rust — Auto-action integration

**File:** `src-tauri/src/app_state/mod.rs` and `src-tauri/src/npc/runner.rs`

### 5.1 Create `npc/runner.rs`

- [ ] Create `src-tauri/src/npc/runner.rs`.
- [ ] Define `pub fn maybe_schedule_npc_action(...)` — given the current session state and a
  reference to the `DesktopAppState`, spawns a `tokio::task` if the current action window
  belongs to an NPC player ID.

### 5.2 Determine when to trigger NPC auto-action

The host session needs to check for NPC action windows after:
- Tournament start (`host_start_tournament`)
- Any action submission that advances the hand (`submit_table_action`)
- Any hand settlement that opens the next hand's first action window

- [ ] After each of these state transitions, call `maybe_schedule_npc_action` to check
  whether the newly opened action window belongs to an NPC.
- [ ] Pass the NPC config list, current action window player ID, and session handle to the
  runner.

### 5.3 Implement the NPC action runner task

- [ ] In the tokio task:
  1. Sleep for a random duration in `[300ms, 1200ms]` seeded from the action window ID.
  2. Re-read the current action window from the session state (it may have been superseded
     by a timeout or disconnect — abort if the window ID no longer matches).
  3. Read the NPC's hole cards and the current board state from the session snapshot.
  4. Determine whether this is a pre-flop or post-flop decision.
  5. Call `choose_preflop_action` or `choose_postflop_action` with the current game state.
  6. Submit the action through the existing `submit_table_action` path.
  7. After submission, call `maybe_schedule_npc_action` again to chain to the next action
     window if it is also NPC-owned.

### 5.4 Seat seeding logic

- [ ] Derive the `seed: u64` for each action decision from:
  `hash(action_window_id + npc_player_id)` — deterministic, varies per NPC per window.

### 5.5 Handle NPC participation in lobby flow

Before the tournament starts, NPCs must:
- Be added as participants to the session
- Claim their pre-assigned seats
- Set themselves as ready

- [ ] In `host_start_session` (or a new `add_npc_players` command called after it), for each
  NPC config:
  - Register the NPC player ID with the session using the same `claim_seat` path that real
    players use, but called internally (not over TCP).
  - Mark the NPC as ready immediately.
- [ ] Ensure NPC seats count against `activeSeatCount` and decrement `openSeatCount` in the
  lobby status, so real players see the correct open seat count.

### 5.6 NPC display name in participant list

- [ ] NPC participants appear in the lobby with their configured `display_name`.
- [ ] No special marker is required in the protocol (NPCs are opaque to clients).
  The lobby UI may optionally show an "(AI)" suffix — this is a frontend concern (Part 8).

---

## Part 6: Rust — `add_npc_players` Tauri command

**File:** `src-tauri/src/commands.rs`

### 6.1 Add the command

- [ ] Add:
  ```rust
  #[tauri::command]
  pub fn add_npc_players(
      app: AppHandle,
      state: State<'_, DesktopAppState>,
      request: AddNpcPlayersRequest,
  ) -> Result<HostSessionStatus, String> {
      let result = state.add_npc_players(request)?;
      let _ = app.emit("desktop://session-update", ());
      Ok(result)
  }
  ```
- [ ] Register the command in `lib.rs`'s `invoke_handler`.

### 6.2 Implement `DesktopAppState::add_npc_players`

- [ ] The method:
  - Requires an active host session (`start_host_session` must have been called).
  - Validates that the total seats (existing real players + new NPCs) does not exceed
    `max_players`.
  - Validates that the session is still in the `waitingForPlayers` phase (not yet started).
  - For each NPC in the request: calls the internal seat-claim and ready-state paths.
  - Stores the NPC configs in the session for use by the auto-action runner.
  - Returns the updated `HostSessionStatus`.

### 6.3 Update `HostSessionStatus` to expose NPC info

- [ ] Add `npc_count: u8` to `HostSessionStatus` so the frontend can show how many NPC
  seats are active.
- [ ] (Optional) Add `npc_seats: Vec<u8>` listing which seat indices are NPC-controlled.

---

## Part 7: Rust — Tests

### 7.1 Unit tests for pre-flop tier

Already specified in Part 2.4. Add to `src-tauri/src/npc/preflop.rs`:

- [ ] Test module covering at least 10 distinct hand combinations.

### 7.2 Unit tests for post-flop category

Already specified in Part 3.4. Add to `src-tauri/src/npc/postflop.rs`:

- [ ] Test module covering made hands of each category and draw detection.

### 7.3 Unit tests for strategy functions

Already specified in Part 4.6. Add to `src-tauri/src/npc/strategy.rs`:

- [ ] Test module covering both styles, all tiers/categories, and clamp edge cases.

### 7.4 Integration test: NPC completes a hand

**File:** `src-tauri/src/app_state/mod.rs` (test module)

- [ ] Test: start a host session with 1 real player and 1 NPC.
  - Real player claims seat 0, NPC is auto-assigned seat 1.
  - Real player marks ready; NPC is already ready.
  - Start the tournament.
  - Verify the hand progresses to completion (NPC auto-acts on each window).
  - Verify `submit_table_action` from the real player side still works during the hand.

### 7.5 Test: NPC does not act on a stale action window

- [ ] Test: NPC schedules an action, but the window changes (e.g., real player folds out of
  order) before the NPC's sleep elapses. Verify the NPC's action is silently dropped and the
  new window is handled correctly.

---

## Part 8: Frontend — Host setup NPC configuration

**File:** `src/screens/HostTournamentSetupScreen.tsx`

### 8.1 Add NPC count selector to the host setup form

- [ ] Add a `npc_count` field to `HostDraft` (defaults to 0).
- [ ] Add a `npc_style` field to `HostDraft` (`"aggressive"` | `"conservative"`, defaults to
  `"aggressive"`).
- [ ] Add a "NPC players" select (0–up to `maxPlayers - 1` options) to the setup form.
- [ ] Add an "NPC style" radio or select (Aggressive / Conservative) visible only when
  `npc_count > 0`.
- [ ] Persist `npc_count` and `npc_style` in `hostDraft` shell state (they already persist
  via the existing `hostDraft` localStorage path).

### 8.2 Update `startHostSession` flow to add NPCs

**File:** `src/screens/HostTournamentSetupScreen.tsx`

- [ ] After `startHostSession` succeeds and the session is live, if `hostDraft.npcCount > 0`:
  - Build the NPC config array: `npcCount` entries, each with a generated display name
    (e.g., "Bot Alpha", "Bot Beta", "Bot Gamma"…) and the selected style.
  - Call `addNpcPlayers({ npcs: [...] })` via a new API bridge function.
  - Update `hostSession` state with the returned status.
- [ ] Handle errors from `addNpcPlayers` and display them inline.

### 8.3 Add `addNpcPlayers` to the API bridge

**File:** `src/api/desktop.ts`

- [ ] Add types:
  ```typescript
  export type NpcStyle = "aggressive" | "conservative";
  export type NpcConfig = { displayName: string; style: NpcStyle };
  export type AddNpcPlayersRequest = { npcs: NpcConfig[] };
  ```
- [ ] Add `addNpcPlayers(request: AddNpcPlayersRequest): Promise<HostSessionStatus>` using
  `invoke("add_npc_players", { request })`.
- [ ] Add browser-mock stub to `DesktopBrowserMocks`.

### 8.4 NPC name generation helper

**File:** `src/app/shell.ts` or inline in the host screen

- [ ] Generate display names for NPCs from a short list of generic names (e.g., "Bot Alpha",
  "Bot Beta", "Bot Gamma", "Bot Delta", …). Fall back to "Bot {N}" if the index exceeds the
  list.

---

## Part 9: Frontend — Lobby NPC display

**File:** `src/screens/TournamentLobbyScreen.tsx`

### 9.1 Mark NPC seats in the lobby

- [ ] If `hostSession.npcSeats` (or `npcCount`) is available, identify NPC-occupied seats by
  checking whether the participant's `player_id` matches the NPC pattern `"npc-seat-{N}"`.
- [ ] In `buildLiveSeats`, detect NPC participants and set their `detail` field to include
  "(AI)" or "Bot · Always ready".
- [ ] NPC seats show as "Ready" immediately in the seat card (they are always ready).
- [ ] Do not render a "Take seat" button for NPC seats.

### 9.2 NPC seats do not affect the "You: Ready" badge logic

- [ ] Confirm `seatsStillWaiting` only counts non-NPC participants who are not yet ready.
  NPCs are always ready, so they should not inflate the waiting count.

---

## Part 10: Frontend — Tests

### 10.1 HostTournamentSetupScreen NPC controls test

- [ ] Test: when `npcCount > 0`, the NPC style selector appears.
- [ ] Test: when `npcCount === 0`, the style selector is hidden.
- [ ] Test: `addNpcPlayers` is called with the correct payload after `startHostSession`
  succeeds and `npcCount > 0`.

### 10.2 Lobby NPC seat display test

- [ ] Test: NPC participant with player ID `"npc-seat-0"` renders with "(AI)" or "Bot" label.
- [ ] Test: NPC seat shows "Ready" badge without requiring user interaction.
- [ ] Test: no "Take seat" button appears on an NPC-occupied seat.

---

## Part 11: Validation and sign-off

### 11.1 Run all Rust tests

- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

### 11.2 Run all frontend tests

- [ ] `npm run lint`
- [ ] `npm run test`

### 11.3 Manual QA checklist

- [ ] Start a host session with 2 NPC players (1 Aggressive, 1 Conservative) and 1 real player.
- [ ] Verify NPCs appear in the lobby as ready, with distinguishable names.
- [ ] Start the tournament. Verify NPCs act within 1–2 seconds of their turn.
- [ ] Play several hands to confirm NPC actions are legal (no invalid raises).
- [ ] Verify Aggressive NPC bets/raises more frequently than Conservative NPC.
- [ ] Verify NPCs can be eliminated and transition to observer status correctly.
- [ ] Run a host session with 0 NPCs and confirm no behaviour change.
- [ ] Run a host session with `maxPlayers` NPCs (no real players other than the host) and
  confirm the game completes without deadlock.

### 11.4 Commit and push

- [ ] Commit all Rust and frontend changes with message referencing Phase 1.
- [ ] Push to GitHub.

---

## Deliverables

- [ ] `src-tauri/src/npc/` module with pre-flop, post-flop, strategy, and runner submodules
- [ ] `add_npc_players` Tauri command wired into the host session flow
- [ ] NPC auto-action runner integrated with the tournament engine action window loop
- [ ] Host setup form NPC count and style selectors
- [ ] Lobby NPC seat indicators
- [ ] API bridge `addNpcPlayers` function
- [ ] Unit tests: pre-flop tier, post-flop category, strategy selection
- [ ] Integration test: NPC completes a full hand
- [ ] Frontend tests: host setup NPC controls, lobby NPC display
- [ ] Manual QA sign-off
