# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `4a54fa5f86a8e3eca9caf664759a460ad65240cb`
- GitHub Actions run: `30237109973`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T04:32:29.250884+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

WebDriverError: WebDriver POST /session/8c9ac555-e00e-4841-8b54-82959ec84c71/element/node-CC67AE2D-6492-46CB-B44A-1ADE5432DA8E/click returned HTTP 400: {"value":{"error":"element click intercepted","message":"","stacktrace":""}}

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/8c9ac555-e00e-4841-8b54-82959ec84c71/execute/async error raise exceeds remaining stack: None
- **FAIL** — full-game runtime smoke
