# Desktop Poker Fix 7 TODO — Finish Core Extraction Cleanup and Runtime Diagnostics

This TODO is intentionally explicit for Claude Code. Do tasks in priority order. Prefer small, test-backed patches. Do not introduce hidden fallback behavior to make tests pass.

Fix 6 got the project close to a clean shared-core baseline. Fix 7 should be a focused cleanup pass, not a new feature expansion.

---

## P0.1 — Update README and developer docs to workspace Cargo commands

**Files:**

- `README.md`
- `CLAUDE.md`
- `.claude/skills/lint-n-test/SKILL.md` if present
- any docs/scripts that mention cargo validation

### Problem

The repo now has a root Cargo workspace and `crates/poker-core`, but `README.md` still contains validation commands like:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Those commands are no longer sufficient for validation because they can miss `poker-core`.

### Required change

Search for old validation commands:

```bash
rg -n "cargo fmt|cargo clippy|cargo test|cargo tree|--manifest-path src-tauri/Cargo.toml|cd src-tauri" README.md CLAUDE.md .claude docs package.json
```

Replace validation guidance with workspace commands:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test -p poker-core
cargo tree -p poker-core
```

Keep `--manifest-path src-tauri/Cargo.toml` only for commands that explicitly launch or package the desktop Tauri app, such as running a second desktop instance. Do not use it as the primary validation command.

### Suggested README wording

```md
### Rust validation

This repository is a Cargo workspace. Run Rust validation from the repository root:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test -p poker-core
cargo tree -p poker-core
```

`src-tauri` is the desktop adapter crate. `crates/poker-core` is the shared rules/state/projection crate for desktop and future Android bindings.
```

### Acceptance

- No validation section tells developers to validate only with `--manifest-path src-tauri/Cargo.toml`.
- Workspace-wide Rust validation commands appear in README or the repo’s primary developer docs.
- Desktop launch/package commands may still use `src-tauri` where appropriate.

---

## P0.2 — Surface all Rust `HostRuntimeHealth` counters in TypeScript and debug UI

**Files:**

- `src/api/desktop.ts`
- `src/components/DebugPanel.tsx` or equivalent debug inspector component
- frontend tests if debug panel tests exist

### Problem

Rust `HostRuntimeHealth` now includes additional counters such as:

- `stream_clone_error_count`
- `client_registry_error_count`
- `reconnect_mark_error_count`
- `snapshot_sync_error_count`

The frontend type and debug UI still model/display only older fields. This hides diagnostics that the Rust runtime is now recording.

### Required change

Update the frontend `HostRuntimeHealth` type to include all Rust fields using camelCase names.

Suggested type:

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

Update the debug UI so the new counters are visible when non-zero.

Suggested helper:

```tsx
type HealthCounterRow = {
  label: string;
  value: number;
};

function hostHealthCounterRows(health: HostRuntimeHealth): HealthCounterRow[] {
  return [
    { label: "Accept errors", value: health.acceptErrorCount },
    { label: "Stream timeout errors", value: health.streamTimeoutErrorCount },
    { label: "Tick advance errors", value: health.tickAdvanceErrorCount },
    { label: "Publish errors", value: health.publishErrorCount },
    { label: "State lock errors", value: health.stateLockErrorCount },
    { label: "Stream clone errors", value: health.streamCloneErrorCount },
    { label: "Client registry errors", value: health.clientRegistryErrorCount },
    { label: "Reconnect mark errors", value: health.reconnectMarkErrorCount },
    { label: "Snapshot sync errors", value: health.snapshotSyncErrorCount },
  ].filter((row) => row.value !== 0);
}
```

Suggested rendering pattern:

```tsx
{debug.hostRuntimeHealth ? (
  <section aria-label="Host runtime health">
    <h3>Host Runtime Health</h3>

    {debug.hostRuntimeHealth.lastError ? (
      <p role="status">Last error: {debug.hostRuntimeHealth.lastError}</p>
    ) : (
      <p>No host runtime errors recorded.</p>
    )}

    <ul>
      {hostHealthCounterRows(debug.hostRuntimeHealth).map((row) => (
        <li key={row.label}>
          {row.label}: {row.value}
        </li>
      ))}
    </ul>
  </section>
) : null}
```

If the existing UI uses a table or compact debug cards, adapt the display to match current style. Do not hide non-zero counters.

### Tests

If there are debug panel tests, add a fixture with non-zero new counters and assert the labels/values render.

Example intent:

```ts
it("renders new host runtime health counters", () => {
  render(<DebugPanel debugState={stateWithHostHealth({
    streamCloneErrorCount: 2,
    clientRegistryErrorCount: 1,
    reconnectMarkErrorCount: 3,
    snapshotSyncErrorCount: 4,
  })} />);

  expect(screen.getByText(/Stream clone errors: 2/i)).toBeInTheDocument();
  expect(screen.getByText(/Client registry errors: 1/i)).toBeInTheDocument();
  expect(screen.getByText(/Reconnect mark errors: 3/i)).toBeInTheDocument();
  expect(screen.getByText(/Snapshot sync errors: 4/i)).toBeInTheDocument();
});
```

### Acceptance

- TypeScript type includes every Rust `HostRuntimeHealth` field.
- Debug UI can display every non-zero health counter.
- Frontend tests/build/lint pass.

---

## P0.3 — Record remaining host-session authoritative-state lock failures

**Files:**

- `src-tauri/src/networking/runtime/host_session.rs`
- `src-tauri/src/networking/runtime/mod.rs` or wherever `HostRuntimeHealth` lives
- runtime tests if practical

### Problem

Some host-session authoritative-state lock failures still lead to rejection/disconnect/break without updating `HostRuntimeHealth.state_lock_error_count`.

That is a diagnostic gap. A poisoned authoritative-state lock is an internal runtime failure and must be visible.

### Required change

Add a helper in `host_session.rs` or a shared runtime health module:

```rust
fn record_state_lock_error(
    runtime_health: &Arc<Mutex<HostRuntimeHealth>>,
    message: impl Into<String>,
) {
    update_health(runtime_health, |health| {
        health.state_lock_error_count += 1;
        health.record_error(message);
    });
}
```

Use it before every branch in `host_session.rs` that handles a poisoned authoritative-state lock.

Search:

```bash
rg -n "authoritative_state\.lock\(\)|state lock poisoned|lock poisoned|map_err\(\|_\| NetworkingError::new" src-tauri/src/networking/runtime/host_session.rs
```

For patterns like this:

```rust
let previous_state = authoritative_state
    .lock()
    .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))
    .map(|state| state.clone());
```

Prefer explicit handling:

```rust
let previous_state = match authoritative_state.lock() {
    Ok(state) => state.clone(),
    Err(_) => {
        record_state_lock_error(
            &runtime_health,
            format!(
                "authoritative state lock poisoned before applying action for player {player_id}"
            ),
        );

        let _ = send_action_rejection_best_effort(
            &stream_handle,
            &player_id,
            &action_window_id,
            "internal host state unavailable",
        );

        disconnect_client_or_record_health(
            &clients,
            &authoritative_state,
            &runtime_health,
            &player_id,
        );

        break;
    }
};
```

For post-action readback/writeback paths, use the same principle:

```rust
let next_state = match authoritative_state.lock() {
    Ok(state) => state.clone(),
    Err(_) => {
        record_state_lock_error(
            &runtime_health,
            format!(
                "authoritative state lock poisoned after applying action for player {player_id}"
            ),
        );
        disconnect_client_or_record_health(
            &clients,
            &authoritative_state,
            &runtime_health,
            &player_id,
        );
        break;
    }
};
```

Do not convert lock poisoning into invented state. Do not silently skip diagnostics.

### Tests

If there is existing test infrastructure for host-session lock poisoning, add coverage. If not practical, at least add a small unit test for `record_state_lock_error` or the helper that wraps it.

Test intent:

```rust
#[test]
fn state_lock_poison_updates_host_runtime_health() {
    let health = Arc::new(Mutex::new(HostRuntimeHealth::default()));

    record_state_lock_error(&health, "test state lock failure");

    let health = health.lock().unwrap();
    assert_eq!(health.state_lock_error_count, 1);
    assert_eq!(health.last_error.as_deref(), Some("test state lock failure"));
}
```

### Acceptance

- Every authoritative-state lock failure in `host_session.rs` either records health or returns a structured error to a caller that records health.
- No `unwrap_or_else(|_| state.clone())` style invented-state fallback remains in runtime code.
- No lock-poisoning path silently disconnects/breaks without health diagnostics.

---

## P0.4 — Make `ClientRuntime::connect` thread spawn fallible

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- runtime tests if practical

### Problem

`ClientRuntime::connect` still uses raw `thread::spawn`. Raw `thread::spawn` panics if the OS cannot spawn the thread. This is rare, but it is still an avoidable desktop-app crash path.

### Required change

Replace raw spawn with `thread::Builder::spawn(...)` and propagate `NetworkingError`.

Find the current pattern:

```rust
thread::spawn(move || {
    // client runtime loop
});
```

Replace with:

```rust
thread::Builder::new()
    .name(format!("desktop-poker-client-runtime-{player_id}"))
    .spawn(move || {
        // existing client runtime loop body
    })
    .map_err(|error| {
        NetworkingError::new(format!("failed to spawn client runtime thread: {error}"))
    })?;
```

Make sure the function only returns `Ok(ClientRuntime { ... })` after the spawn succeeds.

If the code needs to retain the `JoinHandle`, store it as before. If it currently intentionally detaches the thread, it is acceptable to ignore the returned handle after successful spawn, but include a comment:

```rust
// The client runtime thread is intentionally detached; shutdown is coordinated by the stop flag
// and stream lifecycle.
```

### Tests

If possible, introduce a small spawn abstraction for tests. Do not use OS thread exhaustion as a test technique.

Suggested internal helper:

```rust
type ClientRuntimeSpawner = dyn FnOnce(Box<dyn FnOnce() + Send + 'static>) -> Result<(), String>;

fn spawn_client_runtime_thread(
    player_id: &str,
    f: impl FnOnce() + Send + 'static,
) -> Result<(), NetworkingError> {
    thread::Builder::new()
        .name(format!("desktop-poker-client-runtime-{player_id}"))
        .spawn(f)
        .map(|_| ())
        .map_err(|error| {
            NetworkingError::new(format!("failed to spawn client runtime thread: {error}"))
        })
}
```

Then tests can target a lower-level helper or a `connect_with_spawner` if that fits the current design.

### Acceptance

- No raw `thread::spawn` remains in `src-tauri/src/networking/runtime/client.rs`.
- Client runtime spawn failure returns `NetworkingError`.
- No panic path remains for client runtime thread-spawn failure.

---

## P0.5 — Add a real NPC action-path regression test for missing/invalid acting hole cards

**Files:**

- `src-tauri/src/npc/runner/tests.rs`
- `src-tauri/src/npc/runner/action.rs` if test hooks are needed
- related test helpers

### Problem

Production code now validates that an acting NPC has exactly two hole cards. That is good.

However, helper-level tests are not enough. We need at least one regression test that exercises the actual NPC action path or a near-production extraction of it, proving that corrupt authoritative hole-card state does not submit an action.

### Required change

Add an integration-style NPC runner test.

Preferred test intent:

```rust
#[test]
fn acting_npc_missing_hole_cards_submits_no_action() {
    // Arrange a host server/session with:
    // - an active hand;
    // - an acting NPC player;
    // - a current action window for that NPC.
    //
    // Then corrupt authoritative hand state by removing the NPC's hole-card entry.
    //
    // Act:
    // - run one NPC action attempt through the same function used by the runner loop.
    //
    // Assert:
    // - outcome is RuntimeUnavailable or equivalent non-success;
    // - last_npc_action_error.reason == InternalError;
    // - error message mentions missing hole cards;
    // - no host action was submitted;
    // - no hand-log action count increased.
}
```

If the current test scaffolding cannot easily create a full host-session action window, extract the validation into a small pure helper and test both the helper and one runner-adjacent path.

Suggested helper:

```rust
fn validated_acting_hole_cards<'a>(
    hand: &'a crate::domain::CurrentHand,
    player_id: &str,
) -> Result<&'a [crate::domain::Card], String> {
    match hand.hole_cards_by_player_id.get(player_id) {
        Some(cards) if cards.len() == 2 => Ok(cards.as_slice()),
        Some(cards) => Err(format!(
            "NPC {player_id} has invalid hole-card count {}; expected 2",
            cards.len()
        )),
        None => Err(format!("NPC {player_id} is missing hole cards")),
    }
}
```

Then production code should call the helper and convert the error into `record_npc_internal_error(...)`.

Example production use:

```rust
let hole_cards = match validated_acting_hole_cards(fresh_hand, &fresh_window.player_id) {
    Ok(cards) => cards,
    Err(message) => {
        return record_npc_internal_error(
            runner_state,
            fresh_window.player_id.clone(),
            Some(fresh_hand.hand_number),
            NpcActionErrorReason::InternalError,
            format!("[npc-runner] {message}; no action submitted"),
        );
    }
};
```

Test the helper:

```rust
#[test]
fn validated_acting_hole_cards_rejects_missing_cards() {
    let mut hand = test_current_hand();
    hand.hole_cards_by_player_id.remove("npc-1");

    let error = validated_acting_hole_cards(&hand, "npc-1").unwrap_err();

    assert!(error.contains("missing hole cards"));
}
```

But do not stop at only helper tests if a runner path test is practical.

### Acceptance

- At least one test covers missing/invalid acting NPC hole-card state.
- Preferably one test exercises the actual NPC action attempt path.
- The test proves no action is submitted and a structured NPC error is recorded.
- Production behavior still validates both rule-based and LLM-backed NPCs before decisions.

---

## P1.1 — Re-run and document `poker-core` purity audit

**Files:**

- `crates/poker-core/Cargo.toml`
- `crates/poker-core/src/**`
- `memory.md`

### Problem

`poker-core` was extracted, but Rust validation could not be independently verified during review. The repo should explicitly validate that the core remains platform-neutral.

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

Important: the term `EngineCommand` is expected and harmless. Do not remove `EngineCommand`. If the audit hits `EngineCommand`, classify it as a false positive for `Command`.

### Acceptance

- `cargo tree -p poker-core` is small and platform-neutral.
- The `rg` audit has no code hits for forbidden platform/networking dependencies.
- Any textual/comment hits are reviewed and documented as harmless.

---

## P1.2 — Keep `poker-core` facade real and deterministic enough for future Android

**Files:**

- `crates/poker-core/src/facade.rs`
- `crates/poker-core/src/tournament/**`
- facade tests

### Problem

The new `PokerEngine` facade is a good starting point, but it must not become a fake wrapper disconnected from the real tournament controller. It also needs a deterministic-enough test story for Android/replay work.

### Required checks

Verify:

- `PokerEngine` wraps the real `TournamentController`.
- `EngineCommand::SubmitAction` calls the same action submission path used by desktop.
- `EngineCommand::AdvanceTime` calls the real timer advancement path.
- `export_state_json()` serializes the real `TournamentState`.
- Facade tests use deterministic setup.

If `TournamentController::new(config)` uses OS randomness internally, do not write flaky tests that compare full serialized states after random deck generation. Either:

1. use existing deterministic deck injection such as `set_next_deck`;
2. add a deterministic constructor/test helper;
3. limit the facade determinism test to a config/time path that does not depend on shuffle randomness.

Suggested future-friendly addition if easy:

```rust
impl PokerEngine {
    pub fn set_next_deck_for_test(
        &mut self,
        cards: Vec<crate::domain::Card>,
    ) -> Result<(), PokerCoreError> {
        self.controller
            .set_next_deck(cards)
            .map_err(|error| PokerCoreError::Engine(error.to_string()))
    }
}
```

Only expose this publicly if the project already exposes deterministic deck injection. Otherwise keep it `#[cfg(test)]`.

### Acceptance

- Facade uses real controller/state.
- Tests are not flaky.
- No Android-specific API is added yet.
- No networking enters `poker-core`.

---

## P1.3 — Update `memory.md` honestly after validation

**Files:**

- `memory.md`

### Problem

`memory.md` should not claim commands passed unless they actually passed in the current environment. It should distinguish between:

- review findings;
- Claude Code local validation;
- commands the current agent actually ran.

### Required change

After completing Fix 7 and running validation, add a new entry using the repo timestamp convention:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

Suggested entry:

```md
- <timestamp> — Fix 7 cleanup completed: updated README/dev docs to workspace Cargo validation commands; surfaced all HostRuntimeHealth counters in the frontend debug UI; remaining host-session state-lock failures now update health; ClientRuntime thread spawn is fallible; NPC missing/invalid acting hole-card state has action-path regression coverage; poker-core purity audit remains clean. Validation run: <list exact commands actually run and whether they passed>. Architecture unchanged: Android will be Kotlin/Compose + Rust bindings; Kotlin owns networking/session transport; poker-core owns deterministic poker rules/state/projection only.
```

Do not claim Rust commands passed unless they were actually run and passed.

### Acceptance

- `memory.md` records Fix 7 accurately.
- The entry lists exact validation commands actually run.
- No false completion claims.

---

## P2.1 — Final silent-failure audit for touched files

**Files:**

- files touched by Fix 7
- especially `src-tauri/src/networking/runtime/**`
- `src-tauri/src/npc/**`
- `src/api/desktop.ts`
- debug UI files

### Required audit

Run:

```bash
rg -n "let _ =|\.ok\(\)|unwrap_or\(|unwrap_or_else\(|thread::spawn|continue;|return;" \
  src-tauri/src/networking/runtime src-tauri/src/npc src-tauri/src/app_state crates/poker-core/src
```

For every hit in files touched by Fix 7:

- convert real silent failures to structured errors/diagnostics;
- add a short safety comment for intentional best-effort cleanup;
- leave presentation-only defaults alone if harmless;
- do not rewrite unrelated stable code just to reduce `rg` output.

### Acceptance

- No newly touched runtime/NPC/core code contains unexplained silent failure behavior.
- Any ignored error has a short comment explaining why it is safe.
- No new fallback invents authoritative poker state.

---

## P2.2 — Do not start Android implementation in Fix 7

**Files:**

- docs only, unless a small TODO marker is useful

### Required rule

Do not add:

- an Android Gradle project;
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

Fix 7 is cleanup and stabilization only.

### Acceptance

- No half-built Android implementation appears.
- Existing Android architecture doc remains accurate.

---

## Final validation commands

Run from repo root with Node 24 active:

```bash
npm ci
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

rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated" src-tauri/src/networking
rg -n "hole_cards_by_player_id.*unwrap_or\(|unwrap_or\(&\[\]\)" src-tauri/src/npc src-tauri/src/tournament crates/poker-core/src || true
rg -n "thread::spawn" src-tauri/src/networking/runtime/client.rs
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn|Command" crates/poker-core
```

Expected notes:

- The `poker-core` audit may hit `EngineCommand`; that is expected and harmless.
- The production `dist` grep should have no hits in JS payloads. Source-map hits are only acceptable if source maps are not shipped and the packaging rule is documented.

## Definition of done

- [ ] README/dev docs use workspace-wide Cargo validation commands.
- [ ] Frontend `HostRuntimeHealth` type includes all Rust health counters.
- [ ] Debug UI displays all non-zero host runtime health counters.
- [ ] Remaining host-session authoritative-state lock failures update health or return structured errors.
- [ ] `ClientRuntime::connect` thread spawn failure returns `NetworkingError`, not panic.
- [ ] NPC missing/invalid acting hole-card state has action-path regression coverage or documented near-production equivalent.
- [ ] `poker-core` purity audit is clean.
- [ ] `PokerEngine` facade remains real, small, and platform-neutral.
- [ ] `memory.md` accurately records commands actually run.
- [ ] No new hidden fallbacks or silent failures are introduced.
- [ ] No Android app, Tauri Mobile path, or Android FFI crate is added in Fix 7.
- [ ] All final validation commands pass in an environment with Node 24 and Rust installed.
