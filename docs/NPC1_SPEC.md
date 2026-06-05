# NPC / AI Poker Player — Specification

## Overview

This document specifies NPC (non-player character) AI poker players for the Desktop Poker
host. NPCs fill seats at the host's table and play Texas Hold'em automatically. They are
indistinguishable from real players at the protocol level — Android and desktop clients see
them as ordinary participants.

The feature is planned across three phases, each increasing the expressiveness of how an NPC
plays.

---

## Architecture decisions

### NPCs live inside the host process

The host is authoritative. NPCs are **not** separate TCP clients — they are virtual seats
managed entirely within the host's `DesktopAppState`. The host already knows every player's
hole cards (it dealt them), so NPC decision-making has direct access to the private game state
without any network round-trip or encryption overhead.

### NPCs look like real players over the wire

At the protocol level, NPC actions are submitted through the same path as real player actions.
The networking layer emits the same signed public events. Android clients see NPC moves as
ordinary player actions and do not need any changes.

### NPC player IDs are deterministic and host-scoped

Each NPC gets a stable player ID scoped to the session (e.g., `npc-seat-1`). The host tracks
which player IDs are NPC-controlled and routes action windows for those IDs to the NPC engine
instead of waiting for user input.

### Auto-action is host-side only

When the tournament engine opens an action window for an NPC player ID, the host detects this
and schedules an automatic action (with a short simulated-thinking delay). The frontend never
needs to know which seats are NPC-controlled for the game to function, though it may optionally
show an indicator in the lobby UI.

### NPC seat count is set at session start

The host specifies how many NPC seats to add when creating a tournament. NPCs claim their
seats automatically and are always marked ready. Real players fill the remaining open seats.

---

## Phase 1 — Rule-based aggressive / conservative NPCs

**Goal:** Playable NPC opponents with two fixed personalities. No API calls. Fully
deterministic given a random seed per hand.

### Player styles

#### Aggressive
- **Pre-flop:** Enters pots with the top ~55% of starting hands. Open-raises 2.5x the big
  blind; 3-bets frequently; calls 3-bets with strong and medium holdings.
- **Post-flop:** Continuation-bets ~70% of the time regardless of hit. Bets 65–80% of pot
  with strong hands and semi-bluffs. Rarely check-folds with any equity.
- **All-in tendency:** Willing to go all-in with top pair or better, or as a bluff on
  scare-card rivers.

#### Conservative
- **Pre-flop:** Only enters pots with the top ~20% of starting hands (premium pairs AA–TT,
  AKs, AKo, AQs, AQo, AJs). Limps from early position; raises from late.
- **Post-flop:** Bets only with two pair or better. Check-calls with top pair; check-folds
  everything weaker to a bet above 25% of pot.
- **All-in tendency:** Only goes all-in with a set or better.

### Hand strength categories (pre-flop)

| Tier | Hands | Conservative plays? | Aggressive plays? |
|------|-------|---------------------|-------------------|
| Premium | AA, KK, QQ, JJ, TT, AKs, AKo, AQs, AQo | Yes | Yes |
| Strong | 99, 88, 77, AJs, ATs, KQs, KQo | No | Yes |
| Playable | 66–22, KJs, QJs, JTs, T9s, A9s–A2s, suited connectors | No | Yes |
| Marginal | Offsuit broadway non-ace, weak aces | No | Sometimes (position) |
| Fold | Everything else | Always | Sometimes |

### Post-flop hand strength categories

| Category | Definition |
|----------|------------|
| Monster | Straight+, Flush, Full house, Four of a kind, Straight flush |
| Strong | Two pair, Set, Top pair top kicker |
| Medium | Top pair weak kicker, Middle pair, Overpair (below top pair) |
| Draw | Open-ended straight draw (8+ outs), Flush draw (9+ outs) |
| Weak | Gut-shot (4 outs), Bottom pair, No pair no draw |

### Action selection summary

| Situation | Aggressive response | Conservative response |
|-----------|--------------------|-----------------------|
| Monster (post-flop) | Bet 80% pot, raise or jam | Bet 60% pot |
| Strong (post-flop) | Bet 65% pot | Bet 50% pot, call raises |
| Medium (post-flop) | Bet 40% pot or check-call | Check-call ≤25% pot |
| Draw | Semi-bluff bet 50% pot | Check-call small bets only |
| Weak | Check, fold to bets >20% pot | Check, fold to any bet |
| Facing a bet (strong) | Raise 2.5x | Call |
| Facing a bet (medium) | Call or fold | Fold if bet >25% pot |
| Facing a 3-bet (premium) | 4-bet or call | Call or 4-bet AA/KK only |

### Randomisation

To avoid completely predictable play, each NPC decision has a small chance of deviating from
its base strategy. The deviation probability is seeded from the action window ID so it is
deterministic given the same game state. Aggressive deviates to passive ~15% of the time;
Conservative deviates to aggressive ~8% of the time.

### Simulated thinking delay

Before submitting, an NPC waits 300–1200 ms (uniform random within the range, seeded per
action window). This prevents instant-action tells and makes the game feel natural.

### Phase 1 deliverables

- Rust `npc` module: hand strength evaluation, strategy functions, action selection
- Host session integration: NPC seat tracking, auto-action loop via `tokio` task
- Tauri command: `add_npc_players` — specify count and style at session creation time
- Frontend: NPC count/style selector on host setup; NPC seat indicator in lobby
- Tests: unit tests for hand strength categorisation and action selection; integration test
  for full NPC hand completion

---

## Phase 2 — LLM-powered profile files

**Goal:** Replace hardcoded strategies with plain-English player profiles processed by the
Claude API. The host reads a profile file at NPC creation time and uses it to make decisions.

### Profile file format

A profile lives in the app's data directory under `npc-profiles/`. It is a Markdown file
with a YAML frontmatter block and a free-form body.

```markdown
---
name: Aggressive Alice
style: loose-aggressive
skill: intermediate
---

Alice enters pots wide — any pair, any two suited cards, any broadway hand. Once involved,
she applies constant pressure. She continuation-bets every flop and barrels two streets with
any draw. She rarely folds to a single bet but will release to a check-raise. On the river
she bluffs about 40% of the time when she missed a draw.

Alice has a tell: she raises pot-sized or larger only with the nuts or a total bluff —
never as a value bet with a medium hand.
```

### Decision flow

1. NPC's action window opens.
2. Host assembles a structured game state snapshot (hole cards, board, pot, stacks, position,
   betting history for this street, blind level).
3. Host calls the Claude API:
   - System prompt: poker rules + instruction to return a JSON action
   - User message: profile body + game state snapshot
4. Claude returns `{ "action": "raise", "amount": 480 }` or similar.
5. Host validates the returned action against legal action bounds and submits it.
6. Fallback: if the API call fails or times out (>5 s), submit `checkOrCall`.

### Prompt engineering notes

- The game state is serialised as human-readable text, not raw JSON, so the LLM can reason
  about it naturally.
- Legal actions and bet bounds are stated explicitly so the model does not hallucinate
  invalid amounts.
- The instruction asks for a JSON-only response with a fixed schema to simplify parsing.

### Phase 2 deliverables

- Profile file parser (frontmatter + body)
- Claude API integration in the Rust backend (using `reqwest` or similar)
- Prompt assembly and response parsing
- Fallback strategy when API is unavailable
- Profile management UI (load/unload profiles, assign to NPC seats)
- Tests: mock API response tests, malformed-response fallback tests

---

## Phase 3 — Advanced profiles with opponent modelling

**Goal:** Profiles can describe how an NPC adjusts to the specific opponents it faces, tracks
patterns across hands, and simulates tilt or emotional states.

### New profile capabilities

**Opponent notes section:**
```markdown
## Opponent tendencies
- Tight players: bluff more frequently on later streets.
- Aggressive players: tighten pre-flop range and trap with premium hands.
- Players who limp: isolation-raise their limps with a wide range.
```

**Tilt model:**
```markdown
## Tilt behaviour
After losing three hands in a row, Alice widens her opening range significantly and
becomes more likely to hero-call with marginal holdings.
```

**Session history context:**
The host maintains a per-NPC summary of key hands from the current session (big pots won/lost,
key bluffs caught/succeeded). This summary is appended to the prompt so the LLM can reference
it when making decisions.

### Phase 3 deliverables

- Per-NPC session history tracker (summarised, not full event log)
- Opponent modelling context: aggregate stats per opponent (VPIP, aggression factor,
  showdown frequency) computed from hand history
- Updated prompt assembly to include session context and opponent stats
- Extended profile format with optional opponent-notes and tilt-model sections
- Tests: context assembly, prompt length management (truncation when context is large)

---

## Constraints and non-goals

- **Android interop:** NPCs must not require any Android-side changes. Their actions arrive
  as ordinary protocol events.
- **Sound:** NPC actions do not trigger any sound (sound is intentionally excluded from the
  app for now).
- **Reconnect/resync:** If the host crashes mid-hand, NPC state is not persisted. A restarted
  host starts a new session; the incomplete hand is abandoned like any other crash.
- **Online play:** NPCs are LAN-only. No cloud matchmaking is planned.
- **Multiplayer NPC-only games:** The spec does not prevent it but the UI should not
  advertise it as a feature.
