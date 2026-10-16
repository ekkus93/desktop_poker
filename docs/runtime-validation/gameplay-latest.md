# Latest Linux Release Full-Game Validation

- Result: **PASS**
- Validated commit: `568da27669b1fd046683542dbe0112f0da027e27`
- GitHub Actions run: `38886196031`
- Build outcome: `success`
- Gameplay outcome: `success`
- Recorded at: `2026-10-16T10:22:05.852902+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Result summary

- Completed hands: `6`
- Host instance: `full-game-host-38886196031`
- Client instance: `full-game-client-38886196031`

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: Tauri command 'submit_table_action' failed: [object Object]
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
