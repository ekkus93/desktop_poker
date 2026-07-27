# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `5e9870ea79d9cd53326be9ac4fbd505a73272b6b`
- GitHub Actions run: `30245131886`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T07:17:41.792336+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for all-in showdown settlement; last error: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/a248d0dc-4c81-4bee-91ae-6aed25e93dbb/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **FAIL** — full-game runtime smoke
