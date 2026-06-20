# Desktop Poker Stabilization and Silent-Failure Hardening Spec

## Purpose

This spec defines a stabilization and hardening pass for the Desktop Poker app. The goal is not to add new user-facing gameplay features. The goal is to make the existing app more reliable, more honest about failure, and safer to maintain.

The current codebase already has a solid architecture: a Rust/Tauri backend, React frontend, backend-owned poker state, host/client sessions, signed protocol messages, encrypted private hole-card projection, lobby readiness, table action handling, hand history, NPC players, and LLM/NPC profile support.

The primary risk now is not missing architecture. The primary risk is silent degradation: places where the app falls back to generic behavior, stale state, or no-op handling without making the failure visible. Those paths make testing misleading and can hide serious correctness bugs.

## Non-goals

This pass must not:

- Add new poker variants.
- Add new UI flows unrelated to stabilization.
- Rewrite the whole networking/runtime architecture.
- Add a new LLM provider.
- Add new NPC personality features beyond preserving and surfacing the existing profile/config behavior.
- Hide failures behind generic fallback behavior.
- Replace hard backend errors with frontend-only warnings.

## Global rule: no invisible fallback behavior

A fallback is allowed only when all of the following are true:

1. The spec explicitly allows the fallback.
2. The user or debug surface can see that fallback occurred.
3. The fallback does not pretend the requested operation succeeded.
4. The fallback does not change security/privacy expectations.
5. The fallback is covered by tests.

Forbidden patterns include:

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
try {
  await importantOperation();
} catch {
  // ignored
}
```

These are acceptable only in truly best-effort cleanup paths where failure has no user-visible correctness impact, and even then the code must contain a comment explaining why ignoring the error is safe.

## Required validation commands

Claude Code must keep the project green with these commands:

```bash
npm run lint
npm run build
npm run test
```

If Rust is available:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Extended Rust validation

Run this for deeper hardening passes, but it is not required as the normal baseline:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

`--all-targets --all-features` enables optional feature flags and runs doc tests, benchmarks, and example targets. Do not make this the mandatory baseline unless Phillip explicitly decides to adopt it project-wide.

The app currently expects Node 24.x. Run the frontend checks under Node 24.x locally.

---

# 1. LLM provider and bootstrap state correctness

## Problem

The frontend listens for bootstrap updates on one event name, while backend LLM provider commands emit a different event name. The backend also returns a cached bootstrap snapshot, so fields such as `llmApiKeyConfigured` and `llmProviderType` can become stale after saving or clearing LLM provider config.

This makes the Settings/NPC UI unreliable. The user can save or clear provider config and still see old state until restart.

## Required behavior

There must be exactly one canonical bootstrap update event used by both frontend and backend.

The backend must emit a refreshed `DesktopBootstrapState` payload after any change that affects bootstrap-visible state, including:

- saving LLM provider config,
- clearing LLM provider config,
- provider config becoming invalid,
- debug/runtime catalog changes if those are already surfaced through bootstrap.

The frontend must subscribe to the canonical event and update the bootstrap context from the event payload.

The `bootstrap` command must not return stale provider fields. It must compute provider state from the live provider storage/state at call time, or update the cached bootstrap snapshot every time provider state changes.

## Acceptance criteria

- Saving an OpenAI/Anthropic/local provider config updates Settings UI without restart.
- Clearing provider config updates Settings UI without restart.
- `llmApiKeyConfigured` reflects actual current provider secret availability.
- `llmProviderType` reflects the actual current provider type or `null`.
- Backend and frontend use the same event name.
- Event payload is the refreshed bootstrap state, not `()`.
- Tests cover provider save and clear flows.
- No `let _ = app.emit(...)` on critical bootstrap update paths without logging or explicit error handling.

---

# 2. NPC profile load correctness

## Problem

When an explicit NPC `profile_id` is requested, profile loading can fail and silently degrade to an unprofiled/generic NPC. That is unsafe because the user explicitly selected a profile and the app appears to honor it, while actually changing gameplay behavior.

## Required behavior

If an NPC config explicitly names a profile, that profile is required.

If the selected profile:

- does not exist,
- cannot be read,
- cannot be parsed,
- fails validation,

then the operation must fail clearly.

The app must not add a generic NPC as a replacement unless the user explicitly selected an unprofiled/default NPC.

## Acceptance criteria

- Adding an NPC with a valid profile succeeds and preserves that profile.
- Adding an NPC with a missing explicit profile fails.
- Adding an NPC with corrupt/unreadable explicit profile fails.
- The frontend displays a clear error and does not imply that the selected profile was used.
- Tests prove that explicit profile failure does not create a generic NPC.
- Generic/unprofiled NPCs are still allowed only when no explicit profile was selected.

---

# 3. NPC config-to-player mapping correctness

## Problem

The NPC runtime appears to infer config assignment from `player_id` strings such as `npc-seat-1` and then indexes into the `npc_configs` vector. It also falls back to the first NPC config if the indexed config is missing.

This is dangerous because multiple NPCs can receive the wrong profile/style/config while the game appears to start normally.

## Required behavior

NPC configs must be mapped by stable identity, not by loose vector index parsing.

A valid mapping can use one of these approaches:

- assign and persist an explicit `player_id` on each `NpcPlayerConfig`,
- assign and persist an explicit `seat_index`,
- store an internal map from `player_id` to config when NPCs are created.

The runtime must never fall back to `npc_configs.first()` for a different NPC.

If a config cannot be found for an NPC player that requires one, the runtime must report a visible/debuggable error.

## Acceptance criteria

- Multiple NPCs keep their intended names/styles/profiles.
- Reordering seats does not shift NPC profiles.
- Missing config does not fall back to another NPC’s config.
- Tests cover at least two NPCs with distinct profiles/styles.
- Tests cover missing mapping and assert that it fails visibly.

---

# 4. NPC action submission correctness

## Problem

The NPC runner submits an action and ignores the result, then reports success. If the action is rejected, stale, illegal, or fails due to runtime issues, the NPC loop can believe progress happened when nothing changed.

## Required behavior

NPC action submission must handle the result.

If `submit_action` succeeds:

- record success,
- continue normally.

If `submit_action` fails:

- do not report success,
- record the error in logs and debug state,
- keep enough context to diagnose the action window, player ID, selected action, and error,
- avoid tight retry loops.

## Acceptance criteria

- Failed NPC action submission is visible in debug state or logs.
- The NPC runner returns/records a failure outcome instead of `true`.
- Illegal/stale action tests prove the failure is not swallowed.
- The table does not silently stall with no diagnostic when NPC action submission fails.

---

# 5. Host/client command acknowledgement correctness

## Problem

Several command paths send a mutation to the runtime and then wait for a condition, but the wait helper does not clearly report whether the condition was observed. A timeout can return stale state as if the command succeeded.

This is dangerous for operations such as:

- seat claim,
- ready toggle,
- start table,
- leave session,
- submit table action.

## Required behavior

The await/acknowledgement helper must return an explicit result:

```rust
enum AwaitConditionOutcome {
    Observed,
    TimedOut,
}
```

or simply:

```rust
Result<(), AppError>
```

A mutation command must return an error if the runtime did not acknowledge the expected state change within the timeout.

The returned error must be clear enough for the frontend to show a useful message.

## Acceptance criteria

- Seat claim timeout returns an error, not stale status.
- Ready toggle timeout returns an error, not stale status.
- Table start timeout returns an error, not stale status.
- Table action timeout returns an error, not stale table view.
- Frontend keeps previous state stable and shows explicit error on timeout.
- Tests cover at least one host command timeout and one table action timeout.
- No mutation command claims success solely because the command was sent.

---

# 6. LLM fallback visibility

## Problem

The LLM NPC path can fall back to rule-based behavior when:

- provider config is missing,
- API key is unavailable,
- HTTP request fails,
- response parsing fails,
- response action is invalid.

A rule-based fallback may be acceptable for gameplay continuity, but it must be visible. Otherwise the user thinks they are testing LLM NPCs while generic bots are actually running.

## Required behavior

Rule-based fallback is allowed only if the reason is visible in debug state and, where appropriate, the UI.

The system must capture:

- NPC/player ID,
- profile ID or style,
- provider type,
- fallback reason,
- timestamp or sequence,
- selected fallback action.

Fallback reasons should be structured, not only free-text strings.

Example fallback reasons:

```rust
enum LlmFallbackReason {
    ProviderNotConfigured,
    ApiKeyMissing,
    RequestFailed,
    ResponseParseFailed,
    InvalidAction,
    Timeout,
}
```

## Acceptance criteria

- When provider config is missing, debug UI shows that rule-based fallback is active.
- When LLM parse fails, debug UI/log state shows parse failure and fallback action.
- Tests cover provider missing and invalid response fallback.
- Fallback does not silently pretend an LLM decision was used.

---

# 7. Rule-based NPC fallback should respect profile style

## Problem

The rule-based fallback computes a style from the NPC profile but appears to ignore it, using a generic check/call preference.

## Required behavior

When LLM fallback is used for a profiled NPC, the fallback should still respect the profile’s configured style as much as possible.

Example expectations:

- tight/passive style avoids marginal calls,
- aggressive style prefers legal raises when sensible,
- balanced style uses conservative check/call/fold thresholds.

This does not need to be a strong poker AI. It only needs to avoid collapsing all profiled NPCs into the same generic behavior.

## Acceptance criteria

- Distinct NPC styles can produce distinct fallback actions under the same legal action snapshot.
- Tests cover at least two styles.
- Code does not compute profile style and then discard it.

---

# 8. LLM HTTP client construction must not silently drop configuration

## Problem

The LLM HTTP client builder can fall back to a default client if configured construction fails. That can silently remove intended timeouts or other settings.

## Required behavior

Client construction must return a `Result`.

If a configured HTTP client cannot be built, the caller must receive a visible error. Do not use `unwrap_or_default()` for configured clients.

## Acceptance criteria

- LLM client construction exposes errors.
- Tests cover client builder failure if feasible, or the code is structured so failure cannot be silently swallowed.
- Timeouts remain enforced.
- No silent fallback to an unconfigured default client.

---

# 9. Provider config corruption must be distinct from missing config

## Problem

Provider config loading can treat unreadable or invalid JSON config as equivalent to “not configured.” This hides corrupted config and makes user settings appear to vanish.

## Required behavior

Provider config load outcomes must distinguish:

```rust
enum ProviderConfigLoadState {
    Missing,
    Loaded(ProviderConfig),
    Unreadable { error: String },
    InvalidJson { error: String },
    InvalidSchema { error: String },
}
```

Missing config is normal. Corrupt or unreadable config is an error/warning.

## Acceptance criteria

- Missing provider config shows “not configured.”
- Invalid JSON shows a visible config error.
- Unreadable file shows a visible config error.
- Clearing config intentionally returns to “not configured.”
- Tests cover missing, invalid, and valid provider config.

---

# 10. Provider API keys must not be stored in plaintext JSON for release

## Problem

Provider config is stored in `llm-provider.json`. If this contains API keys, it is not acceptable for release hardening.

## Required behavior

Separate non-secret provider configuration from secrets.

Recommended approach:

- Store non-secret provider metadata in app config JSON.
- Store API keys in OS keychain/keyring using a Tauri-compatible secret storage crate/plugin.
- If keychain is unavailable, fail clearly or require explicit insecure-dev-mode opt-in.

For this stabilization pass, it is acceptable to implement a staged approach if full keychain integration is too large:

Stage 1:

- clearly label plaintext key storage as development-only,
- restrict file permissions,
- warn in Settings,
- do not log keys,
- do not include keys in debug/export state.

Stage 2:

- migrate to OS keychain.

## Acceptance criteria

- API keys are never logged.
- API keys are never exposed in debug state.
- API keys are not stored in normal JSON in release mode, or release mode clearly blocks plaintext storage.
- Tests cover serialization redaction.

---

# 11. Add a real Tauri Content Security Policy

## Problem

`tauri.conf.json` currently has CSP disabled with `csp: null`.

A Tauri app with local networking, session tokens, debug tooling, and LLM API keys should not ship without a CSP.

## Required behavior

Add an explicit CSP that allows only what the app needs.

Start restrictive:

```json
{
  "default-src": "'self'",
  "script-src": "'self'",
  "style-src": "'self' 'unsafe-inline'",
  "img-src": "'self' data: blob:",
  "font-src": "'self'",
  "connect-src": "'self' ipc: http://ipc.localhost ws://127.0.0.1:* http://127.0.0.1:*",
  "object-src": "'none'",
  "base-uri": "'none'",
  "frame-src": "'none'"
}
```

Adjust only for proven app needs.

Do not use:

```json
"default-src": "*"
```

Do not use broad remote script allowances.

## Acceptance criteria

- `csp` is no longer `null`.
- App runs in dev and production build.
- No arbitrary remote scripts are allowed.
- Required Tauri IPC continues to work.
- Any added source is documented with why it is needed.

---

# 12. App data directory resolution must fail loud outside tests

## Problem

If the OS app-data directory cannot be detected, the app may fall back to the current directory or `"."`. This can write app data, provider config, or profiles into the repo/launch directory.

## Required behavior

Outside tests, failure to resolve the app data directory must be a startup/config error.

Tests may inject a temp directory explicitly.

## Acceptance criteria

- Production app does not silently use current working directory for app data.
- Tests use injected temp dirs.
- Error message tells the user what path resolution failed.
- No provider config is accidentally written to the repo root.

---

# 13. Backend event emission observability

## Problem

Many backend commands use `let _ = app.emit(...)`. Some event emission can be best-effort, but important update events should not fail invisibly.

## Required behavior

Create helper functions for event emission:

```rust
emit_session_update(...)
emit_table_update(...)
emit_bootstrap_update(...)
```

For best-effort session/table updates:

- log warning on failure,
- include event name and error.

For critical bootstrap/settings updates:

- return error if the event update is part of the command’s correctness contract,
- or return the refreshed state directly and log event failure.

## Acceptance criteria

- No repeated raw `let _ = app.emit(...)` on critical paths.
- Emission failures are logged.
- Bootstrap update commands return enough state for frontend correctness even if event delivery fails.

---

# 14. Frontend explicit error surfaces

## Required behavior

Frontend screens must continue to show explicit error states for:

- host setup failure,
- NPC add failure,
- join failure,
- lobby unavailable,
- host stopped before table starts,
- table unavailable,
- table action rejected,
- command timeout,
- LLM provider config invalid/corrupt.

Do not convert these into redirects or generic unavailable screens unless the user can see what happened.

## Acceptance criteria

- Tests cover host recovery path.
- Tests cover join/table unavailable route guards.
- Tests cover NPC add failure with visible warning/error.
- Tests cover provider config invalid state.

---

# 15. Documentation updates

Update project documentation to describe:

- canonical bootstrap event behavior,
- NPC profile failure behavior,
- LLM fallback visibility,
- provider secret storage behavior,
- CSP rationale,
- no-silent-fallback policy for future contributors/Claude Code.

## Acceptance criteria

- README or docs include the new hardening policy.
- TODO completion references tests that prove the behavior.
