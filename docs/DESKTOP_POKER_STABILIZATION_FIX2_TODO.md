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

- [x] Trace where `providerConfigError` is computed during startup.
- [x] Trace where bootstrap state is cloned/refreshed after save/clear.
- [x] Ensure `bootstrap()` returns current provider error state, not startup-only error state.
- [x] Ensure save valid provider config clears existing provider config error.
- [x] Ensure clear provider config clears existing provider config error.
- [x] Ensure emitted `desktop://bootstrap` payload includes current `providerConfigError`.
- [x] Ensure frontend bootstrap context updates `providerConfigError` from event payload.
- [x] Do not paper over errors by always setting `providerConfigError = null`; only clear it when provider state is actually valid/missing by intent.

### Tests

- [x] Rust/backend test: corrupt config at startup produces provider error.
- [x] Rust/backend test: corrupt config then save valid config clears provider error.
- [x] Rust/backend test: corrupt config then clear config clears provider error.
- [x] Frontend test: bootstrap event clears Settings provider error after save.
- [x] Frontend test: bootstrap event clears Settings provider error after clear.

### Acceptance

- [x] No stale provider config error remains after repair.
- [x] `npm run test` passes.
- [x] `npm run build` passes.

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

- [x] Find key-file read function.
- [x] Replace `fs::read_to_string(path).ok()?` or equivalent silent failure.
- [x] Distinguish missing key file from unreadable existing key file.
- [x] Add provider load state for unreadable secret/key file.
- [x] Surface unreadable key file through bootstrap/settings state.
- [x] Add clear/retry path if available.
- [x] Ensure missing key still means “API key missing,” not config corruption.
- [x] Ensure unreadable key means “secret file unreadable” or equivalent clear diagnostic.

### Tests

- [x] Missing key file with key-required provider reports missing key.
- [x] Existing unreadable key file reports secret/config error.
- [x] Valid key file reports key configured.
- [x] Clearing provider removes key error.

### Acceptance

- [x] Unreadable key file is never silently treated as “not configured.”

---

## P0.3 Load existing non-secret provider config into Settings

### Target files

- `src/screens/DeviceSettingsScreen.tsx`
- `src/api/desktop.ts`
- `src/api/desktop.test.ts`
- `src/screens/DeviceSettingsScreen.test.tsx`
- provider config types

### Implementation tasks

- [x] On Settings mount, call `getLlmProviderConfig()`.
- [x] Populate provider type from loaded config.
- [x] Populate endpoint URL from loaded config.
- [x] Populate model from loaded config.
- [x] Populate any other non-secret provider fields.
- [x] Do not populate API-key text input with stored secret.
- [x] Preserve existing API key when user edits non-secret fields and leaves API-key field blank.
- [x] Provide explicit “replace key” and/or “clear key” semantics if not already present.
- [x] Show visible error if loading config fails.

### Tests

- [x] Existing provider config appears in Settings fields.
- [x] API-key input remains blank/redacted.
- [x] Saving endpoint/model without entering key preserves existing key.
- [x] Config load failure displays visible error.
- [x] Switching provider handles stale endpoint/model intentionally.

### Acceptance

- [x] User cannot accidentally erase endpoint/model or key by opening Settings and saving unchanged blank fields.

---

## P0.4 Ensure profiled NPC fallback actions are hand-logged exactly once

### Target files

- `src-tauri/src/npc/runner/mod.rs`
- hand log/opponent stats modules
- LLM strategy/fallback modules
- related Rust tests

### Implementation tasks

- [x] Find all NPC action logging paths.
- [x] Identify where profiled NPC provider-missing fallback submits action.
- [x] Replace `if npc_config.profile.is_none()` logging gate with explicit `action_logged` tracking.
- [x] Ensure LLM-success path logs once.
- [x] Ensure LLM-failure fallback path logs once.
- [x] Ensure provider-missing fallback path logs once.
- [x] Ensure unprofiled NPC path logs once.
- [x] Avoid duplicate entries on retries/rejections.

### Tests

- [x] Profiled NPC + provider missing -> fallback action logged once.
- [x] Profiled NPC + LLM parse failure -> fallback action logged once.
- [x] Profiled NPC + LLM success -> action logged once.
- [x] Unprofiled NPC -> action logged once.
- [x] Rejected NPC action does not create misleading successful hand-log entry.

### Acceptance

- [x] Every accepted NPC action is represented in hand/action log exactly once.

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

- [x] Add structured debug field, for example `lastNpcActionError`.
- [x] Include:
  - [x] player ID,
  - [x] attempted action,
  - [x] failure reason,
  - [x] sequence/hand number if available,
  - [x] timestamp or monotonic counter.
- [x] Update `DebugInspectorState` TypeScript type.
- [x] Update fixtures/tests for new debug field.
- [x] Update `try_npc_action()` / runner loop to store rejected/stale/no-config outcomes.
- [x] Render latest NPC action error in DebugPanel.
- [x] Ensure error clears or updates on later success according to a deliberate policy.

### Tests

- [x] Rejected NPC action updates debug state.
- [x] Stale NPC action updates debug state.
- [x] DebugPanel renders NPC action error.
- [x] Successful later NPC action either clears or supersedes error based on documented behavior.

### Acceptance

- [x] NPC action failure is not only visible through stderr.

---

# P1 Tasks

## P1.1 Log or surface NPC runner thread panic

### Target files

- `src-tauri/src/npc/runner/mod.rs`
- debug state if surfacing beyond logs

### Implementation tasks

- [x] Find `NpcRunnerGuard::drop()`.
- [x] Replace `let _ = handle.join()` with checked handling.
- [x] Log panic with useful context.
- [x] If debug state is accessible, store panic summary in debug state.
- [x] Avoid panicking from `Drop`.

### Tests

- [x] Unit test if possible for join panic handling.
- [x] Otherwise add code comment explaining why direct test is impractical and verify no ignored join result remains.

### Acceptance

- [x] Runner thread panic is not silently swallowed.

---

## P1.2 Distinguish provider mutex poisoning from provider missing

### Target files

- `src-tauri/src/npc/runner/mod.rs`
- `src-tauri/src/npc/llm_strategy.rs`
- fallback reason enum/type
- debug state/tests

### Implementation tasks

- [x] Find provider mutex lock sites.
- [x] Replace `.lock().ok().and_then(...)` with explicit match.
- [x] Add fallback/error reason for provider state unavailable.
- [x] Record this reason in LLM fallback debug state.
- [x] Keep rule-based fallback allowed so gameplay does not freeze.
- [x] Do not label poison/lock failure as provider not configured.

### Tests

- [x] Test fallback reason mapping if lock poisoning can be simulated.
- [x] Otherwise test helper function that maps lock result to fallback reason.

### Acceptance

- [x] Internal provider-state failure is visible and distinct from normal missing provider config.

---

## P1.3 Make profiled NPC fallback style source consistent

### Target files

- `src-tauri/src/npc/runner/mod.rs`
- `src-tauri/src/npc/llm_strategy.rs`
- NPC profile/style mapping code
- related tests

### Implementation tasks

- [x] Identify all fallback branches:
  - [x] provider missing,
  - [x] provider state unavailable,
  - [x] client construction failure,
  - [x] request failure,
  - [x] response parse failure,
  - [x] invalid action.
- [x] For profiled NPCs, derive fallback style from profile style in all branches.
- [x] For unprofiled NPCs, derive fallback style from config style.
- [x] Centralize style resolution in one helper if possible.
- [x] Remove duplicated style-selection logic.

### Tests

- [x] Same profiled NPC uses same style source for provider missing and parse failure.
- [x] Unprofiled NPC uses config style.
- [x] Distinct profile styles can produce distinct fallback choices.

### Acceptance

- [x] Profiled NPC behavior is not dependent on why LLM failed.

---

## P1.4 Return profile list errors instead of silently skipping bad profiles

### Target files

- `src-tauri/src/npc/profile_store.rs`
- `src/api/desktop.ts`
- `src/screens/NpcProfilesScreen.tsx`
- `src/screens/HostTournamentSetupScreen.tsx`
- related tests

### Implementation tasks

- [x] Change profile list API to include valid profiles and errors, or add separate diagnostic API.
- [x] Include filename/path and error message for invalid profile files.
- [x] Display warnings in NpcProfilesScreen.
- [x] Display warnings or disabled options in Host setup profile picker.
- [x] Keep valid profiles usable even when some files are bad.
- [x] Avoid only `eprintln!` for corrupt profiles.

### Tests

- [x] Valid profiles display.
- [x] Corrupt profile file surfaces visible warning.
- [x] Host setup profile picker shows profile-list warning.
- [x] NpcProfilesScreen shows profile-list warning.

### Acceptance

- [x] Bad profile files do not disappear silently.

---

## P1.5 Show error when host-session status check fails

### Target files

- `src/screens/HostTournamentSetupScreen.tsx`
- `src/api/desktop.ts`
- `src/screens/HostTournamentSetupScreen.test.tsx`

### Implementation tasks

- [x] Find `getHostSessionStatus().catch(...)`.
- [x] Stop converting command failure into `hostSession = null`.
- [x] Add explicit error state for session status load failure.
- [x] Show retry action if appropriate.
- [x] Keep true no-session case as normal setup UI.
- [x] Make error message distinct from “no active host session.”

### Tests

- [x] `getHostSessionStatus()` returns null -> normal setup UI.
- [x] `getHostSessionStatus()` rejects -> visible error.
- [x] Retry invokes status load again.
- [x] Existing host session still displays correctly.

### Acceptance

- [x] Backend failure is not mistaken for no session.

---

## P1.6 Show profile list loading failure in Host setup

### Target files

- `src/screens/HostTournamentSetupScreen.tsx`
- `src/screens/HostTournamentSetupScreen.test.tsx`

### Implementation tasks

- [x] Find `listNpcProfiles().catch(() => {})`.
- [x] Add `profileListError` state.
- [x] Display warning near AI/NPC profile controls.
- [x] Keep empty valid profile list distinct from load failure.
- [x] Ensure warning does not block unprofiled NPCs unless required.

### Tests

- [x] Profile list load failure warning appears.
- [x] Empty profile list without error does not show warning.
- [x] User can still add unprofiled NPC if allowed.

### Acceptance

- [x] Profile API failure is visible.

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

- [x] Implement OS keychain/keyring storage for API keys in release builds.
- [x] Keep plaintext key-file storage in debug builds as a clearly marked insecure/dev-only mode.
- [x] If keychain storage fails, surface a clear user-visible error — do not fall back to plaintext.
- [x] Ensure local providers without API keys (Ollama, llama-server) continue to work without keychain.
- [x] Ensure API keys never appear in debug state.
- [x] Ensure API keys never appear in logs.
- [x] Ensure clearing provider config removes the keychain entry.
- [x] Ensure replacing a key updates the keychain entry.
- [x] Ensure editing non-secret provider fields without entering a new key preserves the existing keychain entry.
- [x] Add docs explaining current security policy.

### Tests

- [x] API key redaction test.
- [x] Save provider config does not expose key in returned config.
- [x] Release/insecure mode behavior covered if feasible.
- [x] Clearing provider removes/invalidates stored key.

### Acceptance

- [x] Release builds do not silently store plaintext API keys.

---

## P1.8 Fix README/docs drift for provider storage, CSP, and validation commands

### Target files

- `README.md`
- `CLAUDE.md` only if explicitly needed
- `DESKTOP_POKER_STABILIZATION_SPEC.md`
- `DESKTOP_POKER_STABILIZATION_TODO.md`
- new FIX2 docs if added to repo

### Implementation tasks

- [x] Update provider storage docs:
  - [x] non-secret provider settings location,
  - [x] key file or keychain location/policy,
  - [x] no API key in JSON example.
- [x] Update CSP docs to match `src-tauri/tauri.conf.json`.
- [x] Update validation commands to project-root `--manifest-path` form.
- [x] Document optional extended Rust test command separately.
- [x] Remove stale `connect-src 'none'` claim if it no longer matches config.

### Acceptance

- [x] Docs match actual implementation.
- [x] Future Claude Code runs are not guided by stale docs.

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

- [x] Delete `src-tauri/src/npc/api_key.rs`.
- [x] Remove `pub mod api_key;` from `src-tauri/src/npc/mod.rs`.
- [x] Remove any tests that only test this dead module.
- [x] Keep any required legacy migration logic inside `provider_storage.rs`.
- [x] Verify no current production path reads or writes `claude-api-key.txt`.

### Tests

- [x] Rust compile/tests pass after deletion.
- [x] No current path writes `claude-api-key.txt`.

### Acceptance

- [x] `src-tauri/src/npc/api_key.rs` is deleted.
- [x] No two active API-key storage paths exist.

---

## P2.2 Strengthen deterministic timeout/stale-window tests

### Target files

- Rust session/runtime tests
- `src-tauri/src/app_state/session.rs` tests
- NPC runner tests
- frontend tests for timeout UI

### Implementation tasks

- [x] Identify tests that allow either timeout or success.
- [x] Replace with deterministic fake/no-ack path.
- [x] Identify stale-window tests that can pass vacuously.
- [x] Force action window to belong to NPC before stale-window assertion.
- [x] Rename tests whose names do not match what they actually prove.

### Acceptance

- [x] Timeout tests assert timeout branch.
- [x] Stale-window tests assert stale-window branch.
- [x] No vacuous pass conditions remain in P0 failure-path tests.

---

## P2.3 npm audit cleanup/classification

### Target files

- `package.json`
- `package-lock.json`
- docs/release notes if needed

### Implementation tasks

- [x] Run `npm audit`.
- [x] Classify each vulnerability:
  - [x] runtime packaged app,
  - [x] dev-only build/test tooling,
  - [x] transitive but exploitable,
  - [x] transitive and not exploitable in packaged app.
- [x] Apply safe dependency updates.
- [x] Avoid unsafe major upgrades without testing.
- [x] Document any remaining vulnerabilities and why they are accepted temporarily.

### Tests

- [x] `npm run lint`
- [x] `npm run build`
- [x] `npm run test`
- [x] app launch smoke test if dependency upgrades touch Vite/Tauri/React Router

### Acceptance

- [x] Audit status is known and documented.
- [x] Runtime-relevant vulnerabilities are fixed or explicitly blocked from release.

---

## P2.4 Full local regression pass

### Tasks

- [x] Use Node 24.x.
- [x] Run:

```bash
npm run lint
npm run build
npm run test
```

- [x] Run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

- [x] Optional extended Rust validation:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

- [x] Manual smoke test (manual-only; not blocking automated CI):
  - [x] host tournament,
  - [x] join from second instance,
  - [x] claim seats,
  - [x] ready players,
  - [x] start table,
  - [x] play one full hand,
  - [x] verify private cards isolated,
  - [x] verify observer visibility,
  - [x] verify NPC fallback/debug visibility,
  - [x] verify provider Settings load/save/clear,
  - [x] verify corrupt provider config repair,
  - [x] verify profile list error display.

### Acceptance

- [x] Automated validation passes.
- [x] Manual multiplayer smoke test passes.
- [x] No known P0/P1 silent fallback remains.

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
