# Latest Rule-Based NPC Tournament Validation

- Result: **FAIL**
- Validated commit: `9cf9854cccecce34a5cc2135546a6fd1e98814f2`
- GitHub Actions run: `30372183384`
- Build outcome: `success`
- Tournament outcome: `failure`
- Evidence artifact: `linux-rule-based-npc-tournament-evidence`

## Failure

AssertionError: NPC runner emitted an error or fallback diagnostic: [npc-runner] npc-seat-1: submit_action rejected (window=aw-31, action=Raise): raise must satisfy minimum full raise sizing | [npc-runner] npc-seat-1: submit_action rejected (window=aw-31, action=Raise): raise must satisfy minimum full raise sizing | [npc-runner] npc-seat-1: submit_action rejected (window=aw-31, action=Raise): raise must satisfy minimum full raise sizing | [npc-runner] npc-seat-1: submit_action rejected (window=aw-31, action=Raise): raise must satisfy minimum full raise sizing | [npc-runner] npc-seat-1: submit_action rejected (window=aw-31, action=Raise): raise must satisfy minimum full raise sizing | [npc-runner] npc-seat-1: submit_action rejected (window=aw-31, action=Raise): raise must satisfy minimum full raise sizing | [npc-runner] npc-seat-1: submit_action rejected (window=aw-31, action=Raise): raise must satisfy minimum full raise sizing | [npc-runner] npc-seat-1: submit_action rejected (window=aw-31, action=Raise): raise must satisfy minimum full raise sizing | [npc-runner] npc-seat-1: submit_action rejected (window=aw-31, action=Raise): raise must satisfy minimum full raise sizing | [npc-runner] action window expired for player npc-seat-1 (window=aw-31)

## Executed checks

- **PASS** — tauri-driver became ready
- **PASS** — release binary created a real Tauri/WebKit session
- **PASS** — two unprofiled rule-based NPCs were seated and ready
- **PASS** — release table entered hand 1 with the production NPC runner active
- **FAIL** — rule-based NPC tournament
