# Desktop Poker Current Backlog

## Authority and scope

This is the authoritative current backlog for `ekkus93/desktop_poker` on `master`.

Historical specs, TODOs, reviews, response files, and `memory.md` remain useful evidence, but unchecked boxes and historical completion statements are not current truth without source, test, or runtime verification.

Current evidence is recorded in:

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_BASELINE_SPEC.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_BASELINE_TODO.md`
- `docs/MANUAL_QA_CHECKLIST.md`
- `docs/runtime-validation/latest.json`
- `docs/runtime-validation/gameplay-latest.json`
- `docs/runtime-validation/embedded-model-latest.json`
- `docs/runtime-validation/embedded-tournament-latest.json`

## Current release position

The project is an advanced MVP with a strong automated baseline. The direct Linux release binary launches successfully, production-only route behavior is proven, and a complete real-TCP two-player release tournament is proven through legal/rejected actions, multiple settled hands, elimination, matching standings, fresh-profile isolation, and host/client history restoration after release-process restart. A complete release-mode tournament with two profile-backed in-process GGUF NPCs is also proven through two settled hands, ten committed model-selected actions, elimination, and final standings without an NPC-runner diagnostic.

A release tag is still blocked by installed-package/AppImage, physical-LAN, reconnect, keychain, and live rule-based NPC evidence.

Open release blockers:

- `DP-RR-P0-002` — installed package/AppImage completion only;
- `DP-RR-P0-004` — physical two-machine LAN;
- `DP-RR-P0-005` — remaining reconnect/failure matrix;
- `DP-RR-P0-006` — release secret-storage proof;
- `DP-RR-P1-001` — live rule-based NPC tournament.

---

## Desktop release blockers

### DP-RR-P0-001 — Fresh automated baseline

- **Priority:** P0
- **Category:** Validation / CI
- **Required behavior:** Formatting, lint, frontend tests, production build, browser geometry, Rust format, Clippy with warnings denied, workspace tests, focused `poker-core` tests, dependency audit, and npm audits pass against a recorded revision.
- **Evidence:** `.github/workflows/ci.yml`; release-readiness report.
- **Status:** **Completed.** Baseline run `30226608930` passed and retained evidence.

### DP-RR-P0-002 — Release artifacts build, inspect, install, and launch

- **Priority:** P0
- **Category:** Packaging / release runtime
- **Current behavior:** Direct release binary build, static inspection, and graphical launch pass. Home, Host, Join, Settings, Help, guarded routes, debug isolation, browser-mock isolation, complete gameplay, and restart history restoration pass in real Tauri/WebKit sessions. Debian package build and metadata inspection pass.
- **Evidence:** Runtime run `30289835791`; full-game run `30289835913`; `docs/runtime-validation/latest.json`; `docs/runtime-validation/gameplay-latest.json`; release-readiness report.
- **Remaining required behavior:**
  - install the `.deb` in a disposable or safe Linux environment;
  - launch the installed application;
  - verify desktop entry/icon integration and uninstall cleanup;
  - build and launch AppImage, or explicitly decide it is not a release target.
- **Acceptance criteria:** Every claimed distribution artifact launches and corresponds to the tested source revision.
- **Desktop release blocker:** Yes.
- **Status:** **Partially complete.** Direct binary runtime and full tournament PASS; installed package and AppImage remain open.

### DP-RR-P0-003 — Two-local-instance full tournament and isolation

- **Priority:** P0
- **Category:** Multiplayer / desktop runtime
- **Current behavior:** Two isolated release instances complete real TCP host/join, invitation validation, seat claim, ready-state, tournament start, private-card isolation, synchronized public state, rejected illegal raise, quick-size selection, fold, all-in showdowns, three settled hands, elimination, observer transition, matching final standings, and history restoration after both release processes restart. A fresh third profile contains no host/client history.
- **Evidence:** Full-game run `30289835913`; `docs/runtime-validation/gameplay-latest.json`.
- **Resolved runtime defects:**
  - `DP-RR-FIX-004` restored the missing open seat and corrected non-host seat classification.
  - `DP-RR-FIX-005` serialized host runtime transitions and prevented a delayed tick from deleting a newer settled hand.
- **Acceptance criteria:** A full release tournament completes; final state matches; no private data leaks; restarts preserve only the correct instance data.
- **Desktop release blocker:** No longer open.
- **Status:** **Completed.**

### DP-RR-P0-004 — Two-machine LAN full tournament

- **Priority:** P0
- **Category:** Networking / interoperability
- **Current behavior:** Loopback real-TCP runtime and complete gameplay are proven. Two physical machines are not.
- **Required behavior:** Matching release artifacts on two machines use a real LAN IP/firewall path, complete a tournament, preserve confidentiality, and produce matching final state.
- **Acceptance criteria:** Full tournament completes without loopback, forwarding, browser mocks, or manual state edits.
- **Desktop release blocker:** Yes.
- **Status:** **Blocked** by absence of two physical LAN machines in the current environment.

### DP-RR-P0-005 — Reconnect, expiry, observer, host-loss, and error matrix

- **Priority:** P0
- **Category:** Recovery / networking integrity
- **Current behavior:** Invalid invite UI, direct guarded-route recovery, explicit host-port conflict, recovery on a different port, complete gameplay, and post-process history restoration pass in release runtimes.
- **Evidence:** Runtime run `30289835791`; full-game run `30289835913`.
- **Remaining required behavior:**
  - unreachable-host join failure;
  - lobby reconnect;
  - active-hand reconnect before expiry;
  - reconnect after expiry;
  - stale/duplicate action rejection after reconnect;
  - eliminated-observer reconnect;
  - host loss during lobby and during a hand;
  - post-completion reconnect failure.
- **Acceptance criteria:** Every scenario is explicit and truthful; stale authority never survives; terminal failures never leave a playable-looking UI.
- **Desktop release blocker:** Yes.
- **Status:** **Partially complete.** Basic release error paths and restart persistence PASS; interruption matrix remains open.

### DP-RR-P0-006 — Release secret-storage proof

- **Priority:** P0
- **Category:** Security / credentials
- **Current behavior:** Production route behavior and isolated profile directories are available for inspection, but no real keychain test credential has been exercised.
- **Required behavior:** Release API key is stored only in OS keychain; non-secret JSON excludes it; logs/snapshots/debug output redact it; clear removes it; keychain failure is visible and never falls back to plaintext.
- **Acceptance criteria:** A low-privilege test key survives restart through the keychain, is absent from files/logs, is removed through UI, and forced keychain failure is explicit.
- **Desktop release blocker:** Yes.
- **Status:** **Blocked** by absence of a real test Secret Service/keychain session and credential.

### DP-RR-P1-001 — Live rule-based NPC tournament

- **Priority:** P1
- **Category:** NPC runtime
- **Current behavior:** Automated NPC logic coverage exists; no live release tournament has been recorded.
- **Required behavior:** At least one release tournament with seated rule-based NPCs runs through multiple hands and completion; identities/profiles remain correct; no seated NPC lacks a runner; failures are visible.
- **Acceptance criteria:** Legal NPC actions, history, elimination, and standings complete without manual injection.
- **Desktop release blocker:** Yes.
- **Status:** Open.

### DP-RR-P1-002 — Live LLM provider scenarios

- **Priority:** P1
- **Category:** NPC provider runtime
- **Current behavior:** The embedded local GGUF provider has passed both a real single-decision smoke and a complete release-mode tournament. Workflow run `30310372741` loaded the checksum-pinned SmolLM2 135M GGUF in-process, seated `aggressive-alice` and `balanced-sam`, completed two hands, recorded ten committed NPC actions, and reached final standings with no `[npc-runner]` diagnostic.
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

### DP-RR-FIX-004 — Unseated participant consumed the open lobby seat

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
- **Problem:** The tick thread released the tournament-controller lock before authoritative writeback. A final player action could commit a newer hand in that gap, after which the delayed tick replaced authoritative state with its older candidate.
- **Fix:**
  - one transition lock serializes tick, local action, remote action, tournament start, and direct replacement paths;
  - `commit_runtime_state()` rejects table/session mismatch, settled-history regression, and reopening a completed tournament;
  - frontend history persistence merges by hand number and completion navigation waits for a bounded final-history save gate;
  - focused concurrency and settled-history regressions were added.
- **Verification:** Focused tests, full networking tournament tests, formatting, Clippy, runtime run `30289835791`, and full-game run `30289835913` pass.
- **Status:** Resolved.

### DP-RR-DOC-001 — Stale workspace release paths

- README paths were corrected from `src-tauri/target/release` to `target/release`.

---

## Desktop P1/P2 items

### DP-UX-P1-001 — Remove misleading optimistic ready-state override

- **Current behavior:** Lobby still displays an optimistic local ready override before authoritative confirmation.
- **Required behavior:** Show an in-flight state and render only confirmed server state; restore prior state on failure.
- **Status:** Open. Runtime ready propagation passed under normal loopback conditions, but latency/failure honesty remains unproven.

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

- Explain that invite decoding does not prove host reachability; add safe re-check where appropriate.
- **Status:** Open. Invalid garbage input is explicit; unreachable-host recovery remains untested.

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
