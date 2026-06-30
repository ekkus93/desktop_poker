# Desktop Poker Runtime Hardening Fix 4 TODO

This TODO is intentionally explicit for Claude Code. Do the tasks in priority order. Prefer small, test-backed patches. Do not introduce new hidden fallbacks to make tests pass.

## P0.1 — Disable browser mocks outside dev/test

**Files:**

- `src/api/desktop.ts`
- `src/api/desktop.test.ts`
- possibly `src/probe/LayoutProbeApp.tsx`

### Problem

The production API bridge currently checks `window.__DESKTOP_POKER_BROWSER_MOCKS__`. This lets a global object intercept core runtime operations.

### Required change

Gate browser mocks behind Vite dev/test mode.

Suggested replacement in `src/api/desktop.ts`:

```ts
function browserMocksAllowed(): boolean {
  return import.meta.env.DEV || import.meta.env.MODE === "test";
}

function getBrowserMocks() {
  if (!browserMocksAllowed()) {
    return undefined;
  }

  if (typeof window === "undefined") {
    return undefined;
  }

  return window.__DESKTOP_POKER_BROWSER_MOCKS__;
}
```

### Tests

Add a test that proves the helper refuses mocks outside dev/test. Because `import.meta.env` is compile-time-ish under Vite, the cleanest route may be to extract the decision into a pure helper:

```ts
export function browserMocksAllowedForEnv(env: {
  DEV?: boolean;
  MODE?: string;
}): boolean {
  return env.DEV === true || env.MODE === "test";
}

function browserMocksAllowed(): boolean {
  return browserMocksAllowedForEnv(import.meta.env);
}
```

Then test:

```ts
it("disables browser mocks in production-like env", () => {
  expect(browserMocksAllowedForEnv({ DEV: false, MODE: "production" })).toBe(false);
});

it("allows browser mocks in test env", () => {
  expect(browserMocksAllowedForEnv({ DEV: false, MODE: "test" })).toBe(true);
});

it("allows browser mocks in dev env", () => {
  expect(browserMocksAllowedForEnv({ DEV: true, MODE: "development" })).toBe(true);
});
```

### Acceptance

- `npm run build` passes.
- Existing browser mock tests pass.
- No direct use of `window.__DESKTOP_POKER_BROWSER_MOCKS__` remains outside `getBrowserMocks()` and test/probe setup.

---

## P0.2 — Make provider settings-only save transactional

**Files:**

- `src-tauri/src/npc/provider_storage.rs`
- tests in the same file

### Problem

`save_provider_settings_only` logs stale-key deletion errors and still writes new settings. That can leave the old key behind while the UI believes the provider changed cleanly.

### Required change

If existing settings are unreadable/invalid, return an error. If provider changes and old-key deletion fails, return an error and do not write the new settings.

Suggested helper and replacement:

```rust
fn read_existing_provider_settings_for_update(
    sp: &Path,
) -> Result<Option<LlmProviderSettings>, std::io::Error> {
    if !sp.exists() {
        return Ok(None);
    }

    let text = fs::read_to_string(sp).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!(
                "existing provider settings file exists but cannot be read ({}): {error}",
                sp.display()
            ),
        )
    })?;

    serde_json::from_str::<LlmProviderSettings>(&text)
        .map(Some)
        .map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "existing provider settings file is invalid and cannot be safely updated ({}): {error}",
                    sp.display()
                ),
            )
        })
}

pub fn save_provider_settings_only(
    app_data_dir: &Path,
    settings: &LlmProviderSettings,
    store: &dyn ProviderSecretStore,
) -> Result<(), std::io::Error> {
    let sp = settings_path(app_data_dir);
    let current = read_existing_provider_settings_for_update(&sp)?;

    if let Some(current) = current.as_ref() {
        if current.provider != settings.provider {
            store.delete_key(current.provider.as_str()).map_err(|error| {
                std::io::Error::other(format!(
                    "could not clear stale key after provider change ({} -> {}): {error}",
                    current.provider.as_str(),
                    settings.provider.as_str()
                ))
            })?;
        }
    }

    if let Some(parent) = sp.parent() {
        fs::create_dir_all(parent)?;
    }

    let settings_json = serde_json::to_string_pretty(settings)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    fs::write(&sp, settings_json)?;
    Ok(())
}
```

### Tests

Add tests roughly like these:

```rust
#[test]
fn settings_only_provider_switch_fails_if_old_key_delete_fails() {
    let dir = tempfile::tempdir().unwrap();
    let store = FailingSecretStore::new().fail_delete();

    save_provider_config(
        dir.path(),
        Some(&anthropic_config("sk-ant-old")),
        &InMemorySecretStore::default(),
    )
    .unwrap();

    let openai_settings = LlmProviderSettings {
        provider: LlmProviderType::OpenAi,
        endpoint_url: None,
        model: None,
    };

    let result = save_provider_settings_only(dir.path(), &openai_settings, &store);
    assert!(result.is_err());

    let persisted = fs::read_to_string(settings_path(dir.path())).unwrap();
    let current: LlmProviderSettings = serde_json::from_str(&persisted).unwrap();
    assert_eq!(current.provider, LlmProviderType::Anthropic);
}

#[test]
fn settings_only_fails_on_invalid_existing_settings_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(settings_path(dir.path()), "not-json").unwrap();

    let settings = LlmProviderSettings {
        provider: LlmProviderType::OpenAi,
        endpoint_url: None,
        model: None,
    };

    let result = save_provider_settings_only(
        dir.path(),
        &settings,
        &InMemorySecretStore::default(),
    );

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(settings_path(dir.path())).unwrap(), "not-json");
}
```

Adjust helper names if the existing test store API differs.

### Acceptance

- No `eprintln!`-and-continue path remains for stale-key deletion.
- Provider switch is all-or-nothing.
- Tests cover delete failure and invalid existing settings.

---

## P0.3 — Surface unreadable legacy API key file

**Files:**

- `src-tauri/src/npc/provider_storage.rs`
- `src-tauri/src/app_state/app.rs`

### Problem

If `claude-api-key.txt` exists but cannot be read, `load_provider_config` falls through to `Missing`.

### Required change

Add a load-state variant and return it when the legacy file exists but cannot be read.

Suggested enum addition:

```rust
pub enum ProviderConfigLoadState {
    Missing,
    Loaded(LlmProviderConfig),
    Unreadable { error: String },
    InvalidJson { error: String },
    InvalidSchema { error: String },
    KeyUnreadable { error: String },
    LegacyKeyUnreadable { error: String },
}
```

Suggested replacement for the legacy block:

```rust
let legacy = legacy_key_path(app_data_dir);
if legacy.exists() {
    let raw = match fs::read_to_string(&legacy) {
        Ok(raw) => raw,
        Err(error) => {
            return ProviderConfigLoadState::LegacyKeyUnreadable {
                error: format!(
                    "legacy provider key exists but cannot be read ({}): {error}",
                    legacy.display()
                ),
            };
        }
    };

    let key = raw.trim().to_string();
    if !key.is_empty() {
        return ProviderConfigLoadState::Loaded(LlmProviderConfig::from_anthropic_key(key));
    }
}
```

Update `DesktopAppState::detect`:

```rust
crate::npc::provider_storage::ProviderConfigLoadState::Unreadable { error }
| crate::npc::provider_storage::ProviderConfigLoadState::InvalidJson { error }
| crate::npc::provider_storage::ProviderConfigLoadState::InvalidSchema { error }
| crate::npc::provider_storage::ProviderConfigLoadState::KeyUnreadable { error }
| crate::npc::provider_storage::ProviderConfigLoadState::LegacyKeyUnreadable { error } => {
    eprintln!("[app] provider config load error: {error}");
    Some(error.clone())
}
```

### Tests

On Unix, create a file and remove read permissions. On non-Unix, use a directory at the legacy key path if possible.

```rust
#[cfg(unix)]
#[test]
fn unreadable_legacy_key_is_reported_not_missing() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let legacy = legacy_key_path(dir.path());
    fs::write(&legacy, "sk-ant-legacy").unwrap();
    fs::set_permissions(&legacy, fs::Permissions::from_mode(0o000)).unwrap();

    let result = load_provider_config(dir.path(), &InMemorySecretStore::default());

    fs::set_permissions(&legacy, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(matches!(result, ProviderConfigLoadState::LegacyKeyUnreadable { .. }));
}
```

### Acceptance

- Legacy read failure is visible as provider config error.
- Missing legacy file remains normal `Missing`.
- Empty readable legacy file remains normal `Missing`.

---

## P0.4 — Stop profile-backed NPCs from silently using rule-based fallback on internal/provider failures

**Files:**

- `src-tauri/src/npc/runner/action.rs`
- `src-tauri/src/npc/runner/mod.rs`
- `src-tauri/src/npc/llm_strategy.rs` if new fallback reasons are added
- `src-tauri/src/app_state/mod.rs` if new error reason is added
- NPC runner tests

### Problem

Profile-backed NPCs fall back to rule-based decisions for provider unavailable, provider not configured, and LLM client construction failure. `ProviderState::StateUnavailable` is especially bad: a poisoned mutex is an internal failure, not a gameplay mode.

### Required change

Minimum acceptable fix:

- `ProviderState::StateUnavailable` records an error and submits no action.
- LLM client construction failure records an error and submits no action.
- Provider not configured for profile-backed NPC records an error and submits no action, or the UI/API blocks adding profile-backed NPCs without provider config.
- Rule-based fallback remains allowed for NPCs with no profile.

Add a small helper in `action.rs` so this does not duplicate error-recording code everywhere:

```rust
fn record_npc_internal_error(
    runner_state: &mut RunnerState,
    player_id: String,
    hand_number: Option<u32>,
    reason: NpcActionErrorReason,
    message: String,
) -> NpcActionOutcome {
    eprintln!("[npc-runner] {message}");
    runner_state.error_sequence += 1;
    let seq = runner_state.error_sequence;
    if let Ok(mut g) = runner_state.shared_action_error.lock() {
        *g = Some(NpcActionErrorDebug {
            player_id: Some(player_id),
            action: None,
            reason,
            message,
            hand_number,
            sequence: seq,
            submitted: false,
            occurred_at_ms: now_epoch_ms(),
        });
    }
    NpcActionOutcome::RuntimeUnavailable
}
```

Use it for provider-state failure:

```rust
ProviderState::StateUnavailable => {
    let msg = format!(
        "{}: provider state unavailable for profile-backed NPC {}; no action submitted",
        fresh_window.player_id, profile.id
    );
    if let Ok(mut g) = runner_state.shared_fallback.lock() {
        *g = Some(msg.clone());
    }
    return record_npc_internal_error(
        runner_state,
        fresh_window.player_id.clone(),
        Some(fresh_hand.hand_number),
        NpcActionErrorReason::ProviderStateUnavailable,
        msg,
    );
}
```

Use similar handling for LLM client construction failure:

```rust
let client = match LlmClient::new(cfg) {
    Ok(client) => client,
    Err(error) => {
        let msg = format!(
            "{}: failed to build LLM client for profile {}: {error}; no action submitted",
            fresh_window.player_id, profile.id
        );
        if let Ok(mut g) = runner_state.shared_fallback.lock() {
            *g = Some(msg.clone());
        }
        return record_npc_internal_error(
            runner_state,
            fresh_window.player_id.clone(),
            Some(fresh_hand.hand_number),
            NpcActionErrorReason::InternalError,
            msg,
        );
    }
};

let (llm_action, llm_raise, fallback_reason) = choose_llm_action(&client, profile, &snapshot);
```

For provider not configured:

```rust
ProviderState::NotConfigured => {
    let msg = format!(
        "{}: profile-backed NPC {} cannot act because no usable LLM provider is configured",
        fresh_window.player_id, profile.id
    );
    if let Ok(mut g) = runner_state.shared_fallback.lock() {
        *g = Some(msg.clone());
    }
    return record_npc_internal_error(
        runner_state,
        fresh_window.player_id.clone(),
        Some(fresh_hand.hand_number),
        NpcActionErrorReason::InternalError,
        msg,
    );
}
```

### Tests

Add/modify tests to assert no action is submitted for profile-backed NPC internal failures. If existing tests expect fallback, update them because the desired behavior changed.

Test intent:

```rust
#[test]
fn profile_npc_with_poisoned_provider_state_records_error_and_submits_no_action() {
    // Arrange profile-backed npc whose turn is open.
    // Poison or replace api_key_holder so ProviderState::StateUnavailable is reached.
    // Act: try_npc_action(...)
    // Assert: outcome is not Success, last_npc_action_error.reason == ProviderStateUnavailable,
    // and hand_log_action_count() remains 0.
}

#[test]
fn rule_based_npc_without_profile_still_acts_normally() {
    // Arrange no profile.
    // Assert normal rule-based action path still submits successfully.
}
```

### Acceptance

- Internal/provider failure cannot silently become rule-based action.
- Debug state clearly reports why the NPC did not act.
- Rule-based NPC behavior is unchanged.

---

## P0.5 — Remove invented authoritative NPC decision defaults

**Files:**

- `src-tauri/src/npc/runner/action.rs`
- `src-tauri/src/npc/runner/decision.rs`
- NPC runner/decision tests

### Problem

NPC decisions use `unwrap_or(1)`, `unwrap_or(20)`, `unwrap_or(0)`, and `fallback_blind_level()` to invent authoritative game state.

### Required change

Validate state before calling `rule_based_decision` or building `GameStateSnapshot`.

Suggested helper in `action.rs`:

```rust
fn npc_stack_for_decision(
    state: &crate::domain::TournamentState,
    player_id: &str,
) -> Result<u32, String> {
    state
        .seats
        .iter()
        .find(|seat| seat.participant_id.as_deref() == Some(player_id))
        .and_then(|seat| seat.chip_count)
        .ok_or_else(|| format!("missing chip stack for NPC player {player_id}"))
}

fn blind_level_for_decision(
    state: &crate::domain::TournamentState,
) -> Result<crate::domain::BlindLevel, String> {
    state
        .config
        .blind_schedule
        .levels
        .get(state.blind_level_index)
        .cloned()
        .ok_or_else(|| {
            format!(
                "blind level index {} is out of range for {} configured levels",
                state.blind_level_index,
                state.config.blind_schedule.levels.len()
            )
        })
}
```

Use before decisions:

```rust
let stack = match npc_stack_for_decision(&fresh_state, &fresh_window.player_id) {
    Ok(stack) => stack,
    Err(message) => {
        return record_npc_internal_error(
            runner_state,
            fresh_window.player_id.clone(),
            Some(fresh_hand.hand_number),
            NpcActionErrorReason::InternalError,
            message,
        );
    }
};
```

Replace:

```rust
.unwrap_or_else(fallback_blind_level)
```

with:

```rust
let blind_level = match blind_level_for_decision(&fresh_state) {
    Ok(level) => level,
    Err(message) => {
        return record_npc_internal_error(
            runner_state,
            fresh_window.player_id.clone(),
            Some(fresh_hand.hand_number),
            NpcActionErrorReason::InternalError,
            message,
        );
    }
};
```

Then change `rule_based_decision` so it receives the already-resolved big blind/current bet, or returns `Result` instead of inventing defaults.

Preferred signature direction:

```rust
pub(crate) struct RuleDecisionContext<'a> {
    pub style: &'a NpcStyle,
    pub hole_cards: &'a [crate::domain::Card],
    pub board: &'a [crate::domain::Card],
    pub street: StreetPhase,
    pub pot_total: u32,
    pub call_amount: u32,
    pub min_raise_to: Option<u32>,
    pub max_raise_to: Option<u32>,
    pub facing_bet: bool,
    pub stack: u32,
    pub active_count: u8,
    pub dealer_seat: u8,
    pub npc_seat: u8,
    pub big_blind: u32,
    pub current_bet: u32,
    pub legal_actions: &'a [ActionType],
    pub seed: u64,
}

pub(crate) fn rule_based_decision(ctx: RuleDecisionContext<'_>) -> (ActionType, Option<u32>) {
    let position = derive_position(ctx.npc_seat, ctx.dealer_seat, ctx.active_count.max(2));
    let action = if ctx.street == StreetPhase::Preflop {
        let tier = preflop_hand_tier(ctx.hole_cards);
        let facing_raise = ctx.call_amount > ctx.big_blind;
        let raise_count = if ctx.current_bet > ctx.big_blind * 3 {
            2
        } else if facing_raise {
            1
        } else {
            0
        };
        // call choose_preflop_action(...)
    } else {
        // existing postflop path
    };
    first_legal_action(action, ctx.legal_actions)
}
```

If that refactor is too broad, at least make `rule_based_decision` return `Result<(ActionType, Option<u32>), String>` and replace the `.unwrap_or(...)` calls with `ok_or_else(...) ?`.

### Tests

Add tests:

```rust
#[test]
fn npc_decision_fails_when_stack_is_missing() {
    // Arrange acting NPC seat with participant_id but chip_count = None.
    // Assert no action submitted and last error message mentions missing chip stack.
}

#[test]
fn npc_decision_fails_when_blind_index_is_invalid() {
    // Arrange state.blind_level_index >= state.config.blind_schedule.levels.len().
    // Assert no action submitted and last error message mentions blind level index.
}
```

### Acceptance

- No production NPC decision path uses fallback stack/blind/current-hand defaults.
- Invalid authoritative state produces a structured error.

---

## P1.1 — Make multi-NPC add atomic or rollback-safe

**Files:**

- `src-tauri/src/app_state/app_npc.rs`
- host server/lobby APIs as needed
- tests for app state or host session

### Problem

`add_npc_players` can partially register/seat/ready NPCs and then fail before runner startup.

### Required change

Preferred: add a host-server transaction method. Acceptable: rollback applied NPCs on failure.

A minimal rollback scaffold in `app_npc.rs` might look like this:

```rust
#[derive(Clone, Debug)]
enum AppliedNpcStep {
    Registered { player_id: String },
    SeatClaimed { player_id: String, seat_index: u8 },
    ReadySet { player_id: String },
}

fn rollback_npc_steps(
    host_server: &crate::networking::HostServer,
    applied: &[AppliedNpcStep],
) -> Result<(), String> {
    let mut errors = Vec::new();

    for step in applied.iter().rev() {
        let result = match step {
            AppliedNpcStep::ReadySet { player_id } => {
                host_server.set_ready_state(player_id, false).map_err(|e| e.to_string())
            }
            AppliedNpcStep::SeatClaimed { player_id, .. } => {
                // Implement or use an existing leave-seat/unclaim-seat API.
                host_server.release_seat(player_id).map_err(|e| e.to_string())
            }
            AppliedNpcStep::Registered { player_id } => {
                // Implement or use an existing unregister participant API.
                host_server.unregister_participant(player_id).map_err(|e| e.to_string())
            }
        };

        if let Err(error) = result {
            errors.push(error);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}
```

Then use a small helper for mutation:

```rust
let mut applied = Vec::new();

for (npc_config, &seat_index) in npc_configs.iter().zip(open_seats.iter()) {
    if let Err(error) = session
        .host_server
        .register_npc_participant(&npc_config.player_id, &npc_config.display_name)
        .map_err(|e| e.to_string())
    {
        return Err(error);
    }
    applied.push(AppliedNpcStep::Registered {
        player_id: npc_config.player_id.clone(),
    });

    if let Err(error) = session
        .host_server
        .claim_seat(&npc_config.player_id, seat_index)
        .map_err(|e| e.to_string())
    {
        let rollback = rollback_npc_steps(&session.host_server, &applied);
        return Err(format_with_rollback_error(error, rollback));
    }
    applied.push(AppliedNpcStep::SeatClaimed {
        player_id: npc_config.player_id.clone(),
        seat_index,
    });

    if let Err(error) = session
        .host_server
        .set_ready_state(&npc_config.player_id, true)
        .map_err(|e| e.to_string())
    {
        let rollback = rollback_npc_steps(&session.host_server, &applied);
        return Err(format_with_rollback_error(error, rollback));
    }
    applied.push(AppliedNpcStep::ReadySet {
        player_id: npc_config.player_id.clone(),
    });
}
```

Helper:

```rust
fn format_with_rollback_error(original: String, rollback: Result<(), String>) -> String {
    match rollback {
        Ok(()) => original,
        Err(rollback_error) => format!(
            "{original}; additionally failed to rollback partial NPC add: {rollback_error}"
        ),
    }
}
```

### Important

If no `release_seat` or `unregister_participant` API exists, do not fake rollback by only changing UI state. Add authoritative host-server APIs or implement the preferred transaction method under the authoritative state lock.

### Tests

- Add two NPCs and inject failure on the second claim/ready step. Assert neither NPC remains registered/seated.
- Inject runner spawn failure after successful register/seat/ready. Assert rollback occurs.

### Acceptance

- `add_npc_players` is all-or-nothing from caller perspective.
- Rollback failure is reported honestly.

---

## P1.2 — Return `Result` from NPC runner thread spawn

**Files:**

- `src-tauri/src/npc/runner/mod.rs`
- `src-tauri/src/app_state/app_npc.rs`

### Problem

`start_npc_runner` panics if thread spawn fails.

### Required change

Change signature:

```rust
pub fn start_npc_runner(
    host_server: Arc<HostServer>,
    npc_configs: Vec<NpcConfig>,
    stop: Arc<AtomicBool>,
    api_key_holder: Arc<Mutex<Option<LlmProviderConfig>>>,
    shared_tilt: Arc<Mutex<BTreeMap<String, String>>>,
    shared_fallback: Arc<Mutex<Option<String>>>,
    shared_action_error: Arc<Mutex<Option<NpcActionErrorDebug>>>,
) -> Result<thread::JoinHandle<()>, String> {
    thread::Builder::new()
        .name("npc-runner".into())
        .spawn(move || {
            loop_core::npc_runner_loop(
                &host_server,
                &npc_configs,
                &stop,
                &api_key_holder,
                shared_tilt,
                shared_fallback,
                shared_action_error,
            );
        })
        .map_err(|error| format!("failed to spawn npc-runner thread: {error}"))
}
```

Update caller:

```rust
let runner_handle = match crate::npc::runner::start_npc_runner(...) {
    Ok(handle) => handle,
    Err(error) => {
        let rollback = rollback_npc_steps(&session.host_server, &applied);
        return Err(format_with_rollback_error(error, rollback));
    }
};
```

### Acceptance

- No `.expect("failed to spawn npc-runner thread")` remains.
- Spawn failure is propagated as an app error.

---

## P1.3 — Add host runtime health diagnostics

**Files:**

- `src-tauri/src/networking/runtime/mod.rs` or new `health.rs`
- `src-tauri/src/networking/runtime/host.rs`
- `src-tauri/src/app_state` debug/status mapping files
- debug UI types if needed

### Problem

Host loops ignore accept errors, timeout setup failures, timer advancement failures, state lock errors, and publish failures.

### Required change

Add a shared health object to `HostServer`.

Suggested type:

```rust
use serde::Serialize;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostRuntimeHealth {
    pub accept_error_count: u64,
    pub stream_timeout_error_count: u64,
    pub tick_advance_error_count: u64,
    pub publish_error_count: u64,
    pub state_lock_error_count: u64,
    pub last_error: Option<String>,
    pub last_successful_tick_ms: Option<u64>,
    pub last_successful_publish_ms: Option<u64>,
}

impl HostRuntimeHealth {
    fn record_error(&mut self, message: impl Into<String>) {
        self.last_error = Some(message.into());
    }
}
```

Add to `HostServer`:

```rust
runtime_health: Arc<Mutex<HostRuntimeHealth>>,
```

Getter:

```rust
impl HostServer {
    pub fn runtime_health(&self) -> HostRuntimeHealth {
        self.runtime_health
            .lock()
            .map(|health| health.clone())
            .unwrap_or_else(|_| HostRuntimeHealth {
                state_lock_error_count: 1,
                last_error: Some("host runtime health lock poisoned".to_string()),
                ..HostRuntimeHealth::default()
            })
    }
}
```

Small helper for loops:

```rust
fn update_health(
    health: &Arc<Mutex<HostRuntimeHealth>>,
    update: impl FnOnce(&mut HostRuntimeHealth),
) {
    if let Ok(mut guard) = health.lock() {
        update(&mut guard);
    }
}
```

Use in accept loop:

```rust
let Ok(mut stream) = incoming else {
    update_health(&runtime_health, |health| {
        health.accept_error_count += 1;
        health.record_error("host listener failed to accept incoming connection");
    });
    continue;
};

if let Err(error) = stream.set_read_timeout(Some(Duration::from_secs(5))) {
    update_health(&runtime_health, |health| {
        health.stream_timeout_error_count += 1;
        health.record_error(format!("failed to set client read timeout: {error}"));
    });
}

if let Err(error) = stream.set_write_timeout(Some(Duration::from_secs(5))) {
    update_health(&runtime_health, |health| {
        health.stream_timeout_error_count += 1;
        health.record_error(format!("failed to set client write timeout: {error}"));
    });
}
```

Use in tick loop:

```rust
let now = now_epoch_ms();
let next_state = match tournament_runtime.lock() {
    Err(_) => {
        update_health(&runtime_health, |health| {
            health.state_lock_error_count += 1;
            health.record_error("tournament runtime lock poisoned");
        });
        break;
    }
    Ok(mut runtime) => match runtime.as_mut() {
        None => None,
        Some(controller) => {
            let before = controller.state().clone();
            match controller.advance_time(now) {
                Ok(()) => {
                    update_health(&runtime_health, |health| {
                        health.last_successful_tick_ms = Some(now);
                    });
                    let after = controller.state().clone();
                    (after != before).then_some(after)
                }
                Err(error) => {
                    update_health(&runtime_health, |health| {
                        health.tick_advance_error_count += 1;
                        health.record_error(format!("tournament timer advance failed: {error}"));
                    });
                    None
                }
            }
        }
    },
};
```

For publish:

```rust
match publish_runtime_transition(...) {
    Ok(()) => update_health(&runtime_health, |health| {
        health.last_successful_publish_ms = Some(now_epoch_ms());
    }),
    Err(error) => update_health(&runtime_health, |health| {
        health.publish_error_count += 1;
        health.record_error(format!("runtime transition publish failed: {error}"));
    }),
}
```

### Debug UI

Add a nullable `hostRuntimeHealth` to `DebugInspectorState` if a host session exists.

```rust
pub host_runtime_health: Option<crate::networking::HostRuntimeHealth>,
```

Update frontend `DebugInspectorState` type and `DebugPanel` to display non-zero counts and last error.

### Tests

- Unit-test `HostRuntimeHealth` update helper if practical.
- Add a host runtime test that forces a publish error or uses a fake client stream that fails write.
- Add a timer test that forces `advance_time` failure if controller can be put into invalid state.

### Acceptance

- No critical host loop failure is only represented by `_ = ...`.
- Debug state can show host runtime health.

---

## P1.4 — Add client protocol warning diagnostics

**Files:**

- `src-tauri/src/networking/runtime/mod.rs`
- `src-tauri/src/networking/runtime/client.rs`
- client runtime tests
- debug state/UI if needed

### Problem

Client runtime silently drops malformed/invalid frames.

### Required change

Add a non-fatal event:

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum ClientRuntimeEvent {
    Snapshot(Box<SnapshotEvent>),
    PublicEvent { /* existing fields */ },
    PrivateHoleCards(PrivateHoleCardsEvent),
    Reconnecting { player_id: String },
    ResyncRequested { player_id: String, last_seen_server_sequence: u64 },
    ProtocolWarning {
        player_id: String,
        reason: String,
        count: u64,
    },
    SafeError { player_id: String, message: String },
    Disconnected { player_id: String },
}
```

In `client.rs`, add a local warning counter map inside the read loop:

```rust
let mut protocol_warning_counts: std::collections::BTreeMap<String, u64> =
    std::collections::BTreeMap::new();

fn emit_protocol_warning(
    sender: &std::sync::mpsc::Sender<ClientRuntimeEvent>,
    counts: &mut std::collections::BTreeMap<String, u64>,
    player_id: &str,
    reason: impl Into<String>,
) {
    let reason = reason.into();
    let count = counts.entry(reason.clone()).or_insert(0);
    *count += 1;

    // Low-noise policy: emit first occurrence and powers of two.
    if *count == 1 || count.is_power_of_two() {
        let _ = sender.send(ClientRuntimeEvent::ProtocolWarning {
            player_id: player_id.to_string(),
            reason,
            count: *count,
        });
    }
}
```

Replace silent `continue` cases. Examples:

```rust
let Some(message_type) = frame_value.get("messageType").and_then(Value::as_str) else {
    emit_protocol_warning(
        &sender,
        &mut protocol_warning_counts,
        &player_id,
        "incoming frame missing messageType",
    );
    continue;
};
```

```rust
let Ok(envelope) = serde_json::from_value::<EncryptedPrivateEnvelope>(frame_value.clone()) else {
    emit_protocol_warning(
        &sender,
        &mut protocol_warning_counts,
        &player_id,
        "malformed private hole-card envelope",
    );
    continue;
};
```

```rust
if envelope.verify(&crypto_provider, &host_signing_public_key).is_err() {
    emit_protocol_warning(
        &sender,
        &mut protocol_warning_counts,
        &player_id,
        "private hole-card envelope signature verification failed",
    );
    continue;
}
```

```rust
let Ok(plaintext) = crypto_provider.decrypt(/* existing args */) else {
    emit_protocol_warning(
        &sender,
        &mut protocol_warning_counts,
        &player_id,
        "private hole-card payload decrypt failed",
    );
    continue;
};
```

```rust
let Ok(private_payload) = serde_json::from_slice::<PrivateHoleCardsEvent>(&plaintext) else {
    emit_protocol_warning(
        &sender,
        &mut protocol_warning_counts,
        &player_id,
        "private hole-card payload JSON was invalid",
    );
    continue;
};
```

### Tests

Add tests that feed malformed frames and assert `ProtocolWarning` is emitted. Keep hostile-input behavior safe: no panic, no state mutation, no private card leakage.

### Acceptance

- Invalid protocol input is still dropped.
- Repeated invalid input increments a counter.
- Debug tooling can surface the most recent warning.

---

## P1.5 — Fix window persistence test/runtime guard noise

**Files:**

- `src/app/persistence.ts`
- persistence tests or test setup

### Problem

Tests emit expected `Failed to initialize window state persistence` errors.

### Required change

Strengthen guard before importing/using Tauri window APIs. Do not treat catch-and-log as a normal non-Tauri path.

Possible helper:

```ts
function hasUsableTauriWindowRuntime(): boolean {
  const maybeWindow = globalThis.window as
    | (Window & { __TAURI_INTERNALS__?: unknown; __TAURI__?: unknown })
    | undefined;

  if (!maybeWindow) {
    return false;
  }

  // In browser/jsdom tests, this may be absent or partially mocked.
  // Only enter the Tauri path when the official internals object is present.
  return Boolean(maybeWindow.__TAURI_INTERNALS__);
}
```

If the current code already checks this but tests partially mock `__TAURI_INTERNALS__`, then fix the test setup instead: remove fake partial internals or mock `@tauri-apps/api/window` with a valid `getCurrentWindow`/`Window.getCurrent` shape.

### Acceptance

- `npm test` emits no expected persistence initialization error.
- Real Tauri runtime failures still produce actionable errors.

---

## P1.6 — Make default `npm test` reliable

**Files:**

- `package.json`
- `vite.config.ts` or new `vitest.config.ts`

### Problem

Default `npm test` timed out in the review sandbox, while single-threaded Vitest passed.

### Required change

Encode the reliable Vitest mode in the project config or script.

Option A, package script:

```json
{
  "scripts": {
    "test": "vitest run --pool=threads --poolOptions.threads.singleThread=true"
  }
}
```

Option B, config in `vite.config.ts` or `vitest.config.ts`:

```ts
/// <reference types="vitest" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    pool: "threads",
    poolOptions: {
      threads: {
        singleThread: true,
      },
    },
    testTimeout: 10_000,
  },
});
```

Use the form compatible with the current Vite/Vitest config. Avoid duplicate conflicting config.

### Acceptance

- `npm test` finishes reliably.
- Test command is documented in comments if single-threading is retained.

---

## P2.1 — Clean up enum/string serialization `.ok()` paths

**Files:**

- `src-tauri/src/app_state/app.rs`
- any related provider/bootstrap mapping tests

### Problem

Some enum-to-string code uses `serde_json::to_value(...).ok().and_then(...)`. This is low risk, but it is still unnecessary silent failure for a known enum.

### Required change

Use direct helpers.

Example:

```rust
let llm_provider_type = loaded_provider
    .as_ref()
    .map(|config| config.settings.provider.as_str().to_string());
```

### Acceptance

- No unnecessary enum serialization failure path remains for known provider enum values.
- Existing bootstrap tests pass.

---

## P2.2 — Audit remaining `let _ =`, `.ok()`, and `continue` in runtime code

**Files:**

- `src-tauri/src/networking/**`
- `src-tauri/src/npc/**`
- `src-tauri/src/app_state/**`

### Required change

Run:

```bash
rg -n "let _ =|\.ok\(\)|unwrap_or\(|unwrap_or_else\(|continue;" src-tauri/src/networking src-tauri/src/npc src-tauri/src/app_state
```

For each hit touched by this fix:

- Decide whether it is presentation-only, test-only, hostile input handling, or real silent failure.
- Convert real silent failures to structured errors/diagnostics.
- Add comments only when the fallback is intentionally safe.

### Acceptance

- No newly touched code has unexplained silent failures.
- Any remaining ignored error in networking/NPC/app_state has a short comment explaining why it is safe.

---

## P2.3 — Update docs/memory

**Files:**

- `memory.md`
- optionally `docs/` if project practice requires a completed implementation note

### Required change

Add a short `memory.md` entry after implementation using the project’s existing format. Do not fabricate timestamps. Use the actual command specified in the repo conventions, if any.

Record:

- Browser mocks are dev/test only.
- Provider settings-only saves are fail-loud and transactional.
- Profile-backed NPC LLM failures no longer silently become rule-based actions.
- Host/client runtime warnings are visible through debug diagnostics.

---

## Final validation commands

Run all commands from repo root unless noted:

```bash
npm ci
npm run lint
npm run build
npm test
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

If Node 24 is not active, switch to Node 24 first because `package.json` declares:

```json
"engines": {
  "node": "24.x"
}
```

## Definition of done

- [ ] Browser mocks cannot be used in production builds.
- [ ] Provider settings-only save fails before writing if old key deletion fails.
- [ ] Unreadable legacy `claude-api-key.txt` is visible as a provider config error.
- [ ] Profile-backed NPCs do not silently fall back to rule-based behavior for provider/internal failures.
- [ ] NPC decision code does not invent authoritative stack/blind/current-hand values.
- [ ] Multi-NPC add is atomic or rollback-safe.
- [ ] NPC runner thread spawn returns `Result`, not panic.
- [ ] Host runtime health records accept/timeout/tick/publish/lock failures.
- [ ] Client runtime emits protocol warnings for dropped malformed/invalid frames.
- [ ] Window persistence tests no longer print expected errors.
- [ ] `npm test` finishes reliably.
- [ ] Rust fmt/clippy/tests pass.
