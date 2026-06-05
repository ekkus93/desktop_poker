# NPC Phase 2 — LLM-Powered Profile Files TODO

This backlog implements Phase 2 of the NPC spec: replacing hardcoded rule-based strategies
with plain-English player profiles processed by the Claude API. The host reads a profile
file at NPC creation time and uses it to make decisions.

See `docs/NPC1_SPEC.md` for the full design and `docs/NPC1_PHASE1_TODO.md` for Phase 1
(already complete).

Status legend:
- [ ] not started
- [~] in progress
- [x] done

---

## Part 1: Rust — NPC profile data model

**File:** `src-tauri/src/npc/profile.rs`

### 1.1 Create `profile.rs` and declare it in `npc/mod.rs`

- [ ] Create `src-tauri/src/npc/profile.rs`.
- [ ] Add `pub mod profile;` to `src-tauri/src/npc/mod.rs`.

### 1.2 Define the `NpcProfile` struct

- [ ] Define:
  ```rust
  #[derive(Clone, Debug, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct NpcProfile {
      pub id: String,         // filename stem, e.g. "aggressive-alice"
      pub name: String,       // from frontmatter `name:`
      pub style: String,      // from frontmatter `style:`, e.g. "loose-aggressive"
      pub skill: String,      // from frontmatter `skill:`, e.g. "intermediate"
      pub description: String, // the free-form Markdown body
  }
  ```

### 1.3 Define `NpcProfileFrontmatter` for YAML parsing

- [ ] Define a separate struct for YAML deserialization:
  ```rust
  #[derive(Debug, Deserialize)]
  struct NpcProfileFrontmatter {
      name: String,
      style: Option<String>,
      skill: Option<String>,
  }
  ```
- [ ] Use `serde_yaml` (add to `Cargo.toml`) for frontmatter parsing.

### 1.4 Implement `parse_profile(file_stem: &str, content: &str) -> Result<NpcProfile, ProfileError>`

- [ ] Split file content on the second `---` delimiter to separate YAML frontmatter from
  the body.
- [ ] Parse the YAML block with `serde_yaml::from_str`.
- [ ] Trim whitespace from the body.
- [ ] Return a descriptive `ProfileError` on malformed frontmatter or missing `name` field.
- [ ] Validate `name` is non-empty; `style` and `skill` default to `"custom"` / `"unknown"`
  if absent.

### 1.5 Define `ProfileError`

- [ ] Define:
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum ProfileError {
      #[error("missing frontmatter delimiter")]
      MissingDelimiter,
      #[error("invalid YAML frontmatter: {0}")]
      YamlParse(String),
      #[error("profile name is required")]
      MissingName,
      #[error("io error: {0}")]
      Io(#[from] std::io::Error),
  }
  ```

### 1.6 Tests for `parse_profile`

- [ ] Test: valid profile parses to correct `NpcProfile` fields.
- [ ] Test: missing second `---` delimiter → `ProfileError::MissingDelimiter`.
- [ ] Test: invalid YAML in frontmatter → `ProfileError::YamlParse`.
- [ ] Test: missing `name` field → `ProfileError::MissingName`.
- [ ] Test: `style` and `skill` absent → defaults applied without error.
- [ ] Test: body is correctly trimmed and preserved across multi-paragraph text.

---

## Part 2: Rust — Profile filesystem storage

**File:** `src-tauri/src/npc/profile_store.rs`

### 2.1 Create `profile_store.rs`

- [ ] Create `src-tauri/src/npc/profile_store.rs`.
- [ ] Add `pub mod profile_store;` to `src-tauri/src/npc/mod.rs`.

### 2.2 Implement `profiles_dir(app_data_dir: &Path) -> PathBuf`

- [ ] Return `{app_data_dir}/npc-profiles/`.
- [ ] This directory is created on first access if it does not exist.

### 2.3 Implement `list_profiles(profiles_dir: &Path) -> Result<Vec<NpcProfile>, ProfileError>`

- [ ] Scan `profiles_dir` for files with `.md` extension.
- [ ] Parse each file with `parse_profile`, using the file stem as the profile ID.
- [ ] Skip files that fail to parse (log a warning, do not error the whole list).
- [ ] Return profiles sorted alphabetically by `name`.

### 2.4 Implement `load_profile(profiles_dir: &Path, id: &str) -> Result<NpcProfile, ProfileError>`

- [ ] Locate `{profiles_dir}/{id}.md`.
- [ ] Parse and return it, or return `ProfileError::Io` if the file does not exist.

### 2.5 Implement `save_profile(profiles_dir: &Path, profile_content: &str, id: &str) -> Result<(), ProfileError>`

- [ ] Write `{profiles_dir}/{id}.md` (create or overwrite).
- [ ] Validate that `profile_content` parses correctly before writing; return error if invalid.

### 2.6 Implement `delete_profile(profiles_dir: &Path, id: &str) -> Result<(), ProfileError>`

- [ ] Delete `{profiles_dir}/{id}.md`; return `ProfileError::Io` if not found.

### 2.7 Seed built-in profiles on first run

- [ ] On startup (or on first call to `list_profiles` when the directory is empty), copy
  a set of bundled starter profiles from the binary's embedded resources into the profiles
  directory.
- [ ] Starter profiles to ship: `aggressive-alice.md`, `conservative-carlos.md`,
  `balanced-sam.md` (one loose-aggressive, one tight-passive, one balanced).
- [ ] Use `include_str!` to embed the starter profile content in the binary.

### 2.8 Tests for profile store

- [ ] Test: `list_profiles` on an empty directory returns an empty vec.
- [ ] Test: `list_profiles` skips an unparseable file and still returns valid profiles.
- [ ] Test: `load_profile` returns the correct profile for a known ID.
- [ ] Test: `load_profile` on an unknown ID returns `ProfileError::Io`.
- [ ] Test: `save_profile` writes a file that can be read back with `load_profile`.
- [ ] Test: `delete_profile` removes the file; a subsequent `load_profile` returns an error.

---

## Part 3: Rust — Game state serialization for LLM prompts

**File:** `src-tauri/src/npc/prompt.rs`

### 3.1 Create `prompt.rs`

- [ ] Create `src-tauri/src/npc/prompt.rs`.
- [ ] Add `pub mod prompt;` to `src-tauri/src/npc/mod.rs`.

### 3.2 Define `GameStateSnapshot` — the data needed to build a prompt

- [ ] Define:
  ```rust
  pub struct GameStateSnapshot {
      pub hand_number: u32,
      pub street: StreetPhase,
      pub board_cards: Vec<Card>,
      pub hole_cards: Vec<Card>,        // the NPC's own hole cards
      pub pot_total: u32,
      pub call_amount: u32,
      pub min_raise_to: Option<u32>,
      pub max_raise_to: Option<u32>,
      pub stack: u32,
      pub position: Position,
      pub active_player_count: u8,
      pub legal_actions: Vec<ActionType>,
      pub blind_level: BlindLevel,      // current small/big blind amounts
      pub street_history: Vec<StreetAction>, // actions this street in order
  }
  ```

- [ ] Define `StreetAction`:
  ```rust
  pub struct StreetAction {
      pub seat_index: u8,
      pub action_type: ActionType,
      pub amount: Option<u32>,
  }
  ```

### 3.3 Implement `build_game_state_snapshot` to extract a snapshot from live state

- [ ] In `runner.rs` (or a helper), populate a `GameStateSnapshot` from `TournamentState`
  and the current `ActionWindow` immediately before making an LLM decision.

### 3.4 Implement `render_game_state(snapshot: &GameStateSnapshot) -> String`

- [ ] Render the snapshot as human-readable text. Example output:
  ```
  Hand #3 — Flop
  Board: A♠ K♦ 7♣
  Your hole cards: Q♠ J♠
  Pot: 480 chips
  Your stack: 1,240 chips
  Position: Late (button)
  Players still in hand: 3
  Blinds: 25 / 50
  
  Action this street:
  - Seat 1 checked
  - Seat 3 bet 200
  
  Your options:
  - fold
  - call 200
  - raise to 400–1,240
  ```
- [ ] Use Unicode suit symbols (♠ ♥ ♦ ♣) and rank names (Ace, King, …, Two).
- [ ] Format chip counts with thousands separators.
- [ ] List legal actions with bounds explicitly; omit `min_raise_to` / `max_raise_to` lines
  when raising is not a legal action.

### 3.5 Implement `build_system_prompt() -> String`

- [ ] Return a fixed system prompt explaining:
  - The model is playing Texas Hold'em No-Limit poker.
  - It must respond with **only** a JSON object matching the schema:
    `{ "action": "fold" | "check" | "call" | "raise" | "allIn", "amount": <integer or null> }`
  - `amount` is required when `action` is `"raise"` and must be within the stated bounds.
  - `amount` must be `null` for all other actions.
  - No explanation, no commentary — JSON only.

### 3.6 Implement `build_user_message(profile: &NpcProfile, snapshot: &GameStateSnapshot) -> String`

- [ ] Concatenate:
  1. The profile's `description` (the free-form Markdown body).
  2. A separator line (e.g., `---`).
  3. The rendered game state from `render_game_state`.

### 3.7 Tests for prompt rendering

- [ ] Test: `render_game_state` on a preflop snapshot includes hole cards, pot, and legal
  options with correct amounts.
- [ ] Test: `render_game_state` on a flop snapshot includes board cards and street history.
- [ ] Test: `build_user_message` includes both the profile description and the game state.
- [ ] Test: when raising is not legal, no raise line appears in the options section.
- [ ] Test: chip amounts ≥ 1000 are formatted with thousands separators.

---

## Part 4: Rust — Claude API client

**File:** `src-tauri/src/npc/llm_client.rs`

### 4.1 Add dependencies to `Cargo.toml`

- [ ] Add `reqwest` with `json` and `rustls-tls` features (avoid OpenSSL dependency).
- [ ] Add `serde_yaml` for profile frontmatter parsing (Part 1).
- [ ] Add `tokio` with `time` feature if not already present (for timeout).
- [ ] Confirm `serde_json` is available.

### 4.2 Create `llm_client.rs`

- [ ] Create `src-tauri/src/npc/llm_client.rs`.
- [ ] Add `pub mod llm_client;` to `src-tauri/src/npc/mod.rs`.

### 4.3 Define `LlmClient`

- [ ] Define:
  ```rust
  pub struct LlmClient {
      client: reqwest::Client,
      api_key: String,
      model: String,         // e.g. "claude-haiku-4-5-20251001"
      timeout_secs: u64,     // default 5
  }
  ```
- [ ] Implement `LlmClient::new(api_key: String) -> Self` with sensible defaults.
- [ ] The model default should be `claude-haiku-4-5-20251001` (fastest/cheapest for
  real-time poker decisions).

### 4.4 Define the API request/response types

- [ ] Define:
  ```rust
  struct ClaudeRequest {
      model: String,
      max_tokens: u32,
      system: String,
      messages: Vec<ClaudeMessage>,
  }
  
  struct ClaudeMessage {
      role: String,    // "user"
      content: String,
  }
  
  struct ClaudeResponse {
      content: Vec<ClaudeContentBlock>,
  }
  
  struct ClaudeContentBlock {
      r#type: String,  // "text"
      text: String,
  }
  ```
- [ ] All structs derive `Serialize` / `Deserialize`.

### 4.5 Implement `LlmClient::complete(system: &str, user: &str) -> Result<String, LlmError>`

- [ ] POST to `https://api.anthropic.com/v1/messages` with:
  - Header `x-api-key: {api_key}`
  - Header `anthropic-version: 2023-06-01`
  - Header `content-type: application/json`
  - Body: `ClaudeRequest { model, max_tokens: 128, system, messages: [{ role: "user", content: user }] }`
- [ ] Apply a `tokio::time::timeout` of `timeout_secs` seconds to the full request.
- [ ] On success, extract `response.content[0].text`.
- [ ] Return `LlmError::Timeout` on timeout, `LlmError::Api(status, body)` on non-2xx,
  `LlmError::Network(msg)` on transport failure.

### 4.6 Define `LlmError`

- [ ] Define:
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum LlmError {
      #[error("request timed out")]
      Timeout,
      #[error("API error {0}: {1}")]
      Api(u16, String),
      #[error("network error: {0}")]
      Network(String),
      #[error("response parse error: {0}")]
      Parse(String),
  }
  ```

### 4.7 Tests for `LlmClient`

- [ ] Test: `complete` with a mock HTTP server (use `wiremock` or `httpmock` crate) returns
  the text from `content[0].text`.
- [ ] Test: non-2xx response → `LlmError::Api`.
- [ ] Test: request that exceeds timeout → `LlmError::Timeout`.
- [ ] Test: malformed JSON response body → `LlmError::Parse`.

---

## Part 5: Rust — LLM response parsing and action validation

**File:** `src-tauri/src/npc/llm_action.rs`

### 5.1 Create `llm_action.rs`

- [ ] Create `src-tauri/src/npc/llm_action.rs`.
- [ ] Add `pub mod llm_action;` to `src-tauri/src/npc/mod.rs`.

### 5.2 Define `LlmActionResponse`

- [ ] Define:
  ```rust
  #[derive(Debug, Deserialize)]
  pub struct LlmActionResponse {
      pub action: String,
      pub amount: Option<u32>,
  }
  ```

### 5.3 Implement `parse_llm_response(text: &str) -> Result<LlmActionResponse, LlmError>`

- [ ] Attempt `serde_json::from_str` directly.
- [ ] If that fails, search `text` for the first `{` … `}` substring and retry (handles
  responses that include preamble text despite instructions).
- [ ] Return `LlmError::Parse` if no valid JSON action object is found.

### 5.4 Implement `validate_llm_action(response: &LlmActionResponse, snapshot: &GameStateSnapshot) -> (ActionType, Option<u32>)`

- [ ] Map `response.action` string to `ActionType` (case-insensitive):
  - `"fold"` → `ActionType::Fold`
  - `"check"` → `ActionType::Check`
  - `"call"` → `ActionType::Call`
  - `"raise"` → `ActionType::Raise` with `amount`
  - `"allIn"` / `"all_in"` / `"allin"` → `ActionType::AllIn`
- [ ] Verify the resolved `ActionType` is in `snapshot.legal_actions`. If not, fall back to
  `first_check_or_call(legal_actions)`.
- [ ] If action is `Raise` and `amount` is `None`, fall back to `CheckOrCall`.
- [ ] If action is `Raise`, clamp `amount` to `[min_raise_to, max_raise_to]` and to ≤ stack.
- [ ] If `max_raise_to` is `None` and action is `Raise`, downgrade to `CheckOrCall`.

### 5.5 Tests for parsing and validation

- [ ] Test: clean JSON `{ "action": "raise", "amount": 480 }` parses and validates correctly.
- [ ] Test: JSON embedded in prose still extracts the action object.
- [ ] Test: `action: "fold"` when `Fold` is not legal → falls back to `CheckOrCall`.
- [ ] Test: `action: "raise"` with amount above `max_raise_to` → clamped to max.
- [ ] Test: `action: "raise"` with `max_raise_to: None` → `CheckOrCall`.
- [ ] Test: completely invalid JSON → `LlmError::Parse`.
- [ ] Test: unknown action string → fallback to `CheckOrCall`.

---

## Part 6: Rust — LLM NPC strategy integration

**File:** `src-tauri/src/npc/runner.rs` (extend) and `src-tauri/src/npc/llm_strategy.rs` (new)

### 6.1 Create `llm_strategy.rs`

- [ ] Create `src-tauri/src/npc/llm_strategy.rs`.
- [ ] Add `pub mod llm_strategy;` to `src-tauri/src/npc/mod.rs`.

### 6.2 Implement `choose_llm_action`

- [ ] Define:
  ```rust
  pub async fn choose_llm_action(
      client: &LlmClient,
      profile: &NpcProfile,
      snapshot: &GameStateSnapshot,
  ) -> (ActionType, Option<u32>)
  ```
- [ ] Build system prompt via `build_system_prompt()`.
- [ ] Build user message via `build_user_message(profile, snapshot)`.
- [ ] Call `client.complete(system, user)`.
- [ ] On success: `parse_llm_response` → `validate_llm_action`.
- [ ] On any error (timeout, API error, parse error): log the error and fall back to the
  Phase 1 rule-based strategy using the profile's `style` field (map `"loose-aggressive"` /
  `"aggressive"` → `NpcStyle::Aggressive`, everything else → `NpcStyle::Conservative`).

### 6.3 Extend `NpcConfig` to carry an optional profile

- [ ] Add `pub profile: Option<NpcProfile>` to `NpcConfig`.
- [ ] Update `AddNpcPlayersRequest` to accept an optional `profile_id: Option<String>` per
  NPC config — the backend loads the profile by ID from the profile store.

### 6.4 Thread the `LlmClient` into the NPC runner

- [ ] Add `llm_client: Option<Arc<LlmClient>>` to the runner's context (alongside
  `host_server` and `npc_configs`).
- [ ] In `try_npc_action`:
  - If the NPC has a `profile` and an `LlmClient` is available → use `choose_llm_action`.
  - Otherwise → use the existing Phase 1 rule-based path.

### 6.5 Store the `LlmClient` in `DesktopHostSession`

- [ ] Add `llm_client: Option<Arc<LlmClient>>` to `DesktopHostSession`.
- [ ] Initialise it from the stored API key when the session starts (if a key is configured).
- [ ] Pass it into `start_npc_runner`.

### 6.6 Tests for LLM strategy integration

- [ ] Test: when LLM returns a valid raise, the runner submits a raise within legal bounds.
- [ ] Test: when LLM call times out, the runner falls back to the rule-based strategy.
- [ ] Test: when LLM returns an illegal action, `validate_llm_action` produces a legal one.
- [ ] All tests use a mock `LlmClient` (inject via trait or a test-double struct).

---

## Part 7: Rust — API key management

**File:** `src-tauri/src/storage/` (extend) or `src-tauri/src/npc/api_key.rs`

### 7.1 Define API key storage path

- [ ] API key is stored in `{app_data_dir}/claude-api-key.txt` — a plaintext file with only
  the key, no surrounding whitespace.
- [ ] Do **not** store the key in `localStorage` or any frontend-accessible path.

### 7.2 Implement `load_api_key(app_data_dir: &Path) -> Option<String>`

- [ ] Read the key file; return `None` if it does not exist or is empty.
- [ ] Trim whitespace before returning.

### 7.3 Implement `save_api_key(app_data_dir: &Path, key: &str) -> Result<(), std::io::Error>`

- [ ] Write the trimmed key to the key file, creating it if necessary.
- [ ] If `key` is empty after trimming, delete the key file instead.

### 7.4 Expose the key status (not the key itself) to the frontend

- [ ] Add `llm_api_key_configured: bool` to `DesktopBootstrapState` so the UI knows
  whether to show LLM-capable NPC options.
- [ ] Never expose the raw key to the frontend.

### 7.5 Add `set_llm_api_key` and `clear_llm_api_key` Tauri commands

- [ ] `set_llm_api_key(key: String) -> Result<(), String>`:
  - Validate the key is non-empty.
  - Save it via `save_api_key`.
  - If a host session is active, reinitialise its `LlmClient` with the new key.
  - Emit `desktop://bootstrap-update` (or similar) so the frontend can refresh.
- [ ] `clear_llm_api_key() -> Result<(), String>`:
  - Delete the key file.
  - Nil out the `LlmClient` in any active session.

### 7.6 Tests for API key management

- [ ] Test: `save_api_key` followed by `load_api_key` returns the same trimmed key.
- [ ] Test: saving an empty string deletes the key file; subsequent `load_api_key` → `None`.
- [ ] Test: `DesktopBootstrapState.llm_api_key_configured` is `true` when key exists,
  `false` otherwise.

---

## Part 8: Rust — Profile management Tauri commands

**File:** `src-tauri/src/commands.rs`

### 8.1 Add `list_npc_profiles` command

- [ ] Define:
  ```rust
  #[tauri::command]
  pub fn list_npc_profiles(state: State<'_, DesktopAppState>) -> Result<Vec<NpcProfile>, String>
  ```
- [ ] Calls `profile_store::list_profiles`.

### 8.2 Add `get_npc_profile` command

- [ ] Define:
  ```rust
  #[tauri::command]
  pub fn get_npc_profile(state: State<'_, DesktopAppState>, id: String) -> Result<NpcProfile, String>
  ```
- [ ] Calls `profile_store::load_profile`.

### 8.3 Add `save_npc_profile` command

- [ ] Define:
  ```rust
  #[tauri::command]
  pub fn save_npc_profile(
      state: State<'_, DesktopAppState>,
      id: String,
      content: String,
  ) -> Result<NpcProfile, String>
  ```
- [ ] Saves and re-parses the profile; returns the parsed `NpcProfile` on success.

### 8.4 Add `delete_npc_profile` command

- [ ] Define:
  ```rust
  #[tauri::command]
  pub fn delete_npc_profile(state: State<'_, DesktopAppState>, id: String) -> Result<(), String>
  ```
- [ ] Calls `profile_store::delete_profile`. Refuses to delete built-in starter profiles.

### 8.5 Update `add_npc_players` to accept profile IDs

- [ ] Extend `AddNpcPlayersRequest`:
  ```rust
  pub struct NpcConfigRequest {
      pub display_name: String,
      pub style: NpcStyle,
      pub profile_id: Option<String>,  // new field
  }
  ```
- [ ] In `add_npc_players`, if `profile_id` is `Some`, load the profile from the store and
  attach it to the `NpcConfig`.

### 8.6 Register all new commands in `lib.rs`

- [ ] Add `list_npc_profiles`, `get_npc_profile`, `save_npc_profile`, `delete_npc_profile`,
  `set_llm_api_key`, `clear_llm_api_key` to the `invoke_handler` macro.

---

## Part 9: Frontend — API key settings screen

**File:** `src/screens/DeviceSettingsScreen.tsx` (extend) or a new `NpcSettingsScreen.tsx`

### 9.1 Add API key configuration UI

- [ ] Add an "AI Players" section to `DeviceSettingsScreen` (or a new settings screen).
- [ ] Show a masked text input for the Claude API key.
- [ ] Show a `"Key configured"` / `"No key set"` status indicator sourced from
  `bootstrap.llm_api_key_configured`.
- [ ] Provide a "Save key" button that calls `set_llm_api_key`.
- [ ] Provide a "Clear key" button (visible only when a key is configured) that calls
  `clear_llm_api_key`.
- [ ] Show success/error feedback inline.

### 9.2 Add API bridge functions for API key management

**File:** `src/api/desktop.ts`

- [ ] Add `setLlmApiKey(key: string): Promise<void>`.
- [ ] Add `clearLlmApiKey(): Promise<void>`.
- [ ] Add `llmApiKeyConfigured: boolean` to `DesktopBootstrapState` type.

### 9.3 Tests for API key settings UI

- [ ] Test: when `bootstrap.llmApiKeyConfigured` is `false`, status shows "No key set".
- [ ] Test: when `bootstrap.llmApiKeyConfigured` is `true`, status shows "Key configured".
- [ ] Test: clicking "Save key" calls `setLlmApiKey` with the entered value.
- [ ] Test: clicking "Clear key" calls `clearLlmApiKey`.
- [ ] Test: error from `setLlmApiKey` is displayed inline.

---

## Part 10: Frontend — Profile management UI

**File:** `src/screens/NpcProfilesScreen.tsx` (new) and `src/screens/HostTournamentSetupScreen.tsx` (extend)

### 10.1 Create `NpcProfilesScreen`

- [ ] Add a new route `/npc-profiles` for browsing and editing profiles.
- [ ] List all profiles returned by `listNpcProfiles` API call.
- [ ] Show profile name, style, skill in a card per profile.
- [ ] Provide a "View / Edit" action that opens a detail view with the raw Markdown content
  in an editable textarea.
- [ ] Provide a "Save" button that calls `saveNpcProfile`.
- [ ] Provide a "Delete" button (disabled for built-in starter profiles) that calls
  `deleteNpcProfile`.
- [ ] Provide a "New profile" flow: slug input + Markdown editor + Save.

### 10.2 Add API bridge functions for profile management

**File:** `src/api/desktop.ts`

- [ ] Add `NpcProfile` type:
  ```typescript
  export type NpcProfile = {
    id: string;
    name: string;
    style: string;
    skill: string;
    description: string;
  };
  ```
- [ ] Add `listNpcProfiles(): Promise<NpcProfile[]>`.
- [ ] Add `getNpcProfile(id: string): Promise<NpcProfile>`.
- [ ] Add `saveNpcProfile(id: string, content: string): Promise<NpcProfile>`.
- [ ] Add `deleteNpcProfile(id: string): Promise<void>`.

### 10.3 Extend host setup to support profile selection per NPC seat

**File:** `src/screens/HostTournamentSetupScreen.tsx`

- [ ] When the LLM API key is configured (`bootstrap.llmApiKeyConfigured`), show a
  "Profile" select per NPC entry (populated from `listNpcProfiles`).
- [ ] Default to "Rule-based (no profile)" when no profiles are available or the user
  does not select one.
- [ ] Update `AddNpcPlayersRequest` to include `profileId: string | null` per NPC.
- [ ] When key is not configured, hide the profile select and show a hint:
  "Add a Claude API key in settings to use AI profiles."

### 10.4 Update `HostDraft` to carry per-NPC profile selections

- [ ] Add `npcProfileIds: (string | null)[]` to `HostDraft` (one entry per NPC seat,
  ordered by seat index).
- [ ] Add `npcProfileIds: []` to `createDefaultHostDraft`.
- [ ] Add normalization in `normalizeHostDraft`: validate each entry is a string or null,
  fallback to `[]` on invalid input.

### 10.5 Extend lobby display for LLM-profile NPCs

**File:** `src/screens/TournamentLobbyScreen.tsx`

- [ ] In `buildLiveSeats`, if an NPC seat's profile name is available (surfaced through
  `HostSessionStatus.participants`), show it as the seat detail:
  `"(AI) {profileName} · Always ready"`.
- [ ] Fall back to `"(AI) · Always ready"` for rule-based NPCs.

### 10.6 Add navigation entry points for profile management

- [ ] Add a "Manage AI profiles" link/button from `DeviceSettingsScreen` (or the AI section
  of settings) that navigates to `/npc-profiles`.
- [ ] Add the route to the router configuration.

### 10.7 Tests for profile management UI

- [ ] Test: `NpcProfilesScreen` lists profiles returned by `listNpcProfiles`.
- [ ] Test: clicking "Save" on a profile calls `saveNpcProfile` with the correct arguments.
- [ ] Test: clicking "Delete" on a non-builtin profile calls `deleteNpcProfile`.
- [ ] Test: "Delete" is disabled for built-in starter profiles.
- [ ] Test: when `llmApiKeyConfigured` is true, the profile select appears in host setup.
- [ ] Test: when `llmApiKeyConfigured` is false, the profile select is hidden and the hint
  is shown.

---

## Part 11: Rust — Built-in starter profiles

**Directory:** `src-tauri/src/npc/profiles/` (embedded via `include_str!`)

### 11.1 Write `aggressive-alice.md`

- [ ] Write a profile for a loose-aggressive player:
  - Enters pots with the top ~55% of hands.
  - Continuation-bets every flop, barrels draws on the turn.
  - Bluffs on scare-card rivers (~40% of missed draws).
  - Tell: pot-sized or larger bets are always nuts or total bluff.

### 11.2 Write `conservative-carlos.md`

- [ ] Write a profile for a tight-passive player:
  - Only plays premium pairs and AK/AQ.
  - Checks or calls with top pair; bets only with two pair or better.
  - Never bluffs; folds to any bet when holding less than two pair.

### 11.3 Write `balanced-sam.md`

- [ ] Write a profile for a balanced, GTO-approximating player:
  - Mixed ranges — enters ~35% of pots pre-flop.
  - Bets for value and protection; occasionally semi-bluffs draws.
  - Adjusts bet sizing to pot type (dry board vs. wet board).

### 11.4 Embed profiles with `include_str!` in `profile_store.rs`

- [ ] Use `include_str!` to embed each starter profile at compile time.
- [ ] On first run (or empty profiles directory), write them to the profiles directory so
  the user can view and edit them.

---

## Part 12: Validation and sign-off

### 12.1 Run all Rust tests

- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

### 12.2 Run all frontend tests

- [ ] `npm run lint`
- [ ] `npm run test`

### 12.3 Manual QA checklist

- [ ] Configure a Claude API key in settings; confirm `llmApiKeyConfigured` becomes `true`.
- [ ] Navigate to `/npc-profiles`; confirm the 3 starter profiles are listed.
- [ ] Edit a profile body and save; confirm the change persists across app restarts.
- [ ] Create a new profile; confirm it appears in the list and in the host setup profile
  select.
- [ ] Delete a custom profile; confirm it disappears. Attempt to delete a built-in profile;
  confirm the button is disabled.
- [ ] Start a host session with 2 NPC seats: one with a profile, one rule-based.
  - Verify the profile-driven NPC acts within 5 seconds (LLM round-trip).
  - Verify the rule-based NPC acts within 1–2 seconds.
- [ ] Revoke API key mid-session (clear it in settings); verify the LLM NPC falls back to
  rule-based play for the remainder of the session.
- [ ] Simulate LLM timeout (use a profile that triggers a slow path or disable network);
  verify the NPC falls back within 5 seconds and the game continues.
- [ ] Start a session with 0 NPCs; verify no API calls are made.
- [ ] Run a full session to completion with 1 human player + 2 LLM NPCs.

### 12.4 Commit and push

- [ ] Commit all Rust and frontend changes with message referencing NPC Phase 2.
- [ ] Push to GitHub.

---

## Deliverables

- [ ] `src-tauri/src/npc/profile.rs` — profile data model and parser
- [ ] `src-tauri/src/npc/profile_store.rs` — filesystem profile CRUD + starter profiles
- [ ] `src-tauri/src/npc/prompt.rs` — game state renderer and prompt builder
- [ ] `src-tauri/src/npc/llm_client.rs` — Claude API HTTP client with timeout
- [ ] `src-tauri/src/npc/llm_action.rs` — LLM response parser and action validator
- [ ] `src-tauri/src/npc/llm_strategy.rs` — LLM action selection with rule-based fallback
- [ ] API key management: `save`/`load`/`clear` with Tauri commands
- [ ] Profile management: Tauri commands (`list`, `get`, `save`, `delete`)
- [ ] Updated `add_npc_players` accepting optional `profile_id` per NPC
- [ ] `NpcProfilesScreen.tsx` — profile browser and editor
- [ ] API key settings section in `DeviceSettingsScreen`
- [ ] Host setup: profile select per NPC seat (when API key configured)
- [ ] Lobby: profile name in NPC seat detail
- [ ] 3 bundled starter profiles
- [ ] Unit tests: profile parser, prompt renderer, API client, response validator
- [ ] Frontend tests: profile UI, key settings UI
- [ ] Manual QA sign-off
