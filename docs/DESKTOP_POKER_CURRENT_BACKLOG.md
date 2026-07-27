# Desktop Poker Current Backlog

## Authority and scope

This is the authoritative current backlog for `ekkus93/desktop_poker` as of the release-readiness baseline that began from commit `f4fde4d70fb8fe205bf74be7469912efd682045c` on branch `agent/release-readiness-baseline`.

Historical specs, TODOs, reviews, response files, and `memory.md` remain useful evidence, but unchecked boxes and historical completion statements are not treated as current truth without source, test, or runtime verification.

Release-readiness evidence is recorded in:

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_BASELINE_SPEC.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_BASELINE_TODO.md`
- `docs/MANUAL_QA_CHECKLIST.md`

## Source documents reviewed

- `README.md`
- `memory.md`
- `docs/DESKTOP_ARCHITECTURE.md`
- `docs/ANDROID_ARCHITECTURE.md`
- `docs/ANDROID_INTEROP_AUDIT.md`
- `docs/MANUAL_QA_CHECKLIST.md`
- `docs/UIUX_FIXES6.md`
- release/stabilization/networking/core-extraction specs, TODOs, reviews, and response documents under `docs/`
- current frontend, Tauri adapter, networking, NPC, protocol, storage, and `poker-core` source inspected during repository orientation

## Release-blocker summary

The project is an advanced MVP, but it is not yet a release-proven Linux desktop application. Automated coverage is substantial. The blocking gap is current runtime evidence: packaged launch, two-instance play, two-machine LAN play, reconnect/failure behavior, keychain behavior, and live NPC execution against one exact source revision.

Open desktop release blockers:

- `DP-RR-P0-002` through `DP-RR-P0-006`
- `DP-RR-P1-001`

A desktop release candidate should not be tagged until those items satisfy their acceptance criteria or are explicitly reclassified by a documented product decision.

---

## Desktop release blockers

### DP-RR-P0-001 — Fresh automated baseline

- **Priority:** P0
- **Category:** Validation / CI
- **Current behavior:** Current CI evidence is complete against branch source SHA `fd8369ba7267fe76a827cdf48384c9f826159719` and pull-request merge SHA `c79c9f2473f92310c3d65afe8b834f97b2875c5d`.
- **Evidence:** `.github/workflows/ci.yml`; historical `memory.md`; `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`.
- **Affected files:** `.github/workflows/ci.yml`, source/tests if failures are found, release-readiness report.
- **Required behavior:** Formatting, lint, frontend tests, production build, browser geometry, Rust format, Clippy with warnings denied, workspace tests, and `poker-core` dependency audit pass against one recorded SHA.
- **Acceptance criteria:** GitHub Actions jobs complete successfully; exact totals and ignored tests are copied from current logs; unexplained stderr noise is resolved or recorded as a defect.
- **Automated test requirement:** Existing CI plus focused `cargo test -p poker-core --all-targets --all-features` when a capable checkout is available.
- **Manual test requirement:** None.
- **Desktop release blocker:** Yes.
- **Status:** Completed — GitHub Actions run `30224757296` passed all automated gates and retained evidence artifacts.

### DP-RR-P0-002 — Release artifacts build, inspect, and launch

- **Priority:** P0
- **Category:** Packaging / release runtime
- **Current behavior:** The direct release binary and Debian package build successfully and have recorded hashes/metadata. Graphical launch and installed-package execution remain unproven.
- **Evidence:** `README.md`; `src-tauri/tauri.conf.json`; release-readiness report.
- **Affected files:** packaging configuration, README only if defects/inaccuracies are found.
- **Required behavior:** Build release binary and `.deb`; record SHA-256 and size; inspect package metadata; launch the direct binary and installed package; record AppImage as PASS/FAIL/BLOCKED.
- **Acceptance criteria:** Home, Host, Join, Settings, and Help render in a release build without browser mocks; package metadata and identifier are coherent; claimed artifacts correspond to the final tested SHA.
- **Automated test requirement:** Production frontend build and Tauri compilation.
- **Manual test requirement:** Launch direct binary and installed package in a Linux graphical session.
- **Desktop release blocker:** Yes.
- **Status:** Partially complete — build and inspection PASS; graphical launch/install verification BLOCKED by the current execution environment.

### DP-RR-P0-003 — Two-local-instance full tournament and isolation

- **Priority:** P0
- **Category:** Multiplayer / desktop runtime
- **Current behavior:** Real TCP host/client flows and multi-instance storage are implemented and covered by automated tests, but no current release-instance tournament is recorded.
- **Evidence:** `docs/MANUAL_QA_CHECKLIST.md`; README manual QA status; runtime and session tests.
- **Affected files:** source/tests only if a defect is reproduced; manual QA checklist and report.
- **Required behavior:** Two release instances host/join, claim seats, ready, play legal actions through elimination and completion, preserve private-card isolation, synchronize public state, and preserve per-instance local data isolation.
- **Acceptance criteria:** Complete tournament succeeds using `host-a` and `client-b`; final standings match; no private cards leak; restart proves independent history/drafts/settings.
- **Automated test requirement:** Add regression coverage for every practical reproduced defect.
- **Manual test requirement:** Full Checklist A plus instance-isolation checklist.
- **Desktop release blocker:** Yes.
- **Status:** Blocked by current execution environment.

### DP-RR-P0-004 — Two-machine LAN full tournament

- **Priority:** P0
- **Category:** Networking / interoperability
- **Current behavior:** LAN TCP transport is implemented; loopback integration coverage does not prove two physical machines.
- **Evidence:** `docs/MANUAL_QA_CHECKLIST.md`; README limitations.
- **Affected files:** networking/protocol/session code only if a defect is reproduced.
- **Required behavior:** Two machines on one LAN use matching artifacts, real LAN IP, TCP port 43818, direct invite, synchronized tournament, private-state isolation, and matching completion state.
- **Acceptance criteria:** Full tournament completes without loopback, forwarding, browser mocks, or manual state edits; firewall prerequisites are documented.
- **Automated test requirement:** Regression tests for defects found.
- **Manual test requirement:** Full Checklist B on two physical machines.
- **Desktop release blocker:** Yes.
- **Status:** Blocked by absence of two LAN machines.

### DP-RR-P0-005 — Reconnect, expiry, observer, host-loss, and error matrix

- **Priority:** P0
- **Category:** Recovery / networking integrity
- **Current behavior:** Extensive reconnect hardening and automated tests exist, but current release-runtime interruption evidence is absent.
- **Evidence:** networking Fix 10–16 history; `docs/MANUAL_QA_CHECKLIST.md`; client runtime source/tests.
- **Affected files:** `src-tauri/src/networking/runtime/**`, session state, frontend recovery routes if defects are reproduced.
- **Required behavior:** Lobby reconnect, active-hand reconnect, expiry, eliminated-observer reconnect, host loss, port conflict, invalid invite, and unreachable host are explicit, truthful, and preserve authoritative-state rules.
- **Acceptance criteria:** Each scenario passes against release instances; reconnect snapshot is exposed only after command stream installation; stale/duplicate actions do not apply; terminal failures do not leave a playable-looking UI.
- **Automated test requirement:** Focused deterministic regression tests for each defect.
- **Manual test requirement:** Network interruption and process-loss matrix.
- **Desktop release blocker:** Yes.
- **Status:** Blocked by current execution environment.

### DP-RR-P0-006 — Release secret-storage proof

- **Priority:** P0
- **Category:** Security / credentials
- **Current behavior:** Source selects OS keychain in release builds and local 0600 file storage in debug builds. Runtime keychain failure and data-directory inspection have not been proven in this pass.
- **Evidence:** `src-tauri/src/npc/provider_storage.rs`; provider tests; README provider storage section.
- **Affected files:** provider storage and tests only if a defect is reproduced.
- **Required behavior:** Release API key is stored only in OS keychain; settings JSON excludes secrets; logs/debug/snapshots redact key; keychain failure is visible and never falls back to plaintext; clear removes secret or reports failure.
- **Acceptance criteria:** Low-privilege test key survives restart through keychain, is absent from files/logs, and is removed through UI; forced keychain failure produces explicit error.
- **Automated test requirement:** Existing storage/redaction tests plus focused failure-seam tests where practical.
- **Manual test requirement:** Release run with Linux Secret Service/keychain.
- **Desktop release blocker:** Yes.
- **Status:** Blocked by absence of graphical desktop/keychain service.

### DP-RR-P1-001 — Live rule-based NPC tournament

- **Priority:** P1
- **Category:** NPC runtime
- **Current behavior:** Rule-based NPC logic has substantial automated coverage; live release tournament evidence is absent.
- **Evidence:** NPC runner/decision tests; README NPC section.
- **Affected files:** NPC runner, host session, profile UI only if defects are reproduced.
- **Required behavior:** At least one human-plus-NPC or two-NPC release tournament runs through multiple hands and completion; identities/profiles/styles remain correctly associated; no seated NPC lacks a runner.
- **Acceptance criteria:** Live tournament completes with legal actions, history, elimination, and standings; any runner failure is visible and atomic registration rollback remains correct.
- **Automated test requirement:** Regression test for each reproduced failure.
- **Manual test requirement:** Release NPC tournament.
- **Desktop release blocker:** Yes.
- **Status:** Blocked by current execution environment.


---

## Resolved during the release-readiness baseline

### DP-RR-P0-001 — Fresh automated baseline

- **Resolved:** GitHub Actions run `30224757296` passed formatting, lint, Rust workspace tests, focused `poker-core` tests, frontend tests/build, geometry, and both npm audits.
- **Totals:** 588 Rust tests passed, 3 explicitly ignored; 273 frontend tests passed; zero npm vulnerabilities.
- **Evidence:** `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md` and retained `validation-evidence` artifact.

### DP-RR-FIX-001 — Production dependency vulnerability

- **Resolved:** Migrated vulnerable React Router 7 production dependency to React Router 8.3.0 with React/React DOM 19.2.8.

### DP-RR-FIX-002 — Development dependency vulnerabilities

- **Resolved:** Upgraded the ESLint toolchain and PostCSS dependency graph; production and full audits now report zero vulnerabilities.

### DP-RR-FIX-003 — React correctness lint findings

- **Resolved:** Reworked state synchronization and render-time ref usage without suppressing lint rules; all frontend checks pass.

---

## Desktop P1/P2 items

### DP-UX-P1-001 — Remove misleading optimistic ready-state override

- **Priority:** P1
- **Category:** Lobby UX / state honesty
- **Current behavior:** `TournamentLobbyScreen` still derives `localSeatReady` from `optimisticReadyOverride` and flips it before host confirmation.
- **Evidence:** `src/screens/TournamentLobbyScreen.tsx`; `docs/UIUX_FIXES6.md` P1.1.
- **Affected files:** lobby screen and frontend tests.
- **Required behavior:** Disable the ready control and show an in-flight label while awaiting confirmation; render only confirmed server state; preserve pre-click state on error.
- **Acceptance criteria:** No unconfirmed ready state is displayed; test proves disabled in-flight state and error recovery.
- **Automated test requirement:** Vitest interaction test with deferred API promise and rejection.
- **Manual test requirement:** Verify in two-instance lobby under latency/failure.
- **Desktop release blocker:** No unless manual QA demonstrates materially misleading behavior.
- **Status:** Not implemented.

### DP-UX-P1-002 — Settings confirmation mutual exclusion

- **Priority:** P1
- **Category:** Settings UX
- **Current behavior:** Historical review reports independent reset/clear confirmations; current code needs direct verification before implementation.
- **Evidence:** `docs/UIUX_FIXES6.md` P1.2.
- **Affected files:** `src/screens/DeviceSettingsScreen.tsx` and tests.
- **Required behavior:** Only one destructive confirmation may be visible.
- **Acceptance criteria:** Opening one closes the other; interaction test covers both directions.
- **Automated test requirement:** Frontend test.
- **Manual test requirement:** Keyboard/focus smoke test.
- **Desktop release blocker:** No.
- **Status:** Needs source verification.

### DP-UX-P1-003 — NPC profile unsaved-change protection and slug validation

- **Priority:** P1
- **Category:** Profile editor UX / data integrity
- **Current behavior:** Historical review identifies silent navigation loss and cryptic invalid profile IDs; current implementation needs direct verification.
- **Evidence:** `docs/UIUX_FIXES6.md` P1.7–P1.8.
- **Affected files:** `src/screens/NpcProfilesScreen.tsx`, profile API/tests.
- **Required behavior:** Track dirty state, block route/window exit with a clear confirmation, validate IDs against `[a-z0-9-]{1,64}`, and disable save for invalid IDs.
- **Acceptance criteria:** Unsaved edits cannot be lost without confirmation; invalid slug displays actionable hint.
- **Automated test requirement:** Navigation blocker and validation tests.
- **Manual test requirement:** Close-window and route-navigation smoke test.
- **Desktop release blocker:** No.
- **Status:** Needs source verification.

### DP-UX-P1-004 — Accessibility interaction pass

- **Priority:** P1
- **Category:** Accessibility
- **Current behavior:** `UIUX_FIXES6.md` identifies missing focus movement/return, live-region semantics, support-nav active state, and cross-cutting keyboard checks.
- **Evidence:** `docs/UIUX_FIXES6.md`; table confirmation currently uses `role="status"` in inspected source.
- **Affected files:** lobby, table, settings, app frame, CSS, tests.
- **Required behavior:** Required confirmations receive appropriate focus and announcement; focus returns on dismissal; connection changes use polite live regions; required action confirmation uses alert/assertive semantics; all controls have visible focus.
- **Acceptance criteria:** Automated DOM semantics tests plus manual keyboard and screen-reader smoke tests.
- **Automated test requirement:** Frontend accessibility-focused assertions.
- **Manual test requirement:** Keyboard traversal and at least one screen reader.
- **Desktop release blocker:** No, unless critical controls are unreachable.
- **Status:** Partially implemented / needs full audit.

### DP-UX-P2-001 — Long hand-history rendering strategy

- **Priority:** P2
- **Category:** Performance / history UX
- **Current behavior:** Historical review indicates all hands render at once.
- **Evidence:** `docs/UIUX_FIXES6.md` P2.4.
- **Affected files:** hand history screen and tests.
- **Required behavior:** Render recent 100 by default with explicit expansion, or use virtualization.
- **Acceptance criteria:** More than 100 hands does not create an unbounded initial DOM list; user can access full history.
- **Automated test requirement:** Large-fixture frontend test.
- **Manual test requirement:** Long-history responsiveness smoke test.
- **Desktop release blocker:** No.
- **Status:** Needs source verification.

### DP-UX-P2-002 — Improve stale invite and error recovery messaging

- **Priority:** P2
- **Category:** Join/recovery UX
- **Current behavior:** Invite decode does not prove host availability; some error screens require navigation away to recheck.
- **Evidence:** `docs/UIUX_FIXES6.md` P2.9–P2.10.
- **Affected files:** join and error screens, backend reachability API if added.
- **Required behavior:** Explain that decoded invite availability is unconfirmed until join; provide a safe re-check for recoverable LAN-resolution errors.
- **Acceptance criteria:** Messaging does not label an unreachable host as valid/reachable; recheck updates state without stale scenario lock-in.
- **Automated test requirement:** Frontend tests.
- **Manual test requirement:** Unreachable/recovered host scenario.
- **Desktop release blocker:** No.
- **Status:** Not implemented / needs verification.

---

## Android interoperability items

### DP-INT-P1-001 — Align `ACTION_WINDOW_OPENED_EVENT` payload

- **Priority:** P1 after desktop release baseline
- **Category:** Android/Desktop protocol interoperability
- **Current behavior:** Desktop payload includes `actionWindowId`; audited Android model does not have shape identity.
- **Evidence:** `docs/ANDROID_INTEROP_AUDIT.md`.
- **Affected files:** desktop and Android protocol models/fixtures, compatibility tests.
- **Required behavior:** Define one version-1-compatible canonical shape or an explicitly versioned compatibility translation without weakening action-window integrity.
- **Acceptance criteria:** Shared fixture passes on both runtimes; live host/client directions process the event correctly.
- **Automated test requirement:** Cross-runtime fixture/serialization tests.
- **Manual test requirement:** Desktop host + Android client and reverse flow.
- **Desktop release blocker:** No.
- **Status:** Open; deferred until desktop baseline.

### DP-INT-P1-002 — Align `ACTION_REJECTED_EVENT` payload

- **Priority:** P1 after desktop release baseline
- **Category:** Android/Desktop protocol interoperability
- **Current behavior:** Android and desktop represent rejected action identity/reason differently.
- **Evidence:** `docs/ANDROID_INTEROP_AUDIT.md`.
- **Affected files:** protocol models, projector/adapter logic, fixtures/tests in both repositories.
- **Required behavior:** Define canonical rejected-action payload preserving message correlation and user-safe reason.
- **Acceptance criteria:** Both runtimes accept the same fixture and render/handle rejection truthfully.
- **Automated test requirement:** Shared fixture tests.
- **Manual test requirement:** Force rejected action in both host/client directions.
- **Desktop release blocker:** No.
- **Status:** Open; deferred until desktop baseline.

### DP-INT-P1-003 — Live mixed-runtime matrix

- **Priority:** P1 after payload alignment
- **Category:** Android/Desktop end-to-end interoperability
- **Current behavior:** Protocol was audited against Android commit `11666563e712b64004c46950966dd9b98230520f`; no live mixed-runtime proof is recorded.
- **Evidence:** `docs/ANDROID_INTEROP_AUDIT.md`.
- **Affected files:** both repositories if defects emerge.
- **Required behavior:** Desktop host/Android client and Android host/desktop client complete join, lobby, play, private cards, reconnect, elimination, and completion.
- **Acceptance criteria:** Full matrix passes against pinned commits/artifacts with matching final state and confidentiality.
- **Automated test requirement:** Expand fixture and adapter integration coverage.
- **Manual test requirement:** Device/emulator plus desktop runtime.
- **Desktop release blocker:** No.
- **Status:** Blocked by preceding items and environment.

### DP-INT-P2-001 — Define UniFFI-safe core adapter

- **Priority:** P2 / future milestone
- **Category:** Shared Rust core / Android architecture
- **Current behavior:** `poker-core` is platform-neutral and has an initial `PokerEngine` facade, but no Android binding crate exists.
- **Evidence:** `docs/ANDROID_ARCHITECTURE.md`; `crates/poker-core/src/facade.rs`.
- **Affected files:** future `crates/poker-android-ffi/`, bindings tests, Android repository.
- **Required behavior:** Separate UniFFI-safe DTO/command/snapshot adapter; Kotlin retains networking/session transport and Compose UI; no Tauri or networking enters `poker-core`.
- **Acceptance criteria:** Android can create/drive deterministic core state through generated bindings with parity tests.
- **Automated test requirement:** Rust adapter tests and Kotlin binding integration tests.
- **Manual test requirement:** Android runtime smoke test.
- **Desktop release blocker:** No.
- **Status:** Deferred.

---

## Deferred product features

These are intentionally outside the release-readiness milestone and must not displace blocker work:

- room-code discovery or matchmaking;
- internet/WAN hosting, relay, or cloud service;
- Tauri Mobile;
- moving networking into `poker-core`;
- sound effects or music;
- additional poker variants or multi-table tournaments;
- cloud accounts, synchronization, analytics, or telemetry;
- broad visual redesign unrelated to reproduced defects;
- Android implementation before desktop baseline and protocol mismatch resolution.

---

## Superseded/current-status map

| Document class | Current treatment |
|---|---|
| `DESKTOP_POKER_RELEASE_READINESS_BASELINE_SPEC.md` | Current milestone specification. |
| `DESKTOP_POKER_RELEASE_READINESS_BASELINE_TODO.md` | Current execution plan. |
| `DESKTOP_POKER_RELEASE_READINESS_REPORT.md` | Authoritative current evidence and outcome. |
| `DESKTOP_POKER_CURRENT_BACKLOG.md` | Authoritative current backlog. |
| `MANUAL_QA_CHECKLIST.md` | Current executable manual scenario checklist; boxes require a dated run against a recorded SHA. |
| `ANDROID_ARCHITECTURE.md` | Current intended Android architecture boundary. |
| `ANDROID_INTEROP_AUDIT.md` | Current audit baseline until re-audited against newer Android source or live evidence. |
| `UIUX_FIXES6.md` | Historical review input; checkbox state is not authoritative. Current gaps are consolidated here. |
| Earlier stabilization/fix specs and TODOs | Historical implementation and review evidence; not active backlogs. |
| Review/response files | Historical decision records; preserve exact paths when referenced. |
| `memory.md` | Chronological ledger only; not authoritative for current test status or open work. |

## Backlog maintenance rules

1. Reproduce a defect before assigning implementation work.
2. Give every defect a stable identifier and evidence.
3. Add a focused regression test where practical.
4. Do not weaken assertions, broaden accepted outcomes, hide errors, or add silent defaults to make a check pass.
5. Keep desktop adapter, frontend, and shared-core authority boundaries intact.
6. Mark manual work `BLOCKED` or `NOT RUN` rather than inferring success from automation.
7. Consolidate duplicates here instead of creating another competing current backlog.
