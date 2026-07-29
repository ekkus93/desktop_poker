# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `4d3b2c6146690b57f75db60ba5f53c27d12e8170`
- GitHub Actions run: `30416865473`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-29T02:30:42.659257+00:00`
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
