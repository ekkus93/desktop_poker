# Desktop Poker Release Readiness Baseline Specification

## Document status

- **Repository:** `ekkus93/desktop_poker`
- **Target branch:** `master`
- **Purpose:** Re-establish a trustworthy development baseline, prove the existing desktop multiplayer application in real runtime conditions, reconcile stale planning artifacts, and produce an evidence-based decision about the next product milestone.
- **Primary target:** Linux desktop release candidate for the current Tauri 2 application.
- **Secondary target:** Preserve the shared `poker-core` architecture and accurately document the remaining Android/Desktop interoperability work without starting Android implementation in this pass.

## 1. Background

Desktop Poker is already a substantial Linux-first Tauri 2 application. The repository contains:

- a React and TypeScript desktop frontend;
- a Rust Tauri adapter and backend;
- a shared, platform-neutral `crates/poker-core` crate;
- LAN TCP host and client runtimes;
- protocol signing, encryption, replay protection, sequencing, reconnect, and resynchronization;
- single-table Sit 'n Go No-Limit Texas Hold'em tournament logic;
- rule-based and LLM-backed NPC players;
- AI profile management;
- extensive Rust, frontend, integration, and browser geometry tests;
- CI and tagged Tauri release workflows.

The repository's final development sessions concentrated on test expansion, networking correctness, failure visibility, reconnect ordering, and removal of silent failure patterns. The project should therefore be treated as an advanced MVP that requires validation and consolidation, not as an unfinished prototype that needs a broad rewrite.

The central unresolved problem is evidence: the repository has strong automated coverage, but it does not contain a current, authoritative, completed record proving the full release binary through real multi-instance and multi-machine gameplay. Historical TODO files and `memory.md` are useful context, but they contain stale checkboxes, corrected claims, and summaries from different implementation points. They must not be used as substitutes for fresh validation.

## 2. Objective

This work must establish a reproducible and honest release-readiness baseline for the existing desktop application.

At completion, the repository must answer all of the following questions with current evidence:

1. Does a clean checkout pass the complete automated validation suite using the documented toolchain?
2. Can two release instances on one Linux computer host, join, play, reconnect, complete a tournament, and retain isolated local state?
3. Can two physical computers on the same LAN complete the same flow over real TCP?
4. Do important failure paths produce explicit user-visible or diagnostic errors rather than hangs, silent fallbacks, dishonest success responses, or stale UI state?
5. Are debug-only surfaces, browser mocks, probe code, test fixtures, and unsafe secret-storage behavior unreachable from production release builds?
6. Can a live NPC tournament run with the rule engine and with at least one configured LLM provider without blocking the game?
7. Which outstanding UI/UX, accessibility, interoperability, documentation, and packaging items are real current gaps?
8. Is the next milestone a desktop `0.1.0` release candidate, additional desktop stabilization, or Android/Desktop interoperability work?

## 3. Scope

### 3.1 Clean baseline validation

Run the repository from a clean checkout using the currently documented toolchains:

- Rust stable;
- Node.js 24.x;
- npm lockfile installation via `npm ci`;
- Linux packages required by Tauri 2 and WebKitGTK 4.1.

The implementation environment must record exact tool versions and the exact commit SHA tested.

The full automated baseline must include:

```bash
npm ci
npm run format:check
npm run lint
npm run test
npm run build
npm run test:geometry

cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features -- --test-threads=2
cargo test -p poker-core --all-targets --all-features
cargo tree -p poker-core
```

The test totals, ignored tests, warnings, elapsed result, and failures must be recorded from actual command output. Do not copy historical totals from `memory.md`.

### 3.2 Release build validation

Build and exercise the real release binary, not only `tauri dev` or browser mocks.

Required build commands:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --release
npm run tauri build -- --bundles deb
```

If the environment has working AppImage tooling, also run:

```bash
npm run tauri build -- --bundles appimage
```

If AppImage prerequisites are unavailable, record that fact accurately. Do not report AppImage success unless an AppImage was produced and launched.

At minimum, verify:

- release binary starts successfully;
- `.deb` bundle is produced and can be installed or inspected;
- release binary uses real Tauri IPC and real TCP networking;
- debug-only route and probe surfaces are inaccessible;
- production assets do not include browser mock or layout-probe payloads;
- per-instance command-line identity works;
- application data is stored under instance-scoped directories;
- API keys are not written to non-secret JSON configuration;
- release secret storage fails explicitly rather than falling back to plaintext.

### 3.3 Two local release instances

Use distinct instance identifiers with the compiled release binary:

```bash
./src-tauri/target/release/desktop-poker --instance-id host-a
./src-tauri/target/release/desktop-poker --instance-id client-b
```

The test must cover:

- host session creation;
- valid `pkr1_...` invite generation and copy behavior;
- client invite validation and join;
- seat claim and ready state propagation;
- tournament start;
- fold, check, call, bet, raise, and all-in paths when legally available;
- correct public board and pot synchronization;
- private hole-card isolation;
- hand history updates;
- player elimination and observer behavior;
- tournament completion and standings;
- normal leave and host shutdown behavior;
- restart persistence for the host instance;
- no state leakage into the client instance.

The run must use the release binary. A browser test, unit test, or `tauri dev` session does not satisfy this requirement.

### 3.4 Two physical machines on one LAN

Run the release application on two separate Linux machines connected to the same local network.

The test must cover:

- host advertises a reachable LAN address;
- client reaches TCP port `43818` through the invite;
- full tournament play to completion;
- identical public state and final standings;
- private hole cards remain recipient-specific;
- no reliance on loopback-only behavior;
- no manual state repair or debug-route intervention.

Record operating systems, commit SHA, binary hashes, machine roles, IP addresses with the last octet redacted if desired, and any firewall changes required.

### 3.5 Reconnect and failure-path matrix

The real runtime tests must cover at least these scenarios:

1. Client network interruption during the lobby, followed by recovery within the reconnect window.
2. Client network interruption during an open action window, followed by recovery within the reconnect window.
3. Client interruption beyond reconnect expiry.
4. Eliminated observer disconnect and reconnect.
5. Host process termination while a client is connected.
6. Invalid invite payload.
7. Validly decoded invite whose host is unavailable.
8. Two hosts attempting to bind the same port.
9. Required reconnect command-stream installation failure behavior, if it can be induced safely through a targeted test or test seam.
10. Stale or malformed sequence-bearing public event and snapshot behavior through existing automated tests.

For every failure path, verify the outcome is one of:

- explicit command error;
- structured runtime event;
- user-visible recovery state;
- protocol warning;
- diagnostic health counter and logged error;
- documented best-effort shutdown cleanup.

The following are prohibited:

- indefinite spinner;
- silent success after an authoritative mutation failed;
- silent fallback that invents protocol state;
- clearing ordering state because a required sequence is missing;
- showing a reconnect snapshot before action submission is functional;
- swallowing crypto, persistence, socket, lock, or required event errors;
- replacing a real failure with a default value merely to keep the UI moving.

### 3.6 NPC validation

Run a live tournament with rule-based NPCs and verify:

- NPCs are added atomically;
- runner startup failure cannot leave seated NPCs without a runner;
- legal decisions are submitted;
- missing or invalid acting-NPC hole cards produce an internal error and no invented action;
- NPCs do not block tournament progression;
- NPC state remains isolated per player;
- fallback decisions respect the configured profile style.

Run at least one live tournament with a configured LLM provider. A local provider such as Ollama or llama-server is acceptable and preferred for reproducibility.

Verify:

- provider configuration is loaded correctly;
- profile selection is honored;
- valid LLM JSON actions are accepted only when legal;
- malformed, timed out, failed, or illegal responses produce an explicit fallback reason;
- fallback policy respects the profile configuration;
- LLM failure never blocks the hand;
- API keys and secrets are not printed in logs or debug state;
- the debug inspector exposes the last fallback reason without exposing credentials.

Ignored integration tests requiring a local provider should be run when the required local service is available:

```bash
cargo test --workspace -- --ignored
```

Record which provider and model were used.

### 3.7 Production reachability and security audit

Run focused audits against the current source and built output.

Required checks include:

```bash
rg -n "__DESKTOP_POKER_BROWSER_MOCKS__|LayoutProbeApp" dist || true
rg -n "thread::spawn" src-tauri/src/networking/runtime/client.rs
rg -n "server_sequence.*unwrap_or_default|unwrap_or_default\(\).*server_sequence" src-tauri/src/networking/runtime/client.rs
rg -n "last_seen_server_sequence = envelope\.server_sequence" src-tauri/src/networking/runtime/client.rs
rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated" src-tauri/src/networking
rg -n "hole_cards_by_player_id.*unwrap_or\(|unwrap_or\(&\[\]\)" src-tauri/src/npc crates/poker-core/src || true
rg -n "let _ = sender\.send" src-tauri/src/networking/runtime/client.rs
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|thread::spawn|std::process::Command" crates/poker-core
```

Every hit must be inspected rather than judged only by grep count. Expected false positives, helper implementations, test-only code, and best-effort cleanup must be documented precisely.

Also verify:

- Tauri CSP remains restrictive;
- frontend external network traffic is blocked and external requests remain Rust-owned;
- release debug tools are disabled;
- release secret storage uses the platform keychain;
- no API key appears in `llm-provider.json`, logs, debug output, snapshots, or committed fixtures;
- cryptographic associated data never silently becomes an empty byte sequence;
- observer projections cannot contain private cards or action authority.

### 3.8 Backlog reconciliation

Review current code against the following existing repository sources:

- `README.md`;
- `memory.md`;
- `docs/MANUAL_QA_CHECKLIST.md`;
- `docs/UIUX_FIXES6.md`;
- `docs/ANDROID_INTEROP_AUDIT.md`;
- `docs/ANDROID_ARCHITECTURE.md`;
- current stabilization, runtime-hardening, integration-test, and real-multiplayer TODO files under `docs/`.

Do not treat a checked or unchecked historical checkbox as proof. Verify current behavior in source, tests, or manual evidence.

Create an authoritative current backlog at:

- `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`

Each item must include:

- stable identifier;
- priority: P0, P1, P2, or deferred;
- category;
- exact current behavior;
- evidence;
- affected files;
- acceptance criteria;
- whether it blocks the desktop release candidate;
- whether it belongs to desktop, Android interoperability, or later product work.

At minimum, reconcile:

- unfinished P1/P2 UI and accessibility work;
- optimistic lobby ready-state behavior;
- settings and profile editor navigation and confirmation behavior;
- unsaved profile edits;
- profile slug validation;
- saved-history/live-history distinction;
- hand-history scaling;
- focus and ARIA behavior;
- stale invite messaging;
- Android payload shape mismatches;
- live Android/Desktop interop status;
- room discovery and matchmaking as deferred product features;
- sound as optional and off by default;
- packaging limitations.

Historical duplicate TODO documents should not be deleted unless their historical role is preserved. Prefer adding a clear header that points to the new authoritative backlog when a document is superseded.

### 3.9 Failure-driven fixes

Any defect discovered by validation must be handled using this sequence:

1. Record exact reproduction steps and observed behavior.
2. Classify severity and release impact.
3. Add or strengthen an automated regression test where technically practical.
4. Implement the smallest correct fix.
5. Preserve architecture boundaries.
6. Run focused tests.
7. Run the complete validation suite.
8. Re-run the relevant manual scenario.
9. Record final evidence.

Do not perform broad speculative refactors while a concrete release blocker is being repaired.

Do not weaken assertions, expand timing windows without evidence, accept multiple contradictory outcomes, replace errors with defaults, or add fallback behavior merely to make tests pass.

### 3.10 Release-readiness report

Create:

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

The report must include:

- tested commit SHA;
- environment and tool versions;
- automated command results and actual test totals;
- release artifacts produced;
- local two-instance QA results;
- two-machine LAN QA results;
- reconnect/failure matrix;
- rule-based NPC result;
- LLM NPC result;
- production reachability/security audit;
- outstanding release blockers;
- non-blocking backlog;
- Android interoperability status;
- final recommendation.

Use explicit result labels:

- `PASS` — requirement was executed and met;
- `FAIL` — requirement was executed and did not meet acceptance criteria;
- `BLOCKED` — requirement could not be executed because a concrete dependency or environment was unavailable;
- `NOT RUN` — requirement was not attempted.

Do not use `PASS` for reviewed code, historical claims, or automated substitutes for manual runtime work.

## 4. Release decision gates

### Gate A: Automated baseline

Pass only when all required formatting, linting, test, dependency audit, browser geometry, and frontend build commands succeed from a clean checkout.

### Gate B: Release binary

Pass only when the real release binary launches and required production reachability/security checks succeed.

### Gate C: Local multiplayer

Pass only when two local release instances complete a tournament and instance isolation is proven.

### Gate D: LAN multiplayer

Pass only when two physical machines complete a tournament over the LAN.

### Gate E: Recovery

Pass only when required disconnect, reconnect, expiry, host-loss, invalid-invite, unreachable-host, and port-conflict behavior is explicit and recoverable or terminal as designed.

### Gate F: NPC operation

Pass only when rule-based NPCs work in a live tournament and at least one LLM-backed session is either successfully completed or explicitly marked `BLOCKED` with a reproducible environment reason. An LLM provider outage may not block rule-based desktop release behavior.

### Gate G: Release blocker closure

Pass only when no open P0 or desktop-release-blocking P1 item remains in `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`.

## 5. Final milestone decision

The final report must choose one outcome.

### Outcome 1: Desktop release candidate

Choose this when Gates A through G pass, except an externally blocked optional LLM-provider run may be documented without blocking core desktop release behavior.

Required next action: prepare and tag the desktop release using the existing GitHub Actions release workflow.

### Outcome 2: Additional desktop stabilization

Choose this when one or more reproducible desktop release blockers remain.

Required next action: create a focused fix specification and TODO derived only from the recorded failures.

### Outcome 3: Android/Desktop interoperability milestone

Choose this only after the desktop release baseline is trustworthy and no desktop release blocker remains.

The next interoperability milestone must begin by resolving the two currently documented payload-shape mismatches and defining the UniFFI-safe `poker-core` boundary. It must not move TCP networking into `poker-core` and must not use Tauri Mobile unless a new architecture decision explicitly supersedes `docs/ANDROID_ARCHITECTURE.md`.

## 6. Non-goals

This pass must not:

- implement the Android application;
- add Tauri Mobile;
- add a UniFFI crate unless the final release decision explicitly starts a separate follow-up milestone;
- move networking, keychain, LLM, filesystem, or Tauri dependencies into `poker-core`;
- add internet matchmaking, cloud accounts, centralized servers, or room-code discovery;
- redesign the entire frontend;
- rewrite the poker engine;
- add sound as a release requirement;
- change protocol version solely to simplify validation;
- hide defects with compatibility fallbacks;
- mark manual QA complete based on unit or integration tests.

## 7. Engineering constraints

### 7.1 Authority boundaries

- `poker-core` owns deterministic poker rules, tournament transitions, legal-action validation, dealing, settlement, and projections.
- `src-tauri` owns desktop transport, protocol, crypto, persistence, secret storage, NPC integration, and Tauri commands.
- React and TypeScript own rendering, routing, forms, dialogs, and local presentation state.
- The frontend must never become authoritative for game state.

### 7.2 Failure handling

- Required mutations must return honest success or error results.
- Required event delivery failures must be surfaced unless the receiver is intentionally gone during shutdown and the path is explicitly documented as best-effort.
- Required lock, stream, crypto, sequence, persistence, and configuration errors must not be converted into defaults.
- Any fallback must be intentional, typed, observable, and safe.

### 7.3 Testing integrity

- Tests must force the intended branch.
- Tests must not accept both success and failure unless both are genuinely valid protocol outcomes and the assertion distinguishes them meaningfully.
- Timing changes require evidence of scheduler or environment constraints.
- Manual runs require actual release binaries.

### 7.4 Documentation integrity

- Every claim in the final report must identify its evidence.
- Every referenced newly created repository document must be committed at the exact path named in this specification.
- Historical reports must not be overwritten in a way that destroys their audit value.
- `memory.md` may receive a concise completion ledger entry, but it is not the authoritative release report.

## 8. Definition of done

This milestone is complete only when:

1. Both this specification and its corresponding TODO are committed under `docs/`.
2. A clean automated baseline has been executed and recorded.
3. Release artifacts have been produced and inspected.
4. Two local release instances have completed the required QA flow.
5. Two physical LAN machines have completed the required QA flow, or the result is explicitly `BLOCKED` with a concrete environment limitation and the project is not called release-proven.
6. The reconnect and failure-path matrix has current evidence.
7. Rule-based NPC live QA is complete.
8. LLM NPC live QA is complete or explicitly blocked without affecting rule-based operation.
9. Production reachability and security audits pass.
10. `docs/DESKTOP_POKER_CURRENT_BACKLOG.md` exists and is authoritative.
11. `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md` exists and contains honest PASS/FAIL/BLOCKED/NOT RUN results.
12. No desktop-release-blocking P0 or P1 issue remains unaddressed.
13. The report selects one final milestone outcome and explains why.
