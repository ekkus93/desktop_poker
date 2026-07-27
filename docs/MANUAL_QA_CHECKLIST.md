# Manual QA Checklist — Real Multiplayer

> **Current execution status:** Boxes may be checked only after the exact scenario is executed against a recorded commit and artifact. Virtual Linux graphical evidence is valid for release-binary and loopback runtime behavior; it does not substitute for installed-package, physical-LAN, physical-desktop, keychain, or live-provider testing.

Authoritative details are recorded in:

- `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`
- `docs/runtime-validation/latest.json`

For every run, record:

- date and tester;
- exact commit SHA;
- operating system and desktop environment;
- binary/package filename and SHA-256;
- instance IDs and host/client machine roles;
- PASS, FAIL, PARTIAL, BLOCKED, or NOT RUN for each scenario;
- defect identifier for every failure.

---

## Checklist A: Two local release instances on one machine

**Latest partial execution:** GitHub Actions run `30234522553`, commit `d2f4fc82eeb43a9ecec4524e490dda4662e80123`, Ubuntu 24.04/Xvfb/WebKitWebDriver, release SHA-256 `a8dc5d705a573371d64ab3872371598308b697ffa09126a19112e53b57d371fe`.

### Evidence header

- [x] Date recorded
- [x] Tested commit SHA recorded
- [x] Binary SHA-256 recorded
- [x] OS and graphical environment recorded
- [x] Application data paths for host and client recorded

### Host/join and initial gameplay flow

- [x] Host: Home → Host Tournament → configure two players → Start hosting
- [x] Host: live session exposes a compact `pkr1_...` invitation
- [ ] Host: click **Copy invite** and confirm the system clipboard contains the exact payload
- [x] Client: Home → Join Tournament → paste invite → Check invite → Continue to lobby
- [x] Host and client raw statuses contain the same participant IDs and seat indexes
- [x] Host begins in its authoritative occupied seat
- [x] Client claims the distinct remaining open seat through the release UI
- [x] Ready state propagates truthfully under normal loopback conditions
- [ ] Host: verify Start remains disabled before every authoritative prerequisite is met
- [x] Host: click **Start Tournament** after both players are seated and ready
- [x] Both: Lobby transitions to Main Table
- [x] Both: initial local player receives exactly two private hole cards
- [x] Both: initial remote private hole cards are absent
- [x] Both: initial table ID, hand number, street, pot, and board match
- [ ] Both: exercise fold, check, call, bet, legal raise, quick-size, and all-in confirmation
- [ ] Both: illegal raise bounds cannot be submitted
- [ ] Both: only the acting player receives an action tray
- [ ] Both: board, pot, contributions, action owner, street, and stacks remain synchronized through multiple actions
- [ ] Both: hand history accumulates without duplicate records
- [ ] Both: elimination is reflected in observer state for the eliminated player
- [ ] Observer: receives no future private cards and cannot act
- [ ] Both: tournament completion screen shows the same correct standings

### Exit and persistence

- [ ] Client leaves normally
- [ ] Host closes the table normally
- [ ] Restart host instance: display name, host draft, window state, and history restore as designed
- [ ] Restart client instance: host history and host draft do not appear

### Error path coverage

- [x] Host port conflict: second release instance on `43818` fails explicitly rather than hanging or claiming success
- [x] Change the second host to `43819` and confirm hosting succeeds
- [x] Client invalid invite: garbage input displays an inline error and stays on Join
- [ ] Client decoded invite for unavailable host: explicit connection failure
- [ ] Clear a failed deep-link invite and confirm it is not re-imported
- [x] Direct `/lobby` with no session redirects safely
- [x] Direct `/table` with no running session redirects safely
- [ ] Kill host during lobby and confirm the client reaches an explicit terminal state
- [ ] Kill host during a hand and confirm the client does not remain on a playable-looking table

---

## Checklist B: Two machines on the same LAN

**Setup:** Build the same commit on both machines or copy the exact same release artifact. Record matching SHA-256 values, OS details, LAN addresses, and firewall prerequisites.

### Evidence header

- [ ] Date recorded
- [ ] Tested commit SHA recorded
- [ ] Host OS and client OS recorded
- [ ] Matching binary/package SHA-256 recorded
- [ ] Host and client machine roles recorded
- [ ] Firewall prerequisites recorded

### Flow

- [ ] Machine A: host using its real LAN IP
- [ ] Machine B: transfer the `pkr1_...` invite
- [ ] Machine B: invite preview shows Machine A’s IP and port
- [ ] Machine B: join; both participants appear
- [ ] Both: claim seats and ready
- [ ] Host: start tournament
- [ ] Complete one full tournament through completion
- [ ] Verify private cards remain isolated
- [ ] Verify board, pot, stacks, actions, history, elimination, and standings match

### Disconnect/reconnect

- [ ] Lobby network interruption and recovery preserve player and seat
- [ ] Live-hand interruption before expiry restores state and command path
- [ ] Stale pre-disconnect actions are rejected and no duplicate action applies
- [ ] Reconnect after expiry fails explicitly
- [ ] Eliminated player reconnects as observer without cards/action authority
- [ ] Host shutdown produces an explicit terminal client state
- [ ] Post-completion reconnect fails gracefully

---

## Checklist C: Release binary instance isolation

- [x] Three release instances report distinct `instanceId` values
- [x] Three release instances report distinct profile directories
- [x] Client host draft is independently namespaced before joining
- [ ] Host: set display name “Alice” and tournament “Alice’s Game”
- [ ] Client: confirm display name and host draft remain independent
- [ ] Host: complete tournament and verify history
- [ ] Client: verify host history is absent
- [ ] Restart host and verify display name, draft, window state, and history
- [ ] Restart client and verify it remains unaffected

---

## Checklist D: Host port conflict behavior

- [x] Primary release host listens on `43818`
- [x] Second release host attempts the same port
- [x] Second host shows a clear failure and reports no false live session
- [ ] Runtime/debug health information includes useful non-secret bind context
- [x] Second host changes to `43819` and starts successfully

**Current UX note:** the visible message is explicit but generic: `Unable to start hosting.` Backlog item `DP-UX-P2-003` tracks more actionable port-in-use wording.

---

## Checklist E: Release production reachability and secret storage

- [x] Launch real release Tauri/WebKit binary in a graphical Linux session
- [x] Home, Host, Join, Settings, and Help work without development tooling
- [x] Layout probes, browser mocks, fake guarded sessions, and `/debug` are not player-reachable
- [ ] Install `.deb` and repeat the release route smoke test
- [ ] Configure a low-privilege provider key in a real release keychain session
- [ ] Restart and confirm non-secret provider settings restore
- [ ] Confirm key is absent from JSON, logs, snapshots, debug output, and app-data files
- [ ] Clear provider and confirm key deletion or visible failure
- [ ] Force keychain unavailability and confirm no plaintext release fallback

---

## Checklist F: Live NPC operation

- [ ] Complete a release tournament with at least one rule-based NPC
- [ ] Use distinct NPC profiles/styles and verify identity/profile association
- [ ] Confirm every seated NPC has a running decision loop
- [ ] Confirm NPC actions are legal and progress requires no manual injection
- [ ] Confirm NPC history, elimination, and completion behavior
- [ ] Observe at least one accepted legal live-LLM action when a provider is available
- [ ] Exercise unavailable/timeout/invalid-response fallbacks with typed diagnostics
- [ ] Confirm no credential is logged and no disallowed fallback silently acts

---

*Checklist updated from real release runtime evidence on 2026-07-26/27. Unchecked boxes remain unproven.*
