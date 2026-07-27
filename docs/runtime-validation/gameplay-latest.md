# Latest Linux Release Full-Game Validation

- Result: **PASS**
- Validated commit: `147a40fa33e2b329ea9f37b687342de8cfdc4a3c`
- GitHub Actions run: `30300230996`
- Build outcome: `success`
- Gameplay outcome: `success`
- Recorded at: `2026-07-27T19:57:20.222222+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Result summary

- Completed hands: `4`
- Host instance: `full-game-host-30300230996`
- Client instance: `full-game-client-30300230996`

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/b8532dc3-ca7f-402b-aa49-c86c14ca0ed1/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — all-in showdown attempt 3 settled with 4 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **PASS** — both release instances render the same Tournament Complete winner
- **PASS** — fresh third profile contains no host/client hand history
- **PASS** — host and client history restore after release-process restart
