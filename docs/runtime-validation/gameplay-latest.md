# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `33ee1def953cdc8b3cceef1d881090e8b47d8d38`
- GitHub Actions run: `30300799463`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T20:05:08.368288+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for completed hand history to persist before leaving the table; last error: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/13c412d1-689f-46d2-b6ec-ebd38e194b05/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **FAIL** — full-game runtime smoke
