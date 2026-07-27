# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `dfdf9fd7315936333b17a5bacba3baec87f136de`
- GitHub Actions run: `30238171697`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T04:57:17.287510+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for first folded hand to settle on both instances; last error: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/297a48e2-8021-4067-a146-f069cd9827bc/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **FAIL** — full-game runtime smoke
