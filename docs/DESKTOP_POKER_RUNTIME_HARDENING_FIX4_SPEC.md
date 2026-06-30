# Desktop Poker Runtime Hardening Fix 4 Specification

## Purpose

This spec converts the 2026-06-30 code review into an implementation plan for the desktop poker app. The current codebase is meaningfully improved: CSP is enabled, hidden-card projection has tests, NPC profile loading is fail-loud, LLM provider settings are separated from secrets, and accepted NPC action logging no longer lies about rejected actions.

The remaining work is mostly about runtime honesty. The app still has several places where test scaffolding, provider failures, network/protocol errors, and corrupted game state are quietly tolerated. This pass must remove production mock backdoors, make security-sensitive storage transitions transactional, and make runtime fallbacks visible, opt-in, or hard errors.

## Current baseline from review

Repository reviewed: `desktop_poker-master_2606301313.zip`.

Validated in sandbox:

- `npm ci --ignore-scripts` passed, with a Node engine warning because the repo wants Node `24.x` and the sandbox had Node `22.16.0`.
- `npm run lint` passed.
- `npm run build` passed.
- Vitest passed when run single-threaded: `npx vitest run --reporter=dot --testTimeout=10000 --pool=threads --poolOptions.threads.singleThread=true`.
- Default `npm test` timed out in the sandbox after 300 seconds.
- Rust tests were not executed in the sandbox because `cargo` was unavailable there. Claude Code must run Rust tests locally.

## Primary goals

1. **No production browser mock bridge.** Test/probe mocks must not be reachable in production runtime code.
2. **No stale provider secrets after provider changes.** Provider setting changes that require key deletion must fail loudly if deletion cannot be completed.
3. **No hidden legacy-key read failures.** An unreadable legacy key file must surface as a provider config error, not as “missing config.”
4. **No profile-backed NPC silently becoming rule-based because the LLM path broke.** Rule-based behavior is allowed for explicitly rule-based NPCs. LLM-profile fallback must be visible and either opt-in or a hard error.
5. **No invented poker state for NPC decisions.** Missing stack/blind/current-hand state must be treated as an invariant failure, not replaced with plausible defaults.
6. **No non-transactional multi-NPC mutation.** Adding multiple NPCs must either fully succeed or leave the lobby in its previous state.
7. **Runtime diagnostics must retain important network/protocol errors.** Host/client loops may drop hostile or malformed messages, but they must count and expose repeated/drop reasons.
8. **Default test command must finish.** `npm test` should be reliable without requiring a hidden local workaround.

## Non-goals

- Do not rewrite the poker/tournament engine.
- Do not change the card projection privacy model except to add regression tests if needed.
- Do not add broad Tauri permissions.
- Do not weaken the current CSP.
- Do not add persistence compatibility layers unless the TODO explicitly asks for a migration/error case.
- Do not hide new errors behind `console.error`, `eprintln!`, `.ok()`, `unwrap_or(...)`, or `_ = ...` unless the TODO explicitly permits a non-fatal diagnostic counter.

## Global implementation rules

### Fail-loud rule

For state that affects correctness, security, or game authority, do not silently synthesize fallback values. Return an error, record a structured debug failure, or expose a runtime-health warning.

Disallowed patterns in changed code unless explicitly justified in a comment and covered by a test:

```rust
let _ = fallible_call();
fallible_call().ok();
fallible_call().unwrap_or(default_that_masks_state);
fallible_call().unwrap_or_else(default_that_masks_state);
eprintln!("..."); // as the only observable error behavior
continue;          // after parsing/security/protocol failure with no counter or event
```

Presentation-only defaults remain allowed. For example, UI labels like “Main Table” or display-name fallback to a player ID are acceptable because they do not alter authoritative behavior.

### Test/probe code rule

Test/probe APIs must be reachable only in development/test/probe builds. A production bundle must not consult `window.__DESKTOP_POKER_BROWSER_MOCKS__`.

### Runtime-health rule

Network loops must be robust against bad input, but robustness is not the same as silence. Dropped/malformed/stale frames should not spam users, but counts/reasons must be observable in debug tooling.

### Provider-secret rule

Provider settings and provider secrets are one logical configuration transaction even though they live in different backends. If a provider switch requires deleting an old key and that deletion fails, the settings write must not proceed.

### NPC policy rule

There are two distinct NPC modes:

- **Rule-based NPC:** `profile == None`; rule-based decision engine is expected and valid.
- **LLM-profile NPC:** `profile != None`; the host/user chose a profile and expects profile behavior.

For LLM-profile NPCs, fallback to rule-based behavior is allowed only if it is explicitly represented by policy and surfaced in debug UI. Internal failures such as poisoned locks, invalid prompt snapshots, or LLM client construction failures should be treated as errors, not as normal gameplay.

## Required changes by subsystem

## 1. Frontend desktop API mock bridge

### Current problem

`src/api/desktop.ts` declares and uses `window.__DESKTOP_POKER_BROWSER_MOCKS__` in the normal runtime path. Functions such as `startHostSession`, `joinHostSession`, `getTableView`, `submitTableAction`, and `launchAdditionalClientInstance` defer to that global object when present.

That is a production-adjacent test hook. CSP reduces risk, but it does not justify leaving a global action-interception hook in production code.

### Required behavior

- `getBrowserMocks()` must return mocks only in Vite dev mode or Vitest mode.
- In production builds, it must always return `undefined`, even if a global object exists.
- Tests that intentionally use browser mocks must still pass.
- `src/probe/LayoutProbeApp.tsx` may continue to set the global mock, but it must only be useful in dev/probe contexts.

### Acceptance criteria

- Production build compiles.
- Existing `src/api/desktop.test.ts` mock tests pass.
- New test proves production-mode gating logic cannot return mocks.
- No app action path consults `window.__DESKTOP_POKER_BROWSER_MOCKS__` without going through the gated helper.

## 2. Provider settings and secret storage

### Current problem

`src-tauri/src/npc/provider_storage.rs::save_provider_settings_only` attempts to delete the old provider key when the provider changes. If deletion fails, it prints an error and continues to write the new settings. This can leave a stale key behind, especially in debug builds where `PlaintextFileSecretStore` is a single shared file.

The legacy `claude-api-key.txt` migration path also treats unreadable legacy files as missing config because it does:

```rust
if legacy.exists() {
    if let Ok(raw) = fs::read_to_string(&legacy) {
        ...
    }
}
ProviderConfigLoadState::Missing
```

### Required behavior

- `save_provider_settings_only` must be transactional with respect to provider changes:
  - Read existing settings if present.
  - If the existing settings file exists but is unreadable/invalid, return an error and do not write new settings.
  - If provider type changes, delete the old provider key first.
  - If deletion fails, return an error and do not write new settings.
  - Only after the required deletion succeeds should the new settings file be written.
- Legacy key read failure must surface as a distinct load state, e.g. `ProviderConfigLoadState::LegacyKeyUnreadable { error }`.
- `DesktopAppState::detect` must treat `LegacyKeyUnreadable` like the other provider load errors and expose it through `provider_config_error`.
- Tests must simulate deletion failure and unreadable/corrupt files.

### Acceptance criteria

- A failing secret store prevents provider-switch settings from being written.
- Existing key is not carried across provider types in memory or storage.
- An unreadable legacy key does not become `Missing`.
- Clearing provider config still deletes all known provider accounts.
- Debug plaintext store behavior remains explicit and tested.

## 3. NPC LLM fallback policy

### Current problem

`src-tauri/src/npc/runner/action.rs` uses rule-based fallback for profile-backed NPCs when:

- LLM client construction fails.
- Provider is not configured.
- Provider state is unavailable due to lock poisoning.
- LLM request fails.

Some fallback is visible through `lastLlmFallback`, but gameplay continues as if the NPC made a valid profile decision. This can hide broken provider state and make a table behave very differently from what the host configured.

### Required behavior

Implement a clear policy:

1. **Rule-based NPCs are allowed to use rule-based decisions.** No warning needed.
2. **Profile-backed NPCs must not silently become rule-based.** Choose one of the following implementation approaches:
   - **Preferred:** Add an explicit opt-in flag for profile fallback, e.g. `allowRuleBasedLlmFallback`, default `false`. If false, no action is submitted when LLM decision cannot be produced; record a structured NPC action error and debug fallback message. If true, submit a rule-based action and record the fallback event visibly.
   - **Acceptable smaller pass:** For this fix, hard-fail/skip action on provider not configured, provider state unavailable, and client construction failure. Keep rule-based fallback only for request failures if it is visible in debug UI. This is less complete but still removes the worst silent internal failures.
3. `ProviderState::StateUnavailable` must never fall back to rule-based action. Treat it as an internal error.
4. LLM client construction failure must not be mislabeled as `RequestFailed`; use a distinct reason if the enum supports it, or extend the enum.
5. The debug inspector must clearly distinguish “profile fallback used” from “profile NPC could not act.”

### Acceptance criteria

- Tests prove `ProviderState::StateUnavailable` records an error and submits no action.
- Tests prove a missing provider for a profile-backed NPC does not silently submit a rule-based action unless explicit fallback is enabled.
- Tests prove rule-based NPCs without profiles still act normally.
- Debug UI displays the most recent LLM fallback/error clearly.

## 4. NPC decision invariant handling

### Current problem

NPC decision code still invents state:

- Missing NPC stack becomes `1` in `action.rs`.
- Missing blind level becomes a hardcoded L1 level in `action.rs`.
- Missing blind level/current hand becomes `20`/`0` in `decision.rs`.

These are not presentation defaults. They alter poker decisions and can hide corrupted authoritative state.

### Required behavior

- Replace authoritative gameplay defaults with explicit validation before decision construction.
- Missing stack for the acting NPC is an internal error.
- Invalid blind-level index is an internal error.
- Missing current hand while deciding preflop is an internal error.
- The hardcoded `fallback_blind_level()` helper should be deleted from production decision paths. It may remain only in tests if a test needs a fixture helper.

### Acceptance criteria

- No production NPC decision path contains `.unwrap_or(1)`, `.unwrap_or(20)`, `.unwrap_or(0)`, or `fallback_blind_level()` for authoritative state.
- Tests verify invalid stack/blind state records `NpcActionErrorDebug` and does not submit an action.
- Existing rule-based strategy tests continue to pass using explicit fixture state.

## 5. Multi-NPC add transactionality

### Current problem

`src-tauri/src/app_state/app_npc.rs::add_npc_players` prevalidates capacity and profiles, then mutates the host session in a loop:

```rust
register_npc_participant(...)?;
claim_seat(...)?;
set_ready_state(...)?;
```

If NPC #1 succeeds and NPC #2 fails, NPC #1 remains registered/seated/ready, but the runner is not started. This leaves partial state after an error.

### Required behavior

Adding NPCs must be atomic from the app API perspective:

- Either all NPCs are registered, seated, ready, and runner-started.
- Or none of the new NPCs remain in the lobby.

Preferred implementation:

- Add a host-server transaction method that validates and applies all NPCs atomically under the authoritative state lock.

Acceptable implementation for this pass:

- Track applied mutations and roll them back if any later step fails.
- If rollback itself fails, return an error that explicitly reports both the original failure and rollback failure. Do not pretend the operation was cleanly aborted.

### Acceptance criteria

- Test injects failure on second NPC and verifies no partial NPC remains.
- Test injects runner thread spawn failure and verifies no partial NPC remains.
- Normal multi-NPC add still works.

## 6. NPC runner thread spawn

### Current problem

`src-tauri/src/npc/runner/mod.rs::start_npc_runner` panics on thread spawn failure:

```rust
.expect("failed to spawn npc-runner thread")
```

This is a user-facing desktop app and `add_npc_players` already returns `Result`, so this should be reported as a normal operation failure.

### Required behavior

- Change `start_npc_runner` to return `Result<JoinHandle<()>, String>`.
- Propagate the error through `add_npc_players`.
- If NPCs were already registered/seated before spawn failure, roll them back per the transactionality requirement.

### Acceptance criteria

- No panic on thread spawn failure.
- Tests cover the error path by injecting a spawn abstraction or equivalent.

## 7. Host runtime health diagnostics

### Current problem

`src-tauri/src/networking/runtime/host.rs` swallows several important failures:

- Incoming listener errors are ignored with `continue`.
- `set_read_timeout` / `set_write_timeout` errors are ignored.
- `controller.advance_time(...)` errors are converted to `None` and dropped.
- `publish_runtime_transition(...)` errors are ignored.
- Failure to lock/update authoritative state is ignored.

### Required behavior

Introduce host runtime health state that records non-fatal host loop failures without crashing the host:

Minimum fields:

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostRuntimeHealth {
    pub accept_error_count: u64,
    pub stream_timeout_error_count: u64,
    pub tick_advance_error_count: u64,
    pub publish_error_count: u64,
    pub state_lock_error_count: u64,
    pub last_error: Option<String>,
    pub last_successful_tick_ms: Option<u64>,
    pub last_successful_publish_ms: Option<u64>,
}
```

Implementation may live in `src-tauri/src/networking/runtime/mod.rs` or a new `health.rs` module.

Host loops should increment counters and set `last_error` when failures occur. Do not spam stdout/stderr. Do not expose secrets in messages.

### Acceptance criteria

- HostServer exposes a `runtime_health()` getter.
- Debug inspector includes host runtime health when a host session exists.
- Tests verify tick advance failure increments `tick_advance_error_count`.
- Tests verify publish failure increments `publish_error_count`.

## 8. Client protocol warning diagnostics

### Current problem

`src-tauri/src/networking/runtime/client.rs` silently drops malformed or invalid frames with `continue`. This is correct for hostile input, but invisible for debugging protocol mismatches.

Failures currently dropped include:

- Missing `messageType`.
- Malformed private envelope.
- Invalid signature.
- Decrypt failure.
- Malformed private payload.
- Malformed snapshot envelope.
- Unknown/malformed signed envelope.

### Required behavior

Add non-fatal protocol warning diagnostics.

Minimum event variant:

```rust
ProtocolWarning {
    player_id: String,
    reason: String,
    count: u64,
}
```

The runtime should count warning reasons and emit events at a low-noise cadence. A simple first pass may emit every occurrence in tests and keep UI display to “most recent protocol warning.” A better pass emits first occurrence and then powers of two per reason.

### Acceptance criteria

- Malformed frame still does not crash client runtime.
- Invalid signature still drops the frame.
- A warning counter/event is emitted and visible to debug tooling.
- Tests cover at least missing `messageType`, invalid private envelope, invalid signature, and decrypt failure.

## 9. Window persistence noisy stderr

### Current problem

Frontend tests produce repeated stderr messages:

```text
Failed to initialize window state persistence.
TypeError: Cannot read properties of undefined (reading 'currentWindow')
```

The tests pass, but repeated expected errors hide real failures.

### Required behavior

- Strengthen the runtime guard in `src/app/persistence.ts` so it only imports/uses Tauri window APIs when the Tauri API is actually usable.
- Or provide a proper test mock for `@tauri-apps/api/window`.
- Do not rely on catch-and-log as the normal test path.

### Acceptance criteria

- `npm test` no longer prints this expected error.
- Real persistence errors in a Tauri runtime are still logged or surfaced.

## 10. Default Vitest reliability

### Current problem

`npm test` timed out in the sandbox, but single-threaded Vitest passed all tests. This suggests a worker/pool hang or environment mismatch.

### Required behavior

Make the default test command reliable.

Recommended approach:

- Add explicit Vitest pool configuration in `vite.config.ts` or a dedicated `vitest.config.ts`.
- Keep one canonical command: `npm test`.
- Do not rely on developers remembering an undocumented command-line override.

### Acceptance criteria

- `npm test` finishes cleanly on Node 24.
- CI uses the same command.
- If single-thread mode is required, it is encoded in config or package script with a comment explaining why.

## Final validation checklist

Claude Code must run these before declaring completion:

```bash
npm ci
npm run lint
npm run build
npm test
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

If any command cannot be run, report that explicitly with the reason. Do not claim validation that was not performed.

## Completion definition

This fix is complete when:

- Production code cannot use browser mocks.
- Provider config/key transitions are fail-loud and covered by tests.
- Legacy key read failures are visible.
- Profile-backed NPCs do not silently fall back to rule-based behavior on internal/provider failures.
- NPC decisions no longer invent stack/blind/current-hand state.
- Adding multiple NPCs is atomic or explicitly rolled back on failure.
- Host/client runtime failures are exposed through debug diagnostics.
- `npm test` is reliable.
- All validation commands pass or any unavailable command is clearly reported.
