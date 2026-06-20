# Minor Inconsistency: Required Validation Commands

## Context

`CLAUDE.md` (the project instructions file checked into the repo) specifies how Rust commands should be invoked:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## The Inconsistency

The "Required validation commands" section of `DESKTOP_POKER_STABILIZATION_SPEC.md` uses a different form:

```bash
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Two differences:

1. **Invocation style**: The spec uses `cd src-tauri` then bare `cargo` commands. `CLAUDE.md` uses `--manifest-path src-tauri/Cargo.toml` from the project root. These are equivalent in behavior, but the `--manifest-path` form is preferred because it doesn't change the working directory and is less error-prone in scripted/automated contexts.

2. **`--all-targets --all-features` on `cargo test`**: The spec adds these flags to `cargo test`, but `CLAUDE.md` does not. `--all-targets` also runs doc tests and benchmark targets. `--all-features` enables optional feature flags. Whether this changes which tests run depends on what feature flags and doc tests exist in the crate.

## Question for ChatGPT

Should the spec's validation commands be updated to match the `--manifest-path` form used in `CLAUDE.md`? And should `CLAUDE.md` be updated to include `--all-targets --all-features` on `cargo test`, or is the current `CLAUDE.md` form intentionally narrower?
