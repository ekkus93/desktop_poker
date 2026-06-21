# replies5.md

## Response to Claude Code — FIX3 review questions

Thanks for verifying the FIX3 issues against the actual code before implementing. The confirmations for P0.1 and P0.2 are accepted: hand-log writes happening before `submit_action` is a real bug, and `NoConfig`, `RuntimeUnavailable`, and `StaleWindow` not updating `shared_action_error` is also a real gap.

Please implement FIX3 using the decisions below.

---

# Q1 — P1.1: How should `NpcProfile.style` map to `NpcStyle`?

Yes, implementing P1.1 requires adding a mapping from `NpcProfile.style: String` to the existing `NpcStyle` enum.

The purpose is not to turn the profile’s free-form persona string into a perfect strategy model. The purpose is to ensure that a profiled NPC’s rule-based fallback behavior is derived consistently from the profile rather than sometimes using `NpcConfig.style` and sometimes using `NpcProfile.style`.

## Required implementation

Add a small helper, preferably in `src-tauri/src/npc/llm_strategy.rs` or another shared NPC strategy module:

```rust
pub fn profile_style_to_npc_style(profile_style: &str) -> NpcStyle
```

or, if the config fallback is useful for unknown strings:

```rust
pub fn resolve_fallback_style(profile: Option<&NpcProfile>, config_style: NpcStyle) -> NpcStyle
```

But for an explicit profile, the helper must primarily use `profile.style`.

## Mapping guidance

Normalize the string before matching:

- trim whitespace,
- lowercase,
- replace `_` with `-` or otherwise handle both,
- match by useful keywords.

Suggested mapping:

```text
aggressive
loose-aggressive
lag
maniac
bully
pressure
bluffer
bluff-heavy
loose
```

→ `NpcStyle::Aggressive`

```text
conservative
tight
tight-passive
passive
nit
cautious
careful
rock
defensive
risk-averse
```

→ `NpcStyle::Conservative`

For:

```text
balanced
neutral
standard
default
```

If the enum only has `Aggressive` and `Conservative`, map to `NpcStyle::Conservative` for safer fallback. If a `Balanced` enum already exists, use it. Do **not** add a large new style system just for FIX3.

For unknown free-form profile styles, use a deterministic safe fallback:

```text
unknown / unrecognized profile style -> NpcStyle::Conservative
```

and optionally record/log a low-severity diagnostic such as:

```text
unknown profile fallback style 'tricky table captain'; using Conservative rule-based fallback
```

Do not fail the profile just because its human-readable style string is not recognized. `NpcProfile.style` is user/persona data, so strict parsing would be too brittle.

## Required behavior

For profiled NPCs, all fallback branches must call this helper or equivalent centralized logic:

- provider missing,
- provider state unavailable,
- client construction failure,
- request failure,
- response parse failure,
- invalid action.

For unprofiled NPCs, continue using `NpcConfig.style`.

---

# Q2 — P0.3: What should happen to the old API key when switching providers?

Use a **per-provider key retention policy**.

Do **not** delete the old provider’s key just because the user switches providers.

Reason: keychain already stores API keys under separate provider accounts such as `anthropic` and `openai`. Keeping each provider’s key under its own account is convenient and safe as long as the active runtime provider never reuses the wrong provider’s key.

## Required policy

When switching Anthropic → OpenAI:

- do not reuse the Anthropic key as the OpenAI key,
- if no OpenAI key exists and the user leaves the key field blank, return a visible “OpenAI API key required” error,
- keep the Anthropic key in the Anthropic keychain account so switching back can work without re-entry.

When switching OpenAI → Anthropic:

- same policy in reverse.

When switching API-key provider → local provider, such as Ollama or llama-server:

- active runtime provider state must not retain the old API key,
- local provider config must not carry or expose any API key,
- keychain may keep the old per-provider key for future use,
- do not attempt to use an API key for local provider calls.

When clearing provider config:

- clear the active provider config,
- remove all known API-provider secrets if this is a full “clear provider config” operation, unless the UI later grows a separate “forget only current provider” vs “forget all provider keys” distinction.

## Important distinction

Provider switching should retain old provider keys under their own accounts.

Provider clearing should remove provider secrets.

That gives the best balance:

- switching is convenient,
- clearing is privacy/security-respecting.

## Implementation notes

The frontend must not use a coarse boolean like `providerConfigured` to decide key preservation. It needs to know:

- loaded provider type,
- selected provider type,
- whether the selected provider already has a key,
- whether the user entered a replacement key.

The backend must also enforce this. Do not trust only the frontend.

---

# Q3 — P0.4 / P0.5: How should keychain failure be tested?

Use **Option A: introduce a storage trait**.

This is the cleanest way to test the keychain failure paths without depending on real OS keychain behavior during tests.

## Required approach

Abstract secret storage behind a trait, for example:

```rust
pub trait ProviderSecretStore {
    fn read_key(&self, provider: ProviderKind) -> Result<Option<String>, ProviderSecretError>;
    fn write_key(&self, provider: ProviderKind, key: &str) -> Result<(), ProviderSecretError>;
    fn delete_key(&self, provider: ProviderKind) -> Result<(), ProviderSecretError>;
}
```

Then provide implementations:

```text
KeychainProviderSecretStore    // release / secure path
PlaintextDevSecretStore        // debug/dev path
InMemoryTestSecretStore        // normal tests
FailingTestSecretStore         // tests that simulate read/write/delete failures
```

The trait does not need to be over-engineered. Keep it small and focused on provider API-key secrets.

## Why not Option B?

Testing only error-mapping logic is too weak for this issue. The dangerous bug is control flow: migration can log and continue, clear can skip deletion, and save can silently fall back. A trait lets tests prove the actual migration/save/clear control flow.

## Why not Option C?

Thread-local failure flags or `#[cfg(test)]` shims are acceptable in tiny modules, but they tend to become hidden magic. A trait is clearer and safer for this code because secret storage is a real boundary.

## Required tests enabled by the trait

Add tests for:

- keychain write failure during release-style migration returns provider config error,
- failed migration does not rewrite/remove embedded key,
- successful migration writes key and removes embedded key,
- keychain read failure for active provider returns provider error,
- keychain delete failure during clear is surfaced according to policy,
- clearing corrupt settings attempts deletion for all known API-provider accounts.

The tests can run in debug mode using the failing test store while still exercising release-like behavior.

---

# Q4 — P0.2: Should `lastNpcActionError` be structured or a richer string?

Use **Option A: structured type**.

A richer formatted string is a short-term workaround, but this app already has typed debug state crossing the Tauri boundary. Since FIX3 explicitly requires fields like player ID, attempted action, reason, hand/sequence, and submitted/not-submitted, this should be represented as structured data.

## Required Rust shape

Add something like:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NpcActionErrorDebug {
    pub player_id: Option<String>,
    pub action: Option<String>,
    pub reason: NpcActionErrorReason,
    pub message: String,
    pub hand_number: Option<u64>,
    pub sequence: Option<u64>,
    pub submitted: bool,
    pub occurred_at_ms: u64,
}
```

And:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NpcActionErrorReason {
    Rejected,
    StaleWindow,
    RuntimeUnavailable,
    NoConfig,
    ProviderStateUnavailable,
    InvalidAction,
    InternalError,
}
```

If exact timestamp plumbing is awkward, a monotonic counter or sequence number is acceptable. Do not block the task on wall-clock timestamp precision.

## Required TypeScript shape

Update `DebugInspectorState` from:

```ts
lastNpcActionError: string | null
```

to something like:

```ts
lastNpcActionError: {
  playerId: string | null;
  action: string | null;
  reason:
    | "rejected"
    | "staleWindow"
    | "runtimeUnavailable"
    | "noConfig"
    | "providerStateUnavailable"
    | "invalidAction"
    | "internalError";
  message: string;
  handNumber: number | null;
  sequence: number | null;
  submitted: boolean;
  occurredAtMs: number;
} | null;
```

Keep `message` for human-readable rendering, but do not encode all state only inside `message`.

## Required DebugPanel behavior

DebugPanel should render a concise human-readable summary, but tests should assert the structured fields where practical.

---

# Q5 — P2.1 / P2.2: Are these the same gaps from FIX2 or different ones?

Treat FIX3 P2.1 and P2.2 as a **re-audit directive plus known remaining examples**, not necessarily completely new tasks.

The latest review found at least one remaining vacuous pass in the provider-missing profiled NPC fallback test, not just the stale-window test. Specifically, look for a test named approximately:

```text
npc_runner_records_provider_missing_fallback_when_profile_is_set_but_no_provider_configured
```

The problematic pattern was an early return if the NPC action window was not reached. That test should fail if the NPC branch is not exercised.

For timeout tests, search for tests that still accept both success and failure patterns, such as:

```rust
match result {
    Ok(status) => { ... }
    Err(error) => { ... }
}
```

or assertions that allow either `Timeout` or another outcome when the test name claims a deterministic timeout.

## Required action

Do not assume FIX2 fixed all of them.

Please re-audit for:

```text
return; // from test before intended branch
if condition_not_reached { return; }
Ok(_) | Err(_) accepted in timeout test
Timeout | Network(_) accepted when exact timeout is intended
test name says timeout but assertion accepts success
```

## Known target areas

Start with:

- `src-tauri/src/networking/runtime/tests/tournament.rs`
- `src-tauri/src/app_state/tests/sessions_npc.rs`
- NPC runner tests under `src-tauri/src/npc/runner/`

If no remaining issues are found after the re-audit, update the TODO with a note saying the audit was performed and no vacuous branches remain. Do not blindly mark it complete.

---

# Q6 — P0.4 typo: `save_lmm_provider_settings`

That is a typo.

The intended name is:

```text
save_llm_provider_settings
```

or whatever the actual equivalent provider-settings save function is in the current code.

Please correct the typo in:

- `DESKTOP_POKER_STABILIZATION_FIX3_TODO.md`

Do not search for `save_lmm_provider_settings` as a real function name.

---

# Additional implementation guardrails

## For P0.1 hand-log correctness

Do not solve the rejected-action problem by deleting hand logging or moving it to a vague later phase with no tests.

The correct behavior is:

```text
chosen action -> submit -> if accepted, log exactly once -> if rejected, debug error only
```

## For P0.3 provider key behavior

Do not preserve keys based on a global `providerConfigured` boolean.

The key preservation decision must be based on provider identity.

## For P0.4 migration

Do not rewrite old config to remove the embedded key until secure storage write has succeeded.

## For P0.5 clear

On full provider clear, prefer deleting all known API-provider secret accounts. That avoids stale secrets when settings JSON is corrupt.

Known accounts:

```text
anthropic
openai
```

Use the actual provider account strings from the current code. Be careful about `openai` vs `openAi` naming and keep it consistent with the existing keychain account naming.

## For tests

Do not mark P2.1/P2.2 complete unless the tests force the intended branch. A test that passes because the intended NPC window never appears is not acceptable.
