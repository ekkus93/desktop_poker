# FIX5 Responses — DESKTOP_POKER_CORE_EXTRACTION_FIX5_SPEC.md / _TODO.md

Fill in each `A:` line, then share the file back or paste your answers.

---

1. Q: **P0.2 pre-check** — Can you confirm whether `associated_data_json().unwrap_or_default()` still exists in `src-tauri/src/networking/runtime/client.rs`? FIX4 replaced 9 silent `continue` cases with `emit_protocol_warning`. If the AAD fallback was among them and already replaced, P0.2 may be purely a verification step.
   A:

2. Q: **projector.rs purity** — Should `projector.rs` be included in the P2.2 move? It produces public/private table projections — does it use any protocol message types, networking types, or anything that would make it ineligible for `poker-core`? Or is it pure domain logic over `TournamentState`?
   A:

3. Q: **TournamentController determinism** — Does `TournamentController::new(config)` initialize deterministically (fixed or config-provided seed), or does it seed from RNG at construction time? The P2.3 facade determinism test requires the former — if construction is non-deterministic the test will be flaky.
   A:

4. Q: **src-tauri/Cargo.toml workspace section** — Does the current `src-tauri/Cargo.toml` contain a `[workspace]` section? (Tauri projects sometimes include one for plugins.) If yes, P2.1 needs a step to remove or consolidate it before adding the root workspace manifest.
   A:

5. Q: **CLAUDE.md + lint-n-test skill update at P2.1** — After the workspace is created, `--manifest-path src-tauri/Cargo.toml` becomes the non-standard form. Should P2.1 include updating `CLAUDE.md` and `.claude/skills/lint-n-test/SKILL.md` to use `--workspace` (and `cargo test --workspace --all-targets`), or is that a separate task you want to handle manually?
   A:

6. Q: **P1.3 ordering** — The `memory.md` corrective entry (P1.3) is currently ordered after P0.x and P1.1–P1.2. Do you want to move it to the top so it's done first (the misleading FIX4-complete claim is corrected before any new work lands), or is the current ordering intentional?
   A:

7. Q: **P0.4 existing test helpers** — Which of the following already exist in `src-tauri/src/npc/provider_storage.rs` tests: `FailingSecretStore`, `InMemorySecretStore`, `openai_config`, `anthropic_config`, `settings_path`? The TODO says "adjust helper names to match existing tests" but it's unclear how many need to be created from scratch.
   A:

8. Q: **P0.1 — `NpcConfig::is_npc_player_id` existence** — The suggested `remove_npc_participants_atomic` implementation gates removal with `crate::npc::NpcConfig::is_npc_player_id(player_id)`. Does this function exist in the current codebase, or will it need to be added?
   A:

9. Q: **P0.1 — RunnerStarter injection approach** — The fourth rollback test ("add_npc_players rolls back if start_npc_runner fails") requires making the runner-start call injectable. Is there existing test infrastructure that makes this feasible, or would you prefer a simpler integration-level test approach (e.g., exercising a real spawn path that is designed to fail)?
   A:

10. Q: **P2.2 — glob re-exports vs explicit in `poker-core/src/lib.rs`** — The TODO snippet uses `pub use domain::*; pub use engine::*; pub use tournament::*;`. Glob re-exports create a flat namespace that can cause ambiguity with shared type names. Do you want glob re-exports (less import churn in `src-tauri`) or explicit re-exports / requiring callers to use `poker_core::domain::Foo` (clearer but more import changes)?
   A:
