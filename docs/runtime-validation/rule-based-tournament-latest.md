# Latest Rule-Based NPC Tournament Validation

- Result: **PASS**
- Validated commit: `ed3aaf67064b61e7bb83e4a67ba20c0b94e89698`
- GitHub Actions run: `30373977939`
- Build outcome: `success`
- Tournament outcome: `success`
- Evidence artifact: `linux-rule-based-npc-tournament-evidence`

## Result summary

- Completed hands: `14`
- Rule-based NPC actions: `20`

## Executed checks

- **PASS** — tauri-driver became ready
- **PASS** — release binary created a real Tauri/WebKit session
- **PASS** — two unprofiled rule-based NPCs were seated and ready
- **PASS** — release table entered hand 1 with the production NPC runner active
- **PASS** — tournament completed across 14 hands with 20 committed rule-based NPC actions
- **PASS** — both rule-based NPC identities produced live accepted actions
- **PASS** — production runtime log contained no NPC error or fallback diagnostic
- **PASS** — final standings contain the human host and both rule-based NPC players
