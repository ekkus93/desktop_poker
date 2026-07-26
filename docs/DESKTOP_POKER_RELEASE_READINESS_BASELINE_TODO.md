# Desktop Poker Release Readiness Baseline TODO

## Purpose

Implement and verify `docs/DESKTOP_POKER_RELEASE_READINESS_BASELINE_SPEC.md`.

This TODO is an execution plan for re-establishing a trustworthy baseline, proving the current Linux desktop multiplayer application in real runtime conditions, fixing only defects exposed by evidence, reconciling stale planning artifacts, and making an explicit next-milestone decision.

The existing application is an advanced MVP. Do not begin by rewriting the architecture or adding new product features.

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

- [ ] `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`
- [ ] `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- [ ] Current automated validation evidence in the release-readiness report
- [ ] Current manual local multi-instance evidence in the release-readiness report
- [ ] Current two-machine LAN evidence in the release-readiness report
- [ ] Current NPC and LLM evidence in the release-readiness report
- [ ] Updated `README.md` status and release instructions when results require changes
- [ ] A concise final ledger entry in `memory.md`

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

- [ ] The report contains the exact tested SHA and environment information.
- [ ] No later test result is attributed to a different unrecorded commit.
- [ ] Any commit change during defect repair is recorded before the final validation rerun.

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

- [ ] Required native dependencies are available.
- [ ] Environment-only failures are documented separately from product defects.

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

- [ ] `npm ci` succeeds using the committed lockfile.
- [ ] Dependency audit results are recorded honestly.
- [ ] No lockfile churn is introduced without a documented reason.

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
npm run test 2>&1 | tee /tmp/desktop-poker-npm-test.log
rg -n "Failed to initialize window state persistence|currentWindow|Unhandled|unhandled" /tmp/desktop-poker-npm-test.log || true
```

### Acceptance

- [ ] Formatting passes.
- [ ] Lint passes with zero warnings.
- [ ] All non-ignored frontend tests pass.
- [ ] Production frontend build passes.
- [ ] Geometry tests pass or are explicitly `BLOCKED` with an environment reason.
- [ ] Expected stderr is documented; unexplained recurring errors are treated as defects.

---

## P0.5 — Run the complete Rust validation suite

**Files:**

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- Rust source and tests only if a real failure is discovered

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
cargo test --workspace -- --list | rg "ignored|ollama|llama|provider" || true
```

- [ ] Do not claim ignored LLM tests passed until they are run against a configured provider.

### Acceptance

- [ ] Rust formatting passes.
- [ ] Clippy passes with warnings denied.
- [ ] All non-ignored workspace tests pass.
- [ ] Focused `poker-core` tests pass.
- [ ] The actual totals are copied from current output, not `memory.md`.

---

## P0.6 — Audit `poker-core` platform neutrality

**Files:**

- `crates/poker-core/**`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Run:

```bash
cargo tree -p poker-core
```

- [ ] Run the focused source audit:

```bash
rg -n "tauri|keyring|dirs|reqwest|local_ip|std::net|Tcp|Udp|Socket|Mutex<.*Tcp|thread::spawn|std::process::Command" crates/poker-core || true
```

- [ ] Inspect every hit.
- [ ] Distinguish `EngineCommand` false positives from process-spawning `Command` usage.
- [ ] Verify `crates/poker-core/Cargo.toml` contains only platform-neutral dependencies.
- [ ] Verify no feature flag conditionally pulls in Tauri, Android, networking, keychain, filesystem path discovery, or LLM dependencies.
- [ ] Verify deterministic state and command APIs remain available without the desktop crate.

### Acceptance

- [ ] `poker-core` remains reusable by future Android bindings.
- [ ] No platform dependency is introduced to make desktop validation easier.
- [ ] Every grep hit is explained in the report.

---

# P0 — Build and inspect production artifacts

## P0.7 — Build the release binary

**Files:**

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Build:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --release
```

- [ ] Confirm this file exists and is executable:

```bash
test -x src-tauri/target/release/desktop-poker
file src-tauri/target/release/desktop-poker
```

- [ ] Record binary size and SHA-256:

```bash
ls -lh src-tauri/target/release/desktop-poker
sha256sum src-tauri/target/release/desktop-poker
```

- [ ] Launch the binary in a graphical session.
- [ ] Confirm Home, Host, Join, Settings, and Help routes render.
- [ ] Confirm startup does not require browser mocks.
- [ ] Confirm startup errors are explicit if a required platform service is unavailable.

### Acceptance

- [ ] The release binary builds and launches.
- [ ] The report contains its size and hash.
- [ ] A development-only surface is not required for normal operation.

---

## P0.8 — Build and inspect the Debian package

**Files:**

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
dpkg-deb --info "<path-to-deb>"
dpkg-deb --contents "<path-to-deb>"
```

- [ ] Verify application identifier, version, binary path, desktop entry, and icons.
- [ ] Install the package on a test machine or disposable environment when practical.
- [ ] Launch the installed application.
- [ ] Remove it cleanly after testing if the environment requires cleanup.

### Acceptance

- [ ] `.deb` packaging succeeds.
- [ ] Package metadata is coherent.
- [ ] Installed application launches or installation is explicitly blocked and documented.

---

## P0.9 — Build AppImage when tooling is available

**Files:**

- `README.md` if prerequisite documentation is inaccurate
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Check for `linuxdeploy` and `appimagetool`.
- [ ] If available, run:

```bash
npm run tauri build -- --bundles appimage
```

- [ ] Record AppImage size and SHA-256.
- [ ] Mark executable and launch it.
- [ ] If tooling is unavailable, mark this item `BLOCKED`, not `PASS`.
- [ ] Confirm README accurately explains the prerequisite and alternative `.deb`/`.rpm` commands.

### Acceptance

- [ ] AppImage result is recorded as PASS, FAIL, or BLOCKED.
- [ ] No claim is made that AppImage works unless it was produced and launched.

---

## P0.10 — Audit production frontend and debug reachability

**Files:**

- `src/main.tsx`
- `src/app/runtimeGate.ts` if present
- `src/api/desktop.ts`
- `src-tauri/tauri.conf.json`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Rebuild production frontend:

```bash
npm run build
```

- [ ] Audit output:

```bash
rg -n "__DESKTOP_POKER_BROWSER_MOCKS__|LayoutProbeApp|layout-probe|/debug" dist || true
```

- [ ] Inspect every hit.
- [ ] Confirm browser mock resolution is guarded by development/test mode.
- [ ] Launch the release binary with attempted debug/probe query parameters.
- [ ] Confirm the hidden debug route is unavailable when `debugToolsEnabled` is false.
- [ ] Confirm production execution uses Tauri `invoke`/events rather than browser mocks.
- [ ] Confirm no test fixture can create a fake live session in release mode.
- [ ] Confirm CSP remains restrictive in `src-tauri/tauri.conf.json`.
- [ ] Confirm frontend `fetch` cannot reach arbitrary external origins.

### Acceptance

- [ ] Debug/probe code is not player-reachable in release builds.
- [ ] Browser mocks cannot replace backend calls in production.
- [ ] Any string remaining in minified output is analyzed for actual reachability.

---

## P0.11 — Audit provider secret storage

**Files:**

- `src-tauri/src/npc/provider_storage.rs`
- related provider tests
- `README.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Verify release builds select `KeychainSecretStore`.
- [ ] Verify debug builds may use the explicitly documented local secret file.
- [ ] Verify provider settings JSON excludes the API key.
- [ ] Configure a test provider in a release build using a low-privilege test key when available.
- [ ] Inspect application data files.
- [ ] Search logs and data directories for the literal test key.
- [ ] Verify keychain failure returns a visible error and does not write a plaintext fallback.
- [ ] Verify `Debug` formatting redacts the API key.
- [ ] Verify debug inspector state contains no key.
- [ ] Verify clearing a provider removes the corresponding secret or reports deletion failure.
- [ ] Do not use a production credential for this test.

### Acceptance

- [ ] No release plaintext fallback exists.
- [ ] No API key appears in JSON, logs, snapshots, debug output, or committed files.
- [ ] Secret-storage failure is explicit.

---

# P0 — Prove real multiplayer behavior

## P0.12 — Prepare reproducible release-instance test data

**Files:**

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Choose distinct instance IDs: `host-a` and `client-b`.
- [ ] Record the application data paths for both instances.
- [ ] Back up or remove only test-instance data when a clean run is required.
- [ ] Do not delete unrelated user application data.
- [ ] Choose a two-player configuration with a short but valid blind schedule and turn timer suitable for manual QA.
- [ ] Record tournament configuration.
- [ ] Enable logging appropriate for diagnosis without logging secrets.
- [ ] Decide how screenshots and logs will be referenced in the report.

### Acceptance

- [ ] The test can be repeated with the same binary and documented setup.
- [ ] Instance data is isolated before play begins.

---

## P0.13 — Execute two local release instances

**Files:**

- `docs/MANUAL_QA_CHECKLIST.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- source/tests only if defects are found

### Launch

```bash
./src-tauri/target/release/desktop-poker --instance-id host-a
./src-tauri/target/release/desktop-poker --instance-id client-b
```

### Host and join flow

- [ ] Host opens Home.
- [ ] Host selects Host Tournament.
- [ ] Host enters a valid name and tournament configuration.
- [ ] Host starts hosting.
- [ ] Host lobby displays a valid `pkr1_...` invite.
- [ ] Copy button succeeds or exposes an explicit manual-copy fallback.
- [ ] Client opens Join Tournament.
- [ ] Client pastes the invite.
- [ ] Client validates the invite.
- [ ] Preview displays the expected host, port, table, and tournament information.
- [ ] Client joins successfully.
- [ ] Both instances show both participants.

### Lobby flow

- [ ] Host claims a seat.
- [ ] Client claims a different seat.
- [ ] Seat claims propagate to both instances.
- [ ] Host ready state propagates.
- [ ] Client ready state propagates.
- [ ] In-flight ready UI does not lie about confirmed server state.
- [ ] Start remains unavailable before minimum conditions are met.
- [ ] Host starts the tournament when conditions are met.
- [ ] Both instances transition to Main Table.

### Gameplay flow

Exercise every action when legal across one or more hands.

- [ ] Fold.
- [ ] Check.
- [ ] Call.
- [ ] Bet.
- [ ] Raise using slider/manual amount.
- [ ] Use at least one quick-size option.
- [ ] All-in with confirmation.
- [ ] Cancel an all-in confirmation once.
- [ ] Verify illegal raise bounds cannot be submitted.
- [ ] Verify action tray belongs only to the acting player.
- [ ] Verify private hole cards appear only on the owning instance.
- [ ] Verify public board cards match.
- [ ] Verify pot and contributions match.
- [ ] Verify action owner and street labels match.
- [ ] Verify hand history updates on both instances.
- [ ] Verify no duplicate hand-history records.

### Elimination and completion

- [ ] Complete enough hands to eliminate one player.
- [ ] Eliminated player becomes an observer.
- [ ] Observer receives no private cards for future hands.
- [ ] Observer cannot submit actions.
- [ ] Tournament completion screen appears.
- [ ] Final standings match on both instances.
- [ ] Winner and eliminated ordering are correct.

### Session exit and persistence

- [ ] Client leaves normally.
- [ ] Host closes the table normally.
- [ ] Restart `host-a`.
- [ ] Verify host display name, host draft, window state, and saved history restore as designed.
- [ ] Restart `client-b`.
- [ ] Verify client data does not contain host history or host draft values.

### Acceptance

- [ ] A complete tournament succeeds with two local release instances.
- [ ] Public state remains synchronized.
- [ ] Private state remains isolated.
- [ ] Per-instance persistence is proven.
- [ ] Every deviation is logged as a defect rather than rationalized away.

---

## P0.14 — Test local error paths

**Files:**

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- source/tests only if defects are found

### Tasks

- [ ] Start a host on port `43818`.
- [ ] Start a second host instance and attempt to bind `43818`.
- [ ] Verify the second host receives a clear bind/port conflict error.
- [ ] Verify the second host does not hang or claim success.
- [ ] Change the second host to `43819` and verify hosting succeeds.
- [ ] Enter malformed invite text and verify inline parser failure.
- [ ] Enter a validly encoded invite for an unavailable host and verify explicit connection failure.
- [ ] Clear a bad deep-link invite and verify it is not silently re-imported.
- [ ] Attempt to navigate directly to `/lobby` with no session and verify safe redirection/recovery.
- [ ] Attempt to navigate directly to `/table` with no running session and verify safe redirection/recovery.
- [ ] Terminate the host process while the client is connected.
- [ ] Verify client enters an explicit terminal disconnect state.
- [ ] Verify no indefinite loading shell remains.

### Acceptance

- [ ] Every tested error is explicit and truthful.
- [ ] No host or client remains in a state that appears usable when authoritative transport is gone.

---

## P0.15 — Execute two-machine LAN tournament

**Files:**

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- source/tests only if defects are found

### Setup

- [ ] Use the same tested commit and equivalent release binary on both machines.
- [ ] Record OS version and binary SHA-256 on each machine.
- [ ] Record which machine is host and which is client.
- [ ] Confirm both are on the same LAN.
- [ ] Confirm firewall permits TCP port `43818`.
- [ ] Avoid using loopback, SSH port forwarding, or a browser mock.

### Flow

- [ ] Host resolves and displays a real LAN address.
- [ ] Host creates tournament.
- [ ] Client decodes invite and sees the expected LAN address.
- [ ] Client joins over real TCP.
- [ ] Both participants claim seats and ready.
- [ ] Host starts tournament.
- [ ] Complete a full tournament.
- [ ] Verify private cards remain isolated.
- [ ] Verify board, pot, action, history, elimination, and final standings match.

### Acceptance

- [ ] Full tournament succeeds across two physical machines.
- [ ] No debug tooling or manual state edits are used.
- [ ] Firewall or network prerequisites are documented.
- [ ] If two machines are unavailable, mark `BLOCKED`; do not call LAN multiplayer proven.

---

# P0 — Reconnect and recovery matrix

## P0.16 — Reconnect during lobby

**Files:**

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- source/tests only if a defect is found

### Tasks

- [ ] Join host from client.
- [ ] Claim a seat.
- [ ] Interrupt the client's network for approximately five seconds.
- [ ] Verify host marks client reconnecting rather than removing identity immediately.
- [ ] Restore network before reconnect expiry.
- [ ] Verify client reconnects into the same session and seat.
- [ ] Verify ready state is correct after recovery.
- [ ] Verify action submission channel is functional before UI claims reconnection success.

### Acceptance

- [ ] Lobby reconnect succeeds without duplicate participant identity.
- [ ] Snapshot acceptance does not precede required command-stream installation.

---

## P0.17 — Reconnect during active play

### Tasks

- [ ] Start a hand.
- [ ] Interrupt client networking during or immediately before its action window.
- [ ] Restore networking before reconnect expiry.
- [ ] Verify snapshot/resync restores current board, pot, player stacks, action window, and sequence.
- [ ] Verify client can submit the next legal action.
- [ ] Verify no duplicate action is processed.
- [ ] Verify stale pre-disconnect actions are rejected.
- [ ] Verify hand continues to settlement.

### Acceptance

- [ ] Active-play reconnect restores a functional command and event path.
- [ ] No stale or duplicated action corrupts the hand.

---

## P0.18 — Reconnect expiry

### Tasks

- [ ] Disconnect client beyond the configured reconnect window.
- [ ] Verify host eventually changes participant state according to the documented policy.
- [ ] Verify reconnect attempt after expiry fails explicitly.
- [ ] Verify client sees a terminal error and a safe leave/rejoin path.
- [ ] Verify no stale seat can act.

### Acceptance

- [ ] Expired sessions do not silently resurrect.
- [ ] User-visible state accurately reflects termination.

---

## P0.19 — Eliminated observer reconnect

### Tasks

- [ ] Eliminate the client.
- [ ] Confirm observer state.
- [ ] Disconnect and reconnect within the allowed window.
- [ ] Verify client returns as observer.
- [ ] Verify observer has no private cards.
- [ ] Verify observer has no action tray or action authority.
- [ ] Verify public event stream continues.

### Acceptance

- [ ] Observer reconnect preserves confidentiality and lack of action authority.

---

## P0.20 — Host loss

### Tasks

- [ ] Kill the host process during lobby.
- [ ] Verify client receives explicit disconnection.
- [ ] Repeat during a live hand.
- [ ] Verify no client-side fallback pretends the tournament can continue authoritatively.
- [ ] Verify recovery screen provides a valid exit path.

### Acceptance

- [ ] Host loss is terminal and explicit.
- [ ] Frontend does not remain on a stale playable table.

---

# P0 — Safety and silent-failure audit

## P0.21 — Re-run networking hardening audits

**Files:**

- `src-tauri/src/networking/runtime/client.rs`
- related runtime files
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Commands

```bash
rg -n "thread::spawn" src-tauri/src/networking/runtime/client.rs
rg -n "server_sequence.*unwrap_or_default|unwrap_or_default\(\).*server_sequence" src-tauri/src/networking/runtime/client.rs
rg -n "last_seen_server_sequence = envelope\.server_sequence" src-tauri/src/networking/runtime/client.rs
rg -n "try_clone\(|command_connection.*lock\(|connection\.stream = Some|if let Ok\(cloned_stream\)|if let Ok\(mut connection\)" src-tauri/src/networking/runtime/client.rs
rg -n "let _ = sender\.send" src-tauri/src/networking/runtime/client.rs
rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated" src-tauri/src/networking
```

### Tasks

- [ ] Inspect all hits manually.
- [ ] Verify raw thread spawning is still replaced by injectable/observable spawning where required.
- [ ] Verify missing public-event sequence cannot default to zero.
- [ ] Verify missing snapshot sequence cannot clear or invent ordering state.
- [ ] Verify stale snapshots are rejected.
- [ ] Verify reconnect command-stream clone and lock failures emit `SafeError` and stop acceptance.
- [ ] Verify accepted reconnect snapshot is emitted only after command stream installation.
- [ ] Verify remaining `if let Ok(mut connection)` blocks are cleanup-only and precisely commented.
- [ ] Verify ignored channel sends are centralized in the explicit best-effort helper.
- [ ] Verify crypto associated data cannot fall back to empty bytes.

### Acceptance

- [ ] No dangerous silent reconnect or sequence fallback exists.
- [ ] Best-effort behavior is limited to shutdown/teardown event delivery or cleanup.
- [ ] Every accepted exception is explained in the report.

---

## P0.22 — Audit host runtime error visibility

**Files:**

- `src-tauri/src/networking/runtime/host.rs`
- `src-tauri/src/networking/runtime/host_broadcast.rs`
- host runtime health types and tests
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Search for ignored results, broad `continue`, broad `return`, lock poisoning fallbacks, and thread spawn calls.
- [ ] Inspect every socket accept, stream clone, client registry lock, authoritative state lock, tick, publish, snapshot sync, and reconnect-mark failure path.
- [ ] Verify each important failure updates `HostRuntimeHealth` or returns a structured error.
- [ ] Verify `lastError` contains useful context without secrets.
- [ ] Trigger at least one safe diagnostic failure, such as port bind conflict, and inspect debug health state.
- [ ] Verify mutation APIs do not return failure solely because a post-mutation snapshot notification failed without distinguishing committed state.
- [ ] Verify atomic NPC add/remove behavior remains correct.

### Acceptance

- [ ] Host runtime failures are observable.
- [ ] APIs remain honest about whether authoritative mutation committed.
- [ ] No lock poisoning path clones stale authoritative state as a silent substitute.

---

## P0.23 — Audit NPC internal-error behavior

**Files:**

- `src-tauri/src/npc/runner/action.rs`
- `src-tauri/src/npc/runner/**`
- NPC tests

### Tasks

- [ ] Search for empty hole-card fallbacks:

```bash
rg -n "hole_cards_by_player_id.*unwrap_or\(|unwrap_or\(&\[\]\)|unwrap_or_default" src-tauri/src/npc crates/poker-core/src || true
```

- [ ] Verify acting NPC requires exactly the valid hole-card shape.
- [ ] Verify missing or invalid cards record an internal error and submit no action.
- [ ] Verify stale action windows do not count as success.
- [ ] Verify illegal LLM actions produce the expected typed fallback or rejection.
- [ ] Verify no NPC identity is inferred from seat-array position.
- [ ] Verify per-NPC history, opponent model, tilt, profile, and fallback state remain isolated.

### Acceptance

- [ ] NPCs never invent private state.
- [ ] NPC failure cannot silently advance or corrupt authoritative gameplay.

---

# P1 — Live NPC and LLM validation

## P1.1 — Rule-based NPC tournament

**Files:**

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- source/tests only if defects are found

### Tasks

- [ ] Start a release host session with at least two NPCs or one human and one NPC.
- [ ] Use different NPC styles/profiles.
- [ ] Verify all NPCs are added atomically.
- [ ] Verify NPC seats and identities match configured profiles.
- [ ] Complete multiple hands.
- [ ] Verify NPC decisions are legal.
- [ ] Verify tournament continues without manual action injection.
- [ ] Verify style differences are observable where valid opportunities occur.
- [ ] Verify hand history records NPC outcomes.
- [ ] Verify elimination and tournament completion work.

### Acceptance

- [ ] A live rule-based NPC tournament completes.
- [ ] No seated NPC is left without a runner.

---

## P1.2 — Configure a local LLM provider

**Files:**

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Preferred options

- Ollama
- llama-server

### Tasks

- [ ] Record provider type.
- [ ] Record endpoint URL.
- [ ] Record model name.
- [ ] Do not record API secrets.
- [ ] Confirm provider endpoint responds before launching the tournament.
- [ ] Save provider configuration through the release UI.
- [ ] Restart the app and verify non-secret settings restore.
- [ ] Verify key/configured state is correct.

### Acceptance

- [ ] Provider configuration is reproducible.
- [ ] No secret is written into the report.

---

## P1.3 — Live LLM NPC tournament

### Tasks

- [ ] Assign an explicit AI profile to an NPC.
- [ ] Start tournament.
- [ ] Observe at least five LLM decision opportunities if practical.
- [ ] Verify valid JSON action responses are parsed.
- [ ] Verify submitted action is in the legal action set.
- [ ] Verify raise amount is within legal bounds.
- [ ] Verify prompt includes profile and game context without private data from other players.
- [ ] Verify session history and opponent context appear only where intended.
- [ ] Verify game remains responsive during provider request.
- [ ] Verify a successful LLM decision is distinguishable from fallback.
- [ ] Complete at least one hand.

### Acceptance

- [ ] At least one live legal LLM action is accepted, or the run is marked FAIL/BLOCKED with exact provider evidence.
- [ ] The hand never becomes permanently blocked by an LLM request.

---

## P1.4 — LLM fallback matrix

### Tasks

Exercise safe, reproducible cases.

- [ ] Provider unavailable / connection refused.
- [ ] Request timeout.
- [ ] Invalid JSON response, using a mock server or existing test seam if necessary.
- [ ] Legal JSON shape containing an illegal action.
- [ ] Missing required API key for a remote provider, if tested with a non-secret dummy configuration.
- [ ] Explicit profile configured to disallow operational rule-based fallback, if supported.

For each case:

- [ ] Verify typed fallback/error reason.
- [ ] Verify WARN or structured debug diagnostics.
- [ ] Verify no credential appears in output.
- [ ] Verify allowed fallback submits a legal rule-based action.
- [ ] Verify disallowed fallback does not silently act.
- [ ] Verify game behavior remains defined.

### Acceptance

- [ ] All tested fallback cases are observable and policy-compliant.
- [ ] No LLM failure silently becomes an unexplained action.

---

## P1.5 — Run ignored provider tests

### Tasks

- [ ] Start the required local provider.
- [ ] Run:

```bash
cargo test --workspace -- --ignored
```

- [ ] Record each ignored test executed.
- [ ] If tests assume Ollama specifically, record that limitation.
- [ ] If a test reaches a real service nondeterministically, ensure failures provide actionable diagnostics.

### Acceptance

- [ ] Provider tests pass, fail with a real product defect, or are marked BLOCKED with exact setup limitations.

---

# P1 — Reconcile current product backlog

## P1.6 — Inventory historical planning documents

**Files:**

- all relevant `docs/*.md`
- `memory.md`
- `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`

### Tasks

- [ ] List current spec, TODO, review, response, QA, architecture, and audit documents.
- [ ] Identify documents that are historical, current, superseded, or partially stale.
- [ ] Do not rely on filename numbering alone.
- [ ] Compare each unchecked item against current source and tests.
- [ ] Compare each checked item that appears suspicious against current source and tests.
- [ ] Note corrected or overclaimed historical ledger entries.
- [ ] Identify duplicate tasks represented in multiple TODO files.
- [ ] Preserve historical documents unless deletion has a strong documented reason.

### Acceptance

- [ ] A source-of-truth map is included in `DESKTOP_POKER_CURRENT_BACKLOG.md`.
- [ ] Historical context is preserved without pretending it is current status.

---

## P1.7 — Reconcile `docs/UIUX_FIXES6.md`

**Files:**

- `docs/UIUX_FIXES6.md`
- current screen/component files
- current frontend tests
- `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`

### Tasks

For each item, classify `implemented`, `partially implemented`, `not implemented`, `obsolete`, or `deferred` based on current code.

At minimum verify:

- [ ] Port validation touch behavior.
- [ ] NPC add retry behavior.
- [ ] Empty standings guard.
- [ ] Deep-link query cleanup.
- [ ] Clipboard fallback dismissal.
- [ ] Lobby ready-state optimistic override.
- [ ] Mutual exclusion of settings confirmations.
- [ ] Settings and profile navigation.
- [ ] Dense lobby seat ARIA labels.
- [ ] Saved-history versus live-history banner.
- [ ] Built-in profile delete explanation.
- [ ] Unsaved profile change warning.
- [ ] Profile ID slug validation.
- [ ] Provider-specific key hints.
- [ ] Minimum-player hint.
- [ ] Confirmation focus and return focus.
- [ ] ARIA live regions.
- [ ] Global focus-visible styling.
- [ ] Hand-history cap or virtualization.
- [ ] Leave-flow retry behavior.
- [ ] Active support-navigation state.
- [ ] Help scroll behavior.
- [ ] Startup warning detail.
- [ ] Invite reachability/staleness note.
- [ ] Error-state re-check action.
- [ ] Main-table confirmation announcement.
- [ ] Profile heading aliases.
- [ ] Accessibility cross-cutting checklist.

### Acceptance

- [ ] Every current UI/UX gap appears once in the authoritative backlog.
- [ ] Completed behavior is not reimplemented.
- [ ] Release blockers are distinguished from polish.

---

## P1.8 — Reconcile real multiplayer and manual QA documents

**Files:**

- `docs/MANUAL_QA_CHECKLIST.md`
- relevant real multiplayer TODO files
- `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`
- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Tasks

- [ ] Update checklist commands if they still use outdated pre-workspace commands.
- [ ] Mark manual items only after executing them in this pass.
- [ ] Add result date, tested SHA, and environment to completed checklist sections.
- [ ] Add links or references to the release-readiness report evidence.
- [ ] Verify port-conflict, instance-isolation, reconnect, host-loss, and full-tournament scenarios are represented.
- [ ] Add missing manual checks discovered during execution.

### Acceptance

- [ ] Manual QA checklist reflects current commands and evidence.
- [ ] It no longer implies that an old unchecked box describes current status without context.

---

## P1.9 — Reconcile Android interoperability status

**Files:**

- `docs/ANDROID_INTEROP_AUDIT.md`
- `docs/ANDROID_ARCHITECTURE.md`
- desktop protocol models and fixtures
- `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`

### Tasks

- [ ] Verify protocol version remains `1`.
- [ ] Verify join payload codec remains compatible with the audited Android commit.
- [ ] Verify canonical JSON and crypto derivation remain as documented.
- [ ] Recheck `ACTION_WINDOW_OPENED_EVENT` payload mismatch.
- [ ] Recheck `ACTION_REJECTED_EVENT` payload mismatch.
- [ ] Verify fixture tests still pin current desktop shapes.
- [ ] Record that live Android/Desktop interop remains unproven unless it is actually executed.
- [ ] Do not begin Android implementation in this milestone.
- [ ] Keep Kotlin-owned networking and Rust-owned deterministic core boundary intact.
- [ ] Add interoperability work to the backlog as a separate future milestone, not a desktop release blocker unless the intended release promises Android compatibility.

### Acceptance

- [ ] Interop status is current and honest.
- [ ] Known payload mismatches have stable backlog identifiers and acceptance criteria.

---

## P1.10 — Create `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`

### Required structure

- [ ] Title and tested commit SHA.
- [ ] Statement that this is the authoritative current backlog.
- [ ] Source documents reviewed.
- [ ] Release-blocker summary.
- [ ] Desktop items.
- [ ] Android interoperability items.
- [ ] Deferred product features.
- [ ] Superseded-document map.

### Required fields for each item

- [ ] Identifier, e.g. `DP-RR-P0-001`.
- [ ] Priority: P0, P1, P2, deferred.
- [ ] Category.
- [ ] Current behavior.
- [ ] Evidence.
- [ ] Affected files.
- [ ] Required behavior.
- [ ] Acceptance criteria.
- [ ] Automated test requirement.
- [ ] Manual test requirement.
- [ ] Desktop release blocker: yes/no.
- [ ] Status.

### Acceptance

- [ ] No known current gap exists only in an old TODO.
- [ ] Duplicate items are consolidated.
- [ ] Every blocker has exact acceptance criteria.

---

# P1 — Fix evidence-backed defects

## P1.11 — Create a defect record before changing code

For every defect discovered during this milestone:

- [ ] Assign a stable backlog identifier.
- [ ] Record tested SHA.
- [ ] Record environment.
- [ ] Record exact reproduction steps.
- [ ] Record expected result.
- [ ] Record observed result.
- [ ] Record logs/screenshots without secrets.
- [ ] Classify severity.
- [ ] Identify architecture boundary involved.
- [ ] Identify whether an automated regression test is practical.

### Acceptance

- [ ] No defect fix begins from a vague statement such as “multiplayer seems flaky.”

---

## P1.12 — Add a regression test before or with each fix

### Tasks

- [ ] Add the smallest test that forces the failing branch.
- [ ] For protocol/network defects, prefer deterministic unit or integration seams over arbitrary sleeps.
- [ ] For frontend defects, test visible state and user interaction rather than implementation details.
- [ ] For manual-only desktop behavior, add a lower-level automated test where practical and retain the manual scenario.
- [ ] Verify the new test fails on the defective code when practical.
- [ ] Verify the new test passes after the fix.
- [ ] Ensure the test does not accept both contradictory outcomes.

### Forbidden shortcuts

- [ ] Do not weaken assertions.
- [ ] Do not add broad `catch` blocks that discard errors.
- [ ] Do not increase timeouts without evidence.
- [ ] Do not add `unwrap_or_default` to required protocol or private state.
- [ ] Do not make required mutations “best effort.”
- [ ] Do not skip the failing test in CI.

### Acceptance

- [ ] Each practical defect fix has a meaningful regression test.

---

## P1.13 — Implement the smallest correct fix

### Tasks

- [ ] Preserve `poker-core`/desktop/frontend authority boundaries.
- [ ] Preserve protocol compatibility unless the defect explicitly requires a versioned protocol change.
- [ ] Keep errors typed and visible.
- [ ] Keep fallbacks explicit and observable.
- [ ] Update documentation when behavior changes.
- [ ] Run focused tests.
- [ ] Run full frontend and Rust validation.
- [ ] Rebuild release binary.
- [ ] Re-run the original manual scenario.
- [ ] Record final evidence.

### Acceptance

- [ ] The defect is closed by evidence, not by code inspection alone.

---

# P2 — Final validation and reporting

## P2.1 — Re-run the complete clean automated baseline

After all fixes:

- [ ] Confirm working tree state.
- [ ] Record final commit SHA.
- [ ] Run `npm ci`.
- [ ] Run `npm run format:check`.
- [ ] Run `npm run lint`.
- [ ] Run `npm run test`.
- [ ] Run `npm run build`.
- [ ] Run `npm run test:geometry`.
- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --workspace --all-targets --all-features -- --test-threads=2`.
- [ ] Run `cargo test -p poker-core --all-targets --all-features`.
- [ ] Run `cargo tree -p poker-core`.
- [ ] Run ignored provider tests if the provider is available.
- [ ] Record actual final totals and ignored tests.

### Acceptance

- [ ] The final report reflects the final SHA, not an earlier baseline SHA.

---

## P2.2 — Rebuild final release artifacts

### Tasks

- [ ] Build final release binary.
- [ ] Build final `.deb`.
- [ ] Build final AppImage if tooling is available.
- [ ] Record final hashes and sizes.
- [ ] Launch each claimed-working artifact.
- [ ] Confirm package version matches intended release state.

### Acceptance

- [ ] Reported artifacts correspond exactly to the final tested SHA.

---

## P2.3 — Re-run affected manual scenarios

### Tasks

- [ ] Re-run any manual scenario touched by a code fix.
- [ ] Re-run the full two-local-instance tournament after networking, session, projection, or UI fixes.
- [ ] Re-run the two-machine LAN tournament after networking, protocol, crypto, invite, or reconnect fixes.
- [ ] Re-run NPC tournament after NPC, profile, provider, or core fixes.
- [ ] Re-run production reachability checks after frontend boot or build changes.

### Acceptance

- [ ] No fix is declared complete solely because automated tests pass.

---

## P2.4 — Create `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`

### Required top-level sections

- [ ] Executive result.
- [ ] Tested commit and environment.
- [ ] Automated validation.
- [ ] Test totals and ignored tests.
- [ ] Release artifact inventory.
- [ ] Production reachability/security audit.
- [ ] Two-local-instance QA.
- [ ] Two-machine LAN QA.
- [ ] Reconnect and failure matrix.
- [ ] Rule-based NPC QA.
- [ ] LLM NPC QA.
- [ ] Defects discovered and fixes applied.
- [ ] Remaining release blockers.
- [ ] Non-blocking current backlog.
- [ ] Android/Desktop interoperability status.
- [ ] Final milestone recommendation.

### Result vocabulary

Use only:

- `PASS`
- `FAIL`
- `BLOCKED`
- `NOT RUN`

### Evidence rules

- [ ] Every PASS names the command or manual scenario executed.
- [ ] Every FAIL contains reproduction details.
- [ ] Every BLOCKED contains the concrete missing dependency/environment.
- [ ] Every NOT RUN explains why it was not attempted.
- [ ] Historical claims are clearly labeled historical.
- [ ] No API key, token, private key, reconnect token, or sensitive full payload is included.

### Acceptance

- [ ] A reader can determine release readiness without reading `memory.md`.

---

## P2.5 — Update `README.md`

### Tasks

- [ ] Update current product status based on final evidence.
- [ ] Do not call manual QA complete unless Gates C through E passed.
- [ ] Update test commands if they changed.
- [ ] Update artifact paths and packaging prerequisites if required.
- [ ] Update Android interop wording if the audit changed.
- [ ] Link to:
  - [ ] `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`;
  - [ ] `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`;
  - [ ] `docs/MANUAL_QA_CHECKLIST.md`.
- [ ] Keep known limitations explicit.

### Acceptance

- [ ] README status matches the final report.
- [ ] README does not overclaim production readiness.

---

## P2.6 — Update historical documents safely

### Tasks

- [ ] Add a superseded/current-status note to stale TODO documents where helpful.
- [ ] Point readers to `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`.
- [ ] Do not delete old reviews/specs/TODOs solely to reduce clutter.
- [ ] Do not rewrite historical completion claims as though they were created today.
- [ ] Update `docs/MANUAL_QA_CHECKLIST.md` with current execution status and dates.
- [ ] Update `docs/ANDROID_INTEROP_AUDIT.md` only if current evidence changes it.

### Acceptance

- [ ] Historical audit value remains intact.
- [ ] New contributors can identify the authoritative documents quickly.

---

## P2.7 — Add final `memory.md` ledger entry

### Tasks

- [ ] Add date/time, model/tool identity if the project convention requires it, and final commit SHA.
- [ ] Summarize commands actually run.
- [ ] Summarize manual scenarios actually run.
- [ ] Record test totals from final output.
- [ ] Record release artifacts produced.
- [ ] Record final outcome and remaining blockers.
- [ ] Link by exact path to the release-readiness report and current backlog.
- [ ] Do not reproduce the entire report.
- [ ] Do not claim unexecuted work passed.

### Acceptance

- [ ] Ledger is concise and consistent with the authoritative report.

---

# Final release decision gates

## Gate A — Automated baseline

- [ ] `npm ci` passed.
- [ ] Frontend format passed.
- [ ] Frontend lint passed with zero warnings.
- [ ] Frontend tests passed.
- [ ] Frontend production build passed.
- [ ] Browser geometry tests passed.
- [ ] Rust format passed.
- [ ] Clippy passed with warnings denied.
- [ ] Workspace tests passed.
- [ ] Focused `poker-core` tests passed.
- [ ] `poker-core` dependency audit passed.

## Gate B — Production artifact

- [ ] Release binary built and launched.
- [ ] `.deb` built and inspected.
- [ ] AppImage result recorded accurately.
- [ ] Production debug/probe reachability audit passed.
- [ ] Secret-storage audit passed.

## Gate C — Local multiplayer

- [ ] Two local release instances completed a tournament.
- [ ] Private cards remained isolated.
- [ ] Public state remained synchronized.
- [ ] Instance persistence remained isolated.

## Gate D — LAN multiplayer

- [ ] Two physical machines completed a tournament over real LAN TCP.
- [ ] Both used the same tested source revision or matching artifact.

## Gate E — Recovery and errors

- [ ] Lobby reconnect passed.
- [ ] Active-hand reconnect passed.
- [ ] Reconnect expiry passed.
- [ ] Eliminated observer reconnect passed.
- [ ] Host-loss behavior passed.
- [ ] Port conflict passed.
- [ ] Invalid invite passed.
- [ ] Unreachable host passed.

## Gate F — NPC operation

- [ ] Rule-based NPC tournament passed.
- [ ] LLM NPC result is PASS or explicitly BLOCKED for an external environment reason.
- [ ] Fallback behavior is observable and policy-compliant.

## Gate G — Blocker closure

- [ ] `docs/DESKTOP_POKER_CURRENT_BACKLOG.md` exists.
- [ ] No open P0 desktop release blocker remains.
- [ ] No open desktop-release-blocking P1 remains.
- [ ] Final report exists and matches the final tested SHA.

---

# Required final outcome

Select exactly one outcome in `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`.

## Outcome 1 — Desktop release candidate

Choose only when Gates A through G pass, except a genuinely external optional LLM-provider limitation may remain BLOCKED without blocking rule-based core operation.

- [ ] Recommend preparing and tagging the desktop release.
- [ ] State the exact version/tag proposed.
- [ ] State which artifacts will be published.

## Outcome 2 — Additional desktop stabilization

Choose when one or more desktop release blockers remain.

- [ ] List blocker identifiers.
- [ ] Create a focused follow-up spec and TODO based only on reproduced failures.
- [ ] Do not start unrelated features.

## Outcome 3 — Android/Desktop interoperability milestone

Choose only after desktop baseline and release blockers are resolved.

- [ ] Preserve native Kotlin/Compose plus shared Rust core architecture.
- [ ] Preserve Kotlin-owned networking.
- [ ] Resolve current action-window and action-rejected payload shape mismatches first.
- [ ] Define a UniFFI-safe adapter in a separate spec.
- [ ] Do not move desktop networking into `poker-core`.

---

# Completion checklist

- [ ] All P0 tasks executed or explicitly documented as blocked.
- [ ] All discovered defects have evidence and stable backlog IDs.
- [ ] All implemented fixes have focused tests where practical.
- [ ] Full final automated validation passes.
- [ ] Final release artifacts correspond to the final SHA.
- [ ] Local release-instance QA is complete.
- [ ] LAN QA is complete or honestly blocks a release-proven claim.
- [ ] Recovery matrix is complete.
- [ ] Rule-based NPC QA is complete.
- [ ] LLM QA result is recorded.
- [ ] Security and production reachability audits are complete.
- [ ] Current backlog is committed.
- [ ] Release-readiness report is committed.
- [ ] README is consistent with evidence.
- [ ] Historical documents point to the authoritative backlog/report where appropriate.
- [ ] `memory.md` has a concise, honest final entry.
- [ ] Exactly one final milestone outcome is selected.
