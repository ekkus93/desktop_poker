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

- [ ] Identify the canonical frontend event currently used for bootstrap updates.
- [ ] Identify the backend event currently emitted after LLM provider save/clear.
- [ ] Replace split event names with one canonical event: `desktop://bootstrap`.
- [ ] Ensure backend emits a refreshed `DesktopBootstrapState` payload, not `()`.
- [ ] Ensure frontend subscription consumes the payload and updates bootstrap context.
- [ ] Update backend `bootstrap()` command so it does not return stale provider fields.
- [ ] Recompute or refresh:
  - [ ] `llmApiKeyConfigured`
  - [ ] `llmProviderType`
- [ ] Ensure saving provider config updates UI without restart.
- [ ] Ensure clearing provider config updates UI without restart.
- [ ] Do not ignore event emit failure on this critical path.

### Tests

- [ ] Add/update frontend test: provider save event refreshes bootstrap state.
- [ ] Add/update frontend test: provider clear event refreshes bootstrap state.
- [ ] Add/update Rust test: bootstrap command reflects live provider config after save.
- [ ] Add/update Rust test: bootstrap command reflects live provider config after clear.

### Acceptance

- [ ] `npm run test` passes.
- [ ] `npm run build` passes.
- [ ] Settings UI does not require restart to reflect provider status.

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

- [ ] Find all places that load an explicit NPC profile by `profile_id`.
- [ ] Remove `.ok()`, `unwrap_or_default`, or generic `None` fallback for explicit selected profiles.
- [ ] Return a structured error when explicit profile load fails.
- [ ] Make sure the NPC is not added when its explicit profile fails to load.
- [ ] Keep unprofiled/generic NPCs allowed only when no explicit profile was requested.
- [ ] Make the frontend display the profile load error clearly.

### Tests

- [ ] Rust test: valid explicit profile creates profiled NPC.
- [ ] Rust test: missing explicit profile returns error and creates no NPC.
- [ ] Rust test: corrupt explicit profile returns error and creates no NPC.
- [ ] Frontend test: NPC add profile error is visible and does not imply success.

### Acceptance

- [ ] No selected profile silently becomes generic.
- [ ] Error message identifies the failed profile enough to diagnose it.

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

- [ ] Find where NPC runtime maps `player_id` / seat to `npc_configs`.
- [ ] Remove fallback equivalent to `npc_configs.first()`.
- [ ] Add a stable mapping:
  - [ ] by explicit `player_id`, or
  - [ ] by explicit `seat_index`, or
  - [ ] by stored `HashMap<PlayerId, NpcPlayerConfig>`.
- [ ] Ensure each created NPC has exactly one matching config.
- [ ] If mapping is missing, record/return a visible error.
- [ ] Make sure multiple NPCs keep distinct names, styles, and profiles.

### Tests

- [ ] Rust test: two NPCs with different profiles keep correct profiles.
- [ ] Rust test: two NPCs with different styles keep correct styles.
- [ ] Rust test: missing mapping does not fall back to first config.
- [ ] Integration test if available: host with multiple NPCs and inspect/debug identities.

### Acceptance

- [ ] No profile/style shifting between NPCs.
- [ ] No generic fallback when mapping fails.

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

- [ ] Find all `submit_action` calls from NPC logic.
- [ ] Replace ignored result handling with explicit `match`.
- [ ] Return/record failure when submission fails.
- [ ] Add structured NPC action outcome:
  - [ ] success
  - [ ] rejected
  - [ ] stale window
  - [ ] illegal action
  - [ ] runtime unavailable
- [ ] Add failure detail to debug state/logs.
- [ ] Prevent tight retry loops after repeated NPC failure.

### Tests

- [ ] Rust test: illegal NPC action records failure.
- [ ] Rust test: stale NPC action window records failure.
- [ ] Rust test: NPC runner does not claim success after rejected action.
- [ ] Frontend debug test if debug state shape changes.

### Acceptance

- [ ] No `let _ = host_server.submit_action(...)` in NPC action path.
- [ ] Failed NPC actions are diagnosable.

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

- [ ] Find the helper that waits for runtime state after a command.
- [ ] Change it to return explicit observed/timed-out result.
- [ ] For mutation commands, return error if acknowledgement is not observed.
- [ ] Apply to:
  - [ ] seat claim
  - [ ] ready toggle
  - [ ] start table
  - [ ] leave session if applicable
  - [ ] table action
- [ ] Ensure frontend shows timeout/retry error instead of silently accepting stale state.
- [ ] Preserve current state on failure.

### Tests

- [ ] Rust test: seat claim timeout returns error.
- [ ] Rust test: ready toggle timeout returns error.
- [ ] Rust test: table action timeout returns error.
- [ ] Frontend test: timeout error is visible and previous UI state remains stable.

### Acceptance

- [ ] No mutation command returns stale success after timeout.
- [ ] Timeout errors are user-visible or debug-visible.

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

- [ ] Add structured fallback reason enum.
- [ ] Record NPC/player ID.
- [ ] Record provider type.
- [ ] Record profile/style.
- [ ] Record selected fallback action.
- [ ] Show fallback state in debug panel.
- [ ] Ensure fallback does not pretend an LLM decision was used.

### Tests

- [ ] Provider missing produces visible fallback reason.
- [ ] LLM request failure produces visible fallback reason.
- [ ] LLM invalid response produces visible fallback reason.
- [ ] Debug panel renders fallback reason.

### Acceptance

- [ ] User/developer can tell when NPCs are using rule-based fallback.

---

## P1.2 Make rule-based fallback respect NPC profile style

### Target files

Inspect and update:

- `src-tauri/src/llm_strategy.rs`
- NPC profile/style types
- related Rust tests

### Tasks

- [ ] Find `rule_based_fallback`.
- [ ] Remove unused/discarded computed style.
- [ ] Implement basic style-aware fallback:
  - [ ] tight/passive
  - [ ] balanced
  - [ ] aggressive
- [ ] Keep logic simple and deterministic enough to test.

### Tests

- [ ] Tight style differs from aggressive style for a useful legal-action snapshot.
- [ ] Profile style is actually consumed.
- [ ] Existing unprofiled NPC fallback still works.

### Acceptance

- [ ] Code no longer computes profile style and discards it.

---

## P1.3 Remove silent default HTTP client fallback

### Target files

Inspect and update:

- `src-tauri/src/llm_client.rs`
- callers that construct `LlmClient`
- provider config command code if needed

### Tasks

- [ ] Find any `Client::builder().build().unwrap_or_default()`.
- [ ] Replace with `Result`-returning constructor.
- [ ] Propagate construction errors.
- [ ] Keep timeout configuration mandatory.
- [ ] Make errors visible in provider test/debug flow.

### Tests

- [ ] Test constructor success path.
- [ ] Test or code-structure review for failure propagation.
- [ ] Ensure no silent removal of timeout config.

### Acceptance

- [ ] No silent fallback to default HTTP client.

---

## P1.4 Distinguish missing provider config from corrupt config

### Target files

Inspect and update:

- `src-tauri/src/app_state/llm_provider.rs`
- provider settings commands
- bootstrap state type if needed
- Settings frontend if needed

### Tasks

- [ ] Define explicit provider config load states:
  - [ ] missing
  - [ ] loaded
  - [ ] unreadable
  - [ ] invalid JSON
  - [ ] invalid schema
- [ ] Treat missing as normal not-configured.
- [ ] Treat invalid/unreadable as visible config error.
- [ ] Do not swallow JSON parse errors with `.ok()`.
- [ ] Add frontend display for config error if surfaced in bootstrap/settings state.

### Tests

- [ ] Missing file -> not configured.
- [ ] Invalid JSON -> config error.
- [ ] Unreadable file -> config error if feasible.
- [ ] Clear config -> not configured.

### Acceptance

- [ ] Corrupt config no longer looks like “never configured.”

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

- [ ] Confirm whether API keys are stored in `llm-provider.json`.
- [ ] Ensure API keys are never logged.
- [ ] Ensure API keys are not exposed in debug state.
- [ ] Split secret from non-secret provider config.
- [ ] Implement OS keychain storage if feasible in this pass.
- [ ] If not feasible, implement explicit dev-only plaintext mode:
  - [ ] warning in UI,
  - [ ] restricted file permissions,
  - [ ] release-mode block or warning,
  - [ ] documented migration task.
- [ ] Add redaction tests.

### Tests

- [ ] Serialized provider config does not expose API key if keychain path is implemented.
- [ ] Debug state never contains API key.
- [ ] Logs never contain API key in tested paths.
- [ ] Clearing provider removes secret.

### Acceptance

- [ ] Release builds do not silently store API keys in ordinary plaintext JSON.

---

## P1.6 Add real Tauri CSP

### Target files

Inspect and update:

- `src-tauri/tauri.conf.json`
- frontend code if CSP reveals inline/remote assumptions

### Tasks

- [ ] Replace `csp: null` with explicit CSP object/string.
- [ ] Allow Tauri IPC.
- [ ] Allow only required local connections.
- [ ] Allow image/font/style sources needed by the app.
- [ ] Do not allow broad remote scripts.
- [ ] Run app in dev and build modes.
- [ ] Document any non-obvious CSP source.

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

- [ ] `npm run build`
- [ ] `npm run tauri dev`
- [ ] manual check for CSP violations in devtools
- [ ] production build smoke test

### Acceptance

- [ ] CSP is not null.
- [ ] App functionality still works.
- [ ] CSP does not broadly allow arbitrary remote code.

---

## P1.7 App data directory must fail loud outside tests

### Target files

Inspect and update:

- app/instance setup code
- provider config path code
- profile storage path code
- test fixture/setup utilities

### Tasks

- [ ] Find fallback to current directory or `"."`.
- [ ] Replace production fallback with explicit error.
- [ ] Keep tests using injected temp dirs.
- [ ] Make error message clear.
- [ ] Ensure provider config/profile writes cannot land in repo root by accident.

### Tests

- [ ] Production path resolution failure returns error.
- [ ] Test path injection still works.
- [ ] Config/profile storage uses expected app data path.

### Acceptance

- [ ] No silent cwd fallback outside tests.

---

## P1.8 Backend event emission observability

### Target files

Inspect and update:

- Rust files using `app.emit`
- `src-tauri/src/app_state/*`
- `src-tauri/src/networking/*`
- debug/logging utilities

### Tasks

- [ ] Search for `let _ = app.emit`.
- [ ] Classify each event as:
  - [ ] best-effort update,
  - [ ] critical correctness update.
- [ ] Add helper functions:
  - [ ] `emit_session_update`
  - [ ] `emit_table_update`
  - [ ] `emit_bootstrap_update`
- [ ] Best-effort events must log warning on failure.
- [ ] Critical events must return error or return refreshed state directly.
- [ ] Do not introduce noisy repeated logs in tight loops.

### Tests

- [ ] Unit-test helpers if practical.
- [ ] Bootstrap/provider tests cover critical path.
- [ ] Existing session/table tests still pass.

### Acceptance

- [ ] No critical event emit failure is silently ignored.

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

- [ ] Identify test-only implicit session/table fallbacks.
- [ ] Default to strict behavior where possible.
- [ ] Enable fallback only in tests that explicitly need it.
- [ ] Add comments explaining why each fallback is test-only.

### Tests

- [ ] Route guard tests still cover unavailable states.
- [ ] Table tests explicitly opt into table fixtures.

### Acceptance

- [ ] Test harness does not mask production route/session bugs by default.

---

## P2.2 Documentation update

### Target files

Inspect and update:

- `README.md`
- `docs/` if present
- any developer notes/TODO files

### Tasks

- [ ] Document no-silent-fallback policy.
- [ ] Document canonical bootstrap event.
- [ ] Document NPC profile failure behavior.
- [ ] Document LLM fallback visibility.
- [ ] Document provider secret storage behavior.
- [ ] Document CSP rationale.
- [ ] Document required validation commands.

### Acceptance

- [ ] Future Claude Code passes have clear guardrails against unsafe fallbacks.

---

## P2.3 Full regression pass

### Tasks

- [ ] Run frontend checks under Node 24.x:

```bash
npm run lint
npm run build
npm run test
```

- [ ] Run Rust checks:

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

- [ ] All automated checks pass.
- [ ] Manual host/client smoke test passes.
- [ ] No known silent fallback remains in P0/P1 areas.

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
