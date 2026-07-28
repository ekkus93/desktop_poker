# Latest Linux Release Full-Game Validation

- Result: **PASS**
- Validated commit: `f35d8552648091211ed05c99ac8474ae07541be6`
- GitHub Actions run: `30335336680`
- Build outcome: `success`
- Gameplay outcome: `success`
- Recorded at: `2026-07-28T06:41:02.953138+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Result summary

- Completed hands: `6`
- Host instance: `full-game-host-30335336680`
- Client instance: `full-game-client-30335336680`

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: Tauri command 'submit_table_action' failed: raise exceeds remaining stack
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — all-in showdown attempt 3 settled with 4 synchronized hands
- **PASS** — all-in showdown attempt 4 settled with 5 synchronized hands
- **PASS** — all-in showdown attempt 5 settled with 6 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **PASS** — both release instances render the same Tournament Complete winner
- **PASS** — fresh third profile contains no host/client hand history
- **PASS** — host and client history restore after release-process restart
