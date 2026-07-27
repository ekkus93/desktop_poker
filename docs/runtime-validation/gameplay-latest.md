# Latest Linux Release Full-Game Validation

- Result: **FAIL**
- Validated commit: `ecd7dfa2100ebce02d5efaf1b725a15a37451bee`
- GitHub Actions run: `30246145977`
- Build outcome: `success`
- Gameplay outcome: `failure`
- Recorded at: `2026-07-27T07:33:36.351965+00:00`
- Evidence artifact: `linux-release-full-game-evidence`

## Failure

AssertionError: Public table state diverged: {'currentHandNumber': (None, 3), 'streetLabel': ('Waiting', 'Preflop'), 'potTotal': (0, 990), 'boardCards': ([], [{'suitSymbol': '♠', 'label': 'Five of Spades', 'tone': 'dark', 'compactLabel': '5♠'}, {'suitSymbol': '♣', 'label': 'Seven of Clubs', 'tone': 'dark', 'compactLabel': '7♣'}, {'suitSymbol': '♦', 'label': 'Ten of Diamonds', 'tone': 'red', 'compactLabel': '10♦'}, {'suitSymbol': '♣', 'label': 'Jack of Clubs', 'tone': 'dark', 'compactLabel': 'J♣'}, {'suitSymbol': '♠', 'label': 'Jack of Spades', 'tone': 'dark', 'compactLabel': 'J♠'}])}

## Executed checks

- **PASS** — two isolated release instances completed host/join/seat/ready/start
- **PASS** — initial running-hand public state is synchronized
- **PASS** — exactly one action tray is visible; initial actor is host
- **PASS** — out-of-bounds raise was rejected without advancing state: WebDriver POST /session/7ff3b9ae-80be-42c5-917b-51055536a6f4/execute/async error raise exceeds remaining stack: None
- **PASS** — quick-size Min updates the legal raise amount without submitting
- **PASS** — Fold completed hand 1 with synchronized duplicate-free history
- **PASS** — all-in showdown attempt 1 settled with 2 synchronized hands
- **FAIL** — full-game runtime smoke
