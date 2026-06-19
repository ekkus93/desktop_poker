# UNIT_TEST3_TODO.md

This file tracks the next unit-test expansion after a coverage audit of the Rust
backend (post file-split refactor). It targets the highest-value remaining gaps
in **real production logic** — not wrappers, not mocks.

The four gaps, in priority order:

1. **`crypto/provider.rs`** — Ed25519 / X25519 / ChaCha20-Poly1305 provider
   (253 LOC, only 2 tests). Security- and Android-compatibility-critical;
   round-trips and failure modes are not directly tested.
2. **`engine` hand evaluation** — `evaluate_best_holdem_hand` (the poker hand
   ranker) has essentially one test. Category detection and tie-breaks are the
   core correctness surface.
3. **`engine` settlement / side pots** — `settle_showdown` has one deterministic
   test; multi-way all-ins, split pots, and odd-chip edge cases need coverage.
4. **`npc` rule-based decision** — `rule_based_decision` (the default bot brain)
   is only exercised by the single `npc_opponent` integration test; its branches
   are not unit-tested.

Goal: pin down correctness of crypto, poker hand ranking, pot settlement, and
the rule-based NPC, all on the real code paths.

---

## 0. Scope, conventions, and guardrails

### 0.1 Ground rules (apply to every task below)
- [x] Tests must exercise **real production functions** — no fabricated doubles
      that only prove a mock behaves as configured (per `CLAUDE.md`).
- [x] No new `#[ignore]` tests except where they require an external service
      (none expected here).
- [x] Keep each test focused: one behavior per `#[test]`, descriptive names.
- [x] Reuse existing in-file test helpers (e.g. engine's `card(rank, suit)`)
      before adding new ones; add a helper only when it removes duplication
      across ≥2 tests.
- [x] For crypto, where behavior must match Android, add a comment citing the
      Android contract (mirror the style of
      `chacha_key_derivation_matches_android_hkdf_contract`).
- [x] Every new test file/module keeps the file under 700 lines (split into a
      `tests/` submodule dir if it would exceed that).

### 0.2 File placement decisions
- [x] `crypto/provider.rs` — extend the existing in-file `#[cfg(test)] mod tests`.
      If it grows past ~300 lines total, convert `provider.rs` to a
      `provider/` dir module with `provider/tests.rs`.
- [x] Engine hand-eval + settlement — extend `engine/tests.rs` (already a
      submodule). If it crosses 700 lines, split into
      `engine/tests/` with `hand_eval.rs`, `settlement.rs`, `support.rs`.
- [x] NPC rule-based decision — add a `#[cfg(test)] mod tests;` to
      `npc/runner/decision.rs` (it has none today), or a `decision/tests.rs`
      submodule. `rule_based_decision` is `pub(crate)`, reachable from a child
      test module via `use super::*`.

### 0.3 Verification gates (run after each section)
- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml <module>:: -- --test-threads=2`

---

## 1. `crypto/provider.rs` — Ed25519 / X25519 / ChaCha20-Poly1305

Reference surface:
- Provider: `DefaultCryptoProvider` implementing `ProtocolCryptoProvider`.
- Trait methods: `generate_signing_keypair`, `generate_encryption_keypair`,
  `sign(&material, bytes) -> String`,
  `verify(verifying_key_b64, bytes, signature_b64) -> Result<(), ProtocolError>`,
  `encrypt(sender_material, recipient_pub_b64, plaintext, aad) -> Result<EncryptedPayload>`,
  `decrypt(recipient_material, sender_pub_b64, payload, aad) -> Result<Vec<u8>>`.
- Material: `SigningKeyMaterial::{public_key_base64, key_id}`,
  `EncryptionKeyMaterial::{public_key_base64, key_id}`.
- `EncryptedPayload { nonce_base64, ciphertext_base64, recipient_key_id }`.
- Helpers: `key_fingerprint(bytes)`, private `decode_exact::<N>`, private
  `derive_chacha_key`. Encoding is base64url **no padding** (`URL_SAFE_NO_PAD`).

### 1.1 Signing round-trip (Ed25519)
- [x] `sign_then_verify_round_trips`: generate signing keypair, sign a byte
      slice, `verify(public_key_base64, bytes, signature)` returns `Ok(())`.
- [x] `signing_is_deterministic_for_same_key_and_message`: signing the same
      bytes with the same key yields an identical signature string (Ed25519 is
      deterministic) — guards encoding stability.
- [x] `distinct_keypairs_produce_distinct_public_keys`: two generated keypairs
      have different `public_key_base64` and `key_id`.

### 1.2 Signature verification — negative cases
- [x] `verify_rejects_tampered_message`: flip a byte of the signed message →
      `Err`.
- [x] `verify_rejects_wrong_public_key`: verify a valid signature against a
      *different* key's `public_key_base64` → `Err`.
- [x] `verify_rejects_tampered_signature`: mutate the signature base64 → `Err`.
- [x] `verify_rejects_malformed_signature_encoding`: pass non-base64url / wrong
      length → `Err` (exercises `decode_exact::<64>` error path).
- [x] `verify_rejects_malformed_verifying_key`: wrong-length / invalid key
      bytes → `Err` (exercises `decode_exact::<32>` + `VerifyingKey::from_bytes`).

### 1.3 Encryption round-trip (X25519 + ChaCha20-Poly1305)
- [x] `encrypt_then_decrypt_round_trips`: sender encrypts to recipient's public
      key with AAD; recipient `decrypt` returns the original plaintext.
- [x] `encrypted_payload_carries_recipient_fingerprint`: `recipient_key_id`
      equals `key_fingerprint(recipient_public_key_bytes)` /
      `recipient.key_id()`.
- [x] `nonce_and_ciphertext_are_base64url_no_padding`: assert the encoded
      fields contain no `=` padding and decode cleanly.
- [x] `each_encryption_uses_a_fresh_nonce`: encrypting the same plaintext twice
      yields different `nonce_base64` (and different ciphertext).

### 1.4 Decryption — negative cases
- [x] `decrypt_rejects_tampered_ciphertext`: mutate `ciphertext_base64` → `Err`
      (AEAD auth failure).
- [x] `decrypt_rejects_wrong_sender_key`: decrypt with a sender public key that
      doesn't match the one used to encrypt → `Err` (DH mismatch).
- [x] `decrypt_rejects_wrong_recipient_key`: a third party's key material can't
      decrypt → `Err`.
- [x] `decrypt_rejects_mismatched_aad`: encrypt with AAD `A`, decrypt with AAD
      `B` → `Err`.
- [x] `decrypt_rejects_tampered_or_wrong_length_nonce`: bad `nonce_base64`
      (wrong length) → `Err` (exercises `decode_exact::<12>`).
- [x] `decrypt_rejects_malformed_ciphertext_encoding`: non-base64url ciphertext
      → `Err`.

### 1.5 Key material + fingerprint format
- [x] `public_key_base64_is_url_safe_no_pad`: signing and encryption public keys
      encode with no `=` and decode to exactly 32 bytes.
- [x] `key_fingerprint_is_16_lowercase_hex`: extend the existing fingerprint
      test to assert lowercase hex and that it equals the first 8 bytes of
      SHA-256 of the key (mirror Android contract).
- [x] `fingerprint_is_stable_for_same_key`: same key bytes → same fingerprint.

### 1.6 `decode_exact` helper edge cases (via the public methods)
- [x] Confirm the error *messages* include the field label (e.g. "signature",
      "nonce", "recipient public key") so failures are diagnosable — assert on
      `ProtocolError` message substrings where reasonable.
- [x] Wrong-length-but-valid-base64 input is rejected with the
      "must decode to N bytes" message.

---

## 2. `engine` — hand evaluation (`evaluate_best_holdem_hand`)

Reference surface:
- `evaluate_best_holdem_hand(hole_cards: &[Card], board_cards: &[Card]) -> Result<HandStrength, EngineError>`.
- `HandStrength { category: HandCategory, key_ranks: [u8; 5] }`, with
  `impl Ord` (used to compare hands).
- `HandCategory`: `HighCard, OnePair, TwoPair, ThreeOfAKind, Straight, Flush,
  FullHouse, FourOfAKind, StraightFlush`.
- Helper `card(rank, suit)` already exists in `engine/tests.rs`;
  `Rank` (Two..Ace), `Suit` (Clubs/Diamonds/Hearts/Spades).

### 2.1 Test helpers
- [x] Add a small `eval(holes: &[Card], board: &[Card]) -> HandStrength` wrapper
      that unwraps the `Result` for terse assertions.
- [x] Optionally a `cards(&str)`-style parser is **not** required — prefer
      explicit `card(Rank::X, Suit::Y)` for readability and to match existing
      style.

### 2.2 Category detection (one positive test per category)
- [x] `detects_high_card`
- [x] `detects_one_pair`
- [x] `detects_two_pair`
- [x] `detects_three_of_a_kind`
- [x] `detects_straight` (e.g. 5-6-7-8-9, mixed suits)
- [x] `detects_flush`
- [x] `detects_full_house`
- [x] `detects_four_of_a_kind`
- [x] `detects_straight_flush`
- Each asserts `category` and a sane `key_ranks[0]` (the determining rank).

### 2.3 Straight edge cases
- [x] `wheel_straight_ace_plays_low`: A-2-3-4-5 → `Straight` ranked as 5-high
      (assert `key_ranks` reflects 5 as the top card, not Ace).
- [x] `broadway_straight_ace_high`: 10-J-Q-K-A → `Straight`, Ace-high.
- [x] `royal_flush_is_straight_flush_ace_high`: 10-J-Q-K-A suited →
      `StraightFlush`, Ace-high.
- [x] `steel_wheel_straight_flush_ace_low`: A-2-3-4-5 suited → `StraightFlush`
      ranked 5-high.
- [x] `no_false_straight_across_suits_only`: ensure a 5-card flush that is not
      sequential is `Flush`, not `Straight`.

### 2.4 Best-5-of-7 selection
- [x] `picks_best_five_from_seven`: a 7-card pool where the naive first-5 is not
      the best (e.g. board pairs but hole cards make a flush) → returns the flush.
- [x] `board_plays_when_hole_cards_are_irrelevant`: best hand lives entirely on
      the board; both a strong and weak hole-card pair still evaluate to the
      board hand.
- [x] `uses_hole_cards_to_break_into_a_better_category` (e.g. one hole card
      completes quads / a straight).

### 2.5 Tie-breaks and ordering (`HandStrength: Ord`)
- [x] `category_ordering_is_correct`: assert `StraightFlush > FourOfAKind >
      FullHouse > Flush > Straight > ThreeOfAKind > TwoPair > OnePair > HighCard`
      via constructed `HandStrength`s or evaluated hands.
- [x] `flush_tiebreak_by_high_cards`: two flushes, higher top card wins.
- [x] `full_house_tiebreak_trips_then_pair`: e.g. KKK22 beats QQQAA;
      KKKQQ beats KKK22.
- [x] `two_pair_tiebreak_high_low_kicker`: AA992 vs AA88K etc.
- [x] `one_pair_tiebreak_by_kickers`: same pair, kicker order decides.
- [x] `high_card_tiebreak_by_full_key_ranks`.
- [x] `identical_hands_compare_equal` (true chop — equal `HandStrength`).

### 2.6 Error / boundary cases
- [x] `errors_on_too_few_cards`: fewer than 5 total cards → `Err(EngineError)`.
- [x] (If applicable) duplicate-card handling matches current behavior — assert
      the existing contract rather than inventing new behavior.

---

## 3. `engine` — settlement and side pots (`settle_showdown`)

Reference surface:
- `settle_showdown(contributions_by_player_id: &BTreeMap<String, u32>,
  hand_strengths_by_player_id: &BTreeMap<String, HandStrength>,
  odd_chip_order: &[String]) -> Result<SettlementResult, EngineError>`.
- `SettlementResult { winning_player_ids, payouts_by_player_id,
  pot_summaries: Vec<PotSummary> }`.
- Existing: `side_pots_and_odd_chip_are_settled_deterministically` (keep, build
  around it).

### 3.1 Test helpers
- [x] Add a `strength(category, key_ranks)` builder (or reuse `evaluate_*` to
      produce real `HandStrength`s) so settlement tests read clearly.
- [x] Add a `total_payout(&SettlementResult) -> u32` helper for conservation
      assertions.

### 3.2 Basic payouts
- [x] `single_player_takes_uncontested_pot`: one contributor (or one with a hand)
      receives everything.
- [x] `best_hand_wins_whole_pot_heads_up`.
- [x] `payouts_conserve_total_contributions`: assert
      `sum(payouts) == sum(contributions)` in every settlement test (make this a
      shared assertion).

### 3.3 Split pots and odd chips
- [x] `tied_hands_split_pot_evenly`: two equal hands, even pot → equal payouts.
- [x] `odd_chip_goes_to_first_in_odd_chip_order`: odd total, two winners → the
      extra chip goes to the player listed first in `odd_chip_order`.
- [x] `odd_chip_order_is_respected_for_three_way_split`.

### 3.4 Multiple all-ins / side pots
- [x] `two_all_ins_create_main_and_side_pot`: short stack eligible only for the
      main pot; the side pot is contested by the deeper stacks only.
- [x] `short_all_in_cannot_win_side_pot`: short stack has the best hand but only
      wins up to the main pot; deeper stacks split/own the side pot.
- [x] `three_distinct_all_in_amounts_create_two_side_pots`: verify each
      `PotSummary` amount and eligibility.
- [x] `side_pot_won_by_different_player_than_main_pot`.

### 3.5 Folded contributors and edge cases
- [x] `folded_contributor_chips_count_but_cannot_win`: a player who contributed
      but has no entry in `hand_strengths_by_player_id` forfeits their chips to
      the pot without receiving a payout.
- [x] `empty_or_zero_contributions_behavior`: assert the current contract
      (e.g. empty input → empty result or `Err`) rather than guessing.

---

## 4. `npc` — rule-based decision (`rule_based_decision`)

Reference surface (`npc/runner/decision.rs`, `pub(crate)`):
```
rule_based_decision(
  style: &NpcStyle, hole_cards: &[Card], board: &[Card], street: StreetPhase,
  pot_total: u32, call_amount: u32, min_raise_to: Option<u32>,
  max_raise_to: Option<u32>, facing_bet: bool, stack: u32, active_count: u8,
  dealer_seat: u8, npc_seat: u8, blind_level_index: usize,
  state: &TournamentState, legal_actions: &[ActionType], seed: u64,
) -> (ActionType, Option<u32>)
```
This is the default (non-LLM) decision used every NPC turn.

### 4.1 Test setup
- [x] Add a `#[cfg(test)] mod tests;` (or inline module) to
      `npc/runner/decision.rs`; access `rule_based_decision` via `use super::*`.
- [x] Build a **minimal `TournamentState` fixture** sufficient for the function
      (inspect exactly which fields `rule_based_decision` reads from `state` and
      populate only those; reuse existing tournament/state fixtures if one fits).
- [x] Add a `decide(...)` helper with sensible defaults so each test overrides
      only the inputs it cares about (style, hole cards, call_amount, stack,
      facing_bet, legal_actions, seed).

### 4.2 Legality invariants (highest value — property style)
- [x] `never_returns_an_action_outside_legal_actions`: across a matrix of inputs
      (vary street, facing_bet, hand strength, stack), the returned
      `ActionType` is always contained in `legal_actions`.
- [x] `raise_amount_within_min_and_max_when_raising`: whenever the action is
      `Bet`/`Raise`, the returned `Some(amount)` satisfies
      `min_raise_to <= amount <= max_raise_to`.
- [x] `no_raise_amount_when_not_raising`: fold/check/call return `None` (or the
      documented value) for the amount.
- [x] `does_not_raise_when_raise_is_not_legal`: with `legal_actions` excluding
      `Raise`/`Bet`, the function never returns a raise.

### 4.3 Hand-strength behavior
- [x] `folds_trash_facing_a_bet`: weak hand (e.g. 7♣2♦) + `facing_bet=true` +
      `Fold` legal → returns `Fold`.
- [x] `does_not_fold_when_checking_is_free`: `call_amount == 0` → never folds
      (returns `Check`/`Bet`, not `Fold`).
- [x] `raises_or_bets_premium_hand_when_allowed`: e.g. A♠A♥ preflop with raise
      legal → `Bet`/`Raise`.
- [x] `calls_or_checks_medium_hand` (documents the medium-strength branch).

### 4.4 Stack / position behavior
- [x] `short_stack_shoves_when_appropriate`: small `stack` relative to blinds →
      `AllIn` (or call-all-in) per the implemented short-stack rule.
- [x] `position_influences_aggression` if the implementation uses
      `derive_position(npc_seat, dealer_seat, active_count)` — assert the
      documented late- vs early-position difference (only if such a branch
      exists; otherwise note "no positional branch" and skip).

### 4.5 Style differences
- [x] `aggressive_style_bets_more_often_than_conservative`: same hand/board/seed,
      `NpcStyle::Aggressive` vs `NpcStyle::Conservative` → aggressive chooses a
      raise/bet at least as often (assert the concrete divergence on a chosen
      borderline hand).

### 4.6 Determinism
- [x] `same_inputs_and_seed_produce_same_decision`: calling twice with identical
      args (including `seed`) returns identical `(ActionType, Option<u32>)`.
- [x] `different_seeds_can_diverge_on_mixed_strategy_spots` (only assert this if
      the implementation actually randomizes; otherwise document determinism).

---

## 5. Wrap-up and verification

### 5.1 Full verification
- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=2`
      (bounded threads to avoid the known real-TCP timing flakiness).
- [x] Confirm the new tests run and pass; record the new per-module counts
      (crypto, engine, npc) for the record.

### 5.2 Housekeeping
- [x] Ensure no file crossed 700 lines; split into `tests/` submodules if so.
- [x] Append a `memory.md` entry summarizing what was added and the new counts.
- [x] Commit per area (crypto / engine-eval / engine-settlement / npc-decision)
      with no `Co-Authored-By` trailer.

### 5.3 Priority order (recommended implementation sequence)
1. [x] Section 1 (crypto) — highest value, lowest risk, self-contained.
2. [x] Section 2 (hand evaluation) — core correctness, pure function.
3. [x] Section 3 (settlement / side pots) — builds on Section 2 helpers.
4. [x] Section 4 (NPC decision) — most fixture setup; do last.
