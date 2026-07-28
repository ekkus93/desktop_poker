# Desktop Poker Current Backlog

## Authority and scope

This is the authoritative current backlog for `ekkus93/desktop_poker` on `master`.

Historical specs, TODOs, reviews, response files, and `memory.md` remain useful evidence, but unchecked boxes and historical completion statements are not current truth without source, test, or runtime verification.

Current evidence is recorded in:

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_BASELINE_SPEC.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_BASELINE_TODO.md`
- `docs/MANUAL_QA_CHECKLIST.md`
- `docs/runtime-validation/ci-latest.json`
- `docs/runtime-validation/latest.json`
- `docs/runtime-validation/gameplay-latest.json`
- `docs/runtime-validation/appimage-latest.json`
- `docs/runtime-validation/deb-install-latest.json`
- `docs/runtime-validation/reconnect-failure-latest.json`
- `docs/runtime-validation/release-keychain-latest.json`
- `docs/runtime-validation/rule-based-tournament-latest.json`
- `docs/runtime-validation/embedded-model-latest.json`
- `docs/runtime-validation/embedded-tournament-latest.json`

## Current release position

The project is an advanced MVP with a strong automated baseline. The direct Linux release binary launches successfully, production-only route behavior is proven, and complete real-TCP two-player release tournaments are proven through legal/rejected actions, multiple settled hands, elimination, matching standings, fresh-profile isolation, and host/client history restoration after release-process restart. The Linux AppImage builds and passes the production WebDriver smoke. The Debian package builds, installs through `apt`, passes the production WebDriver smoke, launches through its desktop entry with an X11 window, owns valid icon files, and purges without leaving its executable, desktop entry, or icon files.

The release reconnect matrix now passes protocol and real release-runtime validation. It proves explicit unreachable-host failure, lobby and active-hand reconnect on new TCP tuples, same-hand state restoration, accepted post-reconnect play, immediate duplicate rejection, and terminal stale-surface rejection after host loss in both lobby and active play. Expired/stale tokens, already-connected rejection, duplicate-session prevention, eliminated-observer restoration, and completed-tournament restoration are covered by focused protocol regressions. Release credential storage also passes a real Linux Secret Service session, restart recovery, file/log redaction, clear/delete persistence, and forced-unavailable-keychain behavior without plaintext fallback.

Rule-based and embedded local GGUF NPC tournaments pass in release mode without silent fallback diagnostics.

The only remaining desktop release blocker is `DP-RR-P0-004`, the two-machine LAN gate. The user has deferred that work until moving to a more suitable machine. A multi-container network test can provide additional network-path evidence, but under the current acceptance criteria it does not replace final proof on two physical machines unless the gate is explicitly redefined.

Open release blockers:

- `DP-RR-P0-004` — physical two-machine LAN.

---

## Desktop release blockers

### DP-RR-P0-001 — Fresh automated baseline

- **Priority:** P0
- **Category:** Validation / CI
- **Required behavior:** Formatting, lint, frontend tests, production build, browser geometry, Rust format, Clippy with warnings denied, workspace tests, focused `poker-core` tests, dependency audit, and npm audits pass against a recorded revision.
- **Evidence:** `.github/workflows/ci.yml`; `docs/runtime-validation/ci-latest.json`; release-readiness report.
- **Status:** **Completed.** Run `30335336701` passed against commit `f35d8552648091211ed05c99ac8474ae07541be6` with Verify and browser geometry both successful.

### DP-RR-P0-002 — Release artifacts build, inspect, install, and launch

- **Priority:** P0
- **Category:** Packaging / release runtime
- **Current behavior:** Direct release binary build, static inspection, and graphical launch pass. Home, Host, Join, Settings, Help, guarded routes, debug isolation, browser-mock isolation, complete gameplay, and restart history restoration pass in real Tauri/WebKit sessions. The AppImage builds, is executable, and passes the production WebDriver smoke. The Debian package builds and installs through `apt`; its installed executable passes the production WebDriver smoke; its desktop entry launches the packaged process and an X11 window; package-owned desktop and icon files are validated; and purge removes the executable, desktop entry, and icon files.
- **Evidence:** `docs/runtime-validation/latest.json`; `docs/runtime-validation/gameplay-latest.json`; `docs/runtime-validation/appimage-latest.json`; `docs/runtime-validation/deb-install-latest.json`; `.github/workflows/runtime-appimage-validation.yml`; `.github/workflows/runtime-debian-package-validation-v3.yml`; release-readiness report.
- **Acceptance criteria:** Every claimed distribution artifact launches and corresponds to the tested source revision.
- **Desktop release blocker:** No longer open.
- **Status:** **Completed.** Direct binary, AppImage, installed Debian runtime, desktop integration, and purge cleanup all pass.

### DP-RR-P0-003 — Two-local-instance full tournament and isolation

- **Priority:** P0
- **Category:** Multiplayer / desktop runtime
- **Current behavior:** Two isolated release instances complete real TCP host/join, invitation validation, seat claim, ready-state, tournament start, private-card isolation, synchronized public state, rejected illegal raise, quick-size selection, fold, all-in showdowns, multiple settled hands, elimination, observer transition, matching final standings, and history restoration after both release processes restart. A fresh third profile contains no host/client history.
- **Evidence:** Full-game run `30335336680`; `docs/runtime-validation/gameplay-latest.json`.
- **Resolved runtime defects:**
  - `DP-RR-FIX-004` restored the missing open seat and corrected non-host seat classification.
  - `DP-RR-FIX-005` serialized host runtime transitions and prevented a delayed tick from deleting a newer settled hand.
- **Acceptance criteria:** A full release tournament completes; final state matches; no private data leaks; restarts preserve only the correct instance data.
- **Desktop release blocker:** No longer open.
- **Status:** **Completed.**

### DP-RR-P0-004 — Two-machine LAN full tournament

- **Priority:** P0
- **Category:** Networking / interoperability
- **Current behavior:** Real TCP and complete gameplay are proven between isolated release instances on a GitHub-hosted Linux runner. Two physical machines and a real external LAN/firewall path are not yet proven.
- **Required behavior:** Matching release artifacts on two machines use a real LAN IP/firewall path, complete a tournament, preserve confidentiality, and produce matching final state.
- **Acceptance criteria:** Full tournament completes without loopback, forwarding, browser mocks, or manual state edits.
- **Desktop release blocker:** Yes.
- **Status:** **Deferred by the user.** Multi-container network testing will wait until the user moves to a more suitable machine. Final physical-machine proof remains open under the current acceptance criteria.

### DP-RR-P0-005 — Reconnect, expiry, observer, host-loss, and error matrix

- **Priority:** P0
- **Category:** Recovery / networking integrity
- **Current behavior:** The focused protocol suite and release/WebKit matrix pass.
- **Live release evidence:**
  - a cryptographically valid invitation to a stopped listener fails explicitly without retaining a partial client session;
  - lobby connection reset produces a new TCP tuple and restores a normal, usable session;
  - active-hand connection reset produces a new TCP tuple and restores the same table, session, and hand;
  - a post-reconnect client action is accepted and the immediate duplicate is rejected;
  - active-hand host loss becomes terminal and both table and action access reject the stale session;
  - lobby host loss becomes terminal and both table and action access reject the stale session.
- **Focused protocol evidence:**
  - reconnect tokens are scoped to table/session/player identity;
  - stale and expired reconnect tokens are rejected before authority or sequence mutation;
  - already-connected participants and duplicate sessions are rejected;
  - reconnect preserves eliminated-observer state;
  - reconnect to a completed tournament restores completed results without reopening action authority.
- **Resolved defect:** `DP-RR-FIX-006` makes explicit host shutdown stop listener acceptance and close all established client sockets. This forces EOF, failed reconnect, and terminal client state instead of leaving a stale live TCP connection.
- **Evidence:** Workflow run `30335336702`; `.github/workflows/runtime-reconnect-failure-matrix.yml`; `scripts/runtime_reconnect_failure_matrix.py`; `src-tauri/src/networking/runtime/host_shutdown.rs`; `src-tauri/src/networking/runtime/tests/host_shutdown.rs`; `src-tauri/src/networking/runtime/tests/reconnect.rs`; `src-tauri/src/networking/runtime/tests/reconnect_expiry.rs`; `docs/runtime-validation/reconnect-failure-latest.json`; artifact `reconnect-protocol-and-release-failure-evidence`.
- **Acceptance criteria:** Every scenario is explicit and truthful; stale authority never survives; terminal failures never leave a playable-looking UI.
- **Desktop release blocker:** No longer open.
- **Status:** **Completed.** Protocol tests, release build, and release interruption matrix all pass.

### DP-RR-P0-006 — Release secret-storage proof

- **Priority:** P0
- **Category:** Security / credentials
- **Current behavior:** The Linux release build uses the real synchronous Secret Service backend. A test credential is stored through Secret Service, reported configured without being exposed through public config, recovered after release-process restart, absent from app-data files and runtime logs, cleared, and still absent after a second restart. A deliberately unavailable D-Bus/Secret Service endpoint produces an explicit error and creates neither provider state nor a plaintext fallback.
- **Evidence:** Workflow run `30335336694`; `.github/workflows/runtime-release-keychain-validation.yml`; `scripts/runtime_release_keychain.py`; `docs/runtime-validation/release-keychain-latest.json`; artifact `linux-release-keychain-evidence`.
- **Acceptance criteria:** A low-privilege test key survives restart through the keychain, is absent from files/logs, is removed through UI, and forced keychain failure is explicit.
- **Desktop release blocker:** No longer open.
- **Status:** **Completed.**

### DP-RR-P1-001 — Live rule-based NPC tournament

- **Priority:** P1
- **Category:** NPC runtime
- **Current behavior:** Multiple release runs have seated two unprofiled rule-based NPCs and completed full tournaments. The latest retained run completed 12 hands with 19 committed NPC actions, both NPC identities participated, final elimination/standings were produced, and the production runtime log contained no NPC error or fallback diagnostic.
- **Evidence:** Workflow run `30335336686`; `scripts/runtime_rule_based_npc_tournament.py`; `.github/workflows/runtime-rule-based-npc-validation.yml`; `docs/runtime-validation/rule-based-tournament-latest.json`; artifact `linux-rule-based-npc-tournament-evidence`.
- **Acceptance criteria:** Legal NPC actions, history, elimination, and standings complete without manual injection.
- **Desktop release blocker:** No longer open.
- **Status:** **Completed.**

### DP-RR-P1-002 — Live LLM provider scenarios

- **Priority:** P1
- **Category:** NPC provider runtime
- **Current behavior:** The embedded local GGUF provider has passed both a real single-decision smoke and complete release-mode tournaments with two profile-backed NPCs, committed model-selected actions, elimination, and final standings without an `[npc-runner]` diagnostic.
- **Evidence:** `.github/workflows/embedded-model-ci.yml`; `scripts/runtime_embedded_npc_tournament.py`; `docs/runtime-validation/embedded-model-latest.json`; `docs/runtime-validation/embedded-tournament-latest.json`; artifact `embedded-npc-tournament-evidence`.
- **Remaining required behavior:** Exercise embedded model unavailable/load-failure/inference-failure paths with typed diagnostics; separately validate accepted-action and unavailable/timeout/invalid-response behavior for any remote provider advertised as release-ready.
- **Acceptance criteria:** Every supported advertised provider has at least one accepted legal live-model action and explicit failure-path evidence, with no credential leakage or silent rule-based conversion.
- **Desktop release blocker:** No for a rule-based-only release. Embedded local live play is proven; remote providers remain outside the proven release matrix.
- **Status:** **Partially complete. Embedded local full-tournament scenario completed; negative embedded paths and remote-provider scenarios remain open.**

---

## Resolved during release-readiness work

### DP-RR-FIX-001 — Production dependency vulnerability

- Migrated vulnerable React Router 7 to React Router 8.3.0 with React/React DOM 19.2.8.
- Production and full dependency audits pass.

### DP-RR-FIX-002 — Development dependency vulnerabilities

- Upgraded ESLint/PostCSS dependency graph.
- Both npm audits report zero vulnerabilities.

### DP-RR-FIX-003 — React correctness lint findings

- Corrected state synchronization and render-time ref usage without suppressing lint rules.

### DP-RR-FIX-004 — Unseated participant consumed the only open lobby seat

- **Observed:** Runtime run `30233994857`.
- **Problem:** `buildLiveSeats()` placed admitted-but-unseated participants into open seat slots as `pending`, removing the only **Take seat** action. It also classified all seated non-hosts as `host`.
- **Fix:**
  - only authoritative seated participants occupy seat slots;
  - admitted unseated participants leave open seats actionable;
  - occupied seats are classified as `host` or `player` from `isHost`;
  - three focused frontend regression tests were added.
- **Runtime proof:** Current release runs show the client claiming the remaining open seat and continuing through tournament completion.
- **Status:** Resolved.

### DP-RR-FIX-005 — Delayed host tick deleted a newer settled hand

- **Observed:** Repeated full-game runs completed correctly in memory but lost the newest hand from host history after restart.
- **Problem:** The tick thread released the tournament-controller lock before authoritative writeback. A final player action could commit a newer hand during that gap, after which the delayed tick replaced authoritative state with its older candidate.
- **Fix:**
  - one transition lock serializes tick, local action, remote action, tournament start, and direct replacement paths;
  - `commit_runtime_state()` rejects table/session mismatch, settled-history regression, and reopening a completed tournament;
  - frontend history persistence merges by hand number and completion navigation waits for a bounded final-history save gate;
  - focused concurrency and settled-history regressions were added.
- **Verification:** Focused tests, full networking tournament tests, formatting, Clippy, and complete release-runtime tournaments pass.
- **Status:** Resolved.

### DP-RR-FIX-006 — Explicit host stop left accepted TCP sessions alive

- **Observed:** Release reconnect run `30332398233` completed lobby and active-hand reconnect, accepted a post-reconnect action, and rejected the immediate duplicate, but timed out waiting for terminal client state after `stop_host_session`.
- **Problem:** `HostServer::drop()` stopped the listener and tick thread but detached per-client session threads retained their accepted `TcpStream`s. The client therefore received no EOF and could remain attached to a host session that the UI had explicitly removed.
- **Fix:**
  - `DesktopHostSession::drop()` requests explicit networking shutdown;
  - shutdown stops listener acceptance before closing established connections;
  - the connected-client registry is drained before stream locks are taken, avoiding cleanup lock-order deadlock;
  - every registered stream receives `Shutdown::Both`;
  - poisoned locks and non-benign socket failures are logged instead of silently ignored;
  - a focused regression requires `Reconnecting`, explicit reconnect failure, and terminal `Disconnected` events.
- **Verification:** CI run `30335336701` and reconnect workflow run `30335336702` pass.
- **Status:** Resolved.

### DP-RR-DOC-001 — Stale workspace release paths

- README paths were corrected from `src-tauri/target/release` to `target/release`.

---

## Desktop P1/P2 items

### DP-UX-P1-001 — Remove misleading optimistic ready-state override

- **Current behavior:** Lobby still displays an optimistic local ready override before authoritative confirmation.
- **Required behavior:** Show an in-flight state and render only confirmed server state; restore prior state on failure.
- **Status:** Open. Runtime ready propagation and reconnect pass under automated network interruption, but latency/failure presentation honesty still needs a focused UI audit.

### DP-UX-P1-002 — Settings confirmation mutual exclusion

- Only one destructive confirmation should be visible at a time.
- **Status:** Needs current source/test verification.

### DP-UX-P1-003 — NPC profile unsaved-change protection and slug validation

- Track dirty state, confirm before navigation/window close, validate IDs against `[a-z0-9-]{1,64}`.
- **Status:** Needs current source/test verification.

### DP-UX-P1-004 — Accessibility interaction pass

- Confirm focus movement/return, live-region semantics, visible focus, and keyboard/screen-reader usability.
- **Status:** Partially implemented / needs full audit.

### DP-UX-P2-001 — Long hand-history rendering strategy

- Limit or virtualize large histories while preserving access to all hands.
- **Status:** Needs source verification.

### DP-UX-P2-002 — Improve stale invite and error recovery messaging

- Explain that invite decoding does not prove host reachability and preserve actionable connection context without exposing internals.
- **Status:** Open UX improvement. Unreachable-host behavior is now explicit and proven; wording quality remains separately reviewable.

### DP-UX-P2-003 — Improve host bind-error specificity

- **Observed:** Release runtime displays `Unable to start hosting.` for a confirmed port conflict.
- **Required behavior:** Preserve user-safe wording while including actionable context such as port-in-use and the attempted port, without exposing secrets.
- **Desktop release blocker:** No; current behavior is explicit and non-silent.
- **Status:** Open UX improvement.

---

## Android interoperability items

### DP-INT-P1-001 — Align `ACTION_WINDOW_OPENED_EVENT` payload

- Desktop includes `actionWindowId`; audited Android shape differs.
- Shared canonical fixture and live compatibility are required.
- **Status:** Deferred until desktop release gates are complete.

### DP-INT-P1-002 — Align `ACTION_REJECTED_EVENT` payload

- Desktop and Android rejected-action shapes differ.
- **Status:** Deferred until desktop release gates are complete.

### DP-INT-P1-003 — Live mixed-runtime matrix

- Desktop-host/Android-client and reverse must complete join, play, privacy, reconnect, elimination, and completion.
- **Status:** Blocked by payload alignment and environment.

### DP-INT-P2-001 — Define UniFFI-safe core adapter

- `poker-core` remains platform-neutral; no Android binding crate exists.
- Kotlin should retain networking/platform/UI; Rust adapter should expose deterministic core DTOs and commands only.
- **Status:** Deferred.

---

## Deferred product features

These must not displace release blockers:

- room-code discovery or matchmaking;
- internet/WAN hosting, relay, or cloud service;
- Tauri Mobile;
- moving networking into `poker-core`;
- sound effects or music;
- additional variants or multi-table tournaments;
- cloud accounts, synchronization, analytics, or telemetry;
- broad visual redesign unrelated to reproduced defects;
- Android implementation before desktop release and protocol mismatch resolution.

## Backlog maintenance rules

1. Reproduce a defect before assigning implementation work.
2. Give every defect a stable identifier and evidence.
3. Add a focused regression test where practical.
4. Do not weaken assertions, broaden accepted outcomes, hide errors, or add silent defaults to make a check pass.
5. Keep frontend, desktop adapter, networking, and shared-core authority boundaries intact.
6. Mark unexecuted physical/manual work `BLOCKED` or `NOT RUN`; do not infer it from virtual or automated evidence.
7. Work directly on `master`; do not create branches or pull requests unless the user explicitly requests them.
8. Consolidate current work here instead of creating competing backlogs.
