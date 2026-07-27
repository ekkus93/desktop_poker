# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `2e2aa4b5a682c92ba5ee539b34f0f998e7eccfe7`
- GitHub Actions run: `30310372713`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T22:24:41.682714+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for completed hand history to persist before leaving the table; last error: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/1f829839-1450-4767-b4f4-1dc983bfa3af/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — all-in showdown attempt 3 settled with 4 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **FAIL** — full-game runtime smoke
