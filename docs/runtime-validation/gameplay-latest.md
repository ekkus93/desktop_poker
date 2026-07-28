# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `db68202c02a5294eb0a0a3c6d77a8a9ebeff1af2`
- GitHub Actions run: `30324215120`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-28T02:55:16.127653+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for completed hand history to persist before leaving the table; last error: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/777c897a-8581-4451-b187-20e061c784dd/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — all-in showdown attempt 3 settled with 4 synchronized hands
- **PASS** — all-in showdown attempt 4 settled with 5 synchronized hands
- **PASS** — all-in showdown attempt 5 settled with 6 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **FAIL** — full-game runtime smoke
