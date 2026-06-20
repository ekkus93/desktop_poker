# Desktop Poker Stabilization TODO

This TODO implements `DESKTOP_POKER_STABILIZATION_SPEC.md`.

The theme of this pass is: **no invisible fallback behavior**.

Do not add generic fallbacks to make tests pass. Do not silently convert failures into defaults. If a fallback is intentionally allowed, it must be visible in the UI, debug state, logs, and tests.

---

# P0 — Must fix before more feature work

## P0.1 Fix LLM provider/bootstrap event and stale state

### Target files

Inspect and update:

- `src/app/DesktopBootstrapProvider.tsx`
- `src/app/useDesktopBootstrap.tsx`
- `src/api/desktop.ts`
- `src-tauri/src/app_state/mod.rs`
- `src-tauri/src/app_state/commands.rs`
- `src-tauri/src/app_state/llm_provider.rs`
- any Rust command that saves/clears LLM provider config
- any tests under `src/app/*Bootstrap*.test.tsx`
- any Rust tests around bootstrap/provider config

### Tasks

- [x] Identify the canonical frontend event currently used for bootstrap updates.
- [x] Identify the backend event currently emitted after LLM provider save/clear.
- [x] Replace split event names with one canonical event: `desktop://bootstrap`.
- [x] Ensure backend emits a refreshed `DesktopBootstrapState` payload, not `()`.
- [x] Ensure frontend subscription consumes the payload and updates bootstrap context.
- [x] Update backend `bootstrap()` command so it does not return stale provider fields.
- [x] Recompute or refresh:
  - [x] `llmApiKeyConfigured`
  - [x] `llmProviderType`
- [x] Ensure saving provider config updates UI without restart.
- [x] Ensure clearing provider config updates UI without restart.
- [x] Do not ignore event emit failure on this critical path.

### Tests

- [x] Add/update frontend test: provider save event refreshes bootstrap state.
- [x] Add/update frontend test: provider clear event refreshes bootstrap state.
- [x] Add/update Rust test: bootstrap command reflects live provider config after save.
- [x] Add/update Rust test: bootstrap command reflects live provider config after clear.

### Acceptance

- [x] `npm run test` passes.
- [x] `npm run build` passes.
- [x] Settings UI does not require restart to reflect provider status.

---

## P0.2 Make explicit NPC profile load failures fail loudly

### Target files

Inspect and update:

- `src-tauri/src/app_state/app_npc.rs`
- `src-tauri/src/app_state/npc_profiles.rs`
- `src-tauri/src/app_state/commands.rs`
- `src/screens/HostTournamentSetupScreen.tsx`
- `src/screens/NpcProfilesScreen.tsx`
- frontend desktop API types in `src/api/desktop.ts`
- related tests for NPC add/profile behavior

### Tasks

- [x] Find all places that load an explicit NPC profile by `profile_id`.
- [x] Remove `.ok()`, `unwrap_or_default`, or generic `None` fallback for explicit selected profiles.
- [x] Return a structured error when explicit profile load fails.
- [x] Make sure the NPC is not added when its explicit profile fails to load.
- [x] Keep unprofiled/generic NPCs allowed only when no explicit profile was requested.
- [x] Make the frontend display the profile load error clearly.

### Tests

- [x] Rust test: valid explicit profile creates profiled NPC.
- [x] Rust test: missing explicit profile returns error and creates no NPC.
- [x] Rust test: corrupt explicit profile returns error and creates no NPC.
- [x] Frontend test: NPC add profile error is visible and does not imply success.

### Acceptance

- [x] No selected profile silently becomes generic.
- [x] Error message identifies the failed profile enough to diagnose it.

---

## P0.3 Fix NPC config mapping; remove fallback to first config

### Target files

Inspect and update:

- `src-tauri/src/npc_runner.rs`
- `src-tauri/src/app_state/app_npc.rs`
- `src-tauri/src/app_state/mod.rs`
- any `NpcPlayerConfig` type definitions
- tests around NPC runtime and host setup

### Tasks

- [x] Find where NPC runtime maps `player_id` / seat to `npc_configs`.
- [x] Remove fallback equivalent to `npc_configs.first()`.
- [x] Add a stable mapping:
  - [x] by explicit `player_id`, or
  - [x] by explicit `seat_index`, or
  - [x] by stored `HashMap<PlayerId, NpcPlayerConfig>`.
- [x] Ensure each created NPC has exactly one matching config.
- [x] If mapping is missing, record/return a visible error.
- [x] Make sure multiple NPCs keep distinct names, styles, and profiles.

### Tests

- [x] Rust test: two NPCs with different profiles keep correct profiles.
- [x] Rust test: two NPCs with different styles keep correct styles.
- [x] Rust test: missing mapping does not fall back to first config.
- [x] Integration test if available: host with multiple NPCs and inspect/debug identities.

### Acceptance

- [x] No profile/style shifting between NPCs.
- [x] No generic fallback when mapping fails.

---

## P0.4 Do not ignore NPC action submission failures

### Target files

Inspect and update:

- `src-tauri/src/npc_runner.rs`
- `src-tauri/src/networking/runtime.rs`
- `src-tauri/src/app_state/debug.rs`
- `src/components/debug/DebugPanel.tsx`
- related Rust/frontend tests

### Tasks

- [x] Find all `submit_action` calls from NPC logic.
- [x] Replace ignored result handling with explicit `match`.
- [x] Return/record failure when submission fails.
- [x] Add structured NPC action outcome:
  - [x] success
  - [x] rejected
  - [x] stale window
  - [x] illegal action
  - [x] runtime unavailable
- [x] Add failure detail to debug state/logs.
- [x] Prevent tight retry loops after repeated NPC failure.

### Tests

- [x] Rust test: illegal NPC action records failure.
- [x] Rust test: stale NPC action window records failure.
- [x] Rust test: NPC runner does not claim success after rejected action.
- [x] Frontend debug test if debug state shape changes.

### Acceptance

- [x] No `let _ = host_server.submit_action(...)` in NPC action path.
- [x] Failed NPC actions are diagnosable.

---

## P0.5 Make host/client command acknowledgement timeouts explicit

### Target files

Inspect and update:

- `src-tauri/src/app_state/session.rs`
- `src-tauri/src/app_state/commands.rs`
- `src-tauri/src/networking/runtime.rs`
- `src/api/desktop.ts`
- `src/screens/TournamentLobbyScreen.tsx`
- `src/screens/MainTableScreen.tsx`
- related tests

### Tasks

- [x] Find the helper that waits for runtime state after a command.
- [x] Change it to return explicit observed/timed-out result.
- [x] For mutation commands, return error if acknowledgement is not observed.
- [x] Apply to:
  - [x] seat claim
  - [x] ready toggle
  - [x] start table
  - [x] leave session if applicable
  - [x] table action
- [x] Ensure frontend shows timeout/retry error instead of silently accepting stale state.
- [x] Preserve current state on failure.

### Tests

- [x] Rust test: seat claim timeout returns error.
- [x] Rust test: ready toggle timeout returns error.
- [x] Rust test: table action timeout returns error.
- [x] Frontend test: timeout error is visible and previous UI state remains stable.

### Acceptance

- [x] No mutation command returns stale success after timeout.
- [x] Timeout errors are user-visible or debug-visible.

---

# P1 — Security and observability hardening

## P1.1 Surface LLM fallback state

### Target files

Inspect and update:

- `src-tauri/src/llm_strategy.rs`
- `src-tauri/src/llm_client.rs`
- `src-tauri/src/npc_runner.rs`
- `src-tauri/src/app_state/debug.rs`
- `src/components/debug/DebugPanel.tsx`
- NPC/profile UI if relevant

### Tasks

- [x] Add structured fallback reason enum.
- [x] Record NPC/player ID.
- [x] Record provider type.
- [x] Record profile/style.
- [x] Record selected fallback action.
- [x] Show fallback state in debug panel.
- [x] Ensure fallback does not pretend an LLM decision was used.

### Tests

- [x] Provider missing produces visible fallback reason.
- [x] LLM request failure produces visible fallback reason.
- [x] LLM invalid response produces visible fallback reason.
- [x] Debug panel renders fallback reason.

### Acceptance

- [x] User/developer can tell when NPCs are using rule-based fallback.

---

## P1.2 Make rule-based fallback respect NPC profile style

### Target files

Inspect and update:

- `src-tauri/src/llm_strategy.rs`
- NPC profile/style types
- related Rust tests

### Tasks

- [x] Find `rule_based_fallback`.
- [x] Remove unused/discarded computed style.
- [x] Implement basic style-aware fallback:
  - [x] tight/passive
  - [x] balanced
  - [x] aggressive
- [x] Keep logic simple and deterministic enough to test.

### Tests

- [x] Tight style differs from aggressive style for a useful legal-action snapshot.
- [x] Profile style is actually consumed.
- [x] Existing unprofiled NPC fallback still works.

### Acceptance

- [x] Code no longer computes profile style and discards it.

---

## P1.3 Remove silent default HTTP client fallback

### Target files

Inspect and update:

- `src-tauri/src/llm_client.rs`
- callers that construct `LlmClient`
- provider config command code if needed

### Tasks

- [x] Find any `Client::builder().build().unwrap_or_default()`.
- [x] Replace with `Result`-returning constructor.
- [x] Propagate construction errors.
- [x] Keep timeout configuration mandatory.
- [x] Make errors visible in provider test/debug flow.

### Tests

- [x] Test constructor success path.
- [x] Test or code-structure review for failure propagation.
- [x] Ensure no silent removal of timeout config.

### Acceptance

- [x] No silent fallback to default HTTP client.

---

## P1.4 Distinguish missing provider config from corrupt config

### Target files

Inspect and update:

- `src-tauri/src/app_state/llm_provider.rs`
- provider settings commands
- bootstrap state type if needed
- Settings frontend if needed

### Tasks

- [x] Define explicit provider config load states:
  - [x] missing
  - [x] loaded
  - [x] unreadable
  - [x] invalid JSON
  - [x] invalid schema
- [x] Treat missing as normal not-configured.
- [x] Treat invalid/unreadable as visible config error.
- [x] Do not swallow JSON parse errors with `.ok()`.
- [x] Add frontend display for config error if surfaced in bootstrap/settings state.

### Tests

- [x] Missing file -> not configured.
- [x] Invalid JSON -> config error.
- [x] Unreadable file -> config error if feasible.
- [x] Clear config -> not configured.

### Acceptance

- [x] Corrupt config no longer looks like "never configured."

---

## P1.5 Provider API key storage hardening

### Target files

Inspect and update:

- `src-tauri/src/app_state/llm_provider.rs`
- provider config structs
- Settings UI
- README/docs
- tests for serialization/debug state

### Tasks

- [x] Confirm whether API keys are stored in `llm-provider.json`.
- [x] Ensure API keys are never logged.
- [x] Ensure API keys are not exposed in debug state.
- [ ] Split secret from non-secret provider config.
- [ ] Implement OS keychain storage if feasible in this pass.
- [x] If not feasible, implement explicit dev-only plaintext mode:
  - [x] warning in UI,
  - [x] restricted file permissions,
  - [x] release-mode block or warning,
  - [x] documented migration task.
- [x] Add redaction tests.

### Tests

- [ ] Serialized provider config does not expose API key if keychain path is implemented.
- [x] Debug state never contains API key.
- [x] Logs never contain API key in tested paths.
- [x] Clearing provider removes secret.

### Acceptance

- [x] Release builds do not silently store API keys in ordinary plaintext JSON.

---

## P1.6 Add real Tauri CSP

### Target files

Inspect and update:

- `src-tauri/tauri.conf.json`
- frontend code if CSP reveals inline/remote assumptions

### Tasks

- [x] Replace `csp: null` with explicit CSP object/string.
- [x] Allow Tauri IPC.
- [x] Allow only required local connections.
- [x] Allow image/font/style sources needed by the app.
- [x] Do not allow broad remote scripts.
- [x] Run app in dev and build modes.
- [x] Document any non-obvious CSP source.

### Starting CSP

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

### Tests/checks

- [x] `npm run build`
- [ ] `npm run tauri dev`
- [ ] manual check for CSP violations in devtools
- [ ] production build smoke test

### Acceptance

- [x] CSP is not null.
- [x] App functionality still works.
- [x] CSP does not broadly allow arbitrary remote code.

---

## P1.7 App data directory must fail loud outside tests

### Target files

Inspect and update:

- app/instance setup code
- provider config path code
- profile storage path code
- test fixture/setup utilities

### Tasks

- [x] Find fallback to current directory or `"."`.
- [x] Replace production fallback with explicit error.
- [x] Keep tests using injected temp dirs.
- [x] Make error message clear.
- [x] Ensure provider config/profile writes cannot land in repo root by accident.

### Tests

- [x] Production path resolution failure returns error.
- [x] Test path injection still works.
- [x] Config/profile storage uses expected app data path.

### Acceptance

- [x] No silent cwd fallback outside tests.

---

## P1.8 Backend event emission observability

### Target files

Inspect and update:

- Rust files using `app.emit`
- `src-tauri/src/app_state/*`
- `src-tauri/src/networking/*`
- debug/logging utilities

### Tasks

- [x] Search for `let _ = app.emit`.
- [x] Classify each event as:
  - [x] best-effort update,
  - [x] critical correctness update.
- [x] Add helper functions:
  - [x] `emit_session_update`
  - [x] `emit_table_update`
  - [x] `emit_bootstrap_update`
- [x] Best-effort events must log warning on failure.
- [x] Critical events must return error or return refreshed state directly.
- [x] Do not introduce noisy repeated logs in tight loops.

### Tests

- [x] Unit-test helpers if practical.
- [x] Bootstrap/provider tests cover critical path.
- [x] Existing session/table tests still pass.

### Acceptance

- [x] No critical event emit failure is silently ignored.

---

# P2 — Test reliability, cleanup, and docs

## P2.1 Reduce implicit fallback behavior in test harnesses

### Target files

Inspect and update:

- `src/test/fixtures.tsx`
- app shell integration fixtures
- any `allowImplicitTableSession` code
- app shell integration tests

### Tasks

- [x] Identify test-only implicit session/table fallbacks.
- [x] Default to strict behavior where possible.
- [x] Enable fallback only in tests that explicitly need it.
- [x] Add comments explaining why each fallback is test-only.

### Tests

- [x] Route guard tests still cover unavailable states.
- [x] Table tests explicitly opt into table fixtures.

### Acceptance

- [x] Test harness does not mask production route/session bugs by default.

---

## P2.2 Documentation update

### Target files

Inspect and update:

- `README.md`
- `docs/` if present
- any developer notes/TODO files

### Tasks

- [x] Document no-silent-fallback policy.
- [x] Document canonical bootstrap event.
- [x] Document NPC profile failure behavior.
- [x] Document LLM fallback visibility.
- [x] Document provider secret storage behavior.
- [x] Document CSP rationale.
- [x] Document required validation commands.

### Acceptance

- [x] Future Claude Code passes have clear guardrails against unsafe fallbacks.

---

## P2.3 Full regression pass

### Tasks

- [x] Run frontend checks under Node 24.x:

```bash
npm run lint
npm run build
npm run test
```

- [x] Run Rust checks:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] Manual smoke test:
  - [ ] launch host instance,
  - [ ] launch client instance,
  - [ ] join via invite,
  - [ ] claim seat,
  - [ ] ready players,
  - [ ] start table,
  - [ ] play one full hand,
  - [ ] verify private cards are not visible to other players,
  - [ ] verify observer cannot see private cards,
  - [ ] verify showdown cards/history are correct,
  - [ ] verify NPC with selected profile keeps selected profile,
  - [ ] verify LLM fallback indicator appears when provider unavailable.

### Acceptance

- [x] All automated checks pass.
- [ ] Manual host/client smoke test passes.
- [x] No known silent fallback remains in P0/P1 areas.

---

# Implementation notes for Claude Code

## Do not do this

Do not fix failures by adding:

```rust
.unwrap_or_default()
```

```rust
.ok()
```

```rust
let _ = important_result;
```

```rust
or_else(|| fallback_config)
```

```ts
catch {
  // ignored
}
```

Do not add frontend-only fake success states.

Do not make missing/corrupt config look like a normal empty config.

Do not replace selected NPC profiles with generic NPCs.

Do not claim NPC action success unless the backend accepted the action.

## Do this instead

Use explicit `Result` types.

Return clear errors.

Keep prior UI state stable when a mutation fails.

Make fallback state visible.

Add tests for failure paths.

Prefer small focused changes over a broad rewrite.

---

# Suggested branch/checkpoint order

1. `p0-bootstrap-provider-state`
2. `p0-npc-profile-config-mapping`
3. `p0-npc-action-and-command-acks`
4. `p1-llm-fallback-observability`
5. `p1-provider-config-security`
6. `p1-tauri-csp`
7. `p2-docs-regression`

Each checkpoint must keep:

```bash
npm run lint
npm run build
npm run test
```

green before moving to the next checkpoint.
