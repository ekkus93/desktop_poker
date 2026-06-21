# Desktop Poker Stabilization FIX3 TODO

This TODO implements `DESKTOP_POKER_STABILIZATION_FIX3_SPEC.md`.

Each task has a priority:

- **P0**: must fix before more feature work.
- **P1**: important before release.
- **P2**: test/documentation/cleanup.

Global rule: do not make failed operations look successful.

---

# P0 Tasks

## P0.1 Move NPC hand-log writes after successful backend acceptance

### Priority

P0

### Target files

- `src-tauri/src/npc/runner/mod.rs`
- `src-tauri/src/npc/runner/action.rs`
- hand/action log data structures used by NPC runner
- related NPC runner tests

### Implementation tasks

- [x] Find every place `runner_state.hand_log` is appended for NPC actions.
- [x] Identify any hand-log write that occurs before `host_server.submit_action(...)`.
- [x] Refactor action flow so selected action is not recorded as accepted until submit succeeds.
- [x] On `submit_action Ok(())`, append accepted action to hand log exactly once.
- [x] On `submit_action Err(...)`, do not append accepted action to hand log.
- [x] Preserve enough attempted-action details for debug error state.
- [x] Ensure retries do not duplicate accepted hand-log entries.
- [x] Ensure existing LLM success, LLM fallback, provider-missing fallback, and unprofiled paths all use the same accepted-action logging path.

### Tests

- [x] Rejected NPC action does not append to hand log.
- [x] Accepted unprofiled NPC action appends exactly once.
- [x] Accepted profiled LLM-success action appends exactly once.
- [x] Accepted profiled LLM-fallback action appends exactly once.
- [x] Accepted provider-missing profiled fallback action appends exactly once.
- [x] Retry after rejection does not duplicate accepted-action log.

### Acceptance

- [x] Rejected NPC actions are never represented as accepted hand-log actions.
- [x] All accepted NPC actions are logged exactly once.

---

## P0.2 Record all meaningful NPC non-success outcomes in debug state

### Priority

P0

### Target files

- `src-tauri/src/npc/runner/mod.rs`
- `src-tauri/src/npc/runner/action.rs`
- `src-tauri/src/app_state/debug.rs`
- `src/api/desktop.ts`
- `src/components/debug/DebugPanel.tsx`
- `src/components/debug/DebugPanel.test.tsx`
- related fixtures/tests

### Implementation tasks

- [x] Audit `NpcActionOutcome`.
- [x] Ensure `Rejected` records `lastNpcActionError`.
- [x] Ensure `StaleWindow` records `lastNpcActionError`.
- [x] Ensure `RuntimeUnavailable` records `lastNpcActionError`.
- [x] Ensure `NoConfig` records `lastNpcActionError`.
- [x] Ensure provider-state-unavailable errors record a distinct reason if applicable.
- [x] Include whether action was submitted or not submitted.
- [x] Include player ID, action if selected, hand/sequence if available, and timestamp/counter.
- [x] Update TypeScript `DebugInspectorState`.
- [x] Update DebugPanel rendering and fixtures.
- [x] Define whether later success clears the error or leaves it as “last error”; implement consistently.

### Tests

- [x] Rejected outcome appears in debug state.
- [x] StaleWindow outcome appears in debug state.
- [x] RuntimeUnavailable or NoConfig outcome appears in debug state.
- [x] DebugPanel renders all supported error reasons.
- [x] Successful action clear/supersede behavior is tested.

### Acceptance

- [x] NPC action failure diagnostics do not depend on stderr.

---

## P0.3 Make API-key preservation provider-aware in Settings and backend

### Priority

P0

### Target files

- `src/screens/DeviceSettingsScreen.tsx`
- `src/screens/DeviceSettingsScreen.test.tsx`
- `src/api/desktop.ts`
- `src-tauri/src/app_state/llm_provider.rs`
- `src-tauri/src/npc/provider_storage.rs`
- related provider tests

### Implementation tasks

- [x] Track loaded provider settings separately from currently selected form provider.
- [x] Preserve existing key only when selected provider matches loaded provider and backend confirms key exists for that provider.
- [x] When switching Anthropic -> OpenAI with blank key, require a new OpenAI key or return visible error.
- [x] When switching OpenAI -> Anthropic with blank key, require a new Anthropic key or return visible error.
- [x] When switching to Ollama/llama-server, active API-key state must be cleared/detached.
- [x] Backend `save_llm_provider_settings` or equivalent must not preserve an old in-memory key across provider type changes.
- [x] Decide and implement stale old-key cleanup policy:
  - [x] delete old provider key when switching away, or
  - [x] keep per-provider key only if explicitly documented and never attached to the wrong provider.
- [x] Make UI text clear when switching key-required providers requires a new key.

### Tests

- [x] Same-provider endpoint/model edit with blank key preserves key.
- [x] Anthropic -> OpenAI with blank key does not reuse Anthropic key.
- [x] OpenAI -> Anthropic with blank key does not reuse OpenAI key.
- [x] API-key provider -> local provider clears active API-key state.
- [x] Local provider save does not read/use old key.
- [x] Backend provider switch does not keep old key in current in-memory config.

### Acceptance

- [x] Wrong-provider API key reuse is impossible.

---

## P0.4 Make release keychain migration failure visible and non-destructive

### Priority

P0

### Target files

- `src-tauri/src/npc/provider_storage.rs`
- provider config load-state types
- `src-tauri/src/app_state/llm_provider.rs`
- frontend bootstrap/settings provider error display
- provider storage tests

### Implementation tasks

- [x] Find old combined config migration logic for embedded `apiKey`.
- [x] In release builds, if keychain write fails, return a provider config/key migration error.
- [x] Do not rewrite settings file to remove embedded `apiKey` unless keychain write succeeds.
- [x] Ensure failed migration is visible in bootstrap/settings state.
- [x] Ensure successful migration removes embedded key from settings file.
- [x] Ensure debug/dev plaintext migration behavior remains explicit and tested.
- [x] Avoid stderr-only migration failure handling.

### Tests

- [x] Successful release migration writes key to keychain and removes embedded key from settings.
- [x] Failed release migration returns provider error.
- [x] Failed release migration does not remove embedded key from settings file.
- [x] Settings UI shows migration failure.
- [x] No release migration failure is only logged.

### Acceptance

- [x] Keychain migration cannot silently lose the API key.

---

## P0.5 Clear provider secrets robustly even when settings JSON is corrupt

### Priority

P0

### Target files

- `src-tauri/src/npc/provider_storage.rs`
- `src-tauri/src/app_state/llm_provider.rs`
- provider storage tests

### Implementation tasks

- [x] Audit `clear_provider_config` / clear secret path.
- [x] Stop relying only on parsing current settings JSON to know what key to delete.
- [x] On clear, delete all known API-key provider secrets if current provider cannot be parsed.
- [x] Known API-key provider accounts should include Anthropic and OpenAI.
- [x] Ensure local providers without keys are unaffected.
- [x] Surface keychain delete failures if they leave active/stale secrets.
- [x] Ensure clearing corrupt settings file removes settings file and relevant secrets.

### Tests

- [x] Clear valid Anthropic config deletes Anthropic secret.
- [x] Clear valid OpenAI config deletes OpenAI secret.
- [x] Clear corrupt settings attempts/deletes known API-provider secrets.
- [x] Keychain delete failure is surfaced or safely handled according to documented policy.
- [x] Clearing local provider does not fail because no API key exists.

### Acceptance

- [x] Corrupt settings cannot prevent secret cleanup.

---

# P1 Tasks

## P1.1 Centralize profiled NPC fallback style resolution

### Priority

P1

### Target files

- `src-tauri/src/npc/runner/mod.rs`
- `src-tauri/src/npc/runner/action.rs`
- `src-tauri/src/npc/llm_strategy.rs`
- NPC profile/style mapping tests

### Implementation tasks

- [x] Add helper that resolves fallback style:
  - [x] profile present -> derive from `NpcProfile.style`,
  - [x] profile absent -> use `NpcConfig.style`.
- [x] Use helper in provider-missing fallback.
- [x] Use helper in provider-state-unavailable fallback.
- [x] Use helper in client-construction-failure fallback.
- [x] Use helper in LLM request failure fallback.
- [x] Use helper in LLM parse failure fallback.
- [x] Use helper in invalid-action fallback.
- [x] Remove duplicated inconsistent style-source logic.
- [x] Document mapping for profile style strings that do not exactly match enum variants.

### Tests

- [x] Profiled provider-missing fallback uses profile style.
- [x] Profiled parse-failure fallback uses same profile style source.
- [x] Profiled client-construction-failure fallback uses profile style.
- [x] Unprofiled fallback uses config style.
- [x] Distinct profile styles produce distinct fallback choices where legal actions allow it.

### Acceptance

- [x] Profiled NPC fallback behavior is consistent across LLM failure reasons.

---

## P1.2 Surface provider lock failures in bootstrap/settings state

### Priority

P1

### Target files

- `src-tauri/src/app_state/mod.rs`
- `src-tauri/src/app_state/llm_provider.rs`
- bootstrap state tests
- TypeScript bootstrap types if needed

### Implementation tasks

- [x] Search provider/bootstrap paths for `.lock().ok()`.
- [x] Replace silent lock-drop behavior with explicit error handling.
- [x] If provider config mutex is poisoned, set provider config/state error.
- [x] If provider error mutex is poisoned, set provider config/state error.
- [x] Ensure bootstrap does not report clean “not configured” for internal lock failure.
- [x] Add helper to convert lock errors into provider state errors.

### Tests

- [x] Unit-test lock error mapping if direct mutex poisoning test is practical.
- [x] Otherwise test helper function with simulated failure.
- [x] Bootstrap state reports provider state unavailable error.

### Acceptance

- [x] Internal provider lock failure is visible.

---

## P1.3 Surface profile directory-entry errors

### Priority

P1

### Target files

- `src-tauri/src/npc/profile_store.rs`
- profile list tests
- frontend profile list rendering if error shape changes

### Implementation tasks

- [x] Find `entries.flatten()` or equivalent.
- [x] Replace with explicit `for entry_result in entries` match.
- [x] Push directory-entry errors into profile list error collection.
- [x] Keep valid profiles visible.
- [x] Include enough path/context for diagnosis.
- [x] Avoid stderr-only handling.

### Tests

- [x] Directory-entry error is collected if feasible.
- [x] Valid profiles remain visible when some entries fail.
- [x] Existing corrupt-file tests still pass.

### Acceptance

- [x] Profile list does not silently discard directory iteration errors.

---

## P1.4 Reject missing/empty API keys inside LLM client

### Priority

P1

### Target files

- `src-tauri/src/npc/llm_client.rs`
- LLM client tests
- LLM strategy caller code

### Implementation tasks

- [x] Find `unwrap_or_default()` on API key.
- [x] Replace with explicit missing-key error for key-required providers.
- [x] Treat empty/whitespace-only key as missing.
- [x] Ensure no HTTP request is attempted with missing key.
- [x] Keep local provider behavior unaffected if no key is required.
- [x] Update caller code to surface missing-key error as fallback reason.

### Tests

- [x] Anthropic missing key returns explicit error before HTTP.
- [x] OpenAI missing key returns explicit error before HTTP.
- [x] Empty string key returns explicit error.
- [x] Local provider without key still works if applicable.

### Acceptance

- [x] No key-required provider silently uses empty API key.

---

## P1.5 Do not swallow active keychain operation failures

### Priority

P1

### Target files

- `src-tauri/src/npc/provider_storage.rs`
- `src-tauri/src/app_state/llm_provider.rs`
- provider storage tests

### Implementation tasks

- [x] Audit keychain read/write/delete error handling.
- [x] Keychain write failure on save must return error.
- [x] Keychain read failure for active provider must return provider config error.
- [x] Keychain delete failure on clear must be surfaced unless proven safe.
- [x] Remove stderr-only handling for active secret operations.
- [x] Keep best-effort cleanup clearly documented if any remains.

### Tests

- [x] Simulated keychain write failure returns command/provider error.
- [x] Simulated keychain read failure returns provider error.
- [x] Simulated keychain delete failure behavior matches documented policy.

### Acceptance

- [x] Active secret operation failures are not swallowed.

---

# P2 Tasks

## P2.1 Remove vacuous NPC test pass conditions

### Priority

P2

### Target files

- `src-tauri/src/networking/runtime/tests/tournament.rs`
- NPC runner tests
- any test with early return on missing NPC window

### Implementation tasks

- [x] Search for tests that return early when NPC action window is not reached.
- [x] Replace early return with assertion failure.
- [x] Force an NPC-owned action window in tests that require one.
- [x] Make failure messages explain setup issue.
- [x] Ensure test names match actual branch.

### Tests

- [x] The specific provider-missing profiled NPC fallback test fails if NPC window is not reached.
- [x] Stale-window tests force NPC-owned window.

### Acceptance

- [x] No NPC test passes without exercising the intended NPC branch.

---

## P2.2 Make timeout tests deterministic

### Priority

P2

### Target files

- `src-tauri/src/app_state/tests/sessions_npc.rs`
- session/runtime timeout tests
- frontend timeout tests if applicable

### Implementation tasks

- [x] Search for timeout tests that accept either `Ok` or `Err`.
- [x] Introduce fake/no-ack runtime path or fixture.
- [x] Assert exact timeout error.
- [x] Rename tests that are actually “state eventually consistent” rather than timeout tests.
- [x] Ensure no timeout test passes through success branch.

### Acceptance

- [x] Timeout tests deterministically prove timeout behavior.

---

## P2.3 Re-run full local validation and document any unverified items

### Priority

P2

### Tasks

- [x] Run with Node 24.x:

```bash
npm run lint
npm run build
npm run test
```

- [x] Run Rust validation:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

- [x] Optional extended Rust validation:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

- [ ] Manual smoke test:
  - [ ] host tournament,
  - [ ] join second instance,
  - [ ] claim seats,
  - [ ] ready players,
  - [ ] start table,
  - [ ] play one full hand,
  - [ ] verify private cards isolated,
  - [ ] verify observer visibility,
  - [ ] verify rejected NPC action does not appear in accepted hand log,
  - [ ] verify NPC action errors in DebugPanel,
  - [ ] verify same-provider Settings edit preserves key,
  - [ ] verify provider switch does not reuse wrong key,
  - [ ] verify provider clear removes relevant secret,
  - [ ] verify corrupt provider config repair.

### Acceptance

- [x] Automated checks pass.
- [ ] Manual smoke test passes.
- [x] Any unverified item is explicitly documented rather than checked off.

---

# Claude Code guardrails

Do not mark a TODO item complete unless the code and tests actually prove it.

Do not use these patterns to make tests pass:

```rust
.ok()
.unwrap_or_default()
let _ = important_result;
or_else(|| unrelated_default)
```

```ts
catch {
  // ignored
}
```

Do not record attempted/rejected actions in accepted gameplay history.

Do not preserve an API key across provider changes unless it is explicitly scoped to the same provider identity.

Do not rewrite old plaintext key config during migration unless secure keychain write succeeded.

Do not let tests pass vacuously when the intended branch did not execute.
