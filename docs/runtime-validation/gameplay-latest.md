# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `7a8c4fa6aa7a91f8b3777a9208f41248e3c6da0e`
- GitHub Actions run: `30243269453`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T06:42:48.035968+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for all-in showdown settlement; last error: WebDriver POST /session/85c09c31-336a-48f6-97a3-ee6d0fbb7adf/execute/async error Disconnected from host: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/c1f55d78-4a16-443e-b644-b0342b62bbdc/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **FAIL** — full-game runtime smoke
