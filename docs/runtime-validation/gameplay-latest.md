# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `d618f663e7120a25c3066950ce1f0ddb99c17605`
- GitHub Actions run: `30240510962`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T05:48:59.622571+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: host cannot call the all-in: {'potTotal': 1030, 'maxRaiseTo': None, 'deadlineEpochMs': 1785131397871, 'minRaiseTo': None, 'callAmount': 970, 'ownerLabel': 'Player full-game-host-30240510962', 'legalActions': ['Fold', 'All-in', 'allIn'], 'currentBet': 1010, 'betOrRaiseLabel': 'Raise to 970+', 'checkOrCallLabel': 'Call 970'}

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/19acf3c1-39c6-47fa-b478-8a3bb0305630/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **FAIL** — full-game runtime smoke
