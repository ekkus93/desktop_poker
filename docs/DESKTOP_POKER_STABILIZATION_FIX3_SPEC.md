# Desktop Poker Stabilization FIX3 Spec

## Purpose

This FIX3 spec defines the next stabilization pass for the Desktop Poker app after the FIX2 implementation review.

The previous pass fixed many major issues, but several checked-off TODO items were not actually complete when compared against the code. FIX3 focuses only on the remaining correctness, observability, provider-secret, and test-hardening issues.

The goal is not to add new poker features. The goal is to eliminate misleading success states and remaining quiet failures.

## Priority scale

- **P0**: Must fix before more feature work. These issues can corrupt gameplay state, preserve/reuse the wrong secret, or make failed backend operations look successful.
- **P1**: Important hardening or observability issue. Should fix before release.
- **P2**: Test/documentation/cleanup issue. Should fix after P0/P1 unless it blocks validation.

## Global rule: accepted state must only be recorded after acceptance

For gameplay mutations, logs/history/debug state must distinguish:

- action selected,
- action attempted,
- action accepted by backend,
- action rejected,
- action not attempted due to stale/no runtime/no config.

Do not record rejected actions in accepted-action history.

Do not silently treat internal failures as normal user states.

## Required validation commands

Run frontend validation from the repository root:

```bash
npm run lint
npm run build
npm run test
```

Run Rust validation from the repository root:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Optional extended Rust validation:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

Do not replace these with `cd src-tauri` commands.

---

# P0 Requirements

## P0.1 NPC accepted-action hand log must be written only after backend acceptance

### Problem

The NPC runner can record an NPC action in `runner_state.hand_log` before `host_server.submit_action(...)` returns success.

If the backend rejects the action, the hand log can still contain the rejected action as if it happened. This is a misleading gameplay/history record.

### Required behavior

The hand/action log must represent accepted actions only.

For every NPC action:

1. choose action,
2. attempt submit,
3. if submit succeeds, write the accepted action to hand log exactly once,
4. if submit fails, write debug/error state only; do not write accepted-action hand log.

### Acceptance criteria

- Rejected NPC action does not create a hand-log entry.
- Accepted NPC action creates exactly one hand-log entry.
- LLM-success NPC action is logged exactly once after backend acceptance.
- LLM-fallback NPC action is logged exactly once after backend acceptance.
- Provider-missing profiled fallback action is logged exactly once after backend acceptance.
- Unprofiled NPC action is logged exactly once after backend acceptance.
- Tests prove rejected action is not logged as accepted.

---

## P0.2 NPC action failure state must cover all non-success outcomes

### Problem

`lastNpcActionError` exists, but only rejected `submit_action` errors are reliably surfaced. Other failure outcomes such as `StaleWindow`, `RuntimeUnavailable`, and `NoConfig` can remain invisible or only appear in logs.

### Required behavior

All meaningful non-success NPC outcomes must update structured debug state.

At minimum, record:

- `Rejected`,
- `StaleWindow`,
- `RuntimeUnavailable`,
- `NoConfig`,
- `ProviderStateUnavailable` if used,
- invalid/missing action-window context if applicable.

Each record should include:

- player ID if known,
- attempted action if one was selected,
- failure reason,
- hand number or sequence if available,
- timestamp or monotonic counter,
- whether the action was submitted or not submitted.

### Acceptance criteria

- `Rejected` updates `lastNpcActionError`.
- `StaleWindow` updates `lastNpcActionError`.
- `RuntimeUnavailable` updates `lastNpcActionError`.
- `NoConfig` updates `lastNpcActionError`.
- DebugPanel renders these errors.
- Errors are not only visible through stderr.
- Tests cover at least rejected, stale-window, and runtime-unavailable/no-config paths.

---

## P0.3 Provider Settings must preserve API keys only for the same provider identity

### Problem

The Settings screen can use a coarse `providerConfigured` boolean to decide whether to preserve an existing API key. That can accidentally preserve or reuse a key when switching providers, such as Anthropic to OpenAI, or when switching from an API-key provider to a local provider.

This can cause wrong-key reuse and stale keychain state.

### Required behavior

API-key preservation must be provider-aware.

Rules:

- Editing non-secret fields for the same API-key provider may preserve the existing key if the user leaves the key field blank.
- Switching from Anthropic to OpenAI must not preserve Anthropic’s key as OpenAI’s key.
- Switching from OpenAI to Anthropic must not preserve OpenAI’s key as Anthropic’s key.
- Switching to a local provider must detach active provider state from old API-key secrets.
- Switching away from an API-key provider should either delete the old key or preserve it only under a clearly documented per-provider account policy.
- The active runtime provider must never hold the previous provider’s key after provider type changes.

### Acceptance criteria

- Same-provider non-secret edit preserves existing key.
- Anthropic -> OpenAI with blank key does not reuse Anthropic key.
- OpenAI -> Anthropic with blank key does not reuse OpenAI key.
- API-key provider -> Ollama/llama-server clears active API-key state.
- Local provider save does not attempt to read/use old key.
- Tests cover provider switching and same-provider edits.
- Frontend UI makes required new-key behavior clear when switching API-key providers.

---

## P0.4 Release keychain migration must fail visibly and preserve recoverability

### Problem

Old combined provider config migration can attempt to move an embedded plaintext API key into keychain. If keychain write fails, the current code can log to stderr and continue. It may also rewrite the settings file without the embedded key, risking key loss after restart.

### Required behavior

In release builds:

- If migration from embedded/plaintext key to keychain fails, return a visible provider config error.
- Do not rewrite/remove the embedded key from config unless keychain write succeeds.
- Do not silently continue with only an in-memory key.
- User must see that secure migration failed and must have a recoverable path.

### Acceptance criteria

- Simulated keychain migration failure returns provider config error.
- Failed migration does not rewrite config to remove the old embedded key.
- Successful migration removes the embedded key from settings file.
- Error is visible through bootstrap/settings state.
- No release-path migration failure is only logged to stderr.

---

## P0.5 Clearing provider config must remove relevant old secrets even if settings JSON is corrupt

### Problem

Clearing provider config can derive the provider to clear by parsing `llm-provider.json`. If that file is corrupt or unreadable, provider detection can fail and keychain entries may remain.

### Required behavior

Clearing provider config must remove relevant stored secrets even when settings JSON is corrupt.

Acceptable strategies:

- track current provider type in memory and pass it to clear,
- attempt deletion for all known keychain accounts,
- maintain a small key index if needed.

Do not silently skip secret deletion because settings JSON is corrupt.

### Acceptance criteria

- Clearing valid Anthropic config deletes Anthropic key.
- Clearing valid OpenAI config deletes OpenAI key.
- Clearing corrupt settings attempts deletion of all known API-provider secrets or otherwise guarantees stale active secrets are removed.
- Keychain delete failures are surfaced or logged with enough visibility.
- Tests cover corrupt settings clear path.

---

# P1 Requirements

## P1.1 Profiled NPC fallback style must be derived from profile style in every fallback branch

### Problem

Profiled NPC fallback behavior is inconsistent. Some fallback branches use `NpcConfig.style`, while LLM request/parse failure branches use `NpcProfile.style`.

This means a profiled NPC can behave differently depending only on why LLM failed.

### Required behavior

If an NPC has an explicit profile, all fallback branches must derive style from the profile.

If an NPC has no profile, fallback may derive style from `NpcConfig.style`.

Fallback branches include:

- provider missing,
- provider state unavailable,
- LLM client construction failure,
- LLM request failure,
- LLM response parse failure,
- invalid LLM action,
- timeout if applicable.

### Acceptance criteria

- Centralized style resolution exists or all branches demonstrably use the same helper.
- Profiled NPC provider-missing fallback uses profile style.
- Profiled NPC parse-failure fallback uses profile style.
- Profiled NPC client-construction-failure fallback uses profile style.
- Unprofiled NPC fallback uses config style.
- Tests cover at least two failure branches for the same profiled NPC.

---

## P1.2 Bootstrap provider lock failures must not degrade to “not configured/no error”

### Problem

Some bootstrap state refresh paths can use `.lock().ok()` on provider-related mutexes. If a mutex is poisoned, bootstrap can silently report no provider/no error.

Lock poisoning is an internal failure, not normal “not configured” state.

### Required behavior

Provider lock failures must surface as provider state errors.

The app may continue running, but the bootstrap/settings state must not lie.

### Acceptance criteria

- Provider mutex poison/lock failure is not converted to `llmProviderType = null` with no error.
- Bootstrap exposes a provider state unavailable/config error.
- Tests cover lock failure if practical, or logic is factored so failure mapping can be unit-tested.
- No provider-related `.lock().ok()` silently drops errors in bootstrap/settings paths.

---

## P1.3 Profile list directory-entry errors must be surfaced

### Problem

Profile listing now returns profile parse/read errors, but directory iterator errors can still be silently discarded by `entries.flatten()`.

### Required behavior

Directory entry iteration errors must be collected and returned as profile list errors.

### Acceptance criteria

- `entries.flatten()` is not used for profile listing if it discards errors.
- Directory entry errors become visible warnings/errors.
- Valid profiles remain usable when some directory entries fail.
- Tests cover directory-entry error if feasible, or code structure makes it clear errors are not discarded.

---

## P1.4 LLM client must not quietly use empty API keys for key-required providers

### Problem

The LLM client can use `unwrap_or_default()` for API key access. This is only safe if every caller reliably checks usability before making a request. It is safer for the client itself to reject missing API keys for key-required providers.

### Required behavior

For Anthropic/OpenAI or other key-required providers:

- missing key must produce an explicit error before HTTP request,
- empty string key must be treated as missing,
- no HTTP call should be attempted with an empty key.

### Acceptance criteria

- Anthropic call with missing key returns explicit client error.
- OpenAI call with missing key returns explicit client error.
- No `unwrap_or_default()` for key-required API credentials.
- Tests cover missing-key client behavior.

---

## P1.5 Keychain/secret operation failures must not be swallowed

### Problem

Some keychain write/delete/read failures may be logged but not surfaced to the command caller or UI.

### Required behavior

Secret operation failures that affect current provider state must return command/provider errors.

Best-effort cleanup failures may be logged, but only if the active provider state is already safely cleared and no stale secret can be reused.

### Acceptance criteria

- Keychain write failure on save returns user-visible error.
- Keychain read failure on active provider returns provider config error.
- Keychain delete failure on clear is surfaced or clearly documented as non-blocking only if safe.
- Tests cover write/read/delete failure paths where feasible.

---

# P2 Requirements

## P2.1 Remove vacuous NPC tests

### Problem

Some NPC tests can pass without exercising the intended branch. For example, a test may return early if the human, not the NPC, receives the first action window.

### Required behavior

Tests named for a specific branch must force that branch or fail.

### Acceptance criteria

- No test returns early and passes because the desired NPC action window did not appear.
- NPC stale-window/fallback tests force an NPC-owned action window.
- If setup cannot produce the branch, test fails with clear assertion.

---

## P2.2 Make timeout tests deterministic

### Problem

Some timeout tests allow either success or timeout. That does not prove the timeout path.

### Required behavior

Tests named as timeout tests must deterministically produce timeout.

Use a fake runtime/no-ack path or equivalent.

### Acceptance criteria

- Timeout tests assert timeout error.
- Tests do not accept either `Ok` or `Err`.
- Test names match the branch being verified.

---

## P2.3 Full validation and manual smoke test must be re-run after FIX3

### Required behavior

After FIX3, rerun:

```bash
npm run lint
npm run build
npm run test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Manual smoke test:

- host tournament,
- join second instance,
- claim seats,
- ready players,
- start table,
- play at least one full hand,
- verify private cards isolated,
- verify observer visibility,
- verify rejected NPC action does not appear in accepted hand log,
- verify NPC action error debug display,
- verify provider switch does not reuse wrong key,
- verify provider clear removes relevant key,
- verify corrupt provider config repair.

### Acceptance criteria

- Automated validation passes locally.
- Manual smoke test passes.
- Any unverified step is explicitly documented.
