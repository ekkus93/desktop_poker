# Latest Rule-Based NPC Tournament Validation

- Result: **PASS**
- Validated commit: `3dcf25a61f514bace60046b11e05991683d821d5`
- GitHub Actions run: `38885326320`
- Build outcome: `success`
- Tournament outcome: `success`
- Evidence artifact: `linux-rule-based-npc-tournament-evidence`

## Result summary

- Completed hands: `15`
- Rule-based NPC actions: `22`

## Executed checks

- **PASS** — tauri-driver became ready
- **PASS** — release binary created a real Tauri/WebKit session
- **PASS** — two unprofiled rule-based NPCs were seated and ready
- **PASS** — release table entered hand 1 with the production NPC runner active
- **PASS** — tournament completed across 15 hands with 22 committed rule-based NPC actions
- **PASS** — both rule-based NPC identities produced live accepted actions
- **PASS** — production runtime log contained no NPC error or fallback diagnostic
- **PASS** — final standings contain the human host and both rule-based NPC players
