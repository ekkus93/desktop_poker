# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `003c652666c3f3c3a7719cba4756fd4f504ce326`
- GitHub Actions run: `30244323091`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T07:02:25.515988+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for all-in showdown settlement; last error: WebDriver POST /session/7de84c34-c349-46d0-a3df-e369a491f1af/execute/async error Disconnected from host: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/b8b6c125-e116-44dc-b6e9-77cbce63b67d/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **FAIL** — full-game runtime smoke
