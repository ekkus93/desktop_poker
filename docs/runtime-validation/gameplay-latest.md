# Latest Linux Release Full-Game Validation

- Result: **PASS**
- Validated commit: `d11ea6b69f194f5b1200fa2c68a0c476a9c9c57f`
- GitHub Actions run: `30325130509`
- Build outcome: `success`
- Gameplay outcome: `success`
- Recorded at: `2026-07-28T03:14:08.733132+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Result summary

- Completed hands: `3`
- Host instance: `full-game-host-30325130509`
- Client instance: `full-game-client-30325130509`

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/0ca15d44-865a-40f9-8857-f4775f21f31b/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **PASS** — both release instances render the same Tournament Complete winner
- **PASS** — fresh third profile contains no host/client hand history
- **PASS** — host and client history restore after release-process restart
