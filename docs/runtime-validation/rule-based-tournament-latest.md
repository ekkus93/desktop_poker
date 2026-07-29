# Latest Rule-Based NPC Tournament Validation

- Result: **FAIL**
- Validated commit: `2d1e2e4a7c3431b63f94b4b5c4e0042cd1918b81`
- GitHub Actions run: `30416201659`
- Build outcome: `success`
- Tournament outcome: `failure`
- Evidence artifact: `linux-rule-based-npc-tournament-evidence`

## Failure

AssertionError: NPC runner emitted an error or fallback diagnostic: [npc-runner] npc-seat-1: submit_action rejected (window=aw-21, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-21, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-21, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-21, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-21, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-21, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-21, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-21, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-21, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-21, action=Raise): rejected action mutated controller state; mutation was rolled back

## Executed checks

- **PASS** — tauri-driver became ready
- **PASS** — release binary created a real Tauri/WebKit session
- **PASS** — two unprofiled rule-based NPCs were seated and ready
- **PASS** — release table entered hand 1 with the production NPC runner active
- **FAIL** — rule-based NPC tournament
