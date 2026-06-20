# Questions and Issues — DESKTOP_POKER_STABILIZATION_FIX2

These are questions and issues identified after reviewing `DESKTOP_POKER_STABILIZATION_FIX2_SPEC.md` and `DESKTOP_POKER_STABILIZATION_FIX2_TODO.md` against the current codebase, before implementation begins.

---

## 1. File path errors in the TODO

Several target file paths in the TODO do not match the actual codebase layout. I will use the correct paths when implementing, but noting them here so the TODO can be corrected if it is shared or used as a reference.

| TODO says | Actual path |
|-----------|-------------|
| `src-tauri/src/npc_runner.rs` (P0.4, P0.5, P1.1, P1.2, P1.3) | `src-tauri/src/npc/runner/mod.rs` |
| `src-tauri/src/llm_strategy.rs` (P1.2, P1.3) | `src-tauri/src/npc/llm_strategy.rs` |
| `src-tauri/src/app_state/npc_profiles.rs` (P1.4) | does not exist; profile listing logic is in `src-tauri/src/npc/profile_store.rs` |
| `src-tauri/src/app_state/profile_store.rs` (P1.4) | does not exist; correct path is `src-tauri/src/npc/profile_store.rs` |

---

## 2. P1.7 — Policy decision required before implementation

The spec offers three options for release-mode API key storage. This is a design decision that must be made before implementation:

**Option A (preferred by spec):** OS keychain/keyring in release builds; plaintext key file is dev-only.

**Option B (acceptable interim):** Release builds block API-key providers unless an explicit insecure plaintext mode flag is set; local providers without API keys remain available.

**Option C (temporary):** Release builds allow plaintext key file; UI displays a persistent warning before and after save; docs clearly state this is insecure and not final.

Which option should be implemented? This decision affects:
- the Rust provider storage layer (`src-tauri/src/npc/provider_storage.rs`),
- the Settings screen UI (`src/screens/DeviceSettingsScreen.tsx`),
- README documentation,
- and the test coverage required.

---

## 3. P2.1 — `npc/api_key.rs` appears to be dead code: confirm full deletion

`src-tauri/src/npc/mod.rs` exports `pub mod api_key;` and `api_key.rs` implements read/write of `claude-api-key.txt`. However, there are no callers of `npc::api_key` anywhere in the codebase outside of `api_key.rs` itself. Legacy key migration from `claude-api-key.txt` is already handled inside `provider_storage.rs`.

My intent is to delete `api_key.rs` entirely and remove the `pub mod api_key;` line from `npc/mod.rs`.

Is that correct, or does this module need to be kept for any migration/compatibility reason?

---

## Summary

Two items require answers before implementation can begin:

- **Question 2**: Which P1.7 key storage policy (A, B, or C)?
- **Question 3**: Confirm `npc/api_key.rs` can be fully deleted.

Item 1 (file path errors) is informational only and does not block implementation.
