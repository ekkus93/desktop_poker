# Latest Linux Release Full-Game Validation

- Result: **PASS**
- Validated commit: `7145b1209b3c5a792f41db4e5f336b44d7b6c439`
- GitHub Actions run: `30323565175`
- Build outcome: `success`
- Gameplay outcome: `success`
- Recorded at: `2026-07-28T02:40:36.208408+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Result summary

- Completed hands: `3`
- Host instance: `full-game-host-30323565175`
- Client instance: `full-game-client-30323565175`

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/d0618bdc-f542-4d3f-95e7-5b5711647778/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **PASS** — both release instances render the same Tournament Complete winner
- **PASS** — fresh third profile contains no host/client hand history
- **PASS** — host and client history restore after release-process restart
