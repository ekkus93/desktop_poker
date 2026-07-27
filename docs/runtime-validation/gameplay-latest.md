# Latest Linux Release Full-Game Validation

- Result: **PASS**
- Validated commit: `0b49e7e888aa832145d8ef815ad8bc62419a26e1`
- GitHub Actions run: `30289835913`
- Build outcome: `success`
- Gameplay outcome: `success`
- Recorded at: `2026-07-27T17:43:39.122649+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Result summary

- Completed hands: `3`
- Host instance: `full-game-host-30289835913`
- Client instance: `full-game-client-30289835913`

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/096fe6e3-c644-42dd-9658-64bdcc45cb12/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **PASS** — both release instances render the same Tournament Complete winner
- **PASS** — fresh third profile contains no host/client hand history
- **PASS** — host and client history restore after release-process restart
