# replies4.md

## Response to Claude Code — DESKTOP_POKER_STABILIZATION_FIX2 questions

Thanks for reviewing the FIX2 spec/TODO against the current codebase before implementation.

Please apply the clarifications below.

---

# 1. File path corrections in the FIX2 TODO

Your path corrections are accepted.

Please update both of these files so future implementation work does not use stale paths:

- `DESKTOP_POKER_STABILIZATION_FIX2_SPEC.md`
- `DESKTOP_POKER_STABILIZATION_FIX2_TODO.md`

Use these corrected paths:

| Incorrect path in TODO/spec | Correct path |
|---|---|
| `src-tauri/src/npc_runner.rs` | `src-tauri/src/npc/runner/mod.rs` |
| `src-tauri/src/llm_strategy.rs` | `src-tauri/src/npc/llm_strategy.rs` |
| `src-tauri/src/app_state/npc_profiles.rs` | `src-tauri/src/npc/profile_store.rs` |
| `src-tauri/src/app_state/profile_store.rs` | `src-tauri/src/npc/profile_store.rs` |

If there are submodules under `src-tauri/src/npc/runner/`, inspect/update the specific submodule files as needed. The TODO can name `src-tauri/src/npc/runner/mod.rs` as the primary entry point, but implementation should follow the actual module structure.

This is a documentation correction only. It should not change the intended behavior of any task.

---

# 2. P1.7 API-key storage policy decision

Implement **Option A** as the target behavior:

> Release builds must use OS keychain/keyring storage for API keys. Plaintext key-file storage must not be the normal release behavior.

## Required policy

### Release builds

Release builds must not silently store API keys in plaintext local files.

Use OS keychain/keyring storage for API keys in release builds.

If keychain storage is unavailable or fails, saving an API-key provider must fail with a clear user-visible error. Do not silently fall back to a plaintext key file.

Local providers that do not need API keys, such as Ollama or llama-server, should continue to work without keychain.

### Debug/development builds

Debug builds may continue to support plaintext key-file storage temporarily, but only as an explicitly marked development/insecure storage mode.

The UI and docs must make this clear.

Do not present debug plaintext storage as production-safe.

### No Option C

Do not implement Option C as the final behavior.

Persistent warning plus plaintext storage in release builds is not enough. A GUI user may not see stderr warnings, and this app should not normalize plaintext API-key storage for release.

### Acceptable implementation details

Using a Rust keychain/keyring crate is acceptable if it works cleanly with the Tauri app.

Suggested behavior:

- Store non-secret provider metadata in the existing provider config JSON.
- Store secret API key material in OS keychain/keyring.
- Return provider config to the frontend without secret material.
- Never expose the API key in debug state, logs, frontend state, or serialized config.
- Clearing provider config must remove/clear the keychain entry if one exists.
- Replacing the key must update the keychain entry.
- Editing non-secret provider fields while leaving the API-key input blank must preserve the existing key.

### If keychain integration becomes unexpectedly large

If implementing OS keychain support becomes too large or platform-blocking, stop and report the blocker.

Do **not** silently downgrade to plaintext release storage.

As an interim fallback only, implement **Option B**:

> Release builds block API-key provider save unless an explicit insecure plaintext mode flag is enabled.

But this should be treated as an interim implementation, not the preferred result.

If Option B is used temporarily:

- local providers without API keys must still work,
- API-key providers must show a clear error explaining that secure key storage is unavailable,
- insecure plaintext mode must require explicit opt-in,
- docs must clearly state that it is not release-safe.

---

# 3. Confirm deletion of `src-tauri/src/npc/api_key.rs`

Yes, delete it if your search is accurate.

If `src-tauri/src/npc/api_key.rs` has no callers outside itself and legacy migration from `claude-api-key.txt` is already handled by `provider_storage.rs`, then keeping this module is more harmful than helpful.

Please:

- delete `src-tauri/src/npc/api_key.rs`,
- remove `pub mod api_key;` from `src-tauri/src/npc/mod.rs`,
- remove any tests that only test the dead module,
- keep any required legacy migration logic inside `provider_storage.rs`,
- verify no current production path reads or writes `claude-api-key.txt`,
- update docs if they mention the old file.

Do not keep two active API-key storage paths.

If you discover a real hidden dependency during compile/tests, stop and report it rather than preserving the dead module silently.

---

# 4. Update the FIX2 docs before implementation

Before implementing the code changes, please update:

- `DESKTOP_POKER_STABILIZATION_FIX2_SPEC.md`
- `DESKTOP_POKER_STABILIZATION_FIX2_TODO.md`

Required doc updates:

1. Correct the stale file paths listed above.
2. Change P1.7 from “choose Option A/B/C” to the actual decision:
   - primary target: Option A, OS keychain/keyring in release builds,
   - no silent plaintext fallback in release,
   - Option B only if keychain integration is blocked and explicitly reported.
3. Add deletion of `src-tauri/src/npc/api_key.rs` as the expected P2.1 action, assuming compile/tests confirm it is dead code.

---

# 5. Validation commands

Keep using the project-root validation command style:

```bash
npm run lint
npm run build
npm run test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Do not replace these with `cd src-tauri` commands.

---

# 6. Implementation guardrails

For the key-storage work, avoid these unsafe patterns:

```rust
keychain_write(...).or_else(|_| write_plaintext_key(...))
```

```rust
keychain_read(...).ok()
```

```rust
let _ = remove_keychain_secret(...);
```

```rust
println!("{:?}", provider_config_with_secret)
```

Failure to read/write/delete a secret must be visible and must not be converted into “not configured” unless the secret truly does not exist.

For deletion of `npc/api_key.rs`, avoid keeping compatibility shims unless there is a real migration requirement. If migration exists, it belongs in `provider_storage.rs`, not in a second active key-storage API.
