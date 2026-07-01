# FIX6 Responses — DESKTOP_POKER_CORE_EXTRACTION_FIX6_SPEC.md / _TODO.md

Fill in each `A:` line, then share this file back (or paste your answers).

---

1. Q: **P0.1 strictness** — Tests are currently 252/252 with no persistence noise. The `afterEach` teardown for `window.__TAURI_INTERNALS__` / `window.__TAURI__` in `setup.ts` is currently missing. Do you want this added as a defensive measure regardless of whether the symptom is currently visible, or only if we can reproduce the noise first?
   A: 

2. Q: **P0.3 health diagnostic destination** — When `sync_snapshots_after_lobby_mutation` fails (i.e., the NPC add mutation succeeded but broadcasting the updated snapshot to connected clients failed), should the error be: (a) recorded on `HostRuntimeHealth` so the frontend can surface it via `get_host_session_status`, or (b) emitted as a `tracing::warn!` and otherwise swallowed silently?
   A: 

3. Q: **P0.4 `client_connect.rs` timeout errors** — If `set_read_timeout` or `set_write_timeout` fails (this is OS-level and very rare in practice), should we: (a) propagate the error as a hard connection failure, or (b) log and continue connecting anyway?
   A: 

4. Q: **P2.1–P2.5 already done** — The poker-core workspace extraction (workspace Cargo.toml, module migration, PokerEngine facade, ANDROID_ARCHITECTURE.md, purity audit) was fully completed in the FIX5 session. Should these be treated as done (quick verification pass only), or do you want a fresh review and re-check of each item?
   A: 
