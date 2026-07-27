# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `5eefb73676e5678f135a4004ca2fb6f4eb13dfe0`
- GitHub Actions run: `30238792921`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T05:11:58.364483+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for first folded hand to settle on both instances; last error: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/336e06a7-a86f-449e-896f-bfa6aafd4277/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **FAIL** — full-game runtime smoke
