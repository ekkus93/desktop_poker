# FIX4 Responses — DESKTOP_POKER_RUNTIME_HARDENING_FIX4_SPEC.md / _TODO.md

Fill in each `A:` line, then share the file back (or paste your answers).

---

**1. NPC fallback policy scope (P0.4 / Spec §3)**

Q: Should we implement the preferred approach — add an `allowRuleBasedLlmFallback: bool`
   flag (default `false`) to `NpcConfig` so the host can opt specific NPCs into visible
   rule-based fallback — or only the acceptable smaller pass (no flag, just hard-fail on
   internal/provider failures, keeping LLM-request failure visible in debug UI)?
   The TODO only specifies the smaller pass.

A: Implement the preferred approach: add `allowRuleBasedLlmFallback: bool` to `NpcConfig`, defaulting to `false`.

Semantics I want:

- `false` means an LLM-profile NPC must not silently degrade into the rule-based bot. If the LLM path cannot produce an accepted decision, return a structured NPC error and surface it in the debug/host UI.
- `true` means the host explicitly opted that NPC into rule-based fallback. The fallback is allowed only for external/provider-operational failures such as request timeout, network error, provider unavailable, missing provider config, or provider rate/error response.
- Internal correctness failures must never use the rule-based fallback, even when `allowRuleBasedLlmFallback` is true. Examples: poisoned provider state lock, malformed in-memory provider state, invalid table/chip/blind/current-hand snapshot, illegal action set, or failed construction caused by invalid stored config.
- Every fallback taken must update the existing `lastLlmFallback`/debug state with a reason, timestamp, provider/profile/player id, and the selected rule-based action. It should be visible enough that the host can tell the NPC is no longer making LLM-backed decisions.
- Existing serialized configs should migrate by treating a missing field as `false`.

This is slightly more work than the smaller TODO pass, but it prevents the ambiguous middle ground where fallback is technically visible in logs/debug state while gameplay still quietly changes behavior.

---

**2. `rule_based_decision` refactor scope (P0.5 / Spec §4)**

Q: Do you want the full `RuleDecisionContext<'_>` struct refactor (all callers pass a
   typed context struct, touching multiple files), or is it acceptable to make
   `rule_based_decision` return `Result<(ActionType, Option<u32>), String>` and propagate
   errors up? The struct version is cleaner long-term but touches more files.

A: Do the full `RuleDecisionContext<'_>` refactor.

I want the rule-based path to receive a typed, already-validated context instead of a raw `GameStateSnapshot` full of `Option` fields. This is the cleaner long-term fix and directly addresses the unsafe defaults that currently exist inside `decision.rs`.

Suggested shape:

```rust
pub struct RuleDecisionContext<'a> {
    pub player_id: &'a str,
    pub legal_actions: &'a [ActionType],
    pub stack: u32,
    pub current_bet: u32,
    pub big_blind: u32,
    pub pot: u32,
    pub hole_cards: &'a [Card],
    pub board_cards: &'a [Card],
    pub street: BettingRound,
}
```

It is fine if the exact fields differ, but the important requirement is that values required for a decision are non-optional at the decision boundary. Build this context in `action.rs` using checked extraction helpers. If any required value is missing or inconsistent, return a structured NPC internal error and do not call `rule_based_decision`.

Do not preserve `.unwrap_or(...)` defaults in the rule decision path just to reduce the diff.

---

**3. Multi-NPC rollback implementation (P1.1 / Spec §5)**

Q: `release_seat` and `unregister_participant` do not currently exist on `HostServer`.
   Two options:
   (a) Add those two methods to HostServer to enable the rollback-on-failure approach
       described in the TODO.
   (b) Implement the preferred approach: an atomic multi-NPC transaction method that
       validates and applies all NPC registrations/seats/ready-states under the
       authoritative state lock (no rollback needed because nothing is committed until
       all steps succeed).
   Which do you prefer?

A: Prefer option (b): implement an atomic multi-NPC transaction method on `HostServer`.

Do not add rollback as the primary design unless the atomic method turns out to be impractical. Rollback is better than the current partial-commit behavior, but it is still easier to get wrong because every future side effect also needs a compensating undo operation.

The desired implementation is a single authoritative method that:

1. Validates every requested NPC profile/player/seat/name.
2. Checks table capacity and requested seat availability.
3. Ensures no duplicate participant ids or names will be introduced.
4. Applies registration, seat claim, and ready state under the authoritative state lock.
5. Returns either all created NPC player ids or no mutation at all.

Suggested API shape:

```rust
pub struct NpcSeatAssignment {
    pub player_id: PlayerId,
    pub display_name: String,
    pub seat_index: Option<usize>,
}

impl HostServer {
    pub fn add_npc_participants_atomic(
        &self,
        assignments: Vec<NpcSeatAssignment>,
    ) -> Result<Vec<PlayerId>, HostServerError> {
        // Validate everything first while holding the authoritative lock.
        // Commit only after every validation step has succeeded.
    }
}
```

`app_npc.rs` should call this once, then start/update the NPC runner only after the atomic host mutation succeeds. If the method returns an error, no NPC should be registered, seated, ready, or runnable.

---

**4. Thread spawn failure testing (P1.2)**

Q: The acceptance criteria for P1.2 say to "inject a spawn abstraction or equivalent"
   so that tests can exercise the thread-spawn-failure path. Two options:
   (a) Introduce a `SpawnFn` type parameter or trait object into `start_npc_runner`
       so tests can inject a spawn function that returns an error.
   (b) Use a `#[cfg(test)]` compile-time override that makes `start_npc_runner`
       return an error directly, without changing the production signature.
   Is either acceptable, or do you have a different preference?

A: Use option (a), but keep the production API clean by hiding the injection behind a small helper.

Avoid a `#[cfg(test)]` branch that forces `start_npc_runner` to fail. That tests a compile-time fake path, not the real production error propagation.

Preferred structure:

```rust
type SpawnResult = std::io::Result<std::thread::JoinHandle<()>>;

fn start_npc_runner_with_spawner<F>(
    deps: NpcRunnerDeps,
    spawn: F,
) -> Result<std::thread::JoinHandle<()>, String>
where
    F: FnOnce(std::thread::Builder, Box<dyn FnOnce() + Send + 'static>) -> SpawnResult,
{
    spawn(
        std::thread::Builder::new().name("npc-runner".to_owned()),
        Box::new(move || run_npc_loop(deps)),
    )
    .map_err(|e| format!("failed to spawn npc-runner thread: {e}"))
}

pub fn start_npc_runner(deps: NpcRunnerDeps) -> Result<std::thread::JoinHandle<()>, String> {
    start_npc_runner_with_spawner(deps, |builder, f| builder.spawn(f))
}
```

The exact function names can vary. The acceptance requirement is that tests can inject a spawner returning `Err(...)` and verify that the caller gets a normal `Result::Err`, not a panic.

---

**5. P1.5 fix location — window persistence test noise**

Q: The `__TAURI_INTERNALS__` guard in `persistence.ts` already works correctly for
   non-Tauri environments. The test noise occurs because `persistence.test.ts`
   explicitly sets `__TAURI_INTERNALS__` on window (to test the Tauri path), and then
   the dynamic `import("@tauri-apps/api/window")` fails because no Tauri runtime
   exists in jsdom. Two fix options:
   (a) Add a vi.mock for `@tauri-apps/api/window` in the persistence tests so the
       dynamic import resolves cleanly in jsdom.
   (b) Remove the explicit `__TAURI_INTERNALS__` setup from the tests that trigger
       the Tauri path, and test window-state behavior differently (e.g. by mocking
       the full Tauri API).
   Which approach do you prefer?

A: Prefer option (a): add a `vi.mock("@tauri-apps/api/window", ...)` in the persistence tests.

The current test is intentionally exercising the Tauri path by setting `__TAURI_INTERNALS__`, so removing that setup weakens coverage. Mocking the dynamic import is the correct fix for jsdom.

Acceptance criteria:

- Tests that set `window.__TAURI_INTERNALS__` should provide a complete enough mock of `@tauri-apps/api/window` for the persistence module to run without stderr noise.
- The non-Tauri guard path should still have a separate test proving that no Tauri import is attempted when `__TAURI_INTERNALS__` is absent.
- `npm test` should not emit the current “Failed to initialize window state persistence” noise in normal passing tests.

Suggested mock direction:

```ts
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    innerPosition: vi.fn().mockResolvedValue({ x: 10, y: 20 }),
    innerSize: vi.fn().mockResolvedValue({ width: 1200, height: 800 }),
    setPosition: vi.fn().mockResolvedValue(undefined),
    setSize: vi.fn().mockResolvedValue(undefined),
    listen: vi.fn().mockResolvedValue(() => undefined),
  }),
  PhysicalPosition: class {
    constructor(public x: number, public y: number) {}
  },
  PhysicalSize: class {
    constructor(public width: number, public height: number) {}
  },
}));
```

---

**6. P2.2 audit scope**

Q: The P2.2 audit task runs `rg` over all of `networking/`, `npc/`, and `app_state/`
   looking for `let _ =`, `.ok()`, `unwrap_or(`, `continue;`. Many hits will be
   pre-existing code untouched by FIX4. Should the audit:
   (a) Cover only code modified or introduced during FIX4 (focused, low risk of
       unintended churn), or
   (b) Cover the entire three directories and convert any unjustified silent failures
       found (broader cleanup, higher churn)?

A: Use a hybrid policy.

For required FIX4 acceptance, use option (a): fix all silent/quiet failure patterns in code modified or introduced by FIX4. That keeps the patch reviewable and avoids accidental broad behavior changes.

However, still run the broader `rg` audit over `networking/`, `npc/`, and `app_state/`. For each broad hit outside the FIX4 diff:

- Fix it immediately only if it is clearly unsafe, low-risk, and directly related to the hardening theme.
- Otherwise, leave it unchanged but document it in the audit notes with a short justification: acceptable, pre-existing follow-up, test-only, intentional best-effort cleanup, or needs design decision.

Do not do a blind mechanical conversion of every `continue;`, `.ok()`, `unwrap_or(...)`, or `let _ =`. Some are legitimate. The goal is to eliminate unjustified quiet failures, not create churn.

---

**7. `decision.rs` inner defaults (P0.5 cross-cutting)**

Q: P0.5 adds validation in `action.rs` before calling `rule_based_decision`, but
   `decision.rs` itself still has `.unwrap_or(20)` (big blind default) and
   `.unwrap_or(0)` (current-hand default) inside `GameStateSnapshot` construction.
   If `rule_based_decision` continues to receive a raw `GameStateSnapshot` with
   `Option<u32>` fields, those inner defaults remain reachable even after the P0.5
   call-site validation.
   Should P0.5 also eliminate those inner defaults in `decision.rs` (requires
   changing `GameStateSnapshot` fields from `Option<u32>` to `u32` or returning
   `Result`), or is the call-site pre-validation in `action.rs` sufficient for now?

A: Yes. P0.5 should eliminate the inner defaults in `decision.rs`.

Call-site pre-validation in `action.rs` is not sufficient if `rule_based_decision` can still be called from another path later with a raw `GameStateSnapshot`. The decision module itself should make invalid state unrepresentable or return an error.

Preferred fix: combine this with the `RuleDecisionContext<'_>` refactor from answer 2. `decision.rs` should receive non-optional `big_blind`, `current_bet`, stack, legal actions, and other required values. Then remove the current `.unwrap_or(20)` and `.unwrap_or(0)` defaults entirely.

If Claude Code chooses the smaller `Result` approach despite answer 2, then `decision.rs` must still return `Err(...)` for missing big blind/current bet instead of defaulting. Either way, no hardcoded poker-state defaults should remain in the authoritative NPC decision path.

Tests should explicitly cover missing big blind/current bet/current hand data and verify that the result is an NPC internal error, not a fold/check/call based on invented defaults.
