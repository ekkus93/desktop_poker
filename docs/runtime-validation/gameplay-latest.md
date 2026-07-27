# Latest Linux Release Full-Game Validation

- Result: **PASS**
- Validated commit: `95cb24a3fd20b0097cad2ac50a9660bc750c6bb6`
- GitHub Actions run: `30304352625`
- Build outcome: `success`
- Gameplay outcome: `success`
- Recorded at: `2026-07-27T20:54:51.435909+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Result summary

- Completed hands: `3`
- Host instance: `full-game-host-30304352625`
- Client instance: `full-game-client-30304352625`

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/58330dd1-385c-4da9-8af2-67139e7e0388/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **PASS** — both release instances render the same Tournament Complete winner
- **PASS** — fresh third profile contains no host/client hand history
- **PASS** — host and client history restore after release-process restart
