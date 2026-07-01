# Desktop Poker Fix 9 Spec — Lock Validation Docs and Harden Public Event Sequencing

## Purpose

Fix 8 brought the project close to a stable shared-core baseline. Fix 9 is a narrow stabilization pass. It should not expand the Android architecture, add a new app, or perform another broad refactor.

The goal is to remove the remaining sources of future confusion and one remaining protocol fallback smell:

1. Developer-facing validation docs must consistently use the current workspace command set.
2. Frontend formatting validation must be documented as `npm run format:check`, not the mutating `npm run format` command.
3. Archived specs/TODOs may still contain pre-workspace Cargo commands, but active docs must clearly say those are historical.
4. Client protocol handling must not silently default a missing signed public-event `server_sequence` to `0`.

## Background

The repository now has a Cargo workspace with:

```text
Cargo.toml
crates/poker-core/
src-tauri/
```

`crates/poker-core` is the shared, platform-neutral poker rules/state/projection crate. `src-tauri` is the desktop adapter crate. Android is still future work and should eventually be native Kotlin/Compose + Rust bindings. Android networking/session transport should remain Kotlin/platform adapter code, not part of `poker-core`.

Fix 8 successfully aligned CI with the workspace and added stronger runtime regression coverage, but a review found the following remaining issues:

- README and some active developer docs still list `npm run format` in validation sections instead of `npm run format:check`.
- Some active docs omit `cargo test -p poker-core` and `cargo tree -p poker-core` from the recommended validation set.
- Historical docs/specs still contain old `--manifest-path src-tauri/Cargo.toml` validation commands. Those can remain if clearly treated as archived historical material, but active instructions must not send agents down the old path.
- `ClientRuntime` still appears to default a missing public-event `server_sequence` to `0` with `unwrap_or_default()`. For signed host-originated public events, a missing sequence should be treated as malformed protocol input and surfaced as a protocol warning.

## Goals

### G1 — Active docs use the correct validation command set

Active developer docs should consistently list the same validation intent:

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

It is acceptable for launch/package commands to use `--manifest-path src-tauri/Cargo.toml` when the text explicitly says the command launches/packages the desktop Tauri adapter. It is not acceptable for validation instructions to validate only `src-tauri`.

### G2 — Formatting validation is non-mutating

Validation docs should use `npm run format:check`. The mutating `npm run format` command may remain documented as a local repair command, but it should not be presented as the validation command.

### G3 — Archived docs are clearly historical

Old generated TODO/spec/review files may contain outdated validation commands. Do not rewrite every archived artifact. Instead, add a clear note in active developer docs explaining that current validation is the workspace command set and older archived files may contain pre-workspace commands.

### G4 — Public event sequence handling is fail-loud

A signed public event envelope without `server_sequence` should be dropped with a `ClientRuntimeEvent::ProtocolWarning`. It must not silently become sequence `0`.

This keeps the client protocol diagnostics consistent with the private-message AAD hardening already completed in earlier fixes.

## Non-goals

Fix 9 must not:

- add an Android Gradle project;
- add Kotlin source files;
- add Tauri Mobile;
- add a `poker-android-ffi` crate;
- move networking into `poker-core`;
- perform a broad Rust module refactor;
- rewrite archived historical specs just to make broad grep output smaller;
- suppress warnings/errors instead of handling the underlying issue.

## Detailed Requirements

### R1 — README validation sections

Update `README.md` so the main validation path includes:

```bash
npm run format:check
npm run lint
npm run test
npm run build
```

and Rust validation includes:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test -p poker-core
cargo tree -p poker-core
```

`npm run format` may be documented separately as a fix command:

```bash
npm run format
```

but it must not be listed as the validation substitute for `format:check`.

### R2 — CLAUDE and skill instructions

Update `CLAUDE.md` and `.claude/skills/lint-n-test/SKILL.md`, if present, so future coding agents run the same workspace validation set. These files should not omit `cargo test -p poker-core`, `cargo tree -p poker-core`, or `npm run format:check`.

### R3 — GitHub/Copilot instructions

Verify `.github/copilot-instructions.md` does not claim the project is a single Rust crate and does not tell agents to validate Rust only with `--manifest-path src-tauri/Cargo.toml`.

If it is already correct after Fix 8, leave it alone except for adding the archived-docs note if that file is the most appropriate active developer instruction location.

### R4 — Archived docs note

Add a concise note to an active developer doc, preferably `README.md` or `CLAUDE.md`:

```md
> Note: older archived review/spec/TODO files may contain pre-workspace Cargo commands such as `--manifest-path src-tauri/Cargo.toml`. Those files are historical. Current validation should use the root workspace commands above.
```

Do not mass-edit archived generated files unless they are part of current active instructions.

### R5 — Public event missing sequence warning

In `src-tauri/src/networking/runtime/client.rs`, find the public-event handling path that currently does something equivalent to:

```rust
server_sequence: envelope.server_sequence.unwrap_or_default(),
```

Replace it with explicit validation:

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

Then pass `server_sequence` into the public event instead of defaulting.

If there are multiple signed host-originated public-event paths, apply the same rule consistently. Be careful not to apply this to local-only events or message types that legitimately do not carry a server sequence.

### R6 — Protocol warning test

Add a focused test that proves a public event envelope missing `server_sequence` emits a `ProtocolWarning` and does not emit/apply the public event.

If constructing the full signed envelope is awkward, extract a small parsing/validation helper and test that helper plus the nearest practical runtime path. Do not settle for a grep-only test if a small runtime/helper test is practical.

### R7 — Memory update

After implementation and validation, update `memory.md` honestly:

- list exact commands run;
- distinguish local Claude Code validation from any external review limitation;
- do not claim Rust validation passed unless it actually ran and passed;
- record that Fix 9 did not change the Android architecture or add Android implementation.

## Acceptance Criteria

Fix 9 is complete only when:

- active docs use `npm run format:check` for formatting validation;
- active docs include workspace Rust commands, including `cargo test -p poker-core` and `cargo tree -p poker-core`;
- old `--manifest-path src-tauri/Cargo.toml` validation guidance is gone from active docs or clearly limited to desktop launch/package commands;
- active docs include a note that older archived generated files may contain historical pre-workspace commands;
- public event envelopes missing `server_sequence` produce `ClientRuntimeEvent::ProtocolWarning` and are dropped;
- no public event missing `server_sequence` silently becomes sequence `0`;
- relevant tests are added/updated;
- no new hidden fallback or silent failure is introduced;
- no Android app, Tauri Mobile path, Android FFI crate, or networking-in-core is added;
- final validation commands pass in an environment with Node 24 and Rust installed.
