# Desktop Poker Release Readiness Report

## Executive result

**Outcome: Additional desktop stabilization / validation required.**

The automated baseline is being executed on the `agent/release-readiness-baseline` branch through GitHub Actions. Real desktop launch, two-instance gameplay, two-machine LAN gameplay, reconnect interruption, packaged-application launch, OS-keychain behavior, and live local-LLM behavior cannot be executed in the current ChatGPT tool environment because it has no graphical desktop, no repository checkout network access, no second LAN machine, and no local Ollama/llama-server process.

Those scenarios are recorded as `BLOCKED`; they are not treated as passed from unit or integration coverage.

This report is the authoritative evidence record for `docs/DESKTOP_POKER_RELEASE_READINESS_BASELINE_TODO.md`.

## Result vocabulary

- `PASS` — the named command or scenario was executed successfully against the recorded revision.
- `FAIL` — the named command or scenario was executed and produced a product or validation failure.
- `BLOCKED` — execution requires a concrete unavailable dependency or environment.
- `NOT RUN` — execution was not attempted for a stated reason.

## Tested commit and environment

### Repository

- Repository: `ekkus93/desktop_poker`
- Base branch: `master`
- Baseline base SHA: `f4fde4d70fb8fe205bf74be7469912efd682045c`
- Execution branch: `agent/release-readiness-baseline`
- Final tested branch SHA: **pending final documentation commit and CI run**

### Execution environments

#### GitHub Actions

- Environment type: GitHub-hosted CI runner
- Workflow: `.github/workflows/ci.yml`
- Runner declared by repository: Ubuntu 24.04
- Node version declared by workflow: Node.js 24
- Rust toolchain declared by workflow: stable with Clippy and rustfmt
- Graphical packaged-application launch: unavailable in the current workflow

#### ChatGPT connected-repository environment

- Repository reads/writes: connected GitHub application
- Local GitHub CLI: unavailable
- Direct `git clone`: blocked because the container cannot resolve `github.com`
- Graphical desktop session: unavailable
- Two physical LAN machines: unavailable
- Local Ollama/llama-server: unavailable

## Automated validation

The branch pull request is used to trigger the repository's real CI workflow. Results and exact job evidence will be inserted after the workflow completes.

| Validation | Result | Evidence |
|---|---|---|
| `npm ci` | NOT RUN | Pending GitHub Actions run. |
| `npm run format:check` | NOT RUN | Pending GitHub Actions run. |
| `npm run lint` | NOT RUN | Pending GitHub Actions run. |
| `npm run test` | NOT RUN | Pending GitHub Actions run. |
| `npm run build` | NOT RUN | Pending GitHub Actions run. |
| `npm run test:geometry` | NOT RUN | Pending GitHub Actions run. |
| `cargo fmt --check` | NOT RUN | Pending GitHub Actions run. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | NOT RUN | Pending GitHub Actions run. |
| `cargo test --workspace --all-targets --all-features -- --test-threads=2` | NOT RUN | Pending GitHub Actions run. |
| `cargo tree -p poker-core` | NOT RUN | Pending GitHub Actions run. |
| Focused `cargo test -p poker-core --all-targets --all-features` | NOT RUN | Not currently a separate CI step; requires a capable checkout environment or workflow expansion. |
| `npm audit --omit=dev` | NOT RUN | Not currently a CI step; requires a capable checkout environment or workflow expansion. |
| `npm audit` | NOT RUN | Not currently a CI step; requires a capable checkout environment or workflow expansion. |
| Ignored provider tests | BLOCKED | No local Ollama/llama-server endpoint is available. |

## Test totals and ignored tests

Current totals will be copied from the GitHub Actions job logs. Historical counts in `memory.md` are not accepted as current evidence.

## Release artifact inventory

| Artifact | Result | Evidence |
|---|---|---|
| Release binary | BLOCKED | A release binary may compile in CI, but the current workflow does not build or upload it on ordinary pull requests and this environment cannot launch a GUI. |
| Debian package | BLOCKED | Requires a package build plus installation/launch in a Linux graphical environment. |
| AppImage | BLOCKED | Requires `linuxdeploy`/`appimagetool` and graphical launch verification. |
| RPM | NOT RUN | Not required by the baseline TODO's primary package gate. |

No artifact is claimed working until its exact hash, size, source SHA, and launch result are recorded.

## Production reachability and security audit

### Static evidence already present in current source

- `src/main.tsx` imports the layout probe only inside `import.meta.env.DEV`.
- Browser mocks in `src/api/desktop.ts` are guarded by development/test environment checks.
- `src-tauri/tauri.conf.json` uses a restrictive CSP and limits frontend connections to Tauri IPC.
- Release provider secrets are documented and implemented through the OS keychain; debug builds use an explicitly development-only local file.

These are source-inspection findings, not substitutes for release-binary reachability or OS-keychain manual tests.

| Scenario | Result | Reason |
|---|---|---|
| Production bundle string/reachability audit | NOT RUN | Pending CI logs and a capable artifact-inspection environment. |
| Release `/debug` route reachability | BLOCKED | Requires launching a release build with a graphical WebView. |
| Browser-mock substitution in release | BLOCKED | Requires release runtime exercise; static guards exist. |
| Release keychain write/read/clear | BLOCKED | Requires a desktop Secret Service/keychain and low-privilege test credential. |
| Plaintext-key search in app data/logs | BLOCKED | Requires release application data from a real run. |

## Two-local-instance QA

**Result: BLOCKED.**

Concrete blocker: the current execution environment has no graphical desktop session and cannot launch two Tauri release instances. None of the host/join, seating, readiness, gameplay, private-card isolation, elimination, completion, persistence, or port-conflict scenarios are marked passed.

The authoritative executable checklist remains `docs/MANUAL_QA_CHECKLIST.md`.

## Two-machine LAN QA

**Result: BLOCKED.**

Concrete blocker: two physical machines on the same LAN are not available to this execution environment. Android/Desktop or desktop/desktop live TCP behavior is not inferred from automated tests.

## Reconnect and failure matrix

| Scenario | Result | Reason |
|---|---|---|
| Lobby reconnect | BLOCKED | Requires controllable live network interruption between running GUI instances. |
| Active-hand reconnect | BLOCKED | Requires controllable live network interruption during gameplay. |
| Reconnect expiry | BLOCKED | Requires live runtime and configured wait window. |
| Eliminated-observer reconnect | BLOCKED | Requires full live tournament and network interruption. |
| Host loss | BLOCKED | Requires live host/client processes. |
| Port conflict | BLOCKED | Requires two live release instances. |
| Invalid invite | BLOCKED | Automated coverage exists historically; release UI scenario was not executed in this pass. |
| Unreachable host | BLOCKED | Requires release UI and a controlled unavailable endpoint. |

## Rule-based NPC QA

**Result: BLOCKED.**

Automated NPC coverage exists, but a live release tournament with NPCs cannot be run without a graphical desktop application environment. No live rule-based tournament is claimed complete.

## LLM NPC QA

**Result: BLOCKED.**

Concrete blockers:

- no local Ollama or llama-server endpoint;
- no graphical release application session;
- no low-privilege remote-provider credential was supplied or used.

Ignored provider tests and live LLM action/fallback scenarios remain outstanding.

## Safety and silent-failure status

Historical hardening work addresses sequence validation, reconnect command-stream installation, explicit best-effort event delivery, crypto associated data, host runtime health, and NPC private-state validation. This pass will use current CI and source reconciliation to ensure those claims remain represented accurately, but runtime failure injection is blocked without a checkout and live processes.

## Defects discovered and fixes applied

No new product defect has yet been reproduced in this pass. Environment limitations are tracked separately and are not product defects.

## Remaining release blockers

The authoritative list is `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`. At minimum, desktop release readiness remains blocked by:

- `DP-RR-P0-001`: fresh automated baseline not yet complete;
- `DP-RR-P0-002`: release binary/package build and launch not proven;
- `DP-RR-P0-003`: two-local-instance full tournament not proven;
- `DP-RR-P0-004`: two-machine LAN full tournament not proven;
- `DP-RR-P0-005`: live reconnect and host-loss matrix not proven;
- `DP-RR-P0-006`: release secret-storage behavior not proven;
- `DP-RR-P1-001`: live rule-based NPC tournament not proven.

## Non-blocking current backlog

See `docs/DESKTOP_POKER_CURRENT_BACKLOG.md` for reconciled UI/UX, documentation, Android interoperability, and deferred feature items.

## Android/Desktop interoperability status

Status remains **audited but not fully proven**. Known payload-shape differences for action-window-opened and action-rejected events remain backlog items. Live mixed-runtime sessions are not part of this desktop release-readiness run and were not executed.

## Final milestone recommendation

**Additional desktop stabilization / validation.**

Do not start the Android/UniFFI milestone yet. First complete the manual release-binary, local multiplayer, LAN multiplayer, reconnect, package, keychain, and NPC gates against one exact source revision. Any reproduced failures should receive stable backlog IDs, focused regression tests where practical, and the smallest correct fix.
