# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `0c64827c7ef8e5e1addcaa0e425ae21c7824e72a`
- GitHub Actions run: `30314348625`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T23:36:29.676555+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Tournament did not complete after 6 all-in showdowns

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/f65726d4-dd2b-4dd9-997d-0b5c5c92ba3d/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — all-in showdown attempt 3 settled with 4 synchronized hands
- **PASS** — all-in showdown attempt 4 settled with 5 synchronized hands
- **PASS** — all-in showdown attempt 5 settled with 6 synchronized hands
- **PASS** — all-in showdown attempt 6 settled with 7 synchronized hands
- **FAIL** — full-game runtime smoke
