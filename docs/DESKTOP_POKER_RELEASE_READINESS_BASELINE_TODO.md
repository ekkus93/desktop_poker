# Desktop Poker Release Readiness Baseline TODO

## Purpose

Implement and verify `docs/DESKTOP_POKER_RELEASE_READINESS_BASELINE_SPEC.md`.

This TODO is an execution plan for re-establishing a trustworthy baseline, proving the current Linux desktop multiplayer application in real runtime conditions, fixing only defects exposed by evidence, reconciling stale planning artifacts, and making an explicit next-milestone decision.

The existing application is an advanced MVP. Do not begin by rewriting the architecture or adding new product features.


## Execution status — 2026-07-26

- Automated validation: **PASS** against product-source SHA `fd8369ba7267fe76a827cdf48384c9f826159719` and evidence-generating pull-request merge SHA `c79c9f2473f92310c3d65afe8b834f97b2875c5d`.
- Final read-only branch validation: **PASS** at commit `283607db670d0bd28a342ac5a417806bc3507d78` in GitHub Actions run `30225613175`.
- Release binary and Debian package build/inspection: **PASS**.
- Graphical binary/package launch, local multi-instance play, physical LAN play, reconnect interruption, release keychain, and live provider tests: **BLOCKED** by unavailable runtime dependencies.
- Evidence: `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`.

Unchecked manual-runtime boxes below are intentionally retained; they are not inferred from automated coverage.

## Mandatory operating rules

- Work from the repository root unless a task explicitly says otherwise.
- Record the exact commit SHA tested.
- Use Node.js 24.x and Rust stable.
- Run validation from a clean checkout or clean working tree.
- Do not use historical test totals as current evidence.
- Do not mark manual QA complete from unit, integration, browser, or mocked tests.
- Do not hide failures with defaults, widened assertions, arbitrary sleeps, silent fallbacks, or broad error suppression.
- Do not add Android, Tauri Mobile, matchmaking, cloud services, sound, or unrelated features.
- Keep `poker-core` platform-neutral.
- Preserve every referenced deliverable at the exact repository path named below.
- Update checkboxes only after verifying the current code or executing the task.
- Commit coherent groups of changes separately. Do not combine unrelated defect fixes into one opaque commit.

## Required deliverables

- [x] `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`
- [x] `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- [x] Current automated validation evidence in the release-readiness report
- [x] Current manual local multi-instance evidence in the release-readiness report
- [x] Current two-machine LAN evidence in the release-readiness report
- [x] Current NPC and LLM evidence in the release-readiness report
- [x] Updated `README.md` status and release instructions when results require changes
- [x] A concise final ledger entry in `memory.md`

---

# P0 — Establish the authoritative baseline

## P0.1 — Capture repository and toolchain state

**Files:**

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Confirm the current branch is `master` or record the actual branch being tested.
- [ ] Confirm the working tree is clean before baseline validation.
- [ ] Record the exact commit SHA:

```bash
git rev-parse HEAD
git status --short
git branch --show-current
```

- [ ] Record operating system information:

```bash
uname -a
cat /etc/os-release
```

- [ ] Record tool versions:

```bash
node --version
npm --version
rustc --version
cargo --version
rustup show active-toolchain
```

- [ ] Verify Node.js reports a 24.x version.
- [ ] Verify Rust stable is active.
- [ ] Record whether the environment is a local workstation, VM, container, or CI runner.
- [ ] Record whether a graphical desktop session is available.
- [ ] Record whether two physical LAN machines are available.
- [ ] Record whether Ollama or llama-server is available for live LLM testing.

### Acceptance

- [x] The report contains the exact tested SHA and environment information.
- [x] No later test result is attributed to a different unrecorded commit.
- [x] Any commit change during defect repair is recorded before the final validation rerun.

---

## P0.2 — Install and verify Linux build dependencies

**Files:**

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Install or confirm the Tauri 2 Linux dependencies:

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libwebkit2gtk-4.1-dev
```

- [ ] Record any distribution-specific package substitutions.
- [ ] Confirm `pkg-config` can resolve required GTK/WebKit libraries if a build fails.
- [ ] Do not change source code to compensate for a missing system package.

### Acceptance

- [x] Required native dependencies are available.
- [x] Environment-only failures are documented separately from product defects.

---

## P0.3 — Perform a clean dependency installation

**Files:**

- `package-lock.json` only if npm legitimately updates it for a required repair
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Remove untracked build output without deleting user data or source files:

```bash
git clean -ndX
```

- [ ] Review the dry-run output before any cleanup.
- [ ] Remove only safe ignored build artifacts if necessary.
- [ ] Run:

```bash
npm ci
```

- [ ] Record install success or the exact failure.
- [ ] Run:

```bash
npm audit --omit=dev
npm audit
```

- [ ] Record production and full dependency audit results.
- [ ] Do not run a forced audit fix automatically.
- [ ] If vulnerabilities are reported, classify whether they are reachable in the packaged desktop application before changing dependencies.

### Acceptance

- [x] `npm ci` succeeds using the committed lockfile.
- [x] Dependency audit results are recorded honestly.
- [x] No lockfile churn is introduced without a documented reason.

---

## P0.4 — Run the complete frontend validation suite

**Files:**

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- Source and test files only if a real failure is discovered

### Tasks

Run each command separately and preserve the actual output summary.

- [ ] Formatting:

```bash
npm run format:check
```

- [ ] Lint:

```bash
npm run lint
```

- [ ] Vitest:

```bash
npm run test
```

- [ ] Record:
  - [ ] number of test files;
  - [ ] number of tests passed;
  - [ ] number of tests failed;
  - [ ] skipped or todo tests;
  - [ ] warnings or stderr noise.

- [ ] Production frontend build:

```bash
npm run build
```

- [ ] Browser geometry tests:

```bash
npm run test:geometry
```

- [ ] If local Playwright browser dependencies are unavailable, install the matching browser version or use the repository's pinned Playwright container.
- [ ] Do not mark geometry tests passed based only on CI workflow configuration.
- [ ] Inspect test output for hidden persistence errors, unhandled promise rejections, React warnings, and test teardown leakage.
- [ ] Specifically check for window-persistence noise:

```bash
npm run test 2>&1 | tee /tmp/desktop-poker-vitest.log
rg -n "window-state|Could not initialize|unhandled|act\(|Warning:" /tmp/desktop-poker-vitest.log
```

### Acceptance

- [x] Formatting passes.
- [x] Lint passes with zero warnings.
- [x] All non-ignored frontend tests pass.
- [x] Production frontend build passes.
- [x] Geometry tests pass or are explicitly `BLOCKED` with an environment reason.
- [x] Expected stderr is documented; unexplained recurring errors are treated as defects.

---

## P0.5 — Run the complete Rust validation suite

**Files:**

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- Source and test files only if a real failure is discovered

### Tasks

- [ ] Formatting:

```bash
cargo fmt --check
```

- [ ] Clippy:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

- [ ] Full workspace tests with CI-equivalent thread limit:

```bash
cargo test --workspace --all-targets --all-features -- --test-threads=2
```

- [ ] Focused shared-core tests:

```bash
cargo test -p poker-core --all-targets --all-features
```

- [ ] Dependency tree:

```bash
cargo tree -p poker-core
```

- [ ] Record:
  - [ ] test totals by crate where visible;
  - [ ] ignored tests;
  - [ ] doctest results;
  - [ ] warnings;
  - [ ] failures;
  - [ ] any test that accepts multiple contradictory outcomes.

- [ ] Inspect ignored tests:

```bash
cargo test --workspace --all-targets --all-features -- --ignored --list
```

- [ ] For each ignored test, record:
  - [ ] exact test name;
  - [ ] why it is ignored;
  - [ ] whether it can run in the current environment;
  - [ ] whether the missing dependency is configuration, hardware, model availability, or test debt.

- [ ] Do not claim ignored LLM tests passed until they are run against a configured provider.

### Acceptance

- [x] Rust formatting passes.
- [x] Clippy passes with warnings denied.
- [x] All non-ignored workspace tests pass.
- [x] Focused `poker-core` tests pass.
- [x] The actual totals are copied from current output, not `memory.md`.

---

## P0.6 — Audit `poker-core` platform neutrality

**Files:**

- `crates/poker-core/**`
- `Cargo.toml`
- `Cargo.lock`
- `docs/ANDROID_ARCHITECTURE.md`
- `docs/ANDROID_INTEROP_AUDIT.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Run the focused source audit:

```bash
rg -n \
  "tauri|keyring|reqwest|local_ip|std::net|TcpListener|TcpStream|UdpSocket|thread::spawn|std::process::Command" \
  crates/poker-core Cargo.toml Cargo.lock
```

- [ ] Inspect every hit.
- [ ] Distinguish `EngineCommand` false positives from process-spawning `Command` usage.
- [ ] Verify `crates/poker-core/Cargo.toml` contains only platform-neutral dependencies.
- [ ] Verify no feature flag conditionally pulls in Tauri, Android, networking, keychain, filesystem path discovery, or LLM dependencies.
- [ ] Verify deterministic state and command APIs remain available without the desktop crate.

### Acceptance

- [x] `poker-core` remains reusable by future Android bindings.
- [x] No platform dependency is introduced to make desktop validation easier.
- [x] Every grep hit is explained in the report.

---

# P0 — Build and inspect production artifacts

## P0.7 — Build the release binary

**Files:**

- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- `package.json`
- `README.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Build:

```bash
npm run tauri build
```

- [ ] Confirm this file exists and is executable:

```text
target/release/desktop-poker
```

- [ ] Record binary size and SHA-256:

```bash
ls -lh target/release/desktop-poker
sha256sum target/release/desktop-poker
```

- [ ] Launch the direct release binary:

```bash
./target/release/desktop-poker --instance-id release-smoke
```

- [ ] Verify:
  - [ ] Home renders.
  - [ ] Host screen renders.
  - [ ] Join screen renders.
  - [ ] Settings screen renders.
  - [ ] Help screen renders.
  - [ ] `/debug` is not reachable.
  - [ ] Browser mocks are not active.
  - [ ] No unexpected console or terminal error appears.

### Acceptance

- [ ] Release binary builds successfully.
- [ ] Release binary launches successfully.
- [x] The report contains its size and hash.
- [ ] No debug-only route or browser mock is reachable.

---

## P0.8 — Build and inspect the Debian package

**Files:**

- `src-tauri/tauri.conf.json`
- `README.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Build:

```bash
npm run tauri build -- --bundles deb
```

- [ ] Locate the generated `.deb`.
- [ ] Record package filename, size, and SHA-256.
- [ ] Inspect metadata:

```bash
dpkg-deb --info target/release/bundle/deb/*.deb
dpkg-deb --contents target/release/bundle/deb/*.deb
```

- [ ] Verify application identifier, version, binary path, desktop entry, and icons.
- [ ] Install in a disposable or otherwise safe Linux environment.
- [ ] Launch the installed application.
- [ ] Confirm installation does not overwrite another instance's application data unexpectedly.

### Acceptance

- [x] `.deb` packaging succeeds.
- [x] Package metadata is coherent.
- [x] Installed application launches or installation is explicitly blocked and documented.

---

## P0.9 — Build AppImage when tooling is available

**Files:**

- `src-tauri/tauri.conf.json`
- `README.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] If required, obtain or install:
  - [ ] `linuxdeploy`
  - [ ] `appimagetool`
- [ ] Build:

```bash
npm run tauri build -- --bundles appimage
```

- [ ] Record:
  - [ ] generated filename;
  - [ ] size;
  - [ ] SHA-256;
  - [ ] tool versions;
  - [ ] success, failure, or blocker reason.
- [ ] If an AppImage is produced, launch it and verify the same smoke surfaces as P0.7.

### Acceptance

- [x] AppImage result is recorded as PASS, FAIL, or BLOCKED.
- [x] No claim is made that AppImage works unless it was produced and launched.

---

# P0 — Production reachability and secret-safety audit

## P0.10 — Prove debug and browser-mock isolation

**Files:**

- `src/main.tsx`
- `src/probe/**`
- `src/api/desktop.ts`
- `src/app/AppShell.tsx`
- `src/components/debug/**`
- `src-tauri/tauri.conf.json`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Audit direct imports and environment guards:

```bash
rg -n \
  "LayoutProbe|DebugPanel|__DESKTOP_POKER_BROWSER_MOCKS__|import\.meta\.env\.(DEV|MODE)|/debug" \
  src vite.config.ts src-tauri/tauri.conf.json
```

- [ ] Inspect production `dist/` for debug strings and chunks:

```bash
rg -n \
  "LayoutProbe|DebugPanel|__DESKTOP_POKER_BROWSER_MOCKS__|Mock Table|Mock Hand" \
  dist
```

- [ ] Verify release navigation does not expose `/debug`.
- [ ] Verify browser mocks cannot replace desktop IPC in release mode.
- [ ] Verify production CSP does not allow arbitrary remote frontend connections.

### Acceptance

- [ ] No debug surface is reachable in a release build.
- [ ] Browser mocks are absent from the production path.
- [ ] Production CSP remains restrictive.
- [ ] Any intentionally retained debug string is documented and unreachable.

---

## P0.11 — Prove provider-secret safety

**Files:**

- `src-tauri/src/npc/provider_storage.rs`
- `src-tauri/src/npc/provider.rs`
- `src-tauri/src/npc/tests.rs`
- `src/api/desktop.ts`
- `src/components/debug/DebugPanel.tsx`
- `src/screens/DeviceSettingsScreen.tsx`
- `README.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Verify release builds select `KeychainSecretStore`.
- [ ] Verify debug builds may use the explicitly documented local secret file.
- [ ] Verify provider settings JSON excludes the API key.
- [ ] Verify `Debug` formatting redacts the API key.
- [ ] Verify debug inspector state contains no key.
- [ ] Search for secret serialization and logging risk:

```bash
rg -n \
  "apiKey|api_key|Authorization|Bearer|claude-api-key|llm-provider-key" \
  src src-tauri README.md docs
```

- [ ] Inspect every hit.
- [ ] Configure a low-privilege test key in a release build.
- [ ] Restart and verify the key is still available through the OS keychain.
- [ ] Inspect the application data directory:

```bash
rg -n "<TEST_KEY_PREFIX>|apiKey|api_key" "<APP_DATA_DIR>"
```

- [ ] Inspect logs and captured output for the key prefix.
- [ ] Clear provider configuration and verify the key is removed.
- [ ] Force keychain unavailability or write failure and verify:
  - [ ] explicit visible failure;
  - [ ] no settings-only success;
  - [ ] no plaintext fallback;
  - [ ] no loss of the prior valid secret/configuration.

### Acceptance

- [x] No release plaintext fallback exists.
- [ ] No API key appears in JSON, logs, snapshots, debug output, or committed files.
- [x] Secret-storage failure is explicit.
- [ ] Clear removes the secret or reports an actionable failure.

---

# P0 — Two-local-instance runtime QA

## P0.12 — Verify per-instance storage isolation before multiplayer QA

**Files:**

- `src-tauri/src/instance.rs`
- `src-tauri/src/app_state/**`
- `src/app/**`
- `README.md`
- `docs/MANUAL_QA_CHECKLIST.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Launch two release instances with unique ids:

```bash
./target/release/desktop-poker --instance-id host-a
./target/release/desktop-poker --instance-id client-b
```

- [ ] Set different display names.
- [ ] Set different host drafts.
- [ ] Create different recent-join lists.
- [ ] Create different saved hand histories.
- [ ] Restart both.
- [ ] Verify values remain isolated.
- [ ] Move and resize both windows.
- [ ] Restart and verify each restores its own geometry.

### Acceptance

- [ ] No display-name leakage.
- [ ] No draft leakage.
- [ ] No recent-join leakage.
- [ ] No hand-history leakage.
- [ ] No window-state leakage.

---

## P0.13 — Complete a real two-instance tournament

**Files:**

- `docs/MANUAL_QA_CHECKLIST.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- Source/tests only if a real defect is found

### Tasks

Follow `docs/MANUAL_QA_CHECKLIST.md` Checklist A completely.

At minimum:

- [ ] Launch host and client with distinct instance ids.
- [ ] Host a direct LAN table.
- [ ] Share the real current `pkr1_...` invite.
- [ ] Join from the second release instance.
- [ ] Confirm both names appear.
- [ ] Claim different seats.
- [ ] Toggle ready on both.
- [ ] Start only after readiness is confirmed.
- [ ] Verify private hole cards remain private.
- [ ] Verify public board/pot/action state matches.
- [ ] Exercise:
  - [ ] fold;
  - [ ] check;
  - [ ] call;
  - [ ] bet;
  - [ ] raise;
  - [ ] all-in when legal.
- [ ] Verify illegal actions are rejected visibly.
- [ ] Continue through elimination.
- [ ] Verify eliminated players observe but cannot act.
- [ ] Complete the tournament.
- [ ] Verify matching final standings.
- [ ] Verify local hand-history persistence on restart.

### Acceptance

- [ ] A complete tournament is recorded as PASS.
- [ ] No private information leaks.
- [ ] No public state divergence occurs.
- [ ] No client is allowed to apply authoritative state locally.
- [ ] Persistence remains isolated per instance.

---

# P0 — Two-machine LAN QA

## P0.14 — Prove real physical-LAN transport

**Files:**

- `docs/MANUAL_QA_CHECKLIST.md`
- `README.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- Source/tests only if a real defect is found

### Tasks

Use two physical machines on the same LAN.

- [ ] Use matching release artifacts.
- [ ] Confirm both machines have routable private LAN addresses.
- [ ] Confirm TCP port `43818` is permitted through the host firewall.
- [ ] Host on machine A.
- [ ] Verify invite contains machine A's real LAN address, not loopback.
- [ ] Join from machine B.
- [ ] Verify exact table/session identity.
- [ ] Complete a full tournament.
- [ ] Verify private-state isolation.
- [ ] Verify synchronized completion.
- [ ] Repeat with at least one restart/reconnect event.
- [ ] Record firewall and OS details.

### Acceptance

- [ ] No loopback address is used.
- [ ] No browser mocks are used.
- [ ] No SSH tunnel or port-forwarding substitutes for LAN transport.
- [ ] Full tournament completes on two physical machines.
- [ ] Firewall requirements are documented clearly.

---

# P0 — Reconnect and failure-mode QA

## P0.15 — Lobby reconnect

- [ ] Connect host and client in the lobby.
- [ ] Interrupt the client process or network.
- [ ] Reconnect within the configured window.
- [ ] Verify the same participant identity, seat, ready state, and reconnect token are restored.
- [ ] Verify no duplicate participant is created.

## P0.16 — Active-hand reconnect

- [ ] Start a hand.
- [ ] Disconnect the active or non-active client.
- [ ] Reconnect within the window.
- [ ] Verify a fresh snapshot is applied.
- [ ] Verify private hole cards are restored only to the correct player.
- [ ] Verify the command stream works after snapshot installation.
- [ ] Submit a legal action.
- [ ] Verify stale/duplicate action replay does not mutate state twice.

## P0.17 — Reconnect expiry

- [ ] Disconnect a participant.
- [ ] Wait beyond the configured reconnect expiry.
- [ ] Attempt reconnect.
- [ ] Verify explicit failure.
- [ ] Verify no duplicate participant or ghost seat is created.

## P0.18 — Eliminated-observer reconnect

- [ ] Eliminate a player.
- [ ] Confirm observe-only state.
- [ ] Disconnect and reconnect within the window.
- [ ] Verify observe-only state remains.
- [ ] Verify private hole cards remain absent.
- [ ] Verify action submission remains disabled.
- [ ] Verify tournament-complete transition remains coherent.

## P0.19 — Host loss and terminal failure

- [ ] Terminate the host during lobby and active play.
- [ ] Verify the client does not remain in a playable-looking state.
- [ ] Verify explicit reconnecting/error UI.
- [ ] Verify terminal failure stops further action submission.
- [ ] Verify recovery guidance is actionable.

## P0.20 — Port conflict, invalid invite, and unreachable host

- [ ] Start one host on `43818`.
- [ ] Attempt a second host on the same address/port.
- [ ] Verify explicit bind failure.
- [ ] Paste malformed invite data.
- [ ] Verify parse/validation error.
- [ ] Use a valid invite to an unavailable host.
- [ ] Verify explicit connection failure.
- [ ] Verify the UI never reports success for any of these cases.

### Acceptance for P0.15–P0.20

- [ ] Every scenario has PASS, FAIL, or BLOCKED status in the report.
- [ ] No silent reconnect fallback exists.
- [ ] No stale command is applied after reconnect.
- [ ] Reconnect snapshot is exposed only after the command stream is usable.
- [ ] Fatal transport failure does not leave a playable-looking UI.

---

# P1 — Live NPC QA

## P1.1 — Rule-based NPC tournament

**Files:**

- `src-tauri/src/npc/**`
- `src-tauri/src/app_state/host_session.rs`
- `src-tauri/src/networking/runtime/**`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Create or load a valid rule-based NPC profile.
- [ ] Host a tournament with one human and at least one NPC, or at least two NPCs.
- [ ] Start only after all required ready conditions are satisfied.
- [ ] Play multiple hands.
- [ ] Verify NPC actions are legal.
- [ ] Verify profile identity, style, and display name remain associated correctly.
- [ ] Verify no seated NPC exists without a live runner.
- [ ] Verify a runner failure is visible.
- [ ] Continue through elimination and completion.
- [ ] Verify hand history and final standings.

### Acceptance

- [ ] Live rule-based tournament completes.
- [ ] NPC actions are legal and synchronized.
- [ ] Registration remains atomic.
- [ ] No NPC private state leaks.
- [ ] Any failure is visible and actionable.

---

## P1.2 — Live local LLM provider

**Files:**

- `src-tauri/src/npc/llm_strategy/**`
- `src-tauri/src/npc/provider.rs`
- `src/screens/DeviceSettingsScreen.tsx`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Start Ollama or llama-server with a supported model.
- [ ] Configure endpoint and model in Settings.
- [ ] Test connection.
- [ ] Run ignored local-provider tests explicitly.
- [ ] Start an LLM-backed NPC tournament.
- [ ] Verify legal action parsing.
- [ ] Verify malformed response fallback is visible in debug/provider state.
- [ ] Verify timeout behavior.
- [ ] Verify provider failure does not corrupt tournament state.
- [ ] Verify action-window closure cancels or rejects stale LLM output.

### Acceptance

- [ ] Ignored local-provider tests pass against a live endpoint.
- [ ] Live LLM NPC acts legally.
- [ ] Timeout/malformed-response fallback is deterministic and visible.
- [ ] No stale LLM action applies after the window closes.

---

## P1.3 — Live remote LLM provider with a test key

- [ ] Configure a low-privilege Anthropic or OpenAI-compatible test key.
- [ ] Verify OS-keychain persistence.
- [ ] Test connection.
- [ ] Run at least one LLM-backed decision.
- [ ] Verify redaction in logs/debug/snapshots.
- [ ] Clear provider configuration.
- [ ] Verify secret removal.

### Acceptance

- [ ] Remote provider path works with a live test key.
- [ ] No key appears in plaintext.
- [ ] Clear removes the key or reports explicit failure.

---

# P1 — Cross-platform readiness

## P1.4 — Validate canonical fixtures against Android when available

**Files:**

- `src-tauri/src/protocol/test_support.rs`
- `src-tauri/src/protocol/models.rs`
- `docs/ANDROID_INTEROP_AUDIT.md`
- Android repository or captured fixture artifacts when available

### Tasks

- [ ] Obtain the real Android CanonicalJson output referenced by the ignored fixture test.
- [ ] Run:

```bash
cargo test --workspace --all-targets --all-features -- --ignored
```

- [ ] Resolve any canonical-byte mismatch.
- [ ] Convert the fixture test from ignored to normal once stable.
- [ ] Verify action-window-opened and action-rejected payload-shape differences are either fixed or documented as explicit deferred blockers.

### Acceptance

- [ ] Android canonical fixture test passes.
- [ ] No undocumented payload-shape difference remains.
- [ ] `poker-core` public API remains suitable for UniFFI binding.

---

# P1 — Documentation reconciliation

## P1.5 — Reconcile README

Update the README with current truth:

- [ ] Current test totals.
- [ ] Exact ignored tests.
- [ ] Current manual-QA status.
- [ ] Current package status.
- [ ] Current secret-storage status.
- [ ] Current LAN requirements.
- [ ] Current limitations.
- [ ] Release-artifact launch instructions.
- [ ] Per-instance launch instructions.
- [ ] Decision on the next milestone.

## P1.6 — Create the authoritative current backlog

- [ ] Add stable backlog IDs.
- [ ] Include priority.
- [ ] Include affected files.
- [ ] Include current behavior.
- [ ] Include required behavior.
- [ ] Include acceptance criteria.
- [ ] Include automated-test requirement.
- [ ] Include manual-test requirement.
- [ ] Include release-blocker status.
- [ ] Reconcile unchecked items from historical TODO files.
- [ ] Reconcile `docs/UIUX_FIXES6.md`.
- [ ] Reconcile Android/Desktop interoperability items.
- [ ] Distinguish:
  - [ ] desktop release blockers;
  - [ ] Android integration blockers;
  - [ ] deferred features;
  - [ ] speculative ideas.

## P1.7 — Update the ledger

Append a concise final entry to `memory.md` containing:

- [ ] tested commit SHA;
- [ ] test totals;
- [ ] ignored tests;
- [ ] release artifact results;
- [ ] manual QA results;
- [ ] open release blockers;
- [ ] next milestone decision.

---

# P2 — Optional follow-up improvements

These are not automatic release blockers unless evidence demonstrates otherwise.

## P2.1 — Improve hand-history scalability

- [ ] Add pagination or virtualization for very long histories if profiling proves it necessary.

## P2.2 — Improve accessibility

- [ ] Complete focus return and live-region behavior for confirmations and connection changes.
- [ ] Perform manual keyboard and screen-reader QA.

## P2.3 — Improve UI honesty

- [ ] Remove optimistic ready-state display if still present.
- [ ] Make host reachability explicit if invite preview still says `Lobby ready` without a live check.
- [ ] Clarify saved-history fallback behavior if corrupt records remain silently hidden.

## P2.4 — Start Android/UniFFI only after the gate is met

- [ ] Begin UniFFI scaffolding only after the final milestone decision explicitly approves Android integration.
- [ ] Keep Kotlin in control of networking/session transport.
- [ ] Keep Rust in control of deterministic poker rules/state/projection.

---

# Final validation gate

Run everything again after all fixes and documentation changes:

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

Record final:

- [ ] branch;
- [ ] commit SHA;
- [ ] operating system;
- [ ] Node/npm/Rust/Cargo versions;
- [ ] frontend totals;
- [ ] Rust totals;
- [ ] ignored tests;
- [ ] package names, sizes, and hashes;
- [ ] manual local multi-instance result;
- [ ] manual two-machine LAN result;
- [ ] reconnect matrix result;
- [ ] rule-based NPC result;
- [ ] live LLM result;
- [ ] secret-storage result;
- [ ] remaining blockers;
- [ ] final milestone decision.

## Final decision

Choose exactly one:

- [ ] **Linux desktop release candidate**
- [x] **Additional desktop stabilization / validation**
- [ ] **Begin Android/UniFFI milestone**

Do not select **Linux desktop release candidate** if release artifacts, real multiplayer, reconnect, package launch, secret storage, or failure behavior remain unproven.

Do not select **Begin Android/UniFFI milestone** if the desktop baseline is unstable, `poker-core` purity is uncertain, or Android/Desktop canonical fixtures remain unresolved.
