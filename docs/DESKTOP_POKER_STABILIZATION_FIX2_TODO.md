# Desktop Poker Stabilization FIX2 TODO

This TODO implements `DESKTOP_POKER_STABILIZATION_FIX2_SPEC.md`.

Each task has a priority:

- **P0**: must fix before more feature work.
- **P1**: important before release.
- **P2**: cleanup/test/docs/maintenance.

Global rule: do not implement invisible fallback behavior. Any fallback must be explicit, visible, and tested.

---

# P0 Tasks

## P0.1 Fix stale `providerConfigError` after provider save/clear

### Target files

- `src-tauri/src/app_state/mod.rs`
- `src-tauri/src/app_state/llm_provider.rs`
- `src-tauri/src/app_state/provider_storage.rs`
- `src/api/desktop.ts`
- `src/app/DesktopBootstrapProvider.tsx`
- `src/app/useDesktopBootstrap.tsx`
- `src/screens/DeviceSettingsScreen.tsx`
- related bootstrap/provider tests

### Implementation tasks

- [ ] Trace where `providerConfigError` is computed during startup.
- [ ] Trace where bootstrap state is cloned/refreshed after save/clear.
- [ ] Ensure `bootstrap()` returns current provider error state, not startup-only error state.
- [ ] Ensure save valid provider config clears existing provider config error.
- [ ] Ensure clear provider config clears existing provider config error.
- [ ] Ensure emitted `desktop://bootstrap` payload includes current `providerConfigError`.
- [ ] Ensure frontend bootstrap context updates `providerConfigError` from event payload.
- [ ] Do not paper over errors by always setting `providerConfigError = null`; only clear it when provider state is actually valid/missing by intent.

### Tests

- [ ] Rust/backend test: corrupt config at startup produces provider error.
- [ ] Rust/backend test: corrupt config then save valid config clears provider error.
- [ ] Rust/backend test: corrupt config then clear config clears provider error.
- [ ] Frontend test: bootstrap event clears Settings provider error after save.
- [ ] Frontend test: bootstrap event clears Settings provider error after clear.

### Acceptance

- [ ] No stale provider config error remains after repair.
- [ ] `npm run test` passes.
- [ ] `npm run build` passes.

---

## P0.2 Treat unreadable API-key file as a provider/secret error

### Target files

- `src-tauri/src/app_state/provider_storage.rs`
- `src-tauri/src/app_state/llm_provider.rs`
- provider config load-state types
- frontend provider/bootstrap type definitions
- `src/screens/DeviceSettingsScreen.tsx`
- related tests

### Implementation tasks

- [ ] Find key-file read function.
- [ ] Replace `fs::read_to_string(path).ok()?` or equivalent silent failure.
- [ ] Distinguish missing key file from unreadable existing key file.
- [ ] Add provider load state for unreadable secret/key file.
- [ ] Surface unreadable key file through bootstrap/settings state.
- [ ] Add clear/retry path if available.
- [ ] Ensure missing key still means “API key missing,” not config corruption.
- [ ] Ensure unreadable key means “secret file unreadable” or equivalent clear diagnostic.

### Tests

- [ ] Missing key file with key-required provider reports missing key.
- [ ] Existing unreadable key file reports secret/config error.
- [ ] Valid key file reports key configured.
- [ ] Clearing provider removes key error.

### Acceptance

- [ ] Unreadable key file is never silently treated as “not configured.”

---

## P0.3 Load existing non-secret provider config into Settings

### Target files

- `src/screens/DeviceSettingsScreen.tsx`
- `src/api/desktop.ts`
- `src/api/desktop.test.ts`
- `src/screens/DeviceSettingsScreen.test.tsx`
- provider config types

### Implementation tasks

- [ ] On Settings mount, call `getLlmProviderConfig()`.
- [ ] Populate provider type from loaded config.
- [ ] Populate endpoint URL from loaded config.
- [ ] Populate model from loaded config.
- [ ] Populate any other non-secret provider fields.
- [ ] Do not populate API-key text input with stored secret.
- [ ] Preserve existing API key when user edits non-secret fields and leaves API-key field blank.
- [ ] Provide explicit “replace key” and/or “clear key” semantics if not already present.
- [ ] Show visible error if loading config fails.

### Tests

- [ ] Existing provider config appears in Settings fields.
- [ ] API-key input remains blank/redacted.
- [ ] Saving endpoint/model without entering key preserves existing key.
- [ ] Config load failure displays visible error.
- [ ] Switching provider handles stale endpoint/model intentionally.

### Acceptance

- [ ] User cannot accidentally erase endpoint/model or key by opening Settings and saving unchanged blank fields.

---

## P0.4 Ensure profiled NPC fallback actions are hand-logged exactly once

### Target files

- `src-tauri/src/npc/runner/mod.rs`
- hand log/opponent stats modules
- LLM strategy/fallback modules
- related Rust tests

### Implementation tasks

- [ ] Find all NPC action logging paths.
- [ ] Identify where profiled NPC provider-missing fallback submits action.
- [ ] Replace `if npc_config.profile.is_none()` logging gate with explicit `action_logged` tracking.
- [ ] Ensure LLM-success path logs once.
- [ ] Ensure LLM-failure fallback path logs once.
- [ ] Ensure provider-missing fallback path logs once.
- [ ] Ensure unprofiled NPC path logs once.
- [ ] Avoid duplicate entries on retries/rejections.

### Tests

- [ ] Profiled NPC + provider missing -> fallback action logged once.
- [ ] Profiled NPC + LLM parse failure -> fallback action logged once.
- [ ] Profiled NPC + LLM success -> action logged once.
- [ ] Unprofiled NPC -> action logged once.
- [ ] Rejected NPC action does not create misleading successful hand-log entry.

### Acceptance

- [ ] Every accepted NPC action is represented in hand/action log exactly once.

---

## P0.5 Surface NPC action submission failures in debug state

### Target files

- `src-tauri/src/npc/runner/mod.rs`
- `src-tauri/src/app_state/debug.rs`
- `src-tauri/src/app_state/mod.rs`
- `src/api/desktop.ts`
- `src/components/debug/DebugPanel.tsx`
- `src/components/debug/DebugPanel.test.tsx`
- Rust tests for NPC action outcomes

### Implementation tasks

- [ ] Add structured debug field, for example `lastNpcActionError`.
- [ ] Include:
  - [ ] player ID,
  - [ ] attempted action,
  - [ ] failure reason,
  - [ ] sequence/hand number if available,
  - [ ] timestamp or monotonic counter.
- [ ] Update `DebugInspectorState` TypeScript type.
- [ ] Update fixtures/tests for new debug field.
- [ ] Update `try_npc_action()` / runner loop to store rejected/stale/no-config outcomes.
- [ ] Render latest NPC action error in DebugPanel.
- [ ] Ensure error clears or updates on later success according to a deliberate policy.

### Tests

- [ ] Rejected NPC action updates debug state.
- [ ] Stale NPC action updates debug state.
- [ ] DebugPanel renders NPC action error.
- [ ] Successful later NPC action either clears or supersedes error based on documented behavior.

### Acceptance

- [ ] NPC action failure is not only visible through stderr.

---

# P1 Tasks

## P1.1 Log or surface NPC runner thread panic

### Target files

- `src-tauri/src/npc/runner/mod.rs`
- debug state if surfacing beyond logs

### Implementation tasks

- [ ] Find `NpcRunnerGuard::drop()`.
- [ ] Replace `let _ = handle.join()` with checked handling.
- [ ] Log panic with useful context.
- [ ] If debug state is accessible, store panic summary in debug state.
- [ ] Avoid panicking from `Drop`.

### Tests

- [ ] Unit test if possible for join panic handling.
- [ ] Otherwise add code comment explaining why direct test is impractical and verify no ignored join result remains.

### Acceptance

- [ ] Runner thread panic is not silently swallowed.

---

## P1.2 Distinguish provider mutex poisoning from provider missing

### Target files

- `src-tauri/src/npc/runner/mod.rs`
- `src-tauri/src/npc/llm_strategy.rs`
- fallback reason enum/type
- debug state/tests

### Implementation tasks

- [ ] Find provider mutex lock sites.
- [ ] Replace `.lock().ok().and_then(...)` with explicit match.
- [ ] Add fallback/error reason for provider state unavailable.
- [ ] Record this reason in LLM fallback debug state.
- [ ] Keep rule-based fallback allowed so gameplay does not freeze.
- [ ] Do not label poison/lock failure as provider not configured.

### Tests

- [ ] Test fallback reason mapping if lock poisoning can be simulated.
- [ ] Otherwise test helper function that maps lock result to fallback reason.

### Acceptance

- [ ] Internal provider-state failure is visible and distinct from normal missing provider config.

---

## P1.3 Make profiled NPC fallback style source consistent

### Target files

- `src-tauri/src/npc/runner/mod.rs`
- `src-tauri/src/npc/llm_strategy.rs`
- NPC profile/style mapping code
- related tests

### Implementation tasks

- [ ] Identify all fallback branches:
  - [ ] provider missing,
  - [ ] provider state unavailable,
  - [ ] client construction failure,
  - [ ] request failure,
  - [ ] response parse failure,
  - [ ] invalid action.
- [ ] For profiled NPCs, derive fallback style from profile style in all branches.
- [ ] For unprofiled NPCs, derive fallback style from config style.
- [ ] Centralize style resolution in one helper if possible.
- [ ] Remove duplicated style-selection logic.

### Tests

- [ ] Same profiled NPC uses same style source for provider missing and parse failure.
- [ ] Unprofiled NPC uses config style.
- [ ] Distinct profile styles can produce distinct fallback choices.

### Acceptance

- [ ] Profiled NPC behavior is not dependent on why LLM failed.

---

## P1.4 Return profile list errors instead of silently skipping bad profiles

### Target files

- `src-tauri/src/npc/profile_store.rs`
- `src/api/desktop.ts`
- `src/screens/NpcProfilesScreen.tsx`
- `src/screens/HostTournamentSetupScreen.tsx`
- related tests

### Implementation tasks

- [ ] Change profile list API to include valid profiles and errors, or add separate diagnostic API.
- [ ] Include filename/path and error message for invalid profile files.
- [ ] Display warnings in NpcProfilesScreen.
- [ ] Display warnings or disabled options in Host setup profile picker.
- [ ] Keep valid profiles usable even when some files are bad.
- [ ] Avoid only `eprintln!` for corrupt profiles.

### Tests

- [ ] Valid profiles display.
- [ ] Corrupt profile file surfaces visible warning.
- [ ] Host setup profile picker shows profile-list warning.
- [ ] NpcProfilesScreen shows profile-list warning.

### Acceptance

- [ ] Bad profile files do not disappear silently.

---

## P1.5 Show error when host-session status check fails

### Target files

- `src/screens/HostTournamentSetupScreen.tsx`
- `src/api/desktop.ts`
- `src/screens/HostTournamentSetupScreen.test.tsx`

### Implementation tasks

- [ ] Find `getHostSessionStatus().catch(...)`.
- [ ] Stop converting command failure into `hostSession = null`.
- [ ] Add explicit error state for session status load failure.
- [ ] Show retry action if appropriate.
- [ ] Keep true no-session case as normal setup UI.
- [ ] Make error message distinct from “no active host session.”

### Tests

- [ ] `getHostSessionStatus()` returns null -> normal setup UI.
- [ ] `getHostSessionStatus()` rejects -> visible error.
- [ ] Retry invokes status load again.
- [ ] Existing host session still displays correctly.

### Acceptance

- [ ] Backend failure is not mistaken for no session.

---

## P1.6 Show profile list loading failure in Host setup

### Target files

- `src/screens/HostTournamentSetupScreen.tsx`
- `src/screens/HostTournamentSetupScreen.test.tsx`

### Implementation tasks

- [ ] Find `listNpcProfiles().catch(() => {})`.
- [ ] Add `profileListError` state.
- [ ] Display warning near AI/NPC profile controls.
- [ ] Keep empty valid profile list distinct from load failure.
- [ ] Ensure warning does not block unprofiled NPCs unless required.

### Tests

- [ ] Profile list load failure warning appears.
- [ ] Empty profile list without error does not show warning.
- [ ] User can still add unprofiled NPC if allowed.

### Acceptance

- [ ] Profile API failure is visible.

---

## P1.7 Make release-mode plaintext key policy explicit

### Target files

- `src-tauri/src/app_state/provider_storage.rs`
- `src-tauri/src/app_state/llm_provider.rs`
- `src/screens/DeviceSettingsScreen.tsx`
- `README.md`
- security/docs files
- tests for secret redaction/storage

### Policy decision

**Option A selected: OS keychain/keyring in release builds.**

Option C is not allowed. Option B is an explicit interim fallback only — stop and report if keychain integration is blocked rather than silently downgrading.

### Implementation tasks

- [ ] Implement OS keychain/keyring storage for API keys in release builds.
- [ ] Keep plaintext key-file storage in debug builds as a clearly marked insecure/dev-only mode.
- [ ] If keychain storage fails, surface a clear user-visible error — do not fall back to plaintext.
- [ ] Ensure local providers without API keys (Ollama, llama-server) continue to work without keychain.
- [ ] Ensure API keys never appear in debug state.
- [ ] Ensure API keys never appear in logs.
- [ ] Ensure clearing provider config removes the keychain entry.
- [ ] Ensure replacing a key updates the keychain entry.
- [ ] Ensure editing non-secret provider fields without entering a new key preserves the existing keychain entry.
- [ ] Add docs explaining current security policy.

### Tests

- [ ] API key redaction test.
- [ ] Save provider config does not expose key in returned config.
- [ ] Release/insecure mode behavior covered if feasible.
- [ ] Clearing provider removes/invalidates stored key.

### Acceptance

- [ ] Release builds do not silently store plaintext API keys.

---

## P1.8 Fix README/docs drift for provider storage, CSP, and validation commands

### Target files

- `README.md`
- `CLAUDE.md` only if explicitly needed
- `DESKTOP_POKER_STABILIZATION_SPEC.md`
- `DESKTOP_POKER_STABILIZATION_TODO.md`
- new FIX2 docs if added to repo

### Implementation tasks

- [ ] Update provider storage docs:
  - [ ] non-secret provider settings location,
  - [ ] key file or keychain location/policy,
  - [ ] no API key in JSON example.
- [ ] Update CSP docs to match `src-tauri/tauri.conf.json`.
- [ ] Update validation commands to project-root `--manifest-path` form.
- [ ] Document optional extended Rust test command separately.
- [ ] Remove stale `connect-src 'none'` claim if it no longer matches config.

### Acceptance

- [ ] Docs match actual implementation.
- [ ] Future Claude Code runs are not guided by stale docs.

---

# P2 Tasks

## P2.1 Delete legacy `npc/api_key.rs`

### Target files

- `src-tauri/src/npc/api_key.rs`
- `src-tauri/src/npc/mod.rs`
- any tests that only test the dead module

### Decision

Delete the module entirely. The module has no callers outside itself. Legacy migration from `claude-api-key.txt` is already handled in `src-tauri/src/npc/provider_storage.rs`. If compile or tests reveal a hidden dependency, stop and report rather than preserving the dead module.

### Implementation tasks

- [ ] Delete `src-tauri/src/npc/api_key.rs`.
- [ ] Remove `pub mod api_key;` from `src-tauri/src/npc/mod.rs`.
- [ ] Remove any tests that only test this dead module.
- [ ] Keep any required legacy migration logic inside `provider_storage.rs`.
- [ ] Verify no current production path reads or writes `claude-api-key.txt`.

### Tests

- [ ] Rust compile/tests pass after deletion.
- [ ] No current path writes `claude-api-key.txt`.

### Acceptance

- [ ] `src-tauri/src/npc/api_key.rs` is deleted.
- [ ] No two active API-key storage paths exist.

---

## P2.2 Strengthen deterministic timeout/stale-window tests

### Target files

- Rust session/runtime tests
- `src-tauri/src/app_state/session.rs` tests
- NPC runner tests
- frontend tests for timeout UI

### Implementation tasks

- [ ] Identify tests that allow either timeout or success.
- [ ] Replace with deterministic fake/no-ack path.
- [ ] Identify stale-window tests that can pass vacuously.
- [ ] Force action window to belong to NPC before stale-window assertion.
- [ ] Rename tests whose names do not match what they actually prove.

### Acceptance

- [ ] Timeout tests assert timeout branch.
- [ ] Stale-window tests assert stale-window branch.
- [ ] No vacuous pass conditions remain in P0 failure-path tests.

---

## P2.3 npm audit cleanup/classification

### Target files

- `package.json`
- `package-lock.json`
- docs/release notes if needed

### Implementation tasks

- [ ] Run `npm audit`.
- [ ] Classify each vulnerability:
  - [ ] runtime packaged app,
  - [ ] dev-only build/test tooling,
  - [ ] transitive but exploitable,
  - [ ] transitive and not exploitable in packaged app.
- [ ] Apply safe dependency updates.
- [ ] Avoid unsafe major upgrades without testing.
- [ ] Document any remaining vulnerabilities and why they are accepted temporarily.

### Tests

- [ ] `npm run lint`
- [ ] `npm run build`
- [ ] `npm run test`
- [ ] app launch smoke test if dependency upgrades touch Vite/Tauri/React Router

### Acceptance

- [ ] Audit status is known and documented.
- [ ] Runtime-relevant vulnerabilities are fixed or explicitly blocked from release.

---

## P2.4 Full local regression pass

### Tasks

- [ ] Use Node 24.x.
- [ ] Run:

```bash
npm run lint
npm run build
npm run test
```

- [ ] Run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] Optional extended Rust validation:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

- [ ] Manual smoke test:
  - [ ] host tournament,
  - [ ] join from second instance,
  - [ ] claim seats,
  - [ ] ready players,
  - [ ] start table,
  - [ ] play one full hand,
  - [ ] verify private cards isolated,
  - [ ] verify observer visibility,
  - [ ] verify NPC fallback/debug visibility,
  - [ ] verify provider Settings load/save/clear,
  - [ ] verify corrupt provider config repair,
  - [ ] verify profile list error display.

### Acceptance

- [ ] Automated validation passes.
- [ ] Manual multiplayer smoke test passes.
- [ ] No known P0/P1 silent fallback remains.

---

# Claude Code implementation guardrails

Do not “fix” failures by adding:

```rust
.ok()
```

```rust
.unwrap_or_default()
```

```rust
let _ = important_result;
```

```rust
or_else(|| unrelated_default)
```

```ts
.catch(() => {})
```

unless the TODO explicitly says the operation is best-effort and the failure is harmless.

For each P0/P1 task, include tests that fail before the fix and pass after the fix.

Prefer small focused commits/checkpoints:

1. Provider state correctness.
2. Settings provider config reload.
3. NPC action/fallback observability.
4. Profile list and host setup visible errors.
5. Release key-storage policy and docs.
6. Test hardening and cleanup.
