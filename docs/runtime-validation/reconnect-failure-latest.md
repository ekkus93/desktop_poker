# Latest Reconnect Protocol and Release Validation

- Result: **FAIL**
- Validated commit: `e24983d4f262da2fce4db6307cc78bb9afd1123d`
- GitHub Actions run: `30330945448`
- Protocol tests: `success`
- Release build: `success`
- Release reconnect matrix: `failure`
- Evidence artifact: `reconnect-protocol-and-release-failure-evidence`

## Failure

AssertionError: post-reconnect client action tray lacks checkOrCall: {'potTotal': 30, 'maxRaiseTo': 1010, 'deadlineEpochMs': 1785216339569, 'minRaiseTo': 40, 'callAmount': 10, 'ownerLabel': 'Failure Matrix Client', 'legalActions': ['Fold', 'Call', 'Raise', 'All-in'], 'currentBet': 20, 'betOrRaiseLabel': 'Raise to 40+', 'checkOrCallLabel': 'Call 10'}

## Executed checks

- **PASS** — host release instance launched
- **PASS** — client release instance launched
- **PASS** — unreachable-host join failed explicitly without retaining a client session
- **PASS** — lobby reconnect replaced the TCP tuple and restored a usable client session
- **FAIL** — release reconnect matrix
