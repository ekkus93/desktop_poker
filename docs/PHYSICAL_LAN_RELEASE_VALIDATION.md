# Desktop Poker Physical LAN Release Validation

## Purpose

This procedure is the required manual evidence for `DP-RR-P0-004`. It validates two matching release artifacts on two separate physical Linux machines over a real LAN path.

A virtual machine pair, two processes on one machine, loopback, SSH port forwarding, containers, network namespaces, browser mocks, or manually edited state do **not** satisfy this gate.

## Required equipment

- Two physical x86-64 Linux machines on the same LAN.
- The same Desktop Poker Debian package on both machines.
- A camera or screenshot tool on each machine.
- Shell access on both machines.
- Permission to open TCP port `43818` on the host machine for the duration of the test.

Use these role names throughout the evidence:

- **Machine A:** host
- **Machine B:** client

## Acceptance criteria

The result is `PASS` only when all of the following are true:

1. Both machines install the exact same `.deb` SHA-256.
2. Both installed binaries have the exact same SHA-256.
3. Machine B connects to Machine A through Machine A's non-loopback LAN address and TCP port `43818`.
4. The two players claim distinct seats, become ready, and start a tournament.
5. Private hole cards remain visible only to their owning player.
6. Public state remains synchronized: hand number, board, pot, action owner, history, elimination, and standings.
7. At least one illegal action is rejected without advancing authoritative state.
8. The tournament reaches `Tournament Complete` without manual state edits.
9. Both machines display the same winner and final standings.
10. The host/client logs contain no panic, silent fallback, protocol-integrity error, or unexplained disconnect.

Any missing evidence is `NOT RUN`. Any mismatch, crash, confidentiality leak, unexplained disconnect, or stale playable state is `FAIL`.

## 1. Create the evidence directories

Run on both machines, using the appropriate role:

```bash
ROLE=host   # use ROLE=client on Machine B
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
EVIDENCE="$HOME/desktop-poker-physical-lan-${ROLE}-${STAMP}"
mkdir -p "$EVIDENCE/screenshots"
printf '%s\n' "$EVIDENCE"
```

Keep the terminal open. Reuse the printed path in every command below.

## 2. Record machine and package identity

Run on both machines:

```bash
{
  date -u --iso-8601=seconds
  uname -a
  cat /etc/os-release
  printf '\nDesktop Poker package:\n'
  dpkg-query -W -f='${Package}\t${Version}\t${Architecture}\t${Status}\n' desktop-poker
  printf '\nInstalled binary:\n'
  file /usr/bin/desktop-poker
  sha256sum /usr/bin/desktop-poker
  printf '\nDesktop entry:\n'
  grep -E '^(Name|Exec|Icon)=' /usr/share/applications/desktop-poker.desktop
} | tee "$EVIDENCE/system-and-package.txt"
```

Record the original package file as well. Replace the path with the actual `.deb` location:

```bash
DEB="$HOME/Downloads/desktop-poker_0.1.0_amd64.deb"
test -f "$DEB"
sha256sum "$DEB" | tee "$EVIDENCE/debian-package.sha256"
dpkg-deb --info "$DEB" > "$EVIDENCE/debian-package.info.txt"
```

Before continuing, compare both machines:

```text
Machine A .deb SHA-256     = Machine B .deb SHA-256
Machine A binary SHA-256   = Machine B binary SHA-256
```

Stop with `FAIL` if either pair differs.

## 3. Record the real LAN path

### Machine A — host

```bash
{
  ip -4 address show
  printf '\nRoutes:\n'
  ip -4 route show
  printf '\nSelected source address:\n'
  ip -4 route get 1.1.1.1
} | tee "$EVIDENCE/network-before.txt"
```

Choose Machine A's LAN address. It must not be any of the following:

- `127.0.0.0/8`
- a container bridge address
- a VPN address
- a public relay or forwarded address

Store it for later commands:

```bash
HOST_LAN_IP="192.168.1.50"  # replace with Machine A's actual LAN address
printf '%s\n' "$HOST_LAN_IP" | tee "$EVIDENCE/host-lan-ip.txt"
```

If UFW is active, open only the Desktop Poker TCP port. Prefer limiting the rule to the local subnet:

```bash
sudo ufw status verbose | tee "$EVIDENCE/ufw-before.txt"
# Example for a 192.168.1.0/24 LAN:
sudo ufw allow from 192.168.1.0/24 to any port 43818 proto tcp
sudo ufw status numbered | tee "$EVIDENCE/ufw-during-test.txt"
```

Do not disable the firewall globally.

### Machine B — client

```bash
{
  ip -4 address show
  printf '\nRoutes:\n'
  ip -4 route show
  printf '\nRoute to host:\n'
  ip -4 route get "$HOST_LAN_IP"
  printf '\nPing preflight:\n'
  ping -c 4 "$HOST_LAN_IP"
} | tee "$EVIDENCE/network-before.txt"
```

A blocked ICMP ping alone is not a failure. TCP connectivity after the host starts is authoritative.

## 4. Start the installed release applications with isolated identities

Launch from terminals so stdout/stderr is retained. Do not use a development build.

### Machine A

```bash
DESKTOP_POKER_INSTANCE_ID="physical-lan-host-${STAMP}" \
  /usr/bin/desktop-poker \
  >"$EVIDENCE/desktop-poker-host.log" 2>&1 &
APP_PID=$!
printf '%s\n' "$APP_PID" | tee "$EVIDENCE/application.pid"
```

### Machine B

```bash
DESKTOP_POKER_INSTANCE_ID="physical-lan-client-${STAMP}" \
  /usr/bin/desktop-poker \
  >"$EVIDENCE/desktop-poker-client.log" 2>&1 &
APP_PID=$!
printf '%s\n' "$APP_PID" | tee "$EVIDENCE/application.pid"
```

Confirm the installed binary is the running process:

```bash
ps -fp "$APP_PID" | tee "$EVIDENCE/application-process.txt"
readlink -f "/proc/$APP_PID/exe" | tee "$EVIDENCE/application-executable.txt"
test "$(readlink -f "/proc/$APP_PID/exe")" = /usr/bin/desktop-poker
```

## 5. Host the tournament on Machine A

In the Desktop Poker UI on Machine A:

1. Select **Host a table**.
2. Confirm the displayed host address is exactly `$HOST_LAN_IP`.
3. Use TCP port `43818`.
4. Set a two-player tournament.
5. Use a starting stack of `1000`.
6. Use the **Fast** blind preset.
7. Use a 15-second turn timer.
8. Use the display name `Physical LAN Host`.
9. Start hosting.
10. Save a screenshot of the lobby and invitation as `screenshots/host-lobby.png`.

While the lobby is open, record the listener on Machine A:

```bash
{
  ss -ltnp 'sport = :43818'
  printf '\nApplication process:\n'
  ps -fp "$APP_PID"
} | tee "$EVIDENCE/host-listener.txt"
```

The listener must be reachable through `$HOST_LAN_IP`; a listener restricted to `127.0.0.1` is `FAIL`.

## 6. Join from Machine B

Copy the invitation from Machine A to Machine B without editing it.

Before pressing **Join**, prove TCP reachability from Machine B:

```bash
nc -vz -w 5 "$HOST_LAN_IP" 43818 \
  2>&1 | tee "$EVIDENCE/tcp-connectivity.txt"
```

If `nc` is unavailable, install the distribution's `netcat-openbsd` package. Do not replace this with loopback testing.

In the Desktop Poker UI on Machine B:

1. Select **Join a table**.
2. Paste the exact invitation.
3. Use the display name `Physical LAN Client`.
4. Join.
5. Save a screenshot as `screenshots/client-joined-lobby.png`.

On Machine A, save `screenshots/host-client-admitted.png` showing both participants.

Stop with `FAIL` if the invitation advertises a loopback, VPN, container, or different host address.

## 7. Start play and verify confidentiality

1. Machine A claims seat 0.
2. Machine B claims seat 1.
3. Both players set ready.
4. Machine A starts the tournament.
5. Save simultaneous screenshots from both machines before the first action:
   - `screenshots/host-hand-1.png`
   - `screenshots/client-hand-1.png`

Compare the screenshots and record the result in `confidentiality.txt`:

```text
PASS/FAIL — Host sees only host hole cards.
PASS/FAIL — Client sees only client hole cards.
PASS/FAIL — Board, pot, hand number, dealer/action indicators match.
PASS/FAIL — No opponent hole-card rank or suit appears in UI, logs, or error text.
```

Any private-card leak is an immediate `FAIL`; do not continue merely to gather a final standing.

## 8. Exercise legal and rejected actions

During a live action window:

1. Submit at least one normal check/call or fold.
2. Submit at least one legal raise.
3. Attempt one raise outside the displayed legal bounds.
4. Confirm the illegal raise is rejected visibly.
5. Confirm the hand number, board, pot, and action owner did not advance because of the rejected action.
6. Save screenshots on both machines before and after the rejection.

Record:

```text
Rejected action attempted:
Displayed error:
Hand number before/after:
Pot before/after:
Action owner before/after:
Result: PASS / FAIL
```

Save this as `illegal-action-check.txt` on both machines.

## 9. Complete the tournament

Play normally until one player is eliminated and both machines display **Tournament Complete**.

Do not:

- edit application data;
- inject commands through debug tools;
- restart into a fabricated final state;
- change firewall/NAT rules during the hand;
- substitute screenshots from another run.

Save final screenshots:

- Machine A: `screenshots/host-tournament-complete.png`
- Machine B: `screenshots/client-tournament-complete.png`

Create `final-comparison.txt` on each machine:

```text
Tournament name:
Winner:
Rank 1 name and chip count:
Rank 2 name and chip count:
Completed hand count:
Final history summary:
Result: PASS / FAIL
```

The two files must agree exactly on all authoritative values.

## 10. Capture final network and log evidence

Run on Machine A:

```bash
{
  date -u --iso-8601=seconds
  ss -tnp '( sport = :43818 or dport = :43818 )'
  printf '\nListener:\n'
  ss -ltnp 'sport = :43818'
} | tee "$EVIDENCE/network-after.txt"
```

Run on Machine B:

```bash
{
  date -u --iso-8601=seconds
  ss -tnp 'dport = :43818'
  printf '\nRoute to host:\n'
  ip -4 route get "$HOST_LAN_IP"
} | tee "$EVIDENCE/network-after.txt"
```

Scan logs on both machines:

```bash
{
  printf '%s\n' '=== panic/error/fallback scan ==='
  grep -Ein 'panic|fatal|fallback|protocol.*error|integrity.*error|disconnect|reconnect|keychain.*error' \
    "$EVIDENCE"/desktop-poker-*.log || true
} | tee "$EVIDENCE/log-scan.txt"
```

Every matching line must be explained in `log-review.txt`. An unexplained integrity error, silent fallback, panic, or disconnect is `FAIL`.

## 11. Stop the applications and restore the firewall

Run on both machines:

```bash
kill "$APP_PID"
wait "$APP_PID" || true
```

On Machine A, remove only the temporary UFW rule that was added for this test. Use the rule number recorded in `ufw-during-test.txt`:

```bash
sudo ufw status numbered
sudo ufw delete NUMBER
sudo ufw status verbose | tee "$EVIDENCE/ufw-after.txt"
```

## 12. Write the result manifest

Create `$EVIDENCE/result.txt` on each machine:

```text
Desktop Poker physical LAN release validation
Role: host / client
UTC start:
UTC finish:
Source commit:
Debian package SHA-256:
Installed binary SHA-256:
Host LAN address:
Host TCP port: 43818
Peer machine hostname:
Tournament completed: PASS / FAIL / NOT RUN
Private-card isolation: PASS / FAIL / NOT RUN
Public-state synchronization: PASS / FAIL / NOT RUN
Illegal-action rejection: PASS / FAIL / NOT RUN
Matching final standings: PASS / FAIL / NOT RUN
Logs reviewed: PASS / FAIL / NOT RUN
Overall result: PASS / FAIL / NOT RUN
Failure details:
Tester:
```

`Overall result` may be `PASS` only when every acceptance criterion at the top of this document passes.

## 13. Package the evidence

Run on both machines:

```bash
parent="$(dirname "$EVIDENCE")"
name="$(basename "$EVIDENCE")"
tar -C "$parent" -czf "$parent/${name}.tar.gz" "$name"
sha256sum "$parent/${name}.tar.gz" | tee "$parent/${name}.tar.gz.sha256"
```

Retain both archives. The host and client evidence must be reviewed together before `DP-RR-P0-004` is marked complete.
