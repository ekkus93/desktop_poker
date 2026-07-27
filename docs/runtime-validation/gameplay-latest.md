# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `0169849a6f55c55cafb28174ece21efa520be17b`
- GitHub Actions run: `30236418491`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T04:17:26.461829+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

WebDriverError: WebDriver POST /session/16cd0206-d750-4ad9-a9b6-1b2e24686d7b/execute/async error raise exceeds remaining stack: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **FAIL** — full-game runtime smoke
