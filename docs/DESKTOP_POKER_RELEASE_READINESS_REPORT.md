# Desktop Poker Release Readiness Report

## Executive result

**Outcome: The direct Linux release binary and initial local multiplayer runtime are now proven, but additional desktop runtime validation is still required before a release tag or Android/UniFFI milestone.**

A real Tauri/WebKit release binary now launches under a Linux graphical session and passes production-route, browser-mock isolation, session-guard, invalid-invite, three-instance isolation, real TCP host/join, lobby seating, ready-state, tournament-start, initial private-card isolation, synchronized public-state, port-conflict, and alternate-port recovery checks.

The remaining release blockers are narrower but still material:

- installed `.deb` graphical launch;
- legal action play through multiple hands, elimination, history, completion, and matching standings;
- restart and per-instance persistence isolation;
- two physical machines on one LAN;
- reconnect, expiry, observer recovery, host loss, and unreachable-host behavior;
- release OS-keychain behavior;
- live rule-based and LLM NPC operation;
- the remaining Android canonical fixture and mixed-runtime interoperability proof.

This report is the authoritative evidence record for `docs/DESKTOP_POKER_RELEASE_READINESS_BASELINE_TODO.md`.

## Result vocabulary

- `PASS` — the named command or scenario was executed successfully against the recorded revision.
- `FAIL` — the named command or scenario was executed and produced a product or validation failure.
- `PARTIAL` — a meaningful subset passed, but the complete release gate remains open.
- `BLOCKED` — execution requires a concrete unavailable dependency or environment.
- `NOT RUN` — execution was not attempted for a stated reason.

## Tested revisions and environments

### Automated and packaging baseline

- Repository: `ekkus93/desktop_poker`
- Branch: `master`
- Release-readiness work merged to `master`: `2e4d1f3835cf24ec57f6530fa85e9dbe7d2c1b6f`
- Final clean-head baseline validation commit before runtime work: `4c63a4efa5b9a9a1f8683f85c8698629024a21f4`
- Baseline GitHub Actions run: `30226608930`
- Baseline runner: Ubuntu 24.04.4 LTS, x86-64, GitHub-hosted Azure VM
- Baseline Node.js: `v24.18.0`
- Baseline npm: `11.16.0`
- Baseline Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14)`
- Baseline Cargo: `cargo 1.97.1 (c980f4866 2026-06-30)`

### Linux release runtime validation

- Validated source commit: `d2f4fc82eeb43a9ecec4524e490dda4662e80123`
- GitHub Actions run: `30234522553`
- Recorded result: `docs/runtime-validation/latest.json`
- Environment: Ubuntu 24.04 GitHub-hosted runner
- Graphical session: Xvfb, 1440×960×24
- Desktop runtime: Tauri 2 / WebKitGTK through `tauri-driver` and WebKitWebDriver
- Session bus: isolated `dbus-run-session`
- Release binary: `target/release/desktop-poker`
- Release binary SHA-256: `a8dc5d705a573371d64ab3872371598308b697ffa09126a19112e53b57d371fe`
- Evidence artifact: `linux-release-runtime-evidence`

The virtual graphical session is valid evidence for application launch and loopback multi-instance behavior. It does not substitute for installed-package testing, a physical desktop session, two physical LAN machines, a real user keychain, or configured live model providers.

## Automated validation baseline

| Validation | Result | Evidence |
|---|---|---|
| `npm ci` | PASS | Committed lockfile installed in baseline jobs. |
| `npm audit --omit=dev` | PASS | Zero vulnerabilities. |
| `npm audit` | PASS | Zero vulnerabilities. |
| `npm run format:check` | PASS | Prettier passed. |
| `npm run lint` | PASS | ESLint passed with zero warnings/errors. |
| `npm run test` | PASS | Baseline: 273 frontend tests passed. |
| `npm run build` | PASS | TypeScript/Vite production build passed. |
| `npm run test:geometry` | PASS | Browser geometry passed. |
| `cargo fmt --check` | PASS | Workspace formatting passed. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS | Clippy passed with warnings denied. |
| Rust workspace tests | PASS | 588 passed, 0 failed, 3 explicitly ignored. |
| Focused `poker-core` tests | PASS | 125 passed. |
| `cargo tree -p poker-core` | PASS | Shared core remains platform-neutral. |

Three focused frontend regression tests were added in `src/screens/useLobbySession.test.ts` for the runtime-discovered lobby projection defect. The retained runtime run proves the corrected production behavior. The baseline test total above remains the last explicitly retained full frontend-suite count in this report.

### Ignored Rust tests

The baseline contains exactly three ignored tests:

1. Two live Ollama tests requiring a configured local model.
2. The Android canonical JSON fixture test requiring the captured Android fixture.

None is claimed as passed.

## Linux release artifact and launch status

### Direct release binary

- Build: **PASS**
- Static inspection: **PASS**
- Graphical launch: **PASS**
- Production Home, Host, Join, Settings, and Help routes: **PASS**
- Browser mocks absent: **PASS**
- Hidden `/debug` route unreachable: **PASS**
- Direct `/lobby` and `/table` without sessions redirect safely: **PASS**
- Invalid invite displays an inline error and remains on Join: **PASS**

The current runtime hash differs from the earlier baseline artifact because the lobby projection fix and runtime validation harness were added after the first release-candidate build.

### Debian package

- Package construction and metadata inspection: **PASS**
- Installed binary path and desktop assets: **PASS — static inspection**
- Installation in a disposable environment: **NOT RUN**
- Installed graphical launch: **NOT RUN**

### AppImage

- Result: **BLOCKED / NOT RUN**
- No AppImage launch is claimed.

## Production reachability and secret-safety status

| Scenario | Result | Evidence |
|---|---|---|
| Release Home/Host/Join/Settings/Help | PASS | Run `30234522553`. |
| Release `/debug` reachability | PASS | Redirected to Home. |
| Browser-mock substitution in release | PASS | `window.__DESKTOP_POKER_BROWSER_MOCKS__` was absent. |
| Fake session through direct guarded routes | PASS | `/lobby` and `/table` redirected safely without a session. |
| Release keychain write/read/clear | BLOCKED | Requires a real Secret Service/keychain session and test credential. |
| Plaintext-key search in release app data/logs | BLOCKED | Requires the keychain scenario. |

## Live local multiplayer runtime evidence

### Three release instances and storage isolation

**Result: PASS for launch-time isolation.**

The run launched three independent release instances:

- `runtime-host-30234522553`
- `runtime-client-30234522553`
- `runtime-conflict-30234522553`

Each reported a distinct application profile directory under `~/.local/share/desktop-poker/profiles/`. The client’s host draft was independently namespaced before it joined the host.

Restart persistence, history separation, window-state restoration, and post-tournament data isolation remain untested.

### Real TCP host/join and lobby flow

**Result: PASS.**

- Host started a real TCP listener on `43818`.
- Host produced a compact `pkr1_…` invitation.
- Client validated the invitation and joined the live host.
- Host and client raw status payloads agreed on participant seat indexes.
- Host began in its authoritative occupied seat.
- Client claimed the remaining open seat through the release UI.
- Both instances agreed on distinct seat assignments.
- Ready state propagated to both instances.
- Host started the tournament.
- Both instances transitioned to Main Table.

### Initial table integrity

**Result: PASS for the first running-hand snapshot.**

- Each instance saw exactly one local seat.
- Each local player received exactly two private hole cards.
- No remote private hole cards were exposed.
- Table ID, current hand number, street, pot, and board matched between host and client.

Legal action play, raise boundaries, action-owner visibility, complete hand history, elimination, observer behavior, and tournament completion remain untested in a release runtime.

### Port conflict and recovery

**Result: PASS.**

- A third release instance attempted to host on occupied port `43818`.
- The UI failed explicitly with `Unable to start hosting.`
- The conflicting instance did not report a false live host session.
- Changing the port to `43819` allowed hosting to start successfully.

The current message is explicit but generic; improving it to include useful bind context remains a UX-quality consideration.

## Runtime defect discovered and fixed

### DP-RR-FIX-004 — Unseated participant consumed the only open lobby seat

**Observed in run `30233994857`.**

After the client joined, the backend correctly reported the host as seated, the client as admitted but unseated, and one open seat. `buildLiveSeats()` nevertheless inserted the unseated participant into the first open seat as a `pending` card. That consumed the only open slot and rendered no **Take seat** button, blocking the client from proceeding.

The same function also classified every seated non-host as `kind: "host"`, which could produce incorrect host-only leave semantics for clients.

**Fix:**

- unseated participants no longer consume authoritative seat slots;
- open seats remain actionable;
- seated participants are classified as `host` or `player` according to `isHost`;
- three focused regression tests cover pending-client, open-seat, and seated-client projection behavior;
- run `30234522553` proves the client can claim the remaining seat and continue through tournament start.

## Remaining runtime gates

### Two-local-instance full tournament

**Result: PARTIAL.** Host/join, seat assignment, ready-state, tournament start, initial privacy, and initial public synchronization pass. The following remain:

- fold, check, call, bet, raise, quick-size, and all-in confirmation;
- illegal-raise rejection;
- acting-player action-tray exclusivity;
- multiple hands and duplicate-free history;
- elimination and observer transition;
- completion and matching standings;
- normal client leave and host close;
- restart and persistence isolation.

### Two-machine LAN tournament

**Result: BLOCKED.** Loopback Xvfb evidence does not replace two physical machines and a real LAN/firewall path.

### Reconnect and failure matrix

**Result: PARTIAL.** Invalid invite, guarded-route recovery, and host-port conflict pass. Lobby reconnect, active-hand reconnect, expiry, stale-action rejection, eliminated-observer reconnect, host loss, and unavailable-host join remain.

### Rule-based NPC tournament

**Result: BLOCKED / NOT RUN.** Automated NPC coverage exists, but no live release tournament was executed.

### Live LLM NPC scenarios

**Result: BLOCKED.** No live provider endpoint or test credential was configured.

### Release keychain scenario

**Result: BLOCKED.** Static storage/redaction tests pass, but a real release keychain session has not been exercised.

## Android/Desktop interoperability status

Status remains **audited but not fully proven**. The ignored Android canonical fixture and known event-shape differences remain open. Runtime work should stay focused on completing the desktop release gates before starting the Android/UniFFI milestone.

## Final milestone recommendation

**Continue desktop runtime validation on `master`. Do not begin the Android/UniFFI milestone yet.**

The next highest-value runtime increment is legal action play through at least one complete hand, followed by deterministic progression to elimination/completion and restart-based persistence isolation. Physical LAN, reconnect interruption, keychain, and live NPC/provider checks remain separate gates.
