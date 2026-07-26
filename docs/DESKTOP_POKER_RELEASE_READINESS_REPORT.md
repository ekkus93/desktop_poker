# Desktop Poker Release Readiness Report

## Executive result

**Outcome: Additional desktop runtime validation is required before a Linux release tag or Android/UniFFI milestone.**

The current automated baseline and Linux release-candidate build are green. A direct release binary and Debian package were built and inspected. The remaining release blockers require a graphical desktop, multiple live application instances, two physical LAN machines, a Linux Secret Service/keychain session, and configured live LLM providers. Those scenarios are recorded as `BLOCKED`; they are not inferred from unit, integration, browser, or packaging tests.

This report is the authoritative evidence record for `docs/DESKTOP_POKER_RELEASE_READINESS_BASELINE_TODO.md`.

## Result vocabulary

- `PASS` — the named command or scenario was executed successfully against the recorded revision.
- `FAIL` — the named command or scenario was executed and produced a product or validation failure.
- `BLOCKED` — execution requires a concrete unavailable dependency or environment.
- `NOT RUN` — execution was not attempted for a stated reason.

## Tested revision and environment

### Repository revision

- Repository: `ekkus93/desktop_poker`
- Base branch: `master`
- Baseline base SHA: `f4fde4d70fb8fe205bf74be7469912efd682045c`
- Execution branch: `agent/release-readiness-baseline`
- Tested product-source SHA: `fd8369ba7267fe76a827cdf48384c9f826159719`
- GitHub pull-request merge SHA used by the evidence-generating CI run: `c79c9f2473f92310c3d65afe8b834f97b2875c5d`
- Evidence-generating GitHub Actions run: `30224757296`
- Final branch validation commit: `283607db670d0bd28a342ac5a417806bc3507d78`
- Final read-only GitHub Actions run: `30225613175`

The evidence-generating workflow checked out the pull-request merge revision. Later commits changed only release-readiness documentation and CI comments; the final read-only run repeated verification, geometry, and Linux artifact construction successfully on the final branch.

### GitHub Actions environment

- Runner: Ubuntu 24.04.4 LTS, x86-64, GitHub-hosted Azure VM
- Kernel: Linux 6.17.0-1020-azure
- Node.js: `v24.18.0`
- npm: `11.16.0`
- Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14)`
- Cargo: `cargo 1.97.1 (c980f4866 2026-06-30)`
- Active toolchain: `stable-x86_64-unknown-linux-gnu`
- Graphical desktop/WebView session: unavailable
- Two physical LAN machines: unavailable
- Local Ollama/llama-server endpoint: unavailable
- Linux Secret Service/keychain session: unavailable

## Automated validation

| Validation | Result | Current evidence |
|---|---|---|
| `npm ci` | PASS | Committed lockfile installed in Verify, geometry, and release-candidate jobs. |
| `npm audit --omit=dev` | PASS | 0 info, low, moderate, high, critical; total 0. |
| `npm audit` | PASS | 0 info, low, moderate, high, critical; total 0. |
| `npm run format:check` | PASS | Prettier check completed successfully. |
| `npm run lint` | PASS | ESLint completed with zero warnings and zero errors. |
| `npm run test` | PASS | 30 files, 273 tests passed, 0 failed, 0 skipped/todo. |
| `npm run build` | PASS | TypeScript and Vite production build passed; 1,835 modules transformed. |
| `npm run test:geometry` | PASS | Playwright geometry job passed in the pinned Playwright 1.59.1 container. |
| `cargo fmt --check` | PASS | Workspace formatting passed. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS | Clippy passed with warnings denied. |
| `cargo test --workspace --all-targets --all-features -- --test-threads=2` | PASS | Desktop adapter: 463 passed, 3 ignored; `poker-core`: 125 passed; total 588 passed, 0 failed, 3 ignored. |
| `cargo test -p poker-core --all-targets --all-features` | PASS | 125 passed, 0 failed, 0 ignored. |
| `cargo tree -p poker-core` | PASS | Only `rand_core`, `serde`, `serde_json`, `thiserror`, and their platform-neutral transitive dependencies. |

### Ignored Rust tests

The current workspace run contains exactly three ignored tests:

1. `npc::llm_strategy::ollama_live_tests::ollama_llama32_postflop_returns_legal_poker_action` — requires a live local Ollama model.
2. `npc::llm_strategy::ollama_live_tests::ollama_llama32_preflop_returns_legal_poker_action` — requires a live local Ollama model.
3. `protocol::models::tests::canonical_bytes_for_join_request_match_known_android_fixture` — requires the captured Android CanonicalJson fixture referenced by `INT_TEST3_TODO.md` §11.1.

None is claimed as passed.

### Frontend output quality

- Test output contained no unhandled promise rejection, React warning, window-persistence initialization failure, or recurring stderr error.
- Production output: `index.html` 0.46 kB, CSS 28.22 kB, main JS 337.47 kB, successful build in 2.29 seconds on the recorded runner.

## `poker-core` platform-neutrality audit

**Result: PASS for the current source revision.**

- The dependency tree contains no Tauri, Android, keyring, HTTP client, LAN transport, filesystem path-discovery, async runtime, or LLM dependencies.
- Repository source search found no `tauri`, `keyring`, `reqwest`, `local_ip`, `std::net`, TCP/UDP socket, process-spawning `Command`, or thread-spawn references under the shared core.
- `crates/poker-core/Cargo.toml` remains limited to deterministic serialization/error/randomness primitives.
- `EngineCommand` domain terminology is not process-spawning `std::process::Command` usage.

## Linux release artifact inventory

### Direct release binary

- Result: **PASS — build and static inspection**
- Path: `target/release/desktop-poker`
- Type: ELF 64-bit LSB PIE executable, x86-64, dynamically linked
- Exact size: 21,908,136 bytes (reported as 21 MB)
- SHA-256: `9f49324fcf431fcef5202d35dd7c000184992305568f2652b6a6c23896c23211`
- Build ID: `e1fd586bd6391d84b9c089a1506bdbc13aa7d11e`
- Graphical launch: **BLOCKED** — no desktop/WebView session is available in the execution environment.

### Debian package

- Result: **PASS — package build and metadata inspection**
- Filename: `Desktop Poker_0.1.0_amd64.deb`
- Path: `target/release/bundle/deb/Desktop Poker_0.1.0_amd64.deb`
- Exact size: 6,738,672 bytes (reported as 6.5 MB)
- SHA-256: `fc534010dfd8c0468511d9e2a24ad6a34e0375de7a55c030d113d43d06286ac7`
- Package: `desktop-poker`
- Version: `0.1.0`
- Architecture: `amd64`
- Installed size: 21,458 KiB
- Maintainer: `ekkus93`
- Dependencies: `libwebkit2gtk-4.1-0`, `libgtk-3-0`
- Installed binary path: `/usr/bin/desktop-poker`
- Desktop entry and 32/128/256@2 icon assets are present.
- Installation and graphical launch: **BLOCKED** — no disposable graphical Linux session is available.

### AppImage

- Result: **BLOCKED**
- This pass intentionally built the deterministic direct binary and `.deb` bundle. AppImage tooling and graphical launch were not available/proven, so AppImage is not claimed working.

The GitHub Actions release-candidate artifact retains the binary, `.deb`, complete release build log, and artifact inventory for 30 days.

## Production reachability and security audit

### Static findings

- `src/main.tsx` imports the layout probe only inside `import.meta.env.DEV`.
- Browser mocks in `src/api/desktop.ts` are guarded by development/test environment checks.
- `src-tauri/tauri.conf.json` uses a restrictive CSP and limits frontend connections to Tauri IPC.
- Release provider secrets select OS keychain storage; debug builds use the explicitly development-only local file.
- `llm-provider.json` contains non-secret settings only; storage/redaction tests pass.

### Runtime findings

| Scenario | Result | Reason |
|---|---|---|
| Release `/debug` route reachability | BLOCKED | Requires a running release WebView. |
| Browser-mock substitution in release | BLOCKED | Static guards pass; live release exercise is unavailable. |
| Release keychain write/read/clear | BLOCKED | Requires Linux Secret Service and a low-privilege test credential. |
| Plaintext-key search in app data/logs | BLOCKED | Requires application data produced by a real release run. |

## Defects discovered and fixes applied

### DP-RR-FIX-001 — Vulnerable production React Router dependency

- `npm audit --omit=dev` exposed a high-severity advisory through `react-router` 7.18.0 that the previous CI did not run.
- Fixed by migrating from `react-router-dom` 7 to `react-router` 8.3.0 and React/React DOM 19.2.8.
- All imports, tests, production build, and browser geometry pass after migration.

### DP-RR-FIX-002 — Vulnerable development dependency graph

- The full audit exposed the ESLint 9 `minimatch`/`brace-expansion` chain and a vulnerable PostCSS version.
- Fixed by upgrading the ESLint toolchain, pinning `@eslint/js` 10.0.1, and overriding PostCSS to 8.5.23.
- Both production and full audits now report zero vulnerabilities.

### DP-RR-FIX-003 — React correctness findings exposed by ESLint 10

- The stricter lint rules found four state-synchronization patterns and one render-time ref read.
- Fixed provider changes in the user event, asynchronous NPC profile loading, bootstrap-derived join state, render-safe launch-attempt presentation, and derived raise sizing.
- No lint rule was disabled or weakened. Formatting, lint, 273 frontend tests, build, audits, and geometry all pass.

### DP-RR-DOC-001 — Stale workspace release paths

- README release paths incorrectly named `src-tauri/target/release` after the repository became a root Cargo workspace.
- Corrected to `target/release` and documented the current audit/release-candidate workflow.

## Manual runtime evidence

### Two-local-instance full tournament

**Result: BLOCKED.** No graphical desktop session is available to launch two Tauri release instances. Host/join, seat claims, readiness, gameplay, private-card isolation, elimination, completion, persistence, and per-instance isolation are not marked passed.

### Two-machine LAN tournament

**Result: BLOCKED.** Two physical machines on the same LAN are unavailable. Loopback integration tests do not substitute for real LAN evidence.

### Reconnect and failure matrix

Lobby reconnect, active-hand reconnect, reconnect expiry, eliminated-observer reconnect, host loss, port conflict, invalid invite UI, and unreachable-host UI are **BLOCKED** pending controllable live processes and network interruption.

### Rule-based NPC tournament

**Result: BLOCKED.** Automated decision/runner coverage passes, but no live release tournament was run.

### Live LLM NPC scenarios

**Result: BLOCKED.** No Ollama/llama-server endpoint or low-privilege remote provider credential was available. The two ignored Ollama tests remain ignored and are not claimed passed.

### Release keychain scenario

**Result: BLOCKED.** Static implementation and automated storage/redaction tests pass, but OS-keychain behavior requires a real Secret Service session.

The executable manual procedure is `docs/MANUAL_QA_CHECKLIST.md`.

## Remaining release blockers

The authoritative list is `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`. The fresh automated baseline is complete, and direct binary/`.deb` creation and inspection are proven. Desktop release readiness remains blocked by:

- graphical launch of the direct binary and installed package;
- two-local-instance full-tournament and storage-isolation QA;
- two-machine LAN full-tournament QA;
- live reconnect, expiry, observer, host-loss, and error-state QA;
- release OS-keychain write/read/clear and failure behavior;
- live rule-based NPC tournament;
- live LLM provider scenarios and the captured Android canonical fixture.

## Android/Desktop interoperability status

Status remains **audited but not fully proven**. Known payload-shape differences for action-window-opened and action-rejected events remain backlog items. The ignored Android canonical fixture test is explicit evidence that mixed-runtime canonical bytes are not yet fully proven.

## Final milestone recommendation

**Continue desktop runtime validation. Do not begin the Android/UniFFI milestone yet.**

The repository now has a trustworthy automated and packaging baseline. The next work should execute the manual release-binary, package-install, local multiplayer, LAN multiplayer, reconnect, keychain, and NPC gates against the recorded source revision. Any reproduced failure should receive a stable backlog ID, a focused regression test where practical, and the smallest correct fix.
