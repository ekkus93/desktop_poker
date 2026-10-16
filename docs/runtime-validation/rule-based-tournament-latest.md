# Latest Rule-Based NPC Tournament Validation

- Result: **FAIL**
- Validated commit: `7ea4e0c1fe009a9e4544fb5fd10b96689a70413a`
- GitHub Actions run: `38883558711`
- Build outcome: `success`
- Tournament outcome: `failure`
- Evidence artifact: `linux-rule-based-npc-tournament-evidence`

## Failure

AssertionError: NPC runner emitted an error or fallback diagnostic: [npc-runner] npc-seat-2: submit_action rejected (window=aw-22, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-2: submit_action rejected (window=aw-22, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-2: submit_action rejected (window=aw-22, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-2: submit_action rejected (window=aw-22, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-2: submit_action rejected (window=aw-22, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-2: submit_action rejected (window=aw-22, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-2: submit_action rejected (window=aw-22, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-2: submit_action rejected (window=aw-22, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-2: submit_action rejected (window=aw-22, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] action window expired for player npc-seat-2 (window=aw-22)

## Executed checks

- **PASS** — tauri-driver became ready
- **PASS** — release binary created a real Tauri/WebKit session
- **PASS** — two unprofiled rule-based NPCs were seated and ready
- **PASS** — release table entered hand 1 with the production NPC runner active
- **FAIL** — rule-based NPC tournament
