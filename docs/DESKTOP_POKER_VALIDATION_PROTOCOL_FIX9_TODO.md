# Desktop Poker Fix 9 TODO — Validation Docs and Public Event Sequence Hardening

This TODO is intentionally explicit for Claude Code. Do tasks in priority order. Prefer small, test-backed patches. Do not introduce hidden fallback behavior to make tests pass.

Fix 9 is a stabilization pass. Do not add features. Do not start Android implementation.

---

## P0.1 — Make active developer docs use the full validation command set

**Files:**

- `README.md`
- `CLAUDE.md`
- `.claude/skills/lint-n-test/SKILL.md` if present
- `.github/copilot-instructions.md`
- any active docs that describe validation commands

### Problem

After Fix 8, GitHub CI is mostly aligned with the new workspace, but some active developer docs still drift from the final validation standard. In particular, docs may still:

- list `npm run format` as a validation command instead of `npm run format:check`;
- omit `cargo test -p poker-core`;
- omit `cargo tree -p poker-core`;
- imply `--manifest-path src-tauri/Cargo.toml` is the normal Rust validation path.

That can cause future agents to run incomplete validation and miss `poker-core` failures.

### Required change

Search active docs:

```bash
rg -n "npm run format|format:check|cargo fmt|cargo clippy|cargo test|cargo tree|--manifest-path src-tauri/Cargo.toml|cd src-tauri|single crate|not a workspace" \
  README.md CLAUDE.md .github .claude docs package.json
```

Update active validation sections to use this command set:

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

Use `npm run format` only as a repair command, not as the validation command.

### Suggested README wording

```md
### Frontend validation

Run frontend validation from the repository root:

```bash
npm ci
npm run format:check
npm run lint
npm run build
npm test
```

Use `npm run format` to apply Prettier fixes when `npm run format:check` fails.

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

### Important

It is acceptable for desktop launch/package instructions to use `--manifest-path src-tauri/Cargo.toml` when the text explicitly says the command launches or packages the desktop Tauri app.

It is not acceptable for primary validation docs to validate only `src-tauri`.

### Acceptance

- README active validation sections use `npm run format:check`, not `npm run format`, for formatting validation.
- `CLAUDE.md` and `.claude/skills/lint-n-test/SKILL.md` include `npm run format:check`.
- Active Rust validation docs include `cargo test -p poker-core` and `cargo tree -p poker-core`.
- Active docs no longer describe the project as a single Rust crate.
- `--manifest-path src-tauri/Cargo.toml` appears only in explicit desktop launch/package contexts or archived historical docs.

---

## P0.2 — Add an archived-docs warning so old generated specs do not mislead future agents

**Files:**

- `README.md` preferred
- `CLAUDE.md` optional
- `.github/copilot-instructions.md` optional

### Problem

Historical generated review/spec/TODO files may still contain old pre-workspace commands such as:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Mass-editing every archived file is unnecessary and can damage historical context. But active docs should clearly tell future agents that the current source of truth is the workspace command set.

### Required change

Add a short note to active developer docs.

Suggested note:

```md
> Note: older archived review/spec/TODO files may contain pre-workspace Cargo commands such as `--manifest-path src-tauri/Cargo.toml`. Those files are historical. Current validation should use the root workspace commands above.
```

### Acceptance

- Active docs distinguish current validation from archived historical instructions.
- Historical docs do not need mass edits.
- Future agents have a clear source of truth.

---

## P0.3 — Harden public-event handling when `server_sequence` is missing

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- client protocol/runtime tests, likely under `src-tauri/src/networking/runtime/tests/**`

### Problem

`ClientRuntime` still appears to default a missing public-event `server_sequence` to `0`, with code equivalent to:

```rust
server_sequence: envelope.server_sequence.unwrap_or_default(),
```

For host-originated signed public events, missing sequence is malformed protocol input. Defaulting to `0` hides the protocol defect and can confuse ordering/replay diagnostics.

Claude Code should treat this like the earlier private-message AAD fallback: fail visibly, emit a protocol warning, and drop the malformed frame.

### Required change

Replace the defaulting path with explicit validation.

Suggested pattern:

```rust
let Some(server_sequence) = envelope.server_sequence else {
    emit_protocol_warning(
        &sender,
        &mut protocol_warning_counts,
        &player_id,
        "public event envelope missing server sequence",
    );
    continue;
};
```

Then pass the validated sequence into the runtime event:

```rust
let _ = sender.send(ClientRuntimeEvent::PublicEvent {
    player_id: player_id.clone(),
    server_sequence,
    // existing fields...
});
```

If the code sends a structured `PublicEvent` type rather than inline fields, use the same idea:

```rust
let public_event = PublicEvent {
    server_sequence,
    // existing fields...
};
```

### Important

Only require `server_sequence` for host-originated public event envelopes where the protocol expects one.

Do not incorrectly require `server_sequence` for local-only messages or for message types that legitimately do not carry one.

### Tests

Add a focused test that proves missing `server_sequence` emits a warning and does not emit/apply the public event.

Preferred test intent:

```rust
#[test]
fn public_event_missing_server_sequence_emits_protocol_warning_and_is_dropped() {
    // Arrange a client runtime read-loop test with a signed public event envelope
    // that is otherwise valid but has server_sequence = None / missing.

    // Act: feed the frame into the same path used by ClientRuntime.

    // Assert:
    // - ClientRuntimeEvent::ProtocolWarning is emitted;
    // - warning reason mentions "missing server sequence";
    // - no ClientRuntimeEvent::PublicEvent is emitted for that frame;
    // - no snapshot/public state is mutated by that malformed public event.
}
```

If constructing a full signed public envelope is too awkward, extract a small helper and test the helper plus the nearest practical runtime path.

Suggested helper direction:

```rust
fn public_event_server_sequence_or_warn(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    server_sequence: Option<u64>,
) -> Option<u64> {
    match server_sequence {
        Some(sequence) => Some(sequence),
        None => {
            emit_protocol_warning(
                sender,
                counts,
                player_id,
                "public event envelope missing server sequence",
            );
            None
        }
    }
}
```

Then production code can do:

```rust
let Some(server_sequence) = public_event_server_sequence_or_warn(
    &sender,
    &mut protocol_warning_counts,
    &player_id,
    envelope.server_sequence,
) else {
    continue;
};
```

### Acceptance

- No public event path uses `server_sequence.unwrap_or_default()`.
- Missing public-event `server_sequence` emits `ClientRuntimeEvent::ProtocolWarning`.
- The malformed public event is dropped.
- Tests cover the behavior or document the strongest practical near-runtime equivalent.

---

## P0.4 — Re-run previous hardening audits

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- `src-tauri/src/npc/runner/action.rs`
- `src-tauri/src/npc/provider_storage.rs`
- `src/main.tsx`
- `src/test/setup.ts`
- `crates/poker-core/src/**`

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
rg -n "server_sequence.*unwrap_or_default|unwrap_or_default\(\).*server_sequence" src-tauri/src/networking/runtime/client.rs
```

### Acceptance

- No window-persistence test noise.
- Production dist does not contain browser mock/probe setup code.
- No private-message empty-AAD fallback.
- No acting NPC empty-hole-card fallback.
- No raw client runtime `thread::spawn`.
- No public-event missing-sequence default to `0`.

---

## P1.1 — Keep `poker-core` platform-neutral

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

`EngineCommand` is an expected false positive for the broad `Command` grep. Do not remove or rename `EngineCommand` just to satisfy the grep.

### Acceptance

- `cargo tree -p poker-core` remains small and platform-neutral.
- Forbidden dependency grep has no code hits except expected harmless terms such as `EngineCommand`.
- Any textual/comment hits are reviewed and documented if necessary.

---

## P1.2 — Update `memory.md` honestly after Fix 9 validation

**Files:**

- `memory.md`

### Problem

`memory.md` must not claim commands passed unless they actually passed in the current implementation environment.

### Required change

After completing Fix 9 and running validation, add a new entry using the project timestamp convention:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

Suggested entry:

```md
- <timestamp> — Fix 9 completed: active developer docs now use `npm run format:check` and workspace-wide Rust validation including `cargo test -p poker-core` and `cargo tree -p poker-core`; active docs note that older archived generated files may contain historical pre-workspace Cargo commands; client public-event handling now treats missing `server_sequence` as malformed input, emits `ProtocolWarning`, and drops the event instead of defaulting to sequence 0. Validation run in this environment: <exact commands actually run and pass/fail status>. Architecture unchanged: Android will be native Kotlin/Compose + Rust bindings; Kotlin owns networking/session transport; poker-core owns deterministic poker rules/state/projection only. No Android app, Tauri Mobile path, FFI crate, or networking-in-core was added.
```

### Acceptance

- `memory.md` lists exact commands actually run.
- No false completion claims.
- Android/core architecture decision remains recorded.

---

## P2.1 — Silent-failure audit for touched files

**Files:**

- files touched by Fix 9
- especially `src-tauri/src/networking/runtime/client.rs`
- docs/CI files touched by Fix 9

### Required audit

For code files touched by Fix 9, run an appropriate subset of:

```bash
rg -n "let _ =|\.ok\(\)|unwrap_or\(|unwrap_or_else\(|unwrap_or_default\(|thread::spawn|continue;|return;" \
  src-tauri/src/networking/runtime src-tauri/src/npc src-tauri/src/app_state crates/poker-core/src
```

For every hit in touched production code:

- convert real silent failures to structured errors/diagnostics;
- add a short comment for intentional best-effort cleanup;
- leave harmless presentation-only defaults alone;
- do not rewrite unrelated stable code just to reduce grep output.

### Acceptance

- No newly touched runtime/NPC/core code contains unexplained silent failure behavior.
- Any ignored error has a short comment explaining why it is safe.
- No new fallback invents authoritative poker state or protocol ordering state.

---

## P2.2 — Do not start Android implementation in Fix 9

**Files:**

- docs only, if needed

### Required rule

Do not add:

- Android Gradle project files;
- Kotlin source files;
- Tauri Mobile configuration;
- `poker-android-ffi` crate;
- networking inside `poker-core`.

Fix 9 is validation/protocol hardening only.

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

rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated" src-tauri/src/networking
rg -n "hole_cards_by_player_id.*unwrap_or\(|unwrap_or\(&\[\]\)" src-tauri/src/npc src-tauri/src/tournament crates/poker-core/src || true
rg -n "thread::spawn" src-tauri/src/networking/runtime/client.rs
rg -n "server_sequence.*unwrap_or_default|unwrap_or_default\(\).*server_sequence" src-tauri/src/networking/runtime/client.rs
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn|Command" crates/poker-core
```

Expected notes:

- `EngineCommand` in `poker-core` is an expected false positive for the broad `Command` grep.
- Desktop launch/package commands may still use `--manifest-path src-tauri/Cargo.toml` if explicitly documented as launch/package commands rather than validation.
- Production dist grep should have no JS payload hits for `LayoutProbeApp` or `__DESKTOP_POKER_BROWSER_MOCKS__`.

## Definition of done

- [ ] Active docs use `npm run format:check` for formatting validation.
- [ ] Active docs include workspace Rust validation with `cargo test -p poker-core` and `cargo tree -p poker-core`.
- [ ] Active docs explain that older archived generated files may contain historical pre-workspace commands.
- [ ] Public-event missing `server_sequence` emits `ProtocolWarning` and is dropped.
- [ ] No public-event missing-sequence path defaults to sequence `0`.
- [ ] Tests cover the public-event missing-sequence behavior or a documented near-runtime equivalent.
- [ ] Previous hardening remains fixed: no test noise, no production mock chunk, no empty-AAD fallback, no empty NPC hole-card fallback, no raw client runtime `thread::spawn`.
- [ ] `poker-core` purity audit remains clean.
- [ ] `memory.md` accurately records commands actually run.
- [ ] No new hidden fallbacks or silent failures are introduced.
- [ ] No Android app, Tauri Mobile path, Android FFI crate, or networking-in-core is added.
- [ ] All final validation commands pass in an environment with Node 24 and Rust installed.
