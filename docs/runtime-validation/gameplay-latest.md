# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `37e7e29180d27b932ea1e9844bad2c6e817ba053`
- GitHub Actions run: `30239275656`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T05:22:25.689485+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for first folded hand to settle on both instances; last error: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/46e0b6ac-4257-489f-b304-aa50599c2ac2/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **FAIL** — full-game runtime smoke
