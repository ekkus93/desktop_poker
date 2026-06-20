# replies3.md

## Response to Claude Code about Rust validation command inconsistency

Good catch. Please update the stabilization handoff files to make the Rust validation commands consistent with the project instructions in `CLAUDE.md`.

This is a documentation/spec consistency issue, not a major design conflict.

## Decision

Use the project-root `--manifest-path src-tauri/Cargo.toml` form for Rust commands.

Do **not** require `cd src-tauri` in the spec or TODO files.

This form is preferred because it is easier to run from scripts and automation without depending on the caller changing directories first.

## Files you must update

Please update both of these files:

- `DESKTOP_POKER_STABILIZATION_SPEC.md`
- `DESKTOP_POKER_STABILIZATION_TODO.md`

Replace the Rust validation command blocks in both files with the baseline command set below.

## Baseline required Rust validation commands

Use these as the normal required Rust validation commands:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Notes:

- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` is the correct validation command because it verifies formatting without modifying files.
- `cargo fmt --manifest-path src-tauri/Cargo.toml` without `--check` is fine as a fix command, but it should not be listed as the required validation command.
- Keep `cargo clippy --all-targets --all-features` because the project already expects strict linting.
- Keep baseline `cargo test --manifest-path src-tauri/Cargo.toml` to match `CLAUDE.md`.

## Extended Rust validation

Do **not** make this mandatory unless Phillip explicitly decides to make it the normal project baseline:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

You may add this as an optional section named `Extended Rust validation`.

Reason: `--all-targets --all-features` can change which code is compiled and tested, especially if optional features are not intended to be enabled together or if examples, benches, doctests, or extra targets have different dependencies. It is useful for deeper hardening, but it should not become a required baseline accidentally.

## Do not update `CLAUDE.md` yet

Do not update `CLAUDE.md` to require:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

unless Phillip explicitly wants that stricter command to become the normal project baseline.

For now, only update:

- `DESKTOP_POKER_STABILIZATION_SPEC.md`
- `DESKTOP_POKER_STABILIZATION_TODO.md`

## Expected final state

After your update, the required validation command sections in the spec and TODO should consistently use:

```bash
npm run lint
npm run build
npm run test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

If you add the optional extended Rust validation section, keep it clearly separate from required validation.
