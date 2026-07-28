# Latest Rule-Based NPC Tournament Validation

- Result: **FAIL**
- Validated commit: `d02b69924f33d62e6f8ab4a7db31acb4f15fd5d2`
- GitHub Actions run: `30393331242`
- Build outcome: `success`
- Tournament outcome: `failure`
- Evidence artifact: `linux-rule-based-npc-tournament-evidence`

## Failure

AssertionError: NPC runner emitted an error or fallback diagnostic: [npc-runner] npc-seat-1: submit_action rejected (window=aw-34, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-34, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-34, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-34, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-34, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-34, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-34, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-34, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] npc-seat-1: submit_action rejected (window=aw-34, action=Raise): rejected action mutated controller state; mutation was rolled back | [npc-runner] action window expired for player npc-seat-1 (window=aw-34)

## Executed checks

- **PASS** — tauri-driver became ready
- **PASS** — release binary created a real Tauri/WebKit session
- **PASS** — two unprofiled rule-based NPCs were seated and ready
- **PASS** — release table entered hand 1 with the production NPC runner active
- **FAIL** — rule-based NPC tournament
