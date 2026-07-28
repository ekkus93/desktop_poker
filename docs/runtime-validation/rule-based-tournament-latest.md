# Latest Rule-Based NPC Tournament Validation

- Result: **PASS**
- Validated commit: `50c0ad29052acccce18e1da7e84f8f1acbbab275`
- GitHub Actions run: `30409484107`
- Build outcome: `success`
- Tournament outcome: `success`
- Evidence artifact: `linux-rule-based-npc-tournament-evidence`

## Result summary

- Completed hands: `32`
- Rule-based NPC actions: `31`

## Executed checks

- **PASS** — tauri-driver became ready
- **PASS** — release binary created a real Tauri/WebKit session
- **PASS** — two unprofiled rule-based NPCs were seated and ready
- **PASS** — release table entered hand 1 with the production NPC runner active
- **PASS** — tournament completed across 32 hands with 31 committed rule-based NPC actions
- **PASS** — both rule-based NPC identities produced live accepted actions
- **PASS** — production runtime log contained no NPC error or fallback diagnostic
- **PASS** — final standings contain the human host and both rule-based NPC players
