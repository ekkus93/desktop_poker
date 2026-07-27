# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `7147e67f8ac9f8599ffa66f9c606f538b431d861`
- GitHub Actions run: `30285939351`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T16:49:50.626915+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for source text 'Player full-game-client-30285939351 won 1940 chip(s).'; last error: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/ca6fd622-c7c6-43bb-9c96-0e70d765ef35/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **PASS** — both release instances render the same Tournament Complete winner
- **PASS** — fresh third profile contains no host/client hand history
- **FAIL** — full-game runtime smoke
