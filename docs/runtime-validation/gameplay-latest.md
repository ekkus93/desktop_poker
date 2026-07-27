# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `267d1e7f99afdbfce57dc07723cc851840bd30e4`
- GitHub Actions run: `30240041166`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T05:38:52.740105+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: client has no legal all-in action: {'potTotal': 30, 'maxRaiseTo': 1010, 'deadlineEpochMs': 1785130787190, 'minRaiseTo': 40, 'callAmount': 10, 'ownerLabel': 'Player full-game-client-30240041166', 'legalActions': ['Fold', 'Call', 'Raise', 'All-in'], 'currentBet': 20, 'betOrRaiseLabel': 'Raise to 40+', 'checkOrCallLabel': 'Call 10'}

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/4d724eb8-379d-42ba-91f4-a8c2a2cfb448/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **FAIL** — full-game runtime smoke
