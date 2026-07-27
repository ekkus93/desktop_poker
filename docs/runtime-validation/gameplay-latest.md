# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `4319285b3f4b9a8efd53a2457814608a26710630`
- GitHub Actions run: `30237756559`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T04:47:15.177932+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for first folded hand to settle on both instances; last error: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/ee029e85-8674-4c38-87a0-fc7cc4e84a89/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **FAIL** — full-game runtime smoke
