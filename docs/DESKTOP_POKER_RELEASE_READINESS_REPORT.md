# Desktop Poker Release Readiness Report

## Executive result

**Outcome: The direct Linux release binary, complete local multiplayer tournament, and per-instance restart persistence are now proven. Additional packaging, physical-network, reconnect, keychain, and live-NPC validation is still required before a release tag or Android/UniFFI milestone.**

A real Tauri/WebKit release binary now passes production-route, browser-mock isolation, session-guard, invalid-invite, three-instance isolation, real TCP host/join, lobby seating, ready-state, tournament-start, private-card isolation, synchronized public state, legal and rejected action handling, multiple settled hands, elimination, observer transition, matching final standings, port-conflict recovery, fresh-profile isolation, and host/client hand-history restoration after release-process restart.

The remaining release blockers are:

- installed `.deb` graphical launch and uninstall validation;
- an explicit AppImage target decision and, if retained, AppImage launch validation;
- two physical machines on one LAN;
- reconnect, expiry, observer recovery, host loss, and unreachable-host behavior;
- release OS-keychain behavior;
- live rule-based NPC operation;
- the remaining Android canonical fixture and mixed-runtime interoperability proof before Android implementation.

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

### Latest Linux release runtime validation

- Validated source commit: `0b49e7e888aa832145d8ef815ad8bc62419a26e1`
- Single/multi-instance GitHub Actions run: `30289835791`
- Full-game GitHub Actions run: `30289835913`
- Recorded results:
  - `docs/runtime-validation/latest.json`
  - `docs/runtime-validation/gameplay-latest.json`
- Environment: Ubuntu 24.04 GitHub-hosted runner
- Graphical session: Xvfb, 1440×960×24
- Desktop runtime: Tauri 2 / WebKitGTK through `tauri-driver` and WebKitWebDriver
- Session bus: isolated `dbus-run-session`
- Release binary: `target/release/desktop-poker`
- Release binary SHA-256: `2355970b00dd39675f2f8b538cf58501887cfe3cce7c2837197a7900403989e4`
- Evidence artifacts:
  - `linux-release-runtime-evidence`
  - `linux-release-full-game-evidence`

The virtual graphical session is valid evidence for release application launch, loopback multi-instance behavior, complete gameplay, and profile-specific restart persistence. It does not substitute for installed-package testing, a physical desktop session, two physical LAN machines, a real user keychain, or configured live model providers.

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

Additional focused regressions were added during runtime validation for lobby projection, completion-history persistence, delayed final-history synchronization, monotonic history merging, host transition serialization, and rejection of settled-history regression. The host transition repair was committed only after its focused tests, the complete networking tournament test module, formatting, and Clippy passed.

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
- Complete two-player release tournament: **PASS**
- Restart history restoration for both isolated profiles: **PASS**

### Debian package

- Package construction and metadata inspection: **PASS**
- Installed binary path and desktop assets: **PASS — static inspection**
- Installation in a disposable environment: **NOT RUN**
- Installed graphical launch: **NOT RUN**
- Uninstall and cleanup behavior: **NOT RUN**

### AppImage

- Result: **BLOCKED / NOT RUN**
- No AppImage launch is claimed.
- The project still needs an explicit decision to retain AppImage as a release target or remove it from the release claim.

## Production reachability and secret-safety status

| Scenario | Result | Evidence |
|---|---|---|
| Release Home/Host/Join/Settings/Help | PASS | Run `30289835791`. |
| Release `/debug` reachability | PASS | Redirected to Home. |
| Browser-mock substitution in release | PASS | `window.__DESKTOP_POKER_BROWSER_MOCKS__` was absent. |
| Fake session through direct guarded routes | PASS | `/lobby` and `/table` redirected safely without a session. |
| Release keychain write/read/clear | BLOCKED | Requires a real Secret Service/keychain session and test credential. |
| Plaintext-key search in release app data/logs | BLOCKED | Requires the keychain scenario. |

## Live local multiplayer runtime evidence

### Three release instances and storage isolation

**Result: PASS.**

The runtime and full-game workflows launched independently namespaced host, client, conflict, and fresh-profile release instances under `~/.local/share/desktop-poker/profiles/`.

Proven behavior:

- distinct instance IDs and profile directories;
- independently namespaced host drafts before joining;
- fresh third profile contains no host/client hand history;
- host and client history restore after both release processes are stopped and restarted with the same instance IDs.

Window-state restoration and every settings field are not separately claimed by the full-game scenario.

### Real TCP host/join and lobby flow

**Result: PASS.**

- Host started a real TCP listener on loopback in the isolated runtime environment.
- Host produced a compact `pkr1_…` invitation.
- Client validated the invitation and joined the live host.
- Host and client raw status payloads agreed on participant seat indexes.
- Host began in its authoritative occupied seat.
- Client claimed the remaining open seat through the release UI.
- Both instances agreed on distinct seat assignments.
- Ready state propagated to both instances.
- Host started the tournament.
- Both instances transitioned to Main Table.

### Complete tournament integrity

**Result: PASS.**

Run `30289835913` proved:

- initial public table state synchronized;
- exactly one action tray was visible and belonged to the acting player;
- an out-of-bounds raise was rejected without advancing state;
- the Min quick-size changed the legal raise amount without submitting;
- fold completed the first hand with synchronized duplicate-free history;
- subsequent all-in showdown sequences settled additional hands;
- three hands completed in the retained passing run;
- the final hand eliminated one player and transitioned that participant to an eliminated observer;
- both release instances rendered the same winner and final standings;
- total final chips remained 2000 in the winner’s stack;
- host and client history both restored after release-process restart.

The retained final history contains exactly hand numbers 1, 2, and 3 with no duplicates. The final hand records a 1940-chip pot and the eliminated host participant.

### Port conflict and recovery

**Result: PASS.**

- A third release instance attempted to host on occupied port `43818`.
- The UI failed explicitly with `Unable to start hosting.`
- The conflicting instance did not report a false live host session.
- Changing the port to `43819` allowed hosting to start successfully.

The current message is explicit but generic; improving it to include useful bind context remains a UX-quality consideration.

## Runtime defects discovered and fixed

### DP-RR-FIX-004 — Unseated participant consumed the only open lobby seat

**Observed in run `30233994857`.**

After the client joined, the backend correctly reported the host as seated, the client as admitted but unseated, and one open seat. `buildLiveSeats()` nevertheless inserted the unseated participant into the first open seat as a `pending` card. That consumed the only open slot and rendered no **Take seat** button, blocking the client from proceeding.

The same function also classified every seated non-host as `kind: "host"`, which could produce incorrect host-only leave semantics for clients.

**Fix:**

- unseated participants no longer consume authoritative seat slots;
- open seats remain actionable;
- seated participants are classified as `host` or `player` according to `isHost`;
- three focused regression tests cover pending-client, open-seat, and seated-client projection behavior;
- retained release runtime evidence proves the client can claim the remaining seat and continue through tournament start.

### DP-RR-FIX-005 — Host tick could overwrite a newer settled hand

**Observed through repeated full-game restart failures before run `30289835913`.**

The host tick thread cloned a controller candidate, released the tournament-runtime lock, and only then wrote the candidate to `authoritative_state`. A player action could commit a newer final hand during that gap. The delayed tick then replaced authoritative state with its older candidate, silently deleting the newest settled hand. Because the controller had already completed the tournament, no later tick necessarily repaired the authoritative copy.

This was an authoritative-state integrity race, not merely a browser-storage timing issue.

**Fix:**

- added one host transition lock shared by tick, local action, remote action, tournament start, and direct authoritative replacement paths;
- serialized controller mutation, authoritative writeback, and transition publication;
- centralized runtime writeback through `commit_runtime_state()`;
- rejected table/session mismatch;
- rejected any candidate that rewrites or removes settled hand history;
- rejected any candidate that reopens a completed tournament;
- retained networking-state merge behavior;
- added regressions proving action submission waits for the transition lock and settled history cannot regress;
- added merge-safe frontend history persistence and an explicit bounded final-history save gate before completion navigation.

**Verification:**

- focused concurrency regressions: PASS;
- complete networking tournament test module: PASS;
- Rust formatting and Clippy with warnings denied: PASS;
- ordinary packaged runtime run `30289835791`: PASS;
- complete release tournament and restart run `30289835913`: PASS.

## Remaining runtime gates

### Two-local-instance full tournament

**Result: PASS.**

The complete release-runtime gate covers host/join, seat assignment, ready-state, tournament start, private-state isolation, legal and illegal action behavior, quick-size, multiple hands, duplicate-free history, elimination, observer transition, matching completion standings, fresh-profile isolation, and host/client restart history restoration.

Normal client leave and host close remain useful shutdown scenarios, but they are no longer prerequisites for claiming that the complete local tournament and persistence gate passed.

### Two-machine LAN tournament

**Result: BLOCKED.** Loopback Xvfb evidence does not replace two physical machines and a real LAN/firewall path.

### Reconnect and failure matrix

**Result: PARTIAL.** Invalid invite, guarded-route recovery, host-port conflict, and recovery on a different port pass. Lobby reconnect, active-hand reconnect, expiry, stale/duplicate action rejection after reconnect, eliminated-observer reconnect, host loss, unavailable-host join, and post-completion reconnect remain.

### Rule-based NPC tournament

**Result: BLOCKED / NOT RUN.** Automated NPC coverage exists, but no live release tournament was executed.

### Live LLM NPC scenarios

**Result: BLOCKED.** No live provider endpoint or test credential was configured.

### Release keychain scenario

**Result: BLOCKED.** Static storage/redaction tests pass, but a real release keychain session has not been exercised.

## Android/Desktop interoperability status

Status remains **audited but not fully proven**. The ignored Android canonical fixture and known event-shape differences remain open. Runtime work should stay focused on completing the desktop release gates before starting the Android/UniFFI milestone.

## Final milestone recommendation

**Continue desktop release validation on `master`. Do not begin the Android/UniFFI milestone yet.**

The next highest-value automatable increment is installed `.deb` launch/uninstall validation, followed by the reconnect and unavailable-host matrix. Physical LAN and real desktop keychain checks require environments not provided by the current GitHub-hosted loopback runner. A live rule-based NPC release tournament should be completed before a normal desktop release claim.
