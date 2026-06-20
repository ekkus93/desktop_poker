# Manual QA Checklist — Real Multiplayer

These checklists require a real desktop environment and cannot be automated. Complete them when validating a release candidate or after significant networking/protocol changes.

---

## Checklist A: Two local release instances on one machine

**Setup:** Build the release binary with `cargo build --manifest-path src-tauri/Cargo.toml --release`. Launch two instances with distinct IDs.

```bash
# Terminal 1 (host)
./src-tauri/target/release/desktop-poker --instance-id host-a

# Terminal 2 (client)
./src-tauri/target/release/desktop-poker --instance-id client-b
```

### Flow

- [ ] Host: Home → Host Tournament → fill in name and player count → Start hosting
- [ ] Host: Lobby shows host player in a seat, invite is visible and copyable
- [ ] Host: Click "Copy invite" and confirm the clipboard contains a `pkr1_...` payload
- [ ] Client: Home → Join Tournament → paste the invite → Check invite → Continue to lobby
- [ ] Both: Lobby shows two participants with correct names
- [ ] Both: Seat selection (host claims a seat; client claims a seat)
- [ ] Both: Ready toggle is reflected for both players
- [ ] Host: Click "Start Tournament" when all players are ready
- [ ] Both: Lobby transitions to Main Table
- [ ] Both: Hand play (fold, call, bet, raise) updates UI on both instances synchronously
- [ ] Both: Hand history accumulates correctly
- [ ] Both: Elimination is reflected in observer state for the eliminated player
- [ ] Both: Tournament completion screen shows correct standings on both instances

### Error path coverage

- [ ] Host port conflict: Start two host instances on the same port and confirm the second shows a clear error (not a silent hang)
- [ ] Client join with invalid invite: Paste garbage into the invite field and confirm the parser error is shown inline
- [ ] Host joins with a different `--instance-id` and confirm separate storage namespace (distinct display name, history)

---

## Checklist B: Two machines on the same LAN

**Setup:** Build the release binary on both machines. Confirm both are on the same LAN subnet and can reach each other on port `43818`.

### Flow

- [ ] Machine A (host): Start hosting, confirm LAN address shown in lobby is the machine's real LAN IP
- [ ] Machine B (client): Copy or manually type the `pkr1_...` invite payload from Machine A
- [ ] Machine B: Join → Check invite → Confirm host details show Machine A's IP and port
- [ ] Machine B: Continue to lobby → Lobby shows both participants
- [ ] Complete one full tournament hand-for-hand through to completion
- [ ] Verify both machines show the same final standings

### Disconnect/reconnect

- [ ] During lobby: disconnect Machine B's network adapter for 5 seconds → reconnect → lobby recovers, player re-appears as connected
- [ ] During live play: disconnect Machine B briefly during a hand → reconnect before reconnect window expires → play continues
- [ ] During live play: disconnect Machine B long enough to exceed the reconnect window → Machine B sees terminal disconnect error; Machine A lobby shows player as reconnecting, then eventually shows seat as stale
- [ ] Host (Machine A) shuts down mid-game → Machine B sees "Disconnected from host" terminal error with option to leave

### Reconnect window behavior

- [ ] A disconnected player who reconnects before expiry rejoins in their existing seat
- [ ] An eliminated player who disconnects and reconnects is readmitted as an observer
- [ ] After tournament completion: reconnect attempt by either party fails gracefully (session is gone)

---

## Checklist C: Release binary instance isolation

Confirm that two release binary instances with different `--instance-id` values do not share state.

- [ ] Instance `host-a`: set display name "Alice" and create a tournament named "Alice's Game"
- [ ] Instance `client-b`: display name remains the default and host draft is independent
- [ ] Instance `host-a`: complete a tournament — hand history is visible under History
- [ ] Instance `client-b`: History screen is empty (no cross-instance bleed)
- [ ] Restart Instance `host-a`: display name, host draft, and history are restored
- [ ] Instance `client-b` is unaffected by `host-a`'s restart

---

## Checklist D: Host port conflict behavior

- [ ] Start `./desktop-poker --instance-id host-a` and begin hosting on port `43818`
- [ ] Start `./desktop-poker --instance-id host-b` and attempt to host on the same port `43818`
- [ ] Confirm host-b shows a clear error when trying to start hosting (not a crash or silent failure)
- [ ] Change host-b to use a different port (e.g., `43819` in Settings) and confirm hosting succeeds

---

*Last updated: 2026-06-20*
