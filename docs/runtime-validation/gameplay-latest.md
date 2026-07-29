# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `909b284d71201bcabaec12f90cf15910fd673231`
- GitHub Actions run: `30417348067`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-29T02:41:19.762291+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for opponent response or all-in settlement; last error: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: Tauri command 'submit_table_action' failed: [object Object]
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **FAIL** — full-game runtime smoke
