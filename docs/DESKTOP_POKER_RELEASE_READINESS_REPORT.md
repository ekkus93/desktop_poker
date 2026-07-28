# Desktop Poker Release Readiness Report

## Executive result

**Outcome: All automated desktop release gates currently defined for the Linux product pass. The only remaining desktop release blocker is the deferred two-machine LAN tournament.**

The validated desktop release now includes:

- a clean automated CI baseline;
- direct Linux release-binary launch through Tauri/WebKit;
- AppImage build and release-runtime smoke;
- Debian package build, installation, desktop-entry launch, icon validation, and purge cleanup;
- complete real-TCP two-player tournaments with privacy, rejected-action, history, elimination, standings, and restart-persistence checks;
- release reconnect and host-loss validation;
- real Linux Secret Service credential persistence and failure-path validation;
- complete rule-based and embedded local GGUF NPC tournaments.

The remaining release blocker is:

- `DP-RR-P0-004` — a complete tournament using matching artifacts on two physical machines over a real LAN/firewall path.

The user has deferred that gate until moving to a more suitable machine. Multi-container networking can provide useful additional isolation and routing evidence, but it does not satisfy the current physical-machine criterion unless that criterion is explicitly changed.

This report and `docs/DESKTOP_POKER_CURRENT_BACKLOG.md` are the current evidence summaries. Machine-readable results remain authoritative for individual automated runs.

## Result vocabulary

- `PASS` — the named command or scenario was executed successfully against the recorded revision.
- `FAIL` — the named command or scenario was executed and produced a product or validation failure.
- `PARTIAL` — a meaningful subset passed, but the complete gate remains open.
- `BLOCKED` — execution requires a concrete unavailable dependency or environment.
- `DEFERRED` — execution is intentionally postponed by the user.
- `NOT RUN` — execution was not attempted for a stated reason.

## Final automated source revision

The final reconnect and CI validation batch tested source commit:

```text
f35d8552648091211ed05c99ac8474ae07541be6
```

The principal retained runs are:

| Gate | Run | Result |
|---|---:|---|
| General CI and browser geometry | `30335336701` | PASS |
| Reconnect protocol and release matrix | `30335336702` | PASS |
| Full release tournament and persistence | `30335336680` | PASS |
| AppImage release validation | `30335336681` | PASS |
| Rule-based NPC tournament | `30335336686` | PASS |
| Linux release keychain | `30335336694` | PASS |
| Installed Debian package | `30335336717` | PASS |

Evidence is retained under `docs/runtime-validation/` and in the named GitHub Actions artifacts.

## Automated validation baseline

**Result: PASS.**

Run `30335336701` reports both `verify` and browser geometry successful. The gate includes:

- committed npm dependency installation;
- production and full npm audits;
- frontend formatting;
- ESLint with no accepted warning bypass;
- frontend tests;
- TypeScript/Vite production build;
- browser geometry;
- Rust formatting;
- Clippy across workspace/all targets/all features with warnings denied;
- Rust workspace tests;
- focused `poker-core` tests;
- direct `poker-core` dependency-tree inspection.

The host-shutdown reconnect regression is part of the passing Rust workspace.

## Linux release artifacts

### Direct release binary

**Result: PASS.**

The release binary passes:

- graphical Tauri/WebKit session creation;
- Home, Host, Join, Settings, and Help routes;
- browser-mock exclusion;
- hidden debug-route exclusion;
- guarded lobby/table routing without a session;
- invalid-invite error handling;
- real TCP multiplayer;
- complete gameplay;
- profile isolation and history restoration.

Evidence:

- `docs/runtime-validation/latest.json`
- `docs/runtime-validation/gameplay-latest.json`

### AppImage

**Result: PASS.**

The AppImage is built, executable, and passes the production WebDriver smoke through the bundled application path.

Evidence:

- run `30335336681`
- `docs/runtime-validation/appimage-latest.json`
- artifact `linux-appimage-evidence`

### Debian package

**Result: PASS.**

The Debian package:

- builds with inspectable metadata;
- installs through `apt`;
- owns the expected executable, desktop entry, and icon files;
- passes the production WebDriver smoke from `/usr/bin/desktop-poker`;
- launches through the installed desktop entry and produces the expected X11 window;
- purges its executable, desktop entry, and package-owned icon files.

Evidence:

- run `30335336717`
- `docs/runtime-validation/deb-install-latest.json`
- artifact `linux-debian-package-v3-evidence`

## Complete local multiplayer runtime

**Result: PASS.**

Run `30335336680` proves two isolated release instances can:

- host and join through real TCP;
- validate and use a compact invitation;
- claim distinct seats;
- synchronize ready state;
- start a tournament;
- preserve private-card isolation;
- agree on public table state;
- reject an illegal raise without advancing authority;
- use legal quick-size/action behavior;
- settle multiple hands;
- eliminate a player and transition that player to observer state;
- display matching final standings;
- preserve total chips;
- restore host and client history after release-process restart;
- keep a fresh third profile free of the other profiles' history.

Evidence:

- `docs/runtime-validation/gameplay-latest.json`
- artifact `linux-release-full-game-evidence`

## Reconnect and failure matrix

**Result: PASS.**

Run `30335336702` passed the focused protocol suite, release build, and real release-runtime interruption matrix.

### Live release scenarios

The release matrix proves:

1. **Unreachable-host join** — a valid invitation whose listener is unavailable fails explicitly and leaves no partial client session.
2. **Lobby reconnect** — forcibly resetting the established client socket produces a different TCP tuple and restores a normal client and connected host participant.
3. **Active-hand reconnect** — the socket reset restores the same table/session and active hand on a new TCP tuple.
4. **Post-reconnect action** — the client submits an accepted call after reconnect.
5. **Immediate duplicate protection** — replaying the action immediately is rejected and does not execute twice.
6. **Active-hand host loss** — explicit host shutdown terminates the client session; table and action commands reject the stale session.
7. **Lobby host loss** — explicit host shutdown terminates the client session; table and action commands reject the stale session.

Machine-readable evidence records `terminated: true`, `reconnecting: false`, and `lastError: "Disconnected from host"` for both host-loss scenarios.

### Focused protocol scenarios

The reconnect tests additionally prove:

- table/session/player-scoped reconnect identity;
- stale-token rejection;
- expired-token rejection before authority or sequence mutation;
- already-connected participant rejection;
- duplicate-session prevention;
- eliminated-observer restoration;
- completed-tournament restoration without reopening action authority.

### Defect discovered and fixed

The first corrected matrix reached successful lobby/active-hand reconnect and duplicate rejection but did not terminate after `stop_host_session`.

Root cause:

- the host listener and tick loop stopped;
- detached per-client session threads still owned accepted TCP streams;
- the remote client therefore received no EOF and could not begin terminal reconnect failure handling.

Fix:

- `DesktopHostSession::drop()` requests explicit networking shutdown;
- listener acceptance is stopped before established connections are closed;
- the connected-client registry is drained before individual stream locks are acquired;
- all registered streams receive `Shutdown::Both`;
- poisoned locks and unexpected socket-close failures are logged;
- a regression requires `Reconnecting`, explicit reconnect failure, and terminal `Disconnected` events.

Evidence:

- `.github/workflows/runtime-reconnect-failure-matrix.yml`
- `scripts/runtime_reconnect_failure_matrix.py`
- `src-tauri/src/networking/runtime/host_shutdown.rs`
- `src-tauri/src/app_state/host_shutdown.rs`
- `src-tauri/src/networking/runtime/tests/host_shutdown.rs`
- `src-tauri/src/networking/runtime/tests/reconnect.rs`
- `src-tauri/src/networking/runtime/tests/reconnect_expiry.rs`
- `docs/runtime-validation/reconnect-failure-latest.json`
- artifact `reconnect-protocol-and-release-failure-evidence`

## Release credential storage

**Result: PASS.**

The Linux release uses the real synchronous Secret Service backend rather than the keyring crate's mock store.

Run `30335336694` proves:

- a test credential is accepted through Secret Service;
- public provider configuration contains no API-key field;
- app-data files and runtime logs contain no credential bytes or plaintext key file;
- the credential is recovered after restarting the same release profile;
- clear removes provider settings and the keychain credential;
- a second restart confirms durable deletion;
- a deliberately invalid D-Bus/Secret Service endpoint produces an explicit error;
- failed secure storage creates no provider state and no plaintext fallback.

Evidence:

- `.github/workflows/runtime-release-keychain-validation.yml`
- `scripts/runtime_release_keychain.py`
- `docs/runtime-validation/release-keychain-latest.json`
- artifact `linux-release-keychain-evidence`

## NPC release behavior

### Rule-based NPCs

**Result: PASS.**

The latest retained run completed a 12-hand release tournament with 19 accepted rule-based NPC actions. Both NPC identities participated, final elimination/standings completed, and the production runtime log contained no NPC error or fallback diagnostic.

Evidence:

- run `30335336686`
- `docs/runtime-validation/rule-based-tournament-latest.json`
- artifact `linux-rule-based-npc-tournament-evidence`

### Embedded local GGUF provider

**Result: PASS for live local-provider play.**

The embedded provider has passed real model loading/decision smoke and complete release tournaments with profile-backed NPCs, committed model-selected actions, elimination, and final standings without an NPC-runner diagnostic.

Negative embedded-provider paths and remote-provider live matrices remain non-blocking follow-up work for any release that advertises those provider modes as supported.

Evidence:

- `docs/runtime-validation/embedded-model-latest.json`
- `docs/runtime-validation/embedded-tournament-latest.json`

## Remaining desktop release gate

### Two-machine LAN tournament

**Result: DEFERRED / NOT RUN.**

The current automated evidence uses isolated release instances on one Linux runner. It does not prove a real external LAN/firewall path between two physical machines.

Required final proof under the current gate:

- matching artifact hashes on two machines;
- a non-loopback host address and real firewall path;
- successful invitation/join;
- private-card isolation;
- legal and rejected actions;
- complete tournament;
- matching final state and standings;
- reviewed host/client logs.

The executable procedure is in `docs/PHYSICAL_LAN_RELEASE_VALIDATION.md`.

The user has deferred this work until moving to a machine suitable for additional isolated network testing. Containerized testing can be performed there as a precursor, but final physical-machine acceptance remains separate unless the gate is intentionally redefined.

## Android/Desktop interoperability status

Android interoperability remains audited but not fully proven. Canonical event-payload differences and mixed-runtime testing remain open. They are not part of the completed Linux desktop reconnect matrix.

## Recommendation

The automated Linux desktop baseline is release-ready except for the explicitly deferred two-machine LAN gate. Do not mark the physical-LAN criterion passed from loopback, containers, or virtual networking without first redefining its acceptance criteria.
