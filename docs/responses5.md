# FIX3 Review Questions — Desktop Claude

These are questions and issues identified after reading `DESKTOP_POKER_STABILIZATION_FIX3_SPEC.md`
and `DESKTOP_POKER_STABILIZATION_FIX3_TODO.md` and cross-checking the current source code.
No code has been written yet.

---

## Confirmed issues (code-verified before implementation)

### P0.1 — Hand-log writes confirmed to precede submit_action

In `src-tauri/src/npc/runner/action.rs`:

- The LLM-success path writes to `runner_state.hand_log` at lines 269–279.
- All non-LLM fallback paths write at lines 369–380.
- `host_server.submit_action(...)` is called at line 382.

Both write sites are before the submit call, so a rejected action is currently recorded
in the hand log as if it were accepted. This confirms P0.1 is a real bug.

The fix is to extract the chosen `(action_type, raise_to)` first, then call
`submit_action`, and only write to the hand log on `Ok(())`. The `action_logged` flag and
the split between the LLM-arm write and the fallthrough write will need to be replaced
with a single post-submit log path.

### P0.2 — StaleWindow, RuntimeUnavailable, NoConfig do not update shared_action_error

In `action.rs`:

- `NoConfig` returns at line 67 — no `shared_action_error` write.
- `RuntimeUnavailable` returns at line 87 — no `shared_action_error` write.
- `StaleWindow` returns at lines 94–95 and 98 — no `shared_action_error` write.
- Only `Rejected` updates `shared_action_error` (line 396).

This confirms P0.2 is a real gap.

---

## Questions

### Q1 — P1.1: How should profile style map to NpcStyle enum?

`action.rs` line 73 has:

```rust
// Fallback style is always npc_config.style so all fallback branches are consistent (P1.3).
// NpcProfile.style is a human-readable persona string for the LLM, not an NpcStyle enum.
let fallback_style: &crate::npc::NpcStyle = &npc_config.style;
```

The spec says: "If an NPC has an explicit profile, all fallback branches must derive style
from the profile."

`NpcConfig.style` is an `NpcStyle` enum. `NpcProfile.style` is a `String` (a persona
description for the LLM prompt, not an enum variant). They are different types.

**Question:** Does implementing P1.1 require adding a mapping from the profile style string
(e.g., `"aggressive"`, `"conservative"`) to an `NpcStyle` enum variant? Is there already
a `from_str`-style mapping defined, or should I add one? Or does `NpcProfile` have a
separate field that already aligns with `NpcStyle`?

---

### Q2 — P0.3: What is the correct policy for the old API key when switching providers?

The keychain already stores keys per-provider using separate accounts
(`anthropic`, `openai`). This means the cross-reuse bug (using Anthropic's key as
OpenAI's key) is naturally prevented by the account naming — a blank key on provider
switch will just find no key under the new provider's account name.

The remaining question is the **cleanup policy**: when a user switches from Anthropic to
OpenAI (or vice versa), should the old provider's key be:

- **Deleted from the keychain** on switch (cleaner, no stale keys), or
- **Left in place under its own account** (so switching back doesn't require re-entering
  the key)?

The TODO says to "decide and implement stale old-key cleanup policy." Which do you prefer?

---

### Q3 — P0.4 / P0.5: How should keychain failure be tested?

Keychain operations in `provider_storage.rs` are gated with
`#[cfg(not(debug_assertions))]`. This means tests run in debug mode and never exercise
the real keychain path. Simulating a keychain write/read/delete failure requires one of:

**Option A — Introduce a storage trait:**
Abstract keychain operations behind a trait so tests can inject a stub that returns
failures. This makes all error paths unit-testable but adds an abstraction layer.

**Option B — Test the error-mapping logic in isolation:**
Unit-test the functions that convert keychain errors into provider config errors, without
actually invoking the keychain. Integration tests for migration/clear behavior are
documented as manual-only in the smoke test checklist.

**Option C — Conditional test helpers:**
Add `#[cfg(test)]`-only shims that simulate failure via a thread-local flag or similar.

Which approach do you want?

---

### Q4 — P0.2: Should lastNpcActionError be a struct or a richer string?

The spec says each error record should include:

- player ID if known
- attempted action if one was selected
- failure reason
- hand number or sequence if available
- timestamp or monotonic counter
- whether the action was submitted or not submitted

Currently `shared_action_error` is `Arc<Mutex<Option<String>>>`, and the TypeScript side
reads it as a plain string in `DebugInspectorState`.

**Option A — Structured type:**
Define a `NpcActionError` struct in Rust, serialize it to the frontend, and update
`DebugInspectorState` with discrete fields. Gives DebugPanel more rendering flexibility
but requires changes on both sides of the Tauri boundary.

**Option B — Richer formatted string:**
Keep the `Option<String>` wire type but include all required fields in a formatted message
(e.g., `"player=npc1 action=Fold reason=StaleWindow hand=3 submitted=false"`). Simpler
but less structured.

Which do you prefer?

---

### Q5 — P2.1 / P2.2: Are these the same gaps from FIX2 or different ones?

FIX2 already addressed:
- **FIX2 P2.1** — the stale-window vacuous test (rewrote it to deterministically advance
  to the NPC's action window before asserting).
- **FIX2 P2.2** — the timeout test (changed assertion from
  `Timeout | Network(_)` to exact `Timeout`).

FIX3 P2.1 mentions "the specific provider-missing profiled NPC fallback test fails if NPC
window is not reached" — which sounds like a *different* test from the stale-window one.
FIX3 P2.2 says "some timeout tests allow either `Ok` or `Err`" — which may be different
tests from the one fixed in FIX2.

**Question:** Are FIX3 P2.1 and P2.2 targeting *different* remaining tests beyond what
FIX2 fixed, or is this a re-audit directive to verify FIX2's fixes were sufficient? If
the former, can you point to the specific test names so I can find them quickly?

---

### Q6 — P0.4: Minor TODO typo

`DESKTOP_POKER_STABILIZATION_FIX3_TODO.md` line 123 reads:

> Backend `save_lmm_provider_settings` or equivalent...

"lmm" appears to be a typo for "llm". Flagging in case this is a real function name I
should search for verbatim vs. a typo to disregard.
