# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `494da40e9cdb61a7f17213ceec53e7c52b5d595c`
- GitHub Actions run: `30236779452`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T04:25:18.841731+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

WebDriverError: WebDriver POST /session/72ea85a2-8948-42d7-b691-495eb38d06d7/execute/async error raise exceeds remaining stack: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **FAIL** — full-game runtime smoke
