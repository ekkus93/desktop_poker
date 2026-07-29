# Latest Rule-Based NPC Tournament Validation

- Result: **PASS**
- Validated commit: `561c374e31414b3581355d8e83216d8aadee2616`
- GitHub Actions run: `30436381577`
- Build outcome: `success`
- Tournament outcome: `success`
- Evidence artifact: `linux-rule-based-npc-tournament-evidence`

## Result summary

- Completed hands: `12`
- Rule-based NPC actions: `19`

## Executed checks

- **PASS** — tauri-driver became ready
- **PASS** — release binary created a real Tauri/WebKit session
- **PASS** — two unprofiled rule-based NPCs were seated and ready
- **PASS** — release table entered hand 1 with the production NPC runner active
- **PASS** — tournament completed across 12 hands with 19 committed rule-based NPC actions
- **PASS** — both rule-based NPC identities produced live accepted actions
- **PASS** — production runtime log contained no NPC error or fallback diagnostic
- **PASS** — final standings contain the human host and both rule-based NPC players
