# FIX4 Responses — DESKTOP_POKER_RUNTIME_HARDENING_FIX4_SPEC.md / _TODO.md

Fill in each `A:` line, then share the file back (or paste your answers).

---

**1. NPC fallback policy scope (P0.4 / Spec §3)**

Q: Should we implement the preferred approach — add an `allowRuleBasedLlmFallback: bool`
   flag (default `false`) to `NpcConfig` so the host can opt specific NPCs into visible
   rule-based fallback — or only the acceptable smaller pass (no flag, just hard-fail on
   internal/provider failures, keeping LLM-request failure visible in debug UI)?
   The TODO only specifies the smaller pass.

A:

---

**2. `rule_based_decision` refactor scope (P0.5 / Spec §4)**

Q: Do you want the full `RuleDecisionContext<'_>` struct refactor (all callers pass a
   typed context struct, touching multiple files), or is it acceptable to make
   `rule_based_decision` return `Result<(ActionType, Option<u32>), String>` and propagate
   errors up? The struct version is cleaner long-term but touches more files.

A:

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

A:

---

**4. Thread spawn failure testing (P1.2)**

Q: The acceptance criteria for P1.2 say to "inject a spawn abstraction or equivalent"
   so that tests can exercise the thread-spawn-failure path. Two options:
   (a) Introduce a `SpawnFn` type parameter or trait object into `start_npc_runner`
       so tests can inject a spawn function that returns an error.
   (b) Use a `#[cfg(test)]` compile-time override that makes `start_npc_runner`
       return an error directly, without changing the production signature.
   Is either acceptable, or do you have a different preference?

A:

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

A:

---

**6. P2.2 audit scope**

Q: The P2.2 audit task runs `rg` over all of `networking/`, `npc/`, and `app_state/`
   looking for `let _ =`, `.ok()`, `unwrap_or(`, `continue;`. Many hits will be
   pre-existing code untouched by FIX4. Should the audit:
   (a) Cover only code modified or introduced during FIX4 (focused, low risk of
       unintended churn), or
   (b) Cover the entire three directories and convert any unjustified silent failures
       found (broader cleanup, higher churn)?

A:

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

A:
