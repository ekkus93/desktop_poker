# Manual QA Checklist — Real Multiplayer

> **Current execution status:** This checklist contains manual scenarios, not historical pass evidence. A box may be checked only after the scenario is executed against a recorded commit and artifact. Record results in `docs/DESKTOP_POKER_RELEASE_READINESS_REPORT.md`. Current open work is consolidated in `docs/DESKTOP_POKER_CURRENT_BACKLOG.md`.

These checklists require a real desktop environment and cannot be replaced by unit, integration, browser-mock, or protocol-fixture tests. Complete them when validating a release candidate or after significant networking/protocol changes.

For every run, record:

- date and tester;
- exact commit SHA;
- operating system and desktop environment;
- binary/package filename and SHA-256;
- instance IDs and host/client machine roles;
- PASS, FAIL, BLOCKED, or NOT RUN for each scenario;
- defect identifier for every failure.

---

## Checklist A: Two local release instances on one machine

**Setup:** From the repository root, build the desktop package with:

```bash
cargo build -p desktop-poker --release
```

The workspace release binary is expected at `target/release/desktop-poker`.

Launch two instances with distinct IDs:

```bash
# Terminal 1 (host)
./target/release/desktop-poker --instance-id host-a

# Terminal 2 (client)
./target/release/desktop-poker --instance-id client-b
```

### Evidence header

- [ ] Date recorded
- [ ] Tested commit SHA recorded
- [ ] Binary SHA-256 recorded
- [ ] OS and desktop environment recorded
- [ ] Application data paths for `host-a` and `client-b` recorded

### Host/join and gameplay flow

- [ ] Host: Home → Host Tournament → fill in name and player count → Start hosting
- [ ] Host: Lobby shows host player in a seat, invite is visible and copyable
- [ ] Host: Click "Copy invite" and confirm the clipboard contains a `pkr1_...` payload
- [ ] Client: Home → Join Tournament → paste the invite → Check invite → Continue to lobby
- [ ] Both: Lobby shows two participants with correct names
- [ ] Both: Seat selection (host claims a seat; client claims a different seat)
- [ ] Both: Ready toggle is reflected truthfully for both players
- [ ] Host: Start remains disabled until all authoritative conditions are met
- [ ] Host: Click "Start Tournament" when all players are ready
- [ ] Both: Lobby transitions to Main Table
- [ ] Both: Hand play exercises fold, check, call, bet, legal raise, quick-size, and all-in confirmation
- [ ] Both: Illegal raise bounds cannot be submitted
- [ ] Both: Only the acting player receives an action tray
- [ ] Both: Private hole cards are visible only to their owner
- [ ] Both: Board, pot, contributions, action owner, street, and stacks remain synchronized
- [ ] Both: Hand history accumulates without duplicate records
- [ ] Both: Elimination is reflected in observer state for the eliminated player
- [ ] Observer: receives no future private cards and cannot act
- [ ] Both: Tournament completion screen shows the same correct standings

### Exit and persistence

- [ ] Client leaves normally
- [ ] Host closes the table normally
- [ ] Restart `host-a`: display name, host draft, window state, and history restore as designed
- [ ] Restart `client-b`: host history and host draft do not appear

### Error path coverage

- [ ] Host port conflict: Start two host instances on the same port and confirm the second shows a clear error, not a silent hang
- [ ] Change the second host to a different port, such as `43819`, and confirm hosting succeeds
- [ ] Client join with invalid invite: paste garbage and confirm the parser error is shown inline
- [ ] Client join with a decoded invite for an unavailable host: confirm explicit connection failure
- [ ] Clear a failed deep-link invite and confirm it is not re-imported
- [ ] Direct `/lobby` navigation with no session redirects or recovers safely
- [ ] Direct `/table` navigation with no running session redirects or recovers safely
- [ ] Kill the host during lobby and confirm the client reaches an explicit terminal state
- [ ] Kill the host during a hand and confirm the client does not remain on a playable-looking table

---

## Checklist B: Two machines on the same LAN

**Setup:** Build the same commit on both machines or copy the exact same release artifact. Record the binary SHA-256 on both machines. Confirm both are on the same LAN subnet and can reach TCP port `43818`.

### Evidence header

- [ ] Date recorded
- [ ] Tested commit SHA recorded
- [ ] Host OS and client OS recorded
- [ ] Matching binary/package SHA-256 recorded
- [ ] Host and client machine roles recorded
- [ ] Firewall prerequisites recorded

### Flow

- [ ] Machine A (host): Start hosting and confirm the lobby shows the machine's real LAN IP
- [ ] Machine B (client): Copy or manually type the `pkr1_...` invite payload from Machine A
- [ ] Machine B: Join → Check invite → Confirm host details show Machine A's IP and port
- [ ] Machine B: Continue to lobby → Lobby shows both participants
- [ ] Both: Claim seats and ready
- [ ] Host: Start tournament
- [ ] Complete one full tournament hand-for-hand through completion
- [ ] Verify private cards remain isolated
- [ ] Verify board, pot, stacks, action, history, elimination, and final standings match

### Disconnect/reconnect

- [ ] During lobby: disconnect Machine B's network adapter briefly → reconnect → same player and seat recover
- [ ] During live play: disconnect Machine B during or immediately before an action window → reconnect before expiry → state and command path recover
- [ ] Verify stale pre-disconnect actions are rejected and no duplicate action is processed
- [ ] Disconnect Machine B long enough to exceed the reconnect window → reconnect fails explicitly and no stale seat can act
- [ ] Eliminate Machine B, disconnect, and reconnect within the window → return as observer without cards or action authority
- [ ] Host shuts down mid-game → client receives an explicit terminal error and valid exit path
- [ ] After tournament completion: reconnect attempt fails gracefully because the session is gone

---

## Checklist C: Release binary instance isolation

Confirm that two release binary instances with different `--instance-id` values do not share state.

- [ ] Instance `host-a`: set display name "Alice" and create tournament "Alice's Game"
- [ ] Instance `client-b`: display name remains independent and host draft is not shared
- [ ] Instance `host-a`: complete a tournament and verify history is visible
- [ ] Instance `client-b`: History screen contains no `host-a` history
- [ ] Restart `host-a`: display name, host draft, and history restore
- [ ] Restart `client-b`: it remains unaffected by `host-a`'s restart
- [ ] Record both application data directories and verify they are distinct

---

## Checklist D: Host port conflict behavior

- [ ] Start `./target/release/desktop-poker --instance-id host-a` and begin hosting on port `43818`
- [ ] Start `./target/release/desktop-poker --instance-id host-b` and attempt to host on the same port
- [ ] Confirm host-b shows a clear bind error rather than crashing, hanging, or claiming success
- [ ] Inspect available debug/runtime health information and confirm the error has useful context without secrets
- [ ] Change host-b to a different port, such as `43819`, and confirm hosting succeeds

---

## Checklist E: Release production reachability and secret storage

- [ ] Launch a release build with attempted debug/probe query parameters
- [ ] Confirm layout probes, browser mocks, fake sessions, and the hidden debug route are not player-reachable
- [ ] Confirm ordinary Home, Host, Join, Settings, and Help routes work without development tooling
- [ ] Configure a low-privilege test provider key through a release build
- [ ] Restart and confirm non-secret provider settings restore
- [ ] Confirm the key is absent from settings JSON, logs, snapshots, debug output, and other app-data files
- [ ] Clear the provider and confirm key deletion succeeds or reports a visible error
- [ ] Force or simulate keychain unavailability and confirm no plaintext release fallback occurs

---

## Checklist F: Live NPC operation

- [ ] Complete a release tournament with at least one rule-based NPC
- [ ] Use distinct NPC profiles/styles and verify identity/profile association
- [ ] Confirm every seated NPC has a running decision loop
- [ ] Confirm NPC actions are legal and tournament progress does not require manual injection
- [ ] Confirm NPC hand history, elimination, and completion behavior
- [ ] When a local provider is available, observe at least one accepted legal LLM action
- [ ] Exercise provider unavailable/timeout/invalid-response fallback cases and confirm typed visible diagnostics
- [ ] Confirm no credential is logged and no disallowed fallback silently acts

---

*Checklist revised for the Cargo workspace and release-readiness baseline on 2026-07-26. Execution boxes intentionally remain unchecked until a current manual run is recorded.*
