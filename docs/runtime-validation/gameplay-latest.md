# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `d632d6c8355b9abda33bbe632a91aa38bb01386d`
- GitHub Actions run: `30285116388`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T16:39:38.135063+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Timed out waiting for source text 'Player full-game-client-30285116388 won 1940 chip(s).'; last error: None

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/c3830fda-b89e-47b1-bc76-a9ccc13eca6d/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **PASS** — all-in showdown attempt 2 settled with 3 synchronized hands
- **PASS** — tournament completed with matching standings and one eliminated observer
- **PASS** — both release instances render the same Tournament Complete winner
- **PASS** — fresh third profile contains no host/client hand history
- **FAIL** — full-game runtime smoke
