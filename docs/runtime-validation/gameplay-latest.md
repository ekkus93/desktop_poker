# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `991a680a0d581d542d3161dd5c6c8fbbebab55d1`
- GitHub Actions run: `30324567730`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-28T03:03:32.960998+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Tournament did not complete after 6 all-in showdowns

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/f0faabfc-c4d1-4da9-9945-88963a3b35ba/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — all-in showdown attempt 3 settled with 4 synchronized hands
- **PASS** — all-in showdown attempt 4 settled with 5 synchronized hands
- **PASS** — all-in showdown attempt 5 settled with 6 synchronized hands
- **PASS** — all-in showdown attempt 6 settled with 7 synchronized hands
- **FAIL** — full-game runtime smoke
