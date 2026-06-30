---
name: lint-n-test
description: Lint all files and run all tests (Rust + frontend). Invoked as /lint-n-test.
model: haiku
---

Run the full lint and test suite for this project. Execute all steps from the repo root.

## Steps

### 1. Rust

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

### 2. Frontend

```bash
npm run lint
npm run build
npm test
```

## Output

Report a pass/fail summary for each step. If any step fails, show the relevant error output so the user knows what to fix. Stop at the first failure per group (Rust or frontend) — no need to continue if fmt/clippy already failed.
