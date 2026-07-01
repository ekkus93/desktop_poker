# Desktop Poker Fix 8 TODO — Lock Shared-Core Baseline and CI Validation

This TODO is intentionally explicit for Claude Code. Do tasks in priority order. Prefer small, test-backed patches. Do not introduce hidden fallback behavior to make tests pass.

Fix 8 is a stabilization pass. Do not add new features. Do not start Android implementation.

---

## P0.1 — Update GitHub CI to validate the full Cargo workspace

**Files:**

- `.github/workflows/ci.yml`
- possibly `.github/workflows/*.yml` if additional CI workflows exist

### Problem

The project now has a root Cargo workspace and `crates/poker-core`, but GitHub CI still validates only the old desktop crate with commands like:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=2
```

Those commands can miss `poker-core` tests, formatting, clippy warnings, and dependency drift.

### Required change

Update CI so Rust validation runs from the repository root against the workspace.

Suggested replacement:

```yaml
- name: Check Rust formatting
  run: cargo fmt --check

- name: Lint Rust
  run: cargo clippy --workspace --all-targets --all-features -- -D warnings

- name: Test Rust
  run: cargo test --workspace --all-targets --all-features -- --test-threads=2

- name: Audit poker-core dependency tree
  run: cargo tree -p poker-core
```

If the workflow uses a Rust cache action, update it to cache the root workspace instead of only `src-tauri`.

Example direction:

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    workspaces: . -> target
```

Adjust the exact cache syntax to match the action’s documentation and current workflow style.

### Important

It is acceptable for Tauri packaging or launch-specific CI steps to use `src-tauri` where appropriate. It is not acceptable for primary validation to validate only `src-tauri`.

### Tests / validation

Run locally if possible:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo tree -p poker-core
```

### Acceptance

- GitHub CI uses workspace-wide Rust validation.
- CI validates `poker-core`.
- No primary validation step uses `--manifest-path src-tauri/Cargo.toml` as a substitute for workspace validation.
- Rust cache configuration is compatible with the root workspace.

---

## P0.2 — Update stale Copilot/GitHub instructions to the workspace layout

**Files:**

- `.github/copilot-instructions.md`
- `README.md`
- `CLAUDE.md`
- `.claude/skills/lint-n-test/SKILL.md`
- any docs that mention old single-crate Rust layout

### Problem

`.github/copilot-instructions.md` still describes the old layout as something like:

```text
Rust backend lives in `src-tauri/` (single crate, not a workspace) — Cargo commands need `--manifest-path src-tauri/Cargo.toml`.
```

That is now false. Future agents may follow those instructions and validate the wrong target.

### Required change

Search for stale guidance:

```bash
rg -n "single crate|not a workspace|--manifest-path src-tauri/Cargo.toml|cd src-tauri|cargo fmt|cargo clippy|cargo test|cargo tree" \
  README.md CLAUDE.md .github .claude docs package.json
```

Replace stale validation guidance with workspace-aware instructions.

Suggested text:

```md
## Rust workspace

This repository is a Cargo workspace.

- `crates/poker-core` is the shared platform-neutral poker rules/state/projection crate.
- `src-tauri` is the Tauri desktop adapter crate.

Run Rust validation from the repository root:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test -p poker-core
cargo tree -p poker-core
```

Use `--manifest-path src-tauri/Cargo.toml` only for commands that specifically launch or package the desktop Tauri app. Do not use it for primary validation.
```

### Acceptance

- `.github/copilot-instructions.md` no longer says the project is a single Rust crate.
- No developer instruction says Rust validation requires `--manifest-path src-tauri/Cargo.toml`.
- Workspace-wide validation commands appear in the primary developer docs.
- Desktop launch/package commands may still mention `src-tauri` if the context is explicit.

---

## P0.3 — Render `streamTimeoutErrorCount` in the debug UI

**Files:**

- `src/api/desktop.ts`
- `src/components/debug/DebugPanel.tsx`
- `src/components/debug/DebugPanel.test.tsx` or equivalent tests

### Problem

The frontend `HostRuntimeHealth` type now includes `streamTimeoutErrorCount`, and the debug panel may use it when deciding whether to show the Host Runtime Health section. But the actual counter is not rendered as a visible row.

This means a non-zero stream-timeout error count can be hidden even though Rust recorded it.

### Required change

Ensure every `HostRuntimeHealth` field is present in the TypeScript type and every non-zero counter is rendered.

Expected type shape:

```ts
export type HostRuntimeHealth = {
  acceptErrorCount: number;
  streamTimeoutErrorCount: number;
  tickAdvanceErrorCount: number;
  publishErrorCount: number;
  stateLockErrorCount: number;
  streamCloneErrorCount: number;
  clientRegistryErrorCount: number;
  reconnectMarkErrorCount: number;
  snapshotSyncErrorCount: number;
  lastError: string | null;
  lastSuccessfulTickMs: number | null;
  lastSuccessfulPublishMs: number | null;
};
```

If the current `DebugPanel` uses inline conditionals, keep that style. Add the missing row:

```tsx
{debugState.hostRuntimeHealth.streamTimeoutErrorCount > 0 && (
  <li>
    <strong>Stream timeout errors:</strong>{" "}
    {debugState.hostRuntimeHealth.streamTimeoutErrorCount}
  </li>
)}
```

Also ensure the newer counters are rendered:

```tsx
{debugState.hostRuntimeHealth.streamCloneErrorCount > 0 && (
  <li>
    <strong>Stream clone errors:</strong>{" "}
    {debugState.hostRuntimeHealth.streamCloneErrorCount}
  </li>
)}
{debugState.hostRuntimeHealth.clientRegistryErrorCount > 0 && (
  <li>
    <strong>Client registry errors:</strong>{" "}
    {debugState.hostRuntimeHealth.clientRegistryErrorCount}
  </li>
)}
{debugState.hostRuntimeHealth.reconnectMarkErrorCount > 0 && (
  <li>
    <strong>Reconnect mark errors:</strong>{" "}
    {debugState.hostRuntimeHealth.reconnectMarkErrorCount}
  </li>
)}
{debugState.hostRuntimeHealth.snapshotSyncErrorCount > 0 && (
  <li>
    <strong>Snapshot sync errors:</strong>{" "}
    {debugState.hostRuntimeHealth.snapshotSyncErrorCount}
  </li>
)}
```

### Tests

Add/update a debug panel test with all counters non-zero.

Example intent:

```ts
it("renders every non-zero host runtime health counter", () => {
  render(
    <DebugPanel
      debugState={stateWithHostRuntimeHealth({
        acceptErrorCount: 1,
        streamTimeoutErrorCount: 2,
        tickAdvanceErrorCount: 3,
        publishErrorCount: 4,
        stateLockErrorCount: 5,
        streamCloneErrorCount: 6,
        clientRegistryErrorCount: 7,
        reconnectMarkErrorCount: 8,
        snapshotSyncErrorCount: 9,
        lastError: "test error",
        lastSuccessfulTickMs: 100,
        lastSuccessfulPublishMs: 200,
      })}
    />,
  );

  expect(screen.getByText(/Accept errors:/i)).toHaveTextContent("1");
  expect(screen.getByText(/Stream timeout errors:/i)).toHaveTextContent("2");
  expect(screen.getByText(/Tick advance errors:/i)).toHaveTextContent("3");
  expect(screen.getByText(/Publish errors:/i)).toHaveTextContent("4");
  expect(screen.getByText(/State lock errors:/i)).toHaveTextContent("5");
  expect(screen.getByText(/Stream clone errors:/i)).toHaveTextContent("6");
  expect(screen.getByText(/Client registry errors:/i)).toHaveTextContent("7");
  expect(screen.getByText(/Reconnect mark errors:/i)).toHaveTextContent("8");
  expect(screen.getByText(/Snapshot sync errors:/i)).toHaveTextContent("9");
});
```

Adapt helper names and component props to the current tests.

### Acceptance

- `streamTimeoutErrorCount` is visibly rendered when non-zero.
- Every Rust health counter has a TypeScript field and a visible non-zero UI row.
- Debug panel tests cover the new/missing row.
- `npm run lint`, `npm run build`, and `npm test` pass.

---

## P0.4 — Make frontend formatting validation clean and enforceable

**Files:**

- `package.json`
- files reported by `npm run format:check`
- `.github/workflows/ci.yml` if CI should enforce formatting
- docs that list validation commands

### Problem

The code can pass lint/build/test while `npm run format:check` fails. The latest review found several frontend files needing Prettier formatting.

Formatting drift is not a runtime bug, but it causes unnecessary review noise and makes diffs harder to read.

### Required change

Ensure `format:check` exists and passes.

If `package.json` already has it, use the existing script. If missing, add a non-mutating check script:

```json
{
  "scripts": {
    "format": "prettier --write \"src/**/*.{ts,tsx,css}\"",
    "format:check": "prettier --check \"src/**/*.{ts,tsx,css}\""
  }
}
```

Adjust the glob if the current project formats additional files.

Run:

```bash
npm run format
npm run format:check
```

If CI is intended to enforce formatting, add:

```yaml
- name: Check frontend formatting
  run: npm run format:check
```

### Acceptance

- `npm run format:check` passes.
- No test suppresses formatting output.
- Primary validation docs include `npm run format:check` if formatting is considered required.
- CI enforces formatting if project convention expects it.

---

## P0.5 — Add real NPC action-path coverage for missing acting hole cards

**Files:**

- `src-tauri/src/npc/runner/tests.rs`
- `src-tauri/src/npc/runner/action.rs` if small test hooks are needed
- `src-tauri/src/networking/runtime/host.rs` only if a test-only state mutation helper is needed

### Problem

The production code now validates acting NPC hole cards through `validated_acting_hole_cards(...)`. Helper tests exist, but the regression coverage is still weaker than ideal: it does not prove the actual NPC action attempt path records a structured error and submits no host action when authoritative hand state is corrupt.

### Required change

Add at least one near-production/action-path regression test.

Preferred: create a host/session state with an acting NPC, corrupt that NPC’s hole-card entry, run the NPC action attempt path, and assert no action is submitted.

If direct access to host authoritative state is needed, add a test-only helper. Keep it behind `#[cfg(test)]`.

Suggested helper direction in the appropriate host/server module:

```rust
#[cfg(test)]
impl HostServer {
    pub(crate) fn mutate_authoritative_state_for_test(
        &self,
        f: impl FnOnce(&mut poker_core::domain::TournamentState),
    ) {
        let mut state = self
            .authoritative_state
            .lock()
            .expect("authoritative state lock should not be poisoned in test");
        f(&mut state);
    }
}
```

Adjust field/module access to match the current `HostServer` layout. Do not expose this helper in production builds.

Test intent:

```rust
#[test]
fn acting_npc_missing_hole_cards_submits_no_action() {
    // Arrange:
    // - host/session with an active hand;
    // - acting NPC player;
    // - current action window for that NPC;
    // - capture hand-log/action count before the NPC attempts to act.

    host_server.mutate_authoritative_state_for_test(|state| {
        let hand = state
            .current_hand
            .as_mut()
            .expect("test should have active hand");
        hand.hole_cards_by_player_id.remove("npc-1");
    });

    // Act:
    // call the same try_npc_action / runner-adjacent function used by the real runner loop.

    // Assert:
    // - outcome is RuntimeUnavailable or equivalent non-success;
    // - last_npc_action_error.reason == InternalError;
    // - error message mentions missing hole cards;
    // - hand-log/action count did not increase;
    // - no accepted host action was applied.
}
```

If a full `HostServer` action-window test is still too expensive, document why in a test comment and add the strongest near-production test available: call the real action-building function after constructing the same `FreshActionWindow`/`CurrentHand` data used by `try_npc_action`.

### Acceptance

- There is at least one test beyond the pure helper test.
- The test exercises the real NPC action attempt path or a clearly documented near-production equivalent.
- The test proves corrupt acting hole-card state submits no action and records structured NPC error.
- Test-only mutation helpers are behind `#[cfg(test)]`.

---

## P1.1 — Keep `poker-core` platform-neutral and document audit result

**Files:**

- `crates/poker-core/Cargo.toml`
- `crates/poker-core/src/**`
- `memory.md`

### Required validation

Run:

```bash
cargo tree -p poker-core
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|Mutex<.*Tcp|thread::spawn|Command" crates/poker-core
```

Expected:

- no Tauri dependency;
- no socket/networking dependency;
- no LLM/provider/keychain dependency;
- no desktop/Android UI dependency;
- no thread spawning;
- no process spawning;
- no platform app-data-path logic.

Important: `EngineCommand` is expected and harmless for the broad `Command` search. Do not remove or rename `EngineCommand` just to satisfy the grep.

### Acceptance

- `cargo tree -p poker-core` is small and platform-neutral.
- Forbidden dependency grep has no code hits except expected harmless terms like `EngineCommand`.
- If a textual/comment hit remains, classify it in `memory.md` or implementation notes.

---

## P1.2 — Update `memory.md` honestly after Fix 8 validation

**Files:**

- `memory.md`

### Problem

`memory.md` should not claim commands passed unless they actually passed in the current implementation environment. It should distinguish local Claude Code validation from external review limitations.

### Required change

After completing Fix 8 and running validation, add a new entry using the project timestamp convention:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

Suggested entry:

```md
- <timestamp> — Fix 8 completed: GitHub CI and Copilot/developer docs now use workspace-wide Rust validation; DebugPanel renders all HostRuntimeHealth counters including stream timeout errors; frontend formatting validation passes; NPC missing/invalid acting hole-card state has action-path or documented near-production regression coverage; poker-core purity audit remains clean. Validation run in this environment: <exact commands actually run and pass/fail status>. Architecture unchanged: Android will be native Kotlin/Compose + Rust bindings; Kotlin owns networking/session transport; poker-core owns deterministic poker rules/state/projection only.
```

Do not claim Rust commands passed unless they were actually run and passed.

### Acceptance

- `memory.md` records exact commands actually run.
- No false completion claims.
- Android/core architecture decision remains recorded.

---

## P1.3 — Verify previous hardening stays fixed

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- `src-tauri/src/npc/runner/action.rs`
- `src-tauri/src/npc/provider_storage.rs`
- `src/main.tsx`
- `src/test/setup.ts`

### Required audits

Run:

```bash
npm test 2>&1 | tee /tmp/desktop-poker-npm-test.log
rg -n "Failed to initialize window state persistence|currentWindow" /tmp/desktop-poker-npm-test.log

npm run build
rg -n "__DESKTOP_POKER_BROWSER_MOCKS__|LayoutProbeApp" dist || true

rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated" src-tauri/src/networking
rg -n "hole_cards_by_player_id.*unwrap_or\(|unwrap_or\(&\[\]\)" src-tauri/src/npc src-tauri/src/tournament crates/poker-core/src || true
rg -n "thread::spawn" src-tauri/src/networking/runtime/client.rs
```

### Acceptance

- No window-persistence test noise.
- Production dist does not contain browser mock/probe setup code.
- No private-message empty-AAD fallback.
- No acting NPC empty-hole-card fallback.
- No raw client runtime `thread::spawn`.

---

## P2.1 — Silent-failure audit for touched files

**Files:**

- files touched by Fix 8
- especially `.github/workflows/ci.yml`, `.github/copilot-instructions.md`, `DebugPanel.tsx`, NPC runner tests/helpers, and runtime files if touched

### Required audit

For code files touched by Fix 8, run an appropriate subset of:

```bash
rg -n "let _ =|\.ok\(\)|unwrap_or\(|unwrap_or_else\(|thread::spawn|continue;|return;" \
  src-tauri/src/networking/runtime src-tauri/src/npc src-tauri/src/app_state crates/poker-core/src
```

For every hit in touched code:

- convert real silent failures to structured errors/diagnostics;
- add a short comment for intentional best-effort cleanup;
- leave harmless presentation-only defaults alone;
- do not rewrite unrelated stable code just to reduce grep output.

### Acceptance

- No newly touched runtime/NPC/core code contains unexplained silent failure behavior.
- Any ignored error has a short comment explaining why it is safe.
- No new fallback invents authoritative poker state.

---

## P2.2 — Do not start Android implementation in Fix 8

**Files:**

- docs only, if needed

### Required rule

Do not add:

- Android Gradle project files;
- Kotlin source files;
- Tauri Mobile configuration;
- `poker-android-ffi` crate;
- networking inside `poker-core`.

A future Android pass can add a binding crate such as:

```text
crates/poker-android-ffi/
  depends on poker-core
  exposes UniFFI/JNI-safe DTOs
  no Tauri dependency
  no Android UI code inside Rust
  no networking inside poker-core
```

Fix 8 is cleanup and stabilization only.

### Acceptance

- No half-built Android implementation appears.
- Existing Android architecture doc remains accurate.

---

## Final validation commands

Run from repo root with Node 24 active:

```bash
npm ci
npm run format:check
npm run lint
npm run build
npm test
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test -p poker-core
cargo tree -p poker-core
```

Focused audits:

```bash
npm test 2>&1 | tee /tmp/desktop-poker-npm-test.log
rg -n "Failed to initialize window state persistence|currentWindow" /tmp/desktop-poker-npm-test.log

npm run build
rg -n "__DESKTOP_POKER_BROWSER_MOCKS__|LayoutProbeApp" dist || true

rg -n "--manifest-path src-tauri/Cargo.toml|single crate|not a workspace" README.md CLAUDE.md .github .claude docs package.json
rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated" src-tauri/src/networking
rg -n "hole_cards_by_player_id.*unwrap_or\(|unwrap_or\(&\[\]\)" src-tauri/src/npc src-tauri/src/tournament crates/poker-core/src || true
rg -n "thread::spawn" src-tauri/src/networking/runtime/client.rs
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn|Command" crates/poker-core
```

Expected notes:

- `EngineCommand` in `poker-core` is an expected false positive for the broad `Command` grep.
- Desktop launch/package commands may still use `--manifest-path src-tauri/Cargo.toml` if explicitly documented as launch/package commands rather than validation.
- Production dist grep should have no JS payload hits for `LayoutProbeApp` or `__DESKTOP_POKER_BROWSER_MOCKS__`.

## Definition of done

- [ ] GitHub CI validates the full Cargo workspace.
- [ ] GitHub CI validates `poker-core`.
- [ ] `.github/copilot-instructions.md` and primary docs describe the workspace layout correctly.
- [ ] README/dev docs use workspace-wide Cargo validation commands.
- [ ] Debug UI renders `streamTimeoutErrorCount` and every other non-zero host runtime health counter.
- [ ] Frontend formatting validation passes.
- [ ] NPC missing/invalid acting hole-card state has action-path or clearly documented near-production regression coverage.
- [ ] `poker-core` purity audit is clean.
- [ ] Previous hardening remains fixed: no test noise, no production mock chunk, no empty-AAD fallback, no empty NPC hole-card fallback, no raw client runtime `thread::spawn`.
- [ ] `memory.md` accurately records commands actually run.
- [ ] No new hidden fallbacks or silent failures are introduced.
- [ ] No Android app, Tauri Mobile path, Android FFI crate, or networking-in-core is added.
- [ ] All final validation commands pass in an environment with Node 24 and Rust installed.
