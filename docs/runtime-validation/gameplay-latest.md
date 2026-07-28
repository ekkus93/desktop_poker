# Latest Linux Release Full-Game Validation

- Result: **PASS**
- Validated commit: `24bc1a51d19c173b920fc56ffe27874046dbaf45`
- GitHub Actions run: `30323351107`
- Build outcome: `success`
- Gameplay outcome: `success`
- Recorded at: `2026-07-28T02:35:13.675169+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Result summary

- Completed hands: `3`
- Host instance: `full-game-host-30323351107`
- Client instance: `full-game-client-30323351107`

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/8611c646-26c2-4f64-83e0-1e16a4ce95f0/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **PASS** — both release instances render the same Tournament Complete winner
- **PASS** — fresh third profile contains no host/client hand history
- **PASS** — host and client history restore after release-process restart
