# FIX5 Responses — DESKTOP_POKER_CORE_EXTRACTION_FIX5_SPEC.md / _TODO.md

Filled answers for Claude Code. Apply these decisions exactly unless the current code has changed since the `desktop_poker-master_2606301520.zip` snapshot.

---

## 1. P0.2 pre-check — `associated_data_json().unwrap_or_default()`

Q: Can you confirm whether `associated_data_json().unwrap_or_default()` still exists in `src-tauri/src/networking/runtime/client.rs`? FIX4 replaced 9 silent `continue` cases with `emit_protocol_warning`. If the AAD fallback was among them and already replaced, P0.2 may be purely a verification step.

A: It still exists in the latest reviewed snapshot. In `src-tauri/src/networking/runtime/client.rs`, the private-payload decrypt path still calls `envelope.associated_data_json().unwrap_or_default().as_slice()` near the `crypto_provider.decrypt(...)` call. Therefore P0.2 is **not** just verification. Replace the fallback with a fail-loud protocol-warning path: if `associated_data_json()` returns `Err`, emit a `ProtocolWarning` such as `"private hole-card associated-data serialization failed"` and `continue` without attempting decrypt. Do not decrypt with empty AAD.

Use this shape:

```rust
let associated_data = match envelope.associated_data_json() {
    Ok(associated_data) => associated_data,
    Err(error) => {
        emit_protocol_warning(
            &sender,
            &mut protocol_warning_counts,
            &player_id,
            format!("private hole-card associated-data serialization failed: {error}"),
        );
        continue;
    }
};

let Ok(plaintext) = crypto_provider.decrypt(
    &private_key,
    &envelope.nonce,
    &envelope.ciphertext,
    associated_data.as_slice(),
) else {
    emit_protocol_warning(
        &sender,
        &mut protocol_warning_counts,
        &player_id,
        "private hole-card payload decrypt failed",
    );
    continue;
};
```

Acceptance addition: `rg -n "associated_data_json\(\).*unwrap_or_default|unwrap_or_default\(\).*associated_data" src-tauri/src/networking/runtime/client.rs` should return no hits.

---

## 2. `projector.rs` purity / move into `poker-core`

Q: Should `projector.rs` be included in the P2.2 move? It produces public/private table projections — does it use any protocol message types, networking types, or anything that would make it ineligible for `poker-core`? Or is it pure domain logic over `TournamentState`?

A: Yes, include `projector.rs` in the P2.2 move to `poker-core`. In the latest snapshot, `src-tauri/src/domain/projector.rs` only depends on standard collections and domain types such as `TournamentState`, `PublicState`, `PrivateState`, `ObserverProjection`, `ParticipantState`, and `DomainError`. It does not depend on Tauri, networking, protocol envelopes, crypto, app state, or UI types. That makes it core-eligible.

Move it as part of the domain/projection boundary. The projection rules are security-sensitive because they decide what each viewer is allowed to see. Keeping them in `poker-core` is correct because Android and desktop must share the same public/private projection semantics.

While moving it, do not broaden the projection API. The core may expose domain projections/snapshots, but it must not know how those snapshots are sent over the network.

---

## 3. `TournamentController` determinism

Q: Does `TournamentController::new(config)` initialize deterministically (fixed or config-provided seed), or does it seed from RNG at construction time? The P2.3 facade determinism test requires the former — if construction is non-deterministic the test will be flaky.

A: `TournamentController::new(...)` itself is deterministic in the reviewed snapshot. It builds the initial `TournamentState` and does not shuffle a deck during construction.

However, the first hand is **not deterministic by default**. `start_tournament(...)` calls `start_next_hand(...)`, and `start_next_hand(...)` uses `self.pending_deck.take().unwrap_or_else(Deck::shuffled)`. `Deck::shuffled()` uses `OsRng`. Therefore any test that starts a hand without injecting a deterministic deck will be non-deterministic.

For P2.3 facade tests, either:

1. keep the determinism test at the pre-start/lobby state only; or
2. inject a deterministic deck before starting the tournament; or
3. add a core facade constructor/test helper that accepts an explicit RNG seed or `Deck`.

Preferred for now: use deterministic-deck injection in tests, because the current code already has `TournamentController::set_next_deck(deck)`.

Example test intent:

```rust
let mut engine = PokerEngine::new_for_test(config, registered_players)?;
engine.set_next_deck_for_test(Deck::from_cards(fixed_cards)?);
engine.submit_command(EngineCommand::StartTournament { now_ms: 1_000 })?;
let snapshot_a = engine.snapshot_for(None)?;

let mut engine_again = PokerEngine::new_for_test(config, registered_players)?;
engine_again.set_next_deck_for_test(Deck::from_cards(fixed_cards)?);
engine_again.submit_command(EngineCommand::StartTournament { now_ms: 1_000 })?;
let snapshot_b = engine_again.snapshot_for(None)?;

assert_eq!(snapshot_a, snapshot_b);
```

Do not write a determinism test that relies on `Deck::shuffled()`.

---

## 4. `src-tauri/Cargo.toml` workspace section

Q: Does the current `src-tauri/Cargo.toml` contain a `[workspace]` section? (Tauri projects sometimes include one for plugins.) If yes, P2.1 needs a step to remove or consolidate it before adding the root workspace manifest.

A: No. In the latest reviewed snapshot, `src-tauri/Cargo.toml` contains `[package]`, `[lib]`, `[build-dependencies]`, `[dependencies]`, and `[dev-dependencies]`, but no `[workspace]` section.

For P2.1, add a **new root** `Cargo.toml` workspace manifest at the repository root. Do not add a nested workspace inside `src-tauri`.

Root manifest direction:

```toml
[workspace]
members = [
    "src-tauri",
    "crates/poker-core",
]
resolver = "2"
```

Then leave `src-tauri/Cargo.toml` as a package manifest and add `poker-core = { path = "../crates/poker-core" }` once the crate exists.

---

## 5. `CLAUDE.md` + lint-n-test skill update at P2.1

Q: After the workspace is created, `--manifest-path src-tauri/Cargo.toml` becomes the non-standard form. Should P2.1 include updating `CLAUDE.md` and `.claude/skills/lint-n-test/SKILL.md` to use `--workspace` (and `cargo test --workspace --all-targets`), or is that a separate task you want to handle manually?

A: Include this in P2.1. Do not leave `CLAUDE.md` or `.claude/skills/lint-n-test/SKILL.md` pointing at the old single-crate command style after creating the workspace.

The current `CLAUDE.md` explicitly says the Rust backend is a single crate and that Cargo commands need `--manifest-path src-tauri/Cargo.toml`. That will become wrong as soon as the workspace is created. The lint-n-test skill also uses the old manifest-path commands. Update both in the same patch as the workspace creation so future Claude Code runs do not keep validating only the Tauri crate.

Use commands like:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Keep the desktop-specific local run instruction using `cargo run --manifest-path src-tauri/Cargo.toml --no-default-features` if that remains the correct way to launch a second Tauri instance. Workspace-wide test/lint commands and desktop runtime commands are different use cases.

Also update the architecture rules in `CLAUDE.md`: after P2.1/P2.2, `poker-core` owns rules/state/projection; `src-tauri` owns Tauri commands, desktop app/session state, desktop storage, desktop networking, and adapters.

---

## 6. P1.3 ordering — corrective `memory.md` entry

Q: The `memory.md` corrective entry (P1.3) is currently ordered after P0.x and P1.1–P1.2. Do you want to move it to the top so it's done first (the misleading FIX4-complete claim is corrected before any new work lands), or is the current ordering intentional?

A: Move it to the top and do it first. Treat it as P0.0 or the first step of P0. The misleading `memory.md` entry should be corrected before more work lands so future sessions do not inherit a false “FIX4 complete / no unsafe silent failures remain” claim.

The corrective entry should be factual and should not claim FIX5 is complete. It should say that the previous FIX4 completion note was too strong and list the known remaining gaps: runner-spawn rollback, empty-AAD fallback, missing NPC hole-card fallback, provider full-save transaction ordering, persistence test noise, and incomplete host health coverage.

Use the repo’s existing timestamp rule: run `date -u +"%Y-%m-%dT%H:%M:%SZ"` immediately before editing `memory.md`. Do not fabricate timestamps.

---

## 7. P0.4 existing provider-storage test helpers

Q: Which of the following already exist in `src-tauri/src/npc/provider_storage.rs` tests: `FailingSecretStore`, `InMemorySecretStore`, `openai_config`, `anthropic_config`, `settings_path`? The TODO says "adjust helper names to match existing tests" but it's unclear how many need to be created from scratch.

A: In the latest reviewed snapshot:

- `anthropic_config(key: &str) -> LlmProviderConfig` exists.
- `settings_path(app_data_dir: &Path) -> PathBuf` exists in the module and is accessible to same-file tests.
- `legacy_key_path(...)` also exists.
- There is **no** generic `FailingSecretStore` with mode flags.
- There is **no** `InMemorySecretStore` by that name.
- There is **no** `openai_config(...)` helper.
- Existing test stores include `FailingWriteSecretStore`, `FailingDeleteSecretStore`, `TrackingSecretStore`, and `file_store(dir)` returning `PlaintextFileSecretStore`.

Use the existing helpers where they fit. For tests that need a successful store, `file_store(dir.path())` is acceptable. For delete-failure tests, use `FailingDeleteSecretStore`. For write-failure tests, use `FailingWriteSecretStore`. For tests that need in-memory tracking of deleted accounts, use `TrackingSecretStore`.

If you need an OpenAI config helper, add this small helper in the same test module:

```rust
fn openai_config(key: &str) -> LlmProviderConfig {
    LlmProviderConfig {
        settings: LlmProviderSettings {
            provider: LlmProviderType::OpenAi,
            endpoint_url: None,
            model: None,
        },
        api_key: Some(key.to_string()),
    }
}
```

If you need only OpenAI settings, do not create a full config helper; use inline `LlmProviderSettings` as the existing tests already do.

---

## 8. P0.1 — `NpcConfig::is_npc_player_id` existence

Q: The suggested `remove_npc_participants_atomic` implementation gates removal with `crate::npc::NpcConfig::is_npc_player_id(player_id)`. Does this function exist in the current codebase, or will it need to be added?

A: It already exists. In the latest reviewed snapshot, `src-tauri/src/npc/mod.rs` has:

```rust
impl NpcConfig {
    pub fn player_id(seat_index: u8) -> String {
        format!("npc-seat-{seat_index}")
    }

    pub fn is_npc_player_id(player_id: &str) -> bool {
        player_id.starts_with("npc-seat-")
    }
}
```

Use that helper in `remove_npc_participants_atomic`. Still validate the exact IDs passed to the removal method. Do not remove arbitrary human participants via this rollback path.

Preferred behavior:

```rust
for player_id in player_ids {
    if !crate::npc::NpcConfig::is_npc_player_id(player_id) {
        return Err(NetworkingError::new(format!(
            "refusing to remove non-NPC participant during NPC rollback: {player_id}"
        )));
    }
}
```

Then remove only those exact NPC IDs from participants, seats, and ready state.

---

## 9. P0.1 — RunnerStarter injection approach

Q: The fourth rollback test ("add_npc_players rolls back if start_npc_runner fails") requires making the runner-start call injectable. Is there existing test infrastructure that makes this feasible, or would you prefer a simpler integration-level test approach (e.g., exercising a real spawn path that is designed to fail)?

A: There is already partial infrastructure: `src-tauri/src/npc/runner/mod.rs` has `start_npc_runner_with_spawner(...)`, and there is a runner-level test that injects spawn failure. That only proves the runner helper returns `Err`; it does **not** prove `DesktopAppState::add_npc_players(...)` rolls back the already-added NPCs when runner startup fails.

For the app-level rollback test, add a small injection point instead of trying to make the real OS spawn path fail. Do not rely on thread limits or platform-specific spawn failure behavior; that would be flaky.

Preferred approach:

1. Define a small runner-start abstraction in `app_state` or `app_npc.rs`.
2. Store it in `DesktopAppState` behind an `Arc`.
3. Production uses the real `crate::npc::runner::start_npc_runner(...)`.
4. Tests construct `DesktopAppState` with a fake starter that returns `Err("injected runner start failure")`.
5. The test asserts that `add_npc_players(...)` returns `Err` and that no requested NPC IDs remain registered/seated/ready in the authoritative host state.

Keep the abstraction narrow. Do not over-engineer it.

Sketch:

```rust
type NpcRunnerStartResult = Result<std::thread::JoinHandle<()>, String>;

type NpcRunnerStarter = dyn Fn(
        Arc<crate::networking::HostServer>,
        Vec<crate::npc::NpcConfig>,
        Arc<std::sync::atomic::AtomicBool>,
        Arc<Mutex<Option<crate::npc::LlmProviderConfig>>>,
        Arc<Mutex<std::collections::BTreeMap<String, String>>>,
        Arc<Mutex<Option<String>>>,
        Arc<Mutex<Option<crate::app_state::NpcActionErrorDebug>>>,
    ) -> NpcRunnerStartResult
    + Send
    + Sync;
```

Or use a small trait if that is cleaner:

```rust
trait NpcRunnerStarter: Send + Sync {
    fn start(
        &self,
        host_server: Arc<crate::networking::HostServer>,
        npc_configs: Vec<crate::npc::NpcConfig>,
        stop: Arc<std::sync::atomic::AtomicBool>,
        api_key_holder: Arc<Mutex<Option<crate::npc::LlmProviderConfig>>>,
        shared_tilt: Arc<Mutex<std::collections::BTreeMap<String, String>>>,
        shared_fallback: Arc<Mutex<Option<String>>>,
        shared_action_error: Arc<Mutex<Option<crate::app_state::NpcActionErrorDebug>>>,
    ) -> Result<std::thread::JoinHandle<()>, String>;
}
```

The closure type alias is probably enough for this project.

Do not use a fake integration test that only asserts a fabricated rollback helper works. The test must execute the real `DesktopAppState::add_npc_players(...)` path.

---

## 10. P2.2 — glob re-exports vs explicit exports in `poker-core/src/lib.rs`

Q: The TODO snippet uses `pub use domain::*; pub use engine::*; pub use tournament::*;`. Glob re-exports create a flat namespace that can cause ambiguity with shared type names. Do you want glob re-exports (less import churn in `src-tauri`) or explicit re-exports / requiring callers to use `poker_core::domain::Foo` (clearer but more import changes)?

A: Use explicit modules and avoid broad glob re-exports. Clarity is more important than minimizing import churn during the extraction. `poker-core` should have stable module boundaries, not a large flat prelude that can create ambiguous names later.

Use this shape in `crates/poker-core/src/lib.rs`:

```rust
pub mod domain;
pub mod engine;
pub mod tournament;

pub use domain::{
    ActionType, BlindLevel, BlindSchedule, Card, HandState, PublicState, PrivateState,
    SnapshotState, TournamentConfig, TournamentPhase, TournamentState,
};

pub use engine::{
    evaluate_best_holdem_hand, legal_actions, settle_showdown, Deck, LegalActionContext,
};

pub use tournament::{
    ActionRequest, RegisteredPlayer, TournamentController, TournamentError,
};
```

That gives common callers a practical top-level API without dumping every internal helper into the root namespace.

Inside `src-tauri`, prefer module-qualified imports where it improves readability:

```rust
use poker_core::domain::{ActionType, TournamentState};
use poker_core::tournament::{ActionRequest, TournamentController};
```

Do not do this:

```rust
use poker_core::*;
```

Exception: test modules may use broader imports only if they stay local and do not obscure which module owns the type.
