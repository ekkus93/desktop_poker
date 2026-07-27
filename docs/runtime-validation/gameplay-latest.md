# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `926cd7cea94f641876223ea7a48d11e930d3dcfb`
- GitHub Actions run: `30303068355`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T20:38:20.394114+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for completed hand history to persist before leaving the table; last error: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/f3b169e8-f875-4180-b4db-864aac88909d/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — all-in showdown attempt 3 settled with 4 synchronized hands
- **PASS** — all-in showdown attempt 4 settled with 5 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **FAIL** — full-game runtime smoke
