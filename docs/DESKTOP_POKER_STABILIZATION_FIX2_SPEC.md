# Desktop Poker Stabilization FIX2 Spec

## Purpose

This spec defines the second stabilization pass for the Desktop Poker app after the first hardening pass.

The previous pass fixed several major problems:

- frontend build now passes,
- bootstrap event naming was mostly canonicalized,
- explicit NPC profile load failures now fail loudly,
- NPC config mapping now uses stable player identity instead of vector-index fallback,
- client-side mutation acknowledgement timeouts are more explicit,
- Tauri CSP is no longer disabled,
- LLM fallback is visible in debug state,
- provider config storage was split into non-secret settings and key material.

This FIX2 pass focuses on the remaining edge cases, stale-state bugs, quiet failures, documentation drift, and tests that still allow failures to be hidden.

## Priority scale

- **P0**: Correctness bug or hidden failure that can mislead the user/developer, corrupt behavior, or hide a broken runtime path. Must fix before more feature work.
- **P1**: Important hardening, observability, security, or UX issue. Should fix before release.
- **P2**: Cleanup, documentation, test strengthening, or maintainability improvement. Should fix after P0/P1 unless it blocks implementation.

## Non-goals

This pass must not:

- add new poker game modes,
- add new LLM providers,
- redesign the whole NPC system,
- rewrite the app shell,
- replace backend correctness with frontend-only warnings,
- introduce new fallback behavior to make tests pass.

## Global rule: no invisible fallback behavior

Do not use hidden fallback patterns for meaningful runtime operations.

Forbidden unless explicitly justified and tested:

```rust
let _ = important_operation();
```

```rust
operation().ok()
```

```rust
operation().unwrap_or_default()
```

```rust
operation().or_else(|| Some(unrelated_default))
```

```ts
catch {
  // ignored
}
```

If a fallback is allowed, it must be visible through one or more of:

- user-facing UI,
- debug panel,
- structured debug state,
- logs that are realistically accessible during development,
- returned command error.

---

# Required validation commands

Run frontend validation from the repository root:

```bash
npm run lint
npm run build
npm run test
```

Run Rust validation from the repository root using the project-root manifest-path form:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Optional extended Rust validation:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

Do not make the extended command mandatory unless the project owner explicitly decides to make it the baseline.

---

# P0 Requirements

## P0.1 Provider config error must not remain stale after save/clear

### Problem

`providerConfigError` can be computed during startup and stored in cached bootstrap state. Later, saving or clearing provider config updates live provider state but may leave the old `providerConfigError` in refreshed bootstrap payloads.

This creates a misleading UI:

1. app starts with corrupt `llm-provider.json`,
2. Settings shows provider config error,
3. user saves valid config or clears config,
4. bootstrap refresh still reports old provider config error until restart.

### Required behavior

Provider config error state must be live/current.

Saving a valid provider config must clear stale config errors.

Clearing provider config must clear stale config errors and return the provider state to normal “not configured.”

Bootstrap refreshes must include the current provider error state, not the startup-only snapshot.

### Acceptance criteria

- Starting with corrupt provider config surfaces `providerConfigError`.
- Saving valid provider config clears `providerConfigError` without restart.
- Clearing provider config clears `providerConfigError` without restart.
- `bootstrap()` returns current provider error state.
- Bootstrap event payload contains current provider error state.
- Tests cover corrupt -> save valid -> no error.
- Tests cover corrupt -> clear -> no error.

---

## P0.2 Unreadable API-key file must be an error, not “not configured”

### Problem

Provider config storage now uses a non-secret provider settings file and a separate key file. However, if the key file exists but cannot be read, the app can silently treat that as “missing key” or “not configured.”

Unreadable key file is not the same as absent key. It indicates broken local state or permissions.

### Required behavior

Provider storage must distinguish:

- missing provider config,
- valid provider config with no key required,
- valid provider config with key present,
- valid provider config with key missing,
- provider key file exists but is unreadable,
- provider key file is invalid/corrupt if validation exists.

Unreadable key file must surface a provider config/secret error.

### Acceptance criteria

- Missing provider config means “not configured.”
- OpenAI/Anthropic provider config with missing key means “API key missing.”
- Existing but unreadable key file surfaces an error.
- Existing but unreadable key file is not silently converted to missing key.
- Settings/debug/bootstrap can show enough information to diagnose the secret storage problem.
- Tests cover missing key vs unreadable key.

---

## P0.3 Settings must load existing non-secret provider config before saving

### Problem

`DeviceSettingsScreen` can initialize provider type from bootstrap but not load the existing non-secret provider config via `getLlmProviderConfig()`.

This can cause config loss:

1. user configures custom endpoint/model,
2. user reopens Settings,
3. endpoint/model fields are blank/default,
4. user saves,
5. existing non-secret settings are overwritten.

### Required behavior

Settings must load existing non-secret provider config on mount.

It must populate:

- provider type,
- endpoint URL,
- model name,
- any other non-secret fields.

It must not prefill API-key input with the stored secret.

If loading config fails, show a visible error instead of silently using blank fields.

### Acceptance criteria

- Reopening Settings shows existing provider type.
- Reopening Settings shows existing endpoint URL and model.
- API key field remains blank/redacted; it is not populated with secret material.
- Saving without entering a new API key preserves existing key unless the user explicitly clears/replaces it.
- Loading failure is visible.
- Tests cover preserving endpoint/model.
- Tests cover preserving existing key when non-secret fields are edited.

---

## P0.4 Profiled NPC fallback action must be logged to hand log

### Problem

For profiled NPCs, provider-not-configured fallback can submit a rule-based action but skip hand-log recording because hand-log fallback recording is gated on `profile.is_none()`.

This can damage:

- session history,
- opponent stats,
- tilt state,
- future LLM context,
- debugging.

### Required behavior

Every submitted NPC action must be recorded exactly once in the hand/action log, regardless of whether the NPC is profiled or unprofiled.

Do not decide logging based only on `profile.is_none()`.

Use an explicit `action_logged` or equivalent mechanism.

### Acceptance criteria

- Profiled NPC with provider missing uses rule-based fallback and logs action.
- Profiled NPC with LLM parse failure logs action exactly once.
- Unprofiled NPC logs action exactly once.
- No duplicate log entries for a single NPC action.
- Tests cover profiled provider-missing fallback logging.

---

## P0.5 NPC action submission failures must be visible beyond stderr

### Problem

NPC action submission failures are now no longer completely ignored, but failure visibility still depends too much on `eprintln!`.

In a GUI desktop app, stderr is often invisible.

### Required behavior

NPC action failure state must be stored in structured debug state.

At minimum, expose:

- player ID,
- selected action,
- failure reason,
- table/session sequence if available,
- timestamp or sequence number.

DebugPanel must render the most recent NPC action error.

### Acceptance criteria

- Failed NPC action updates debug state.
- DebugPanel displays failed NPC action information.
- Tests cover rejected/stale/failed NPC action visibility.
- NPC runner does not claim success on failed submission.
- No meaningful NPC action failure is only visible through stderr.

---

# P1 Requirements

## P1.1 NPC runner thread panic must be logged or surfaced

### Problem

`NpcRunnerGuard::drop()` can join the runner thread and ignore the result. If the thread panicked, the panic is discarded.

### Required behavior

Thread join failure/panic must be logged or stored in debug state.

### Acceptance criteria

- `handle.join()` result is checked.
- Thread panic produces at least a warning log.
- Preferably, panic information is stored in debug state.
- No silent `let _ = handle.join()` remains in NPC runner code.

---

## P1.2 Provider mutex poisoning must not masquerade as “provider missing”

### Problem

NPC provider config access can do:

```rust
api_key_holder.lock().ok().and_then(|g| g.clone())
```

If the mutex is poisoned, the code treats that as no provider and falls back to rule-based behavior.

Mutex poisoning is an internal state failure, not a normal “provider not configured” condition.

### Required behavior

Provider lock failure must produce a distinct fallback/error reason such as:

```rust
ProviderStateUnavailable
```

Rule-based fallback may still be used to keep the table moving, but the reason must be accurate and visible.

### Acceptance criteria

- Mutex poisoning/lock failure is not recorded as provider missing.
- LLM fallback debug state distinguishes provider missing from provider state unavailable.
- Tests cover lock failure if practical, or code structure makes the distinction explicit.

---

## P1.3 Profiled NPC fallback style must be consistent

### Problem

There are two style sources:

- `NpcConfig.style`
- `NpcProfile.style`

Fallback behavior can use different style sources depending on why LLM failed. A profiled NPC may behave one way when provider is missing and another way when parsing fails.

### Required behavior

If an explicit profile exists, fallback style must be derived consistently from the profile.

If no profile exists, fallback style may use `NpcConfig.style`.

### Acceptance criteria

- Profiled NPC fallback uses profile style for provider missing, request failure, parse failure, and invalid action.
- Unprofiled NPC fallback uses config style.
- Tests cover at least two failure reasons for the same profiled NPC and verify consistent style source.

---

## P1.4 Corrupt/unreadable profiles must not disappear silently from profile list

### Problem

Explicitly loading a selected profile now fails loudly, but listing profiles can still skip corrupt files with only stderr output.

This causes user-created profiles to “disappear” from the UI without a repair path.

### Required behavior

Profile listing should return both valid profiles and profile file errors.

The UI should show profile list errors as warnings, ideally with filename/path and parse/read error.

### Acceptance criteria

- Valid profiles still display.
- Corrupt profile file appears as a visible warning/error.
- Unreadable profile file appears as a visible warning/error if feasible.
- Profile list loading does not silently drop bad profile files.
- Tests cover corrupt profile list behavior.

---

## P1.5 Host setup must not treat host-session status failure as “no session”

### Problem

`HostTournamentSetupScreen` can catch `getHostSessionStatus()` failure and set `hostSession` to `null`.

Backend failure and “no active host session” are different states.

### Required behavior

If checking host session status fails, show a visible error or runtime-unavailable state.

Do not silently treat backend/Tauri command failure as no session.

### Acceptance criteria

- No active session displays normal no-session/setup UI.
- Backend command failure displays an error.
- Error can be retried or recovered.
- Tests cover failed `getHostSessionStatus()`.

---

## P1.6 Profile list loading failure must be visible in Host setup

### Problem

Host setup currently may catch profile list load errors and ignore them. The profile dropdown then appears empty.

### Required behavior

If `listNpcProfiles()` fails, show an inline warning in Host setup and/or NPC configuration area.

### Acceptance criteria

- Profile load failure is visible.
- Empty profile list due to no profiles remains distinct from load failure.
- Tests cover profile list load failure.

---

## P1.7 Release-mode plaintext API-key policy must be explicit

### Problem

Provider key storage was improved by moving API keys out of the JSON config into a separate key file with restrictive permissions. However, release builds still write plaintext API keys to local files.

A stderr warning is not enough for a GUI app.

### Required behavior

**Decision: Option A — OS keychain/keyring in release builds.**

- Release builds must use OS keychain/keyring storage for API keys.
- Plaintext key-file storage must not be the normal release behavior.
- If keychain storage is unavailable or fails, saving an API-key provider must fail with a clear user-visible error. Do not silently fall back to a plaintext key file.
- Local providers that do not need API keys (Ollama, llama-server) must continue to work without keychain.
- Debug builds may continue to support plaintext key-file storage as an explicitly marked development/insecure storage mode only.

**Option B as an interim only (not preferred):**

If OS keychain integration turns out to be too large or platform-blocking, stop and report the blocker. Do not silently downgrade to plaintext release storage. As an explicit interim-only fallback, implement Option B:

- Release builds block API-key provider save unless an explicit insecure plaintext mode flag is enabled.
- Local providers without API keys must still work.
- API-key providers must show a clear error explaining that secure key storage is unavailable.
- Insecure plaintext mode must require explicit opt-in.
- Docs must clearly state that Option B is not release-safe.

**Option C is not allowed.** Persistent warning plus plaintext storage in release is not sufficient.

### Acceptance criteria

- Policy is documented.
- UI behavior matches policy.
- API keys are never logged or exposed in debug state.
- Tests cover redaction.
- Release builds do not silently store plaintext secrets.

---

## P1.8 README and docs must match current provider storage and CSP behavior

### Problem

README/docs can now drift from implementation:

- provider config docs may still imply API keys live in `llm-provider.json`,
- CSP docs may not match `tauri.conf.json`,
- validation commands may not match `CLAUDE.md`.

### Required behavior

Documentation must accurately describe:

- provider settings file,
- provider key file or keychain policy,
- whether plaintext storage is dev-only or release-allowed,
- CSP currently configured in `tauri.conf.json`,
- canonical validation commands.

### Acceptance criteria

- README no longer shows stale provider config containing `apiKey`.
- README names the correct provider storage files/secret backend.
- README CSP section matches actual `tauri.conf.json`.
- Validation commands use `--manifest-path src-tauri/Cargo.toml`.

---

# P2 Requirements

## P2.1 Delete legacy `npc/api_key.rs`

### Problem

Legacy module `src-tauri/src/npc/api_key.rs` implements the old `claude-api-key.txt` storage path. It has no callers outside itself. Legacy key migration from `claude-api-key.txt` is already handled inside `src-tauri/src/npc/provider_storage.rs`.

Keeping it creates confusion and regression risk.

### Required behavior

Delete the module entirely:

- delete `src-tauri/src/npc/api_key.rs`,
- remove `pub mod api_key;` from `src-tauri/src/npc/mod.rs`,
- remove any tests that only test this dead module,
- keep any required legacy migration logic inside `provider_storage.rs`,
- verify no current production path reads or writes `claude-api-key.txt`.

If compile or tests reveal a hidden dependency, stop and report rather than preserving the dead module silently.

### Acceptance criteria

- `src-tauri/src/npc/api_key.rs` is deleted.
- No current provider code writes `claude-api-key.txt`.
- No two active API-key storage paths exist.
- Compile and tests pass.

---

## P2.2 Strengthen timeout and stale-window tests

### Problem

Some tests can pass without strongly proving the intended failure branch. For example, a timeout test may allow success if state is confirmed, or an NPC stale-window test may pass without the action window belonging to the NPC.

### Required behavior

Tests should deterministically exercise the named failure path.

### Acceptance criteria

- Timeout tests force acknowledgement absence and assert timeout error.
- Stale-window tests force a stale NPC-owned action window.
- Test names match the branch they actually verify.
- No vacuous pass conditions for P0 failure paths.

---

## P2.3 npm audit cleanup plan

### Problem

`npm audit` still reports vulnerabilities in the JS toolchain.

### Required behavior

Create a dependency update plan.

If vulnerabilities are dev-only and not relevant to packaged app runtime, document that. If runtime-relevant, update immediately.

### Acceptance criteria

- `npm audit` reviewed.
- Vulnerabilities classified as runtime/dev-only.
- Safe updates applied.
- Remaining unavoidable issues documented with rationale.

---

## P2.4 Full local regression checklist

### Required behavior

After P0/P1 fixes, run a local regression pass under the expected toolchain.

### Acceptance criteria

- Node 24.x is used.
- `npm run lint` passes.
- `npm run build` passes.
- `npm run test` passes.
- Rust format/clippy/test commands pass.
- Manual host/client smoke test passes:
  - host tournament,
  - join client,
  - claim seat,
  - ready,
  - start table,
  - play one hand,
  - verify private hole cards are isolated,
  - verify observer visibility,
  - verify NPC fallback/debug indicators,
  - verify provider Settings save/clear behavior.
