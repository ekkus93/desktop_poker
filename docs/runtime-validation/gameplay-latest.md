# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `9afe88ffffa7e76852d9c51af55f832967593aa0`
- GitHub Actions run: `30291090972`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T17:55:25.399247+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for source text 'Player full-game-host-30291090972 won 120 chip(s).'; last error: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/da2a75b7-3d9f-45d3-badf-770f7774c3c2/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — all-in showdown attempt 3 settled with 4 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **PASS** — both release instances render the same Tournament Complete winner
- **PASS** — fresh third profile contains no host/client hand history
- **FAIL** — full-game runtime smoke
