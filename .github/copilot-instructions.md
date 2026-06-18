# Copilot Instructions for `desktop_poker`

## Project goal

Build a **real desktop poker client/host** with **Tauri + Rust** for **single-table Sit 'n Go No-Limit Texas Hold'em**. This app is not just a mock UI or simulator shell. It must support real LAN play and multi-instance local testing.

## Build, test, and run

Rust backend lives in `src-tauri/` (single crate, not a workspace) — Cargo commands need `--manifest-path src-tauri/Cargo.toml`. Frontend is React 19 + TypeScript + Vite, driven via npm.

- Rust: `cargo fmt --manifest-path src-tauri/Cargo.toml`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`.
- Frontend: `npm run lint` (ESLint, `--max-warnings 0`), `npm run format` (Prettier), `npm run test` (Vitest), `npm run build` (`tsc && vite build`).
- Run the app: `npm run tauri dev` (debug) / `npm run tauri build` (release bundle).
- LLM integration tests in `src-tauri/src/npc/` are `#[ignore]`d and need a local Ollama; run with `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored`.

## Core architecture rules

- Use **Rust** as the source of truth for:
  - networking
  - protocol handling
  - canonical serialization
  - crypto
  - reconnect/resync
  - tournament/session orchestration
  - authoritative state projection
  - persistence
- The frontend should stay focused on:
  - rendering
  - screen flow
  - user interaction
  - transient UI state only
- Do **not** let frontend state become the source of truth for gameplay.

## MVP scope and product boundaries

- Variant: **No-Limit Texas Hold'em**
- Mode: **single-table Sit 'n Go**
- Players: **2 to 10**
- Authority model: **host-authoritative**
- Transport: **raw TCP over local LAN**
- Roster freezes once the tournament starts
- No rebuys or top-ups
- Eliminated players remain **read-only observers**
- Host and clients exchange **signed messages**
- Private hole cards are delivered through **encrypted recipient-only payloads**

## Compatibility source of truth

For any Android/Desktop interoperability work, use this precedence order:

1. **Current Android code**
2. **Current Android protocol/session tests**
3. `docs/ANDROID_DESKTOP_COMPAT_ANSWERS.md`
4. `README.md`
5. Older spec/todo markdown

If older markdown conflicts with the Android runtime behavior, **the Android code wins**.

## Canonical Android references

When implementing compatibility-sensitive behavior, mirror the current Android definitions described in `docs/ANDROID_DESKTOP_COMPAT_ANSWERS.md`, especially:

- `networking/protocol/ProtocolModels.kt`
- `crypto/CanonicalJson.kt`
- `networking/protocol/ProtocolSecurity.kt`
- `networking/session/TournamentSessionFlows.kt`
- `networking/QrJoinPayloadCodec.kt`
- `domain/JoinPayload.kt`
- protocol/session tests under `networking/src/test/...`

## Protocol and networking rules

- Match Android protocol field names and message semantics exactly.
- Keep the current `ProtocolMessageType` set compatible, including legacy compatibility behavior where documented.
- Public live-play updates should use **signed public events**.
- Snapshots are for:
  - join
  - reconnect
  - explicit resync
- Private hole cards should use the **encrypted private envelope** path.
- Preserve sender counters **per sender across the session**.
- Preserve one global authoritative host `serverSequence`.
- Mirror Android replay rejection behavior and reconnect acceptance rules.

## Canonical serialization and crypto rules

- Canonical signing bytes must match Android **byte-for-byte**.
- Use compact canonical JSON:
  - UTF-8
  - lexicographically sorted object keys at every nesting level
  - preserved array order
  - no insignificant whitespace
  - omitted null optional fields
- Sign the full envelope with `signature = null`.
- Use the Android-compatible crypto model:
  - **Ed25519** for signatures
  - **X25519** for key agreement
  - **ChaCha20-Poly1305** for encrypted private payloads
- Use **base64url without padding** for encoded keys, signatures, nonces, ciphertext, and opaque tokens where Android does.

## Join flow rules

- The canonical v1 join path is **direct payload join**.
- Treat room-code discovery / NSD as **not stable canonical behavior** unless implemented and proven on both platforms.
- Support the compact Android join payload format:
  - canonical JSON
  - gzip-compressed
  - base64url without padding
  - `pkr1_` prefix
- Legacy raw JSON payload parsing may be supported for compatibility, but the compact format is the default path.
- Current Android default host port is **43818**.

## Gameplay and rules

- Match Android tournament and hand semantics exactly where compatibility matters.
- Preserve:
  - fixed blind presets
  - host-authoritative timeout handling
  - short all-in semantics
  - showdown revealing all remaining hands in v1
  - observer-only behavior for eliminated players
- If the written spec and Android engine behavior diverge, treat Android runtime behavior as canonical until both are updated deliberately.

## Multi-instance and desktop-specific rules

- The desktop app must support **multiple instances on one machine**.
- Do not introduce single-instance assumptions that block local host/client testing.
- Keep per-instance identity, reconnect data, and local storage isolated.
- Both **debug and release** desktop builds should default to the **real LAN runtime path**.
- Debug or simulator tools must be **opt-in** and must not masquerade as the production path.
- Run a second local instance against the already-running Vite dev server with `cargo run --manifest-path src-tauri/Cargo.toml --no-default-features`; do **not** run `npm run tauri dev` twice (both try to bind Vite port 1420).
- Per-instance identity/storage is keyed by `DESKTOP_POKER_INSTANCE_ID`; `DESKTOP_POKER_JOIN_PAYLOAD` pre-fills a `pkr1_...` join payload for testing.

## Implementation guidance

- Prefer small, typed Rust modules aligned with the planned backend layout:
  - `domain`
  - `engine`
  - `tournament`
  - `protocol`
  - `networking`
  - `crypto`
  - `storage`
  - `interop`
  - `app_state`
  - `npc` (rule-based + LLM-driven computer players)
- The `npc` module drives computer players via a rule-based strategy plus an optional LLM path (multi-provider: Anthropic, OpenAI, Ollama, llama-server). LLM calls have a hard timeout and fall back to the rule-based strategy on any error — never block a hand on an LLM.
- Reuse helpers and shared abstractions instead of duplicating protocol or projection logic.
- Add tests for any protocol, crypto, reconnect, replay-protection, projection, or poker-rule change.
- Do **not** create mock-only tests that fail to execute meaningful production code. If a test only proves that fabricated doubles behave as configured, it does not count as validation and should not be added.
- Mocks are allowed only to isolate external boundaries while the test still executes real production behavior in the actual unit under test.
- Any critical behavior or regression must have at least one test that exercises the real production code path responsible for that behavior.
- When implementing a compatibility-sensitive behavior, cite the Android source/test being mirrored in comments or PR notes if the behavior is non-obvious.

## What to avoid

- Do not build a fake same-process-only happy path and present it as LAN support.

## Memory file
- You have access to a persistent memory file, memory.md, that stores context about the project, previous interactions, and user preferences.
- Use this memory to inform your decisions, remember user preferences, and maintain continuity across sessions. 
- Before sending back a response, update memory.md with any new relevant information learned during the interaction. Make sure to timestamp and format entries clearly.
- Include the GitHub Copilot model used in the entry in the heading line so memory history records both time and model (for example: `## 2024-06-01T12:00:00Z - GPT-5.4 - User prefers concise responses`).
- **NEVER fabricate or guess timestamps.** Always obtain the current time by running `date -u +"%Y-%m-%dT%H:%M:%SZ"` in the terminal immediately before writing the entry. If the entry describes a specific commit, use `git log -1 --format="%aI" <hash>` for that commit's actual timestamp.
- For each entry, add an ISO 8601 timestamp and a brief description of the information added. For example:
```

## 2024-06-01T12:00:00Z - GPT-5.4 - User prefers concise responses
- User has expressed a preference for concise, to-the-point answers without unnecessary elaboration.
```


- Do not default debug builds to simulator mode.
- Do not expose unfinished room-code discovery as if it is production-ready.
- Do not leak private hole-card data into public or observer projections.
- Do not invent desktop-only protocol changes without explicitly versioning and coordinating them with Android.
- Do not present mock-only tests as evidence that product behavior is correct.

## Memory file
- You have access to a persistent memory file, memory.md, that stores context about the project, previous interactions, and user preferences.
- Use this memory to inform your decisions, remember user preferences, and maintain continuity across sessions. 
- Before sending back a response, update memory.md with any new relevant information learned during the interaction. Make sure to timestamp and format entries clearly.
- Include the GitHub Copilot model used for the entry in the heading line so memory history records both time and model (for example: `## 2024-06-01T12:00:00Z - GPT-5.4 - User prefers concise responses`).
- **NEVER fabricate or guess timestamps.** Always obtain the current time by running `date -u +"%Y-%m-%dT%H:%M:%SZ"` in the terminal immediately before writing the entry. If the entry describes a specific commit, use `git log -1 --format="%aI" <hash>` for that commit's actual timestamp.
- For each entry, add an ISO 8601 timestamp and a brief description of the information added. For example:
```markdown

## 2024-06-01T12:00:00Z - GPT-5.4 - User prefers concise responses
- User has expressed a preference for concise, to-the-point answers without unnecessary elaboration.
```

