# Desktop Poker Fix 8 Spec — Lock the Shared-Core Baseline

## Purpose

Fix 8 is a focused stabilization pass after the `poker-core` extraction and Fix 7 cleanup. Do not add new product features. Do not begin Android implementation. Do not move networking into `poker-core`.

The goal is to lock in the new shared-core baseline so future desktop and Android work starts from a trustworthy repository state.

## Background

The project now has a root Cargo workspace and a shared `crates/poker-core` crate. Pure poker rules/state/projection code was moved out of the Tauri desktop crate. The desktop app is now a platform adapter around the shared Rust core.

The latest review found that the runtime code is materially improved, but several cleanup issues remain:

1. Some developer/CI instructions still validate only `src-tauri` instead of the full Cargo workspace.
2. `.github/copilot-instructions.md` still describes the old single-crate layout.
3. GitHub Actions Rust validation still uses `--manifest-path src-tauri/Cargo.toml`, which can miss `poker-core` issues.
4. Frontend `HostRuntimeHealth` models the `streamTimeoutErrorCount` field, but the debug UI does not render it.
5. Frontend formatting is not clean under `npm run format:check`.
6. The NPC missing/invalid hole-card regression is currently helper-level rather than an action-path regression.
7. `memory.md` must distinguish commands actually run by Claude Code from commands verified by external review.

## Architecture stance

Keep the current architecture direction:

- `poker-core` owns deterministic poker rules, tournament state transitions, legal action validation, hand/projection logic, and portable state/facade types.
- The Tauri desktop crate owns desktop platform adaptation: Tauri commands, desktop session management, desktop networking/runtime, keychain/storage integration, and UI bridge code.
- A future Android app will be native Kotlin/Compose + Rust bindings. Kotlin will own Android networking/session transport. Rust `poker-core` will not own networking.

## Non-goals

Fix 8 must not add:

- Android Gradle project files.
- Kotlin source files.
- Tauri Mobile configuration.
- A `poker-android-ffi` crate.
- Networking, sockets, Tauri, keyring, OS app-data path, LLM/provider, or UI dependencies to `poker-core`.
- New game rules or UI features.

## Required outcomes

Fix 8 is complete only when:

1. README, Claude/Copilot instructions, and CI all use workspace-wide Rust validation commands.
2. GitHub Actions validates `poker-core`, not just `src-tauri`.
3. Debug UI displays every non-zero `HostRuntimeHealth` counter, including stream timeout errors.
4. Frontend formatting validation is clean.
5. NPC missing/invalid acting hole-card state has a real action-path regression test, or a clearly documented near-production equivalent if full integration is impractical.
6. `poker-core` remains platform-neutral by audit.
7. `memory.md` records exact validation commands actually run and does not overclaim.
8. All final validation commands pass in an environment with Node 24 and Rust installed.

## Validation commands

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

Expected audit notes:

- `EngineCommand` inside `poker-core` is expected and harmless for the broad `Command` search.
- Desktop launch/package commands may still use `--manifest-path src-tauri/Cargo.toml` if they are explicitly about launching/packaging the Tauri app, not validation.
- Production `dist` grep must have no JS payload hits for `LayoutProbeApp` or `__DESKTOP_POKER_BROWSER_MOCKS__`. Source-map hits are only acceptable if release packaging does not ship source maps and that packaging rule is documented.

## Implementation principles

- Prefer small, test-backed patches.
- Do not suppress errors to make tests pass.
- Do not hide expected stderr noise.
- Do not add fake UI-only rollback or diagnostics.
- Do not invent authoritative poker state.
- When an ignored runtime error is intentionally safe, add a short comment explaining why.
- Keep `poker-core` platform-neutral.

