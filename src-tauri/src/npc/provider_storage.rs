use std::{
    fs,
    path::{Path, PathBuf},
};

use super::provider::{LlmProviderConfig, LlmProviderSettings};

/// Non-secret provider settings (provider type, endpoint, model).
const SETTINGS_FILE_NAME: &str = "llm-provider.json";
/// Secret API key stored separately with restricted file permissions (debug builds only).
const KEY_FILE_NAME: &str = "llm-provider-key.dat";
/// Legacy plain-text key written by Phase 2; read once for migration.
const LEGACY_KEY_FILE_NAME: &str = "claude-api-key.txt";
/// Keychain service name used in release builds.
#[cfg(not(debug_assertions))]
const KEYCHAIN_SERVICE: &str = "desktop-poker-llm-provider";

/// All known per-provider keychain account identifiers.
/// Used for exhaustive clear when provider settings are unavailable (P0.5).
const KNOWN_PROVIDER_ACCOUNTS: &[&str] = &["anthropic", "openAi"];

// ---------------------------------------------------------------------------
// ProviderSecretStore trait — abstract secret storage boundary.
//
// Implementations:
//   KeychainSecretStore      — release builds, OS keychain per-provider account.
//   PlaintextFileSecretStore — debug builds, single shared key file.
//   InMemorySecretStore      — tests, in-process hash map (behind #[cfg(test)]).
//   FailingSecretStore       — tests, simulates keychain failures.
//
// The `provider` parameter maps to a keychain account name in release builds;
// it is ignored in debug builds (single shared file, not per-provider).
// ---------------------------------------------------------------------------

pub trait ProviderSecretStore {
    fn read_key(&self, provider: &str) -> Result<Option<String>, String>;
    fn write_key(&self, provider: &str, key: &str) -> Result<(), String>;
    fn delete_key(&self, provider: &str) -> Result<(), String>;
}

// ---------------------------------------------------------------------------
// Release build: OS keychain implementation.
// ---------------------------------------------------------------------------

pub struct KeychainSecretStore;

#[cfg(not(debug_assertions))]
impl ProviderSecretStore for KeychainSecretStore {
    fn read_key(&self, provider: &str) -> Result<Option<String>, String> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, provider)
            .map_err(|e| format!("keychain entry creation failed: {e}"))?;
        match entry.get_password() {
            Ok(pw) if pw.is_empty() => Ok(None),
            Ok(pw) => Ok(Some(pw)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("keychain read failed: {e}")),
        }
    }

    fn write_key(&self, provider: &str, key: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, provider)
            .map_err(|e| format!("keychain entry creation failed: {e}"))?;
        entry
            .set_password(key)
            .map_err(|e| format!("keychain write failed: {e}"))
    }

    fn delete_key(&self, provider: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, provider)
            .map_err(|e| format!("keychain entry creation failed: {e}"))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("keychain delete failed: {e}")),
        }
    }
}

// In debug builds we still need a KeychainSecretStore to satisfy the type
// system at call sites even if it is never reached at runtime.
#[cfg(debug_assertions)]
impl ProviderSecretStore for KeychainSecretStore {
    fn read_key(&self, _provider: &str) -> Result<Option<String>, String> {
        Err("KeychainSecretStore not available in debug builds".to_string())
    }
    fn write_key(&self, _provider: &str, _key: &str) -> Result<(), String> {
        Err("KeychainSecretStore not available in debug builds".to_string())
    }
    fn delete_key(&self, _provider: &str) -> Result<(), String> {
        Err("KeychainSecretStore not available in debug builds".to_string())
    }
}

// ---------------------------------------------------------------------------
// Debug build: plaintext file implementation (one shared key file).
// ---------------------------------------------------------------------------

pub struct PlaintextFileSecretStore {
    key_path: PathBuf,
}

impl PlaintextFileSecretStore {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            key_path: app_data_dir.join(KEY_FILE_NAME),
        }
    }
}

impl ProviderSecretStore for PlaintextFileSecretStore {
    fn read_key(&self, _provider: &str) -> Result<Option<String>, String> {
        read_key_file(&self.key_path)
    }

    fn write_key(&self, _provider: &str, key: &str) -> Result<(), String> {
        write_key_file(&self.key_path, key).map_err(|e| e.to_string())
    }

    fn delete_key(&self, _provider: &str) -> Result<(), String> {
        if self.key_path.exists() {
            fs::remove_file(&self.key_path).map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }
}

/// Construct the appropriate secret store for the current build configuration.
pub fn default_secret_store(app_data_dir: &Path) -> Box<dyn ProviderSecretStore + Send + Sync> {
    #[cfg(debug_assertions)]
    return Box::new(PlaintextFileSecretStore::new(app_data_dir));
    #[cfg(not(debug_assertions))]
    {
        let _ = app_data_dir;
        return Box::new(KeychainSecretStore);
    }
}

/// Outcome of attempting to load provider configuration from disk.
#[derive(Debug)]
pub enum ProviderConfigLoadState {
    /// No config file exists and no legacy key was found. This is the normal
    /// "never configured" state and is not an error.
    Missing,
    /// Config was read and parsed successfully.
    Loaded(LlmProviderConfig),
    /// Config file exists but could not be read (permissions, I/O error).
    Unreadable { error: String },
    /// Config file was read but is not valid JSON.
    InvalidJson { error: String },
    /// Config file is valid JSON but does not match the expected schema.
    InvalidSchema { error: String },
    /// Settings file loaded fine but the API key file exists and cannot be read.
    /// Distinct from missing key — indicates broken local state or permissions.
    KeyUnreadable { error: String },
    /// The legacy `claude-api-key.txt` file exists but cannot be read.
    /// Distinct from `Missing` — the file is present but inaccessible, which
    /// indicates broken permissions or filesystem state.
    LegacyKeyUnreadable { error: String },
}

fn settings_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(SETTINGS_FILE_NAME)
}

#[cfg(test)]
fn key_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(KEY_FILE_NAME)
}

fn legacy_key_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(LEGACY_KEY_FILE_NAME)
}

/// Read the raw API key from `llm-provider-key.dat`.
///
/// Returns `Ok(None)` when the file does not exist or is empty.
/// Returns `Err(message)` when the file exists but cannot be read — this is
/// distinct from a missing file and must not be silently treated as "no key."
fn read_key_file(path: &Path) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).map_err(|e| {
        format!(
            "key file exists but cannot be read ({}): {e}",
            path.display()
        )
    })?;
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed))
    }
}

/// Write `key` to `path` and restrict permissions to owner-only on Unix.
fn write_key_file(path: &Path, key: &str) -> Result<(), std::io::Error> {
    fs::write(path, key)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Load the provider config from disk, returning a rich load state.
///
/// ## File layout (current)
/// - `llm-provider.json` — non-secret settings (`LlmProviderSettings`)
/// - Key storage per build type:
///   - Release: OS keychain (per-provider account)
///   - Debug: `llm-provider-key.dat` (single shared file)
///
/// ## Migration paths
/// 1. **Combined old format** — `llm-provider.json` contains both settings and
///    `apiKey` field (written by an older version). On first load the key is
///    moved to the appropriate key storage and the settings file is rewritten
///    without it. If the key storage write fails (P0.4), the settings file is
///    NOT rewritten and a visible error is returned instead.
/// 2. **Legacy plain-text key** — `claude-api-key.txt` from Phase 2 is promoted
///    to an Anthropic config if neither current file exists.
pub fn load_provider_config(
    app_data_dir: &Path,
    store: &dyn ProviderSecretStore,
) -> ProviderConfigLoadState {
    let sp = settings_path(app_data_dir);

    if sp.exists() {
        let text = match fs::read_to_string(&sp) {
            Ok(t) => t,
            Err(e) => {
                return ProviderConfigLoadState::Unreadable {
                    error: e.to_string(),
                }
            }
        };

        // Parse as a JSON value so we can inspect for the legacy `apiKey` field.
        let json_val: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                return ProviderConfigLoadState::InvalidJson {
                    error: e.to_string(),
                }
            }
        };

        // Deserialise non-secret settings (unknown fields like `apiKey` are
        // silently ignored, so old combined files parse fine here).
        let settings: LlmProviderSettings = match serde_json::from_value(json_val.clone()) {
            Ok(s) => s,
            Err(e) => {
                return ProviderConfigLoadState::InvalidSchema {
                    error: e.to_string(),
                }
            }
        };

        // Keyless providers must not touch platform secret storage. This keeps
        // Ollama, llama-server, and embedded GGUF usable on systems without a
        // Secret Service while preserving fail-closed behavior for API providers.
        let api_key = if settings.provider.uses_api_key() {
            let provider_str = settings.provider.as_str();
            match store.read_key(provider_str) {
                Err(e) => {
                    // Key storage exists/attempted but returned an error — surface explicitly.
                    return ProviderConfigLoadState::KeyUnreadable { error: e };
                }
                Ok(Some(key)) => Some(key),
                Ok(None) => {
                    // Migration: extract key embedded in old combined format.
                    let embedded = json_val
                        .get("apiKey")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string());

                    if let Some(ref key) = embedded {
                        // P0.4: Only rewrite settings file if key storage write succeeded.
                        // A failed write must not erase the embedded key — that would cause
                        // silent key loss on next load.
                        match store.write_key(provider_str, key) {
                            Ok(()) => {
                                if let Ok(json) = serde_json::to_string_pretty(&settings) {
                                    if let Err(e) = fs::write(&sp, json) {
                                        eprintln!(
                                            "[provider-storage] migration: failed to rewrite settings \
                                             file: {e}"
                                        );
                                    } else {
                                        eprintln!(
                                            "[provider-storage] migrated API key from embedded field"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                // Migration failed — return a visible error and leave settings intact.
                                return ProviderConfigLoadState::KeyUnreadable {
                                    error: format!("migration to secure storage failed: {e}"),
                                };
                            }
                        }
                    }
                    embedded
                }
            }
        } else {
            None
        };

        return ProviderConfigLoadState::Loaded(LlmProviderConfig { settings, api_key });
    }

    // Fall back to the legacy plain-text API key.
    let legacy = legacy_key_path(app_data_dir);
    if legacy.exists() {
        let raw = match fs::read_to_string(&legacy) {
            Ok(raw) => raw,
            Err(e) => {
                // The file exists but cannot be read — this is distinct from
                // "not configured" and must not silently become Missing.
                return ProviderConfigLoadState::LegacyKeyUnreadable {
                    error: format!(
                        "legacy provider key exists but cannot be read ({}): {e}",
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

    ProviderConfigLoadState::Missing
}

/// Write the provider config.
///
/// The API key is stored via the injected `store` (keychain in release, file in debug).
/// If `config` is `None`, the settings file is removed and all known API-provider secret
/// accounts are deleted — even when the settings file is corrupt or missing (P0.5).
pub fn save_provider_config(
    app_data_dir: &Path,
    config: Option<&LlmProviderConfig>,
    store: &dyn ProviderSecretStore,
) -> Result<(), std::io::Error> {
    let sp = settings_path(app_data_dir);

    match config {
        None => {
            // P0.5: Delete all known provider accounts unconditionally — don't rely on
            // being able to parse the settings file (it may be corrupt or missing).
            // P1.5: Collect delete failures; attempt all deletes before returning an error.
            let mut delete_errors: Vec<String> = Vec::new();
            for account in KNOWN_PROVIDER_ACCOUNTS {
                if let Err(e) = store.delete_key(account) {
                    delete_errors.push(format!("delete '{account}': {e}"));
                }
            }
            if !delete_errors.is_empty() {
                return Err(std::io::Error::other(format!(
                    "provider secret clear failed: {}",
                    delete_errors.join("; ")
                )));
            }

            if sp.exists() {
                fs::remove_file(&sp)?;
            }
        }
        Some(cfg) => {
            if let Some(parent) = sp.parent() {
                fs::create_dir_all(parent)?;
            }

            let current = read_existing_provider_settings_for_update(&sp)?;
            let key = cfg.api_key.as_deref().unwrap_or("").trim().to_string();
            let new_account = cfg
                .settings
                .provider
                .uses_api_key()
                .then(|| cfg.settings.provider.as_str());
            let old_account = current.as_ref().and_then(|settings| {
                settings
                    .provider
                    .uses_api_key()
                    .then(|| settings.provider.as_str())
            });
            let provider_changed = current
                .as_ref()
                .is_some_and(|settings| settings.provider != cfg.settings.provider);

            // Keyless providers never touch the keychain. When changing away from an
            // API provider, clear its old account before persisting the new settings.
            if provider_changed {
                match (old_account, new_account, key.is_empty()) {
                    (Some(old), Some(new), false) => {
                        store.write_key(new, &key).map_err(|error| {
                            std::io::Error::other(format!(
                                "could not write provider key for {new}: {error}"
                            ))
                        })?;
                        if let Err(error) = store.delete_key(old) {
                            let rollback = store.delete_key(new).err();
                            return Err(std::io::Error::other(format!(
                                "could not clear stale provider key for {old}: {error};                                  new-key rollback: {}",
                                rollback.as_deref().unwrap_or("success")
                            )));
                        }
                    }
                    (Some(old), Some(new), true) => {
                        let mut errors = Vec::new();
                        for account in [old, new] {
                            if let Err(error) = store.delete_key(account) {
                                errors.push(format!("delete '{account}': {error}"));
                            }
                        }
                        if !errors.is_empty() {
                            return Err(std::io::Error::other(format!(
                                "provider secret switch failed: {}",
                                errors.join("; ")
                            )));
                        }
                    }
                    (Some(old), None, _) => {
                        store.delete_key(old).map_err(|error| {
                            std::io::Error::other(format!(
                                "could not clear stale provider key for {old}: {error}"
                            ))
                        })?;
                    }
                    (None, Some(new), false) => {
                        store.write_key(new, &key).map_err(|error| {
                            std::io::Error::other(format!(
                                "could not write provider key for {new}: {error}"
                            ))
                        })?;
                    }
                    (None, Some(new), true) => {
                        store.delete_key(new).map_err(|error| {
                            std::io::Error::other(format!(
                                "could not delete provider key for {new}: {error}"
                            ))
                        })?;
                    }
                    (None, None, _) => {}
                }
            } else if let Some(account) = new_account {
                if key.is_empty() {
                    store.delete_key(account).map_err(|error| {
                        std::io::Error::other(format!(
                            "could not delete provider key for {account}: {error}"
                        ))
                    })?;
                } else {
                    store.write_key(account, &key).map_err(|error| {
                        std::io::Error::other(format!(
                            "could not write provider key for {account}: {error}"
                        ))
                    })?;
                }
            }

            // Secret operations succeeded (or were unnecessary for a keyless
            // provider), so it is safe to write non-secret settings.
            let settings_json = serde_json::to_string_pretty(&cfg.settings)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            fs::write(&sp, settings_json)?;
        }
    }
    Ok(())
}

/// Save only the non-secret provider settings, preserving any existing stored key.
///
/// Use this when the user edits endpoint/model/provider fields without entering
/// a new API key. If the provider type changes, the old key is deleted via the
/// store so it cannot be re-associated with the new provider (P0.3).
/// Read existing settings only if the file is absent (returns `Ok(None)`) or
/// fully valid (returns `Ok(Some(...))`).  If the file exists but cannot be
/// read or parsed, returns an error — the caller must not overwrite a corrupt
/// settings file without understanding what it contains.
fn read_existing_provider_settings_for_update(
    sp: &Path,
) -> Result<Option<LlmProviderSettings>, std::io::Error> {
    if !sp.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(sp).map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!(
                "existing provider settings file exists but cannot be read ({}): {e}",
                sp.display()
            ),
        )
    })?;
    serde_json::from_str::<LlmProviderSettings>(&text)
        .map(Some)
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "existing provider settings file is invalid and cannot be safely updated \
                     ({}): {e}",
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

    // If the existing settings file is unreadable or corrupt, refuse to
    // overwrite it — returning an error surfaces the problem instead of
    // silently destroying state.
    let current = read_existing_provider_settings_for_update(&sp)?;

    // When the provider type changes, clear every API-provider account that
    // could otherwise be associated with the new settings. Keyless-to-keyless
    // changes require no secret-store access.
    if let Some(current) = current.as_ref() {
        if current.provider != settings.provider {
            let mut accounts = Vec::new();
            if current.provider.uses_api_key() {
                accounts.push(current.provider.as_str());
            }
            if settings.provider.uses_api_key() {
                accounts.push(settings.provider.as_str());
            }
            accounts.dedup();

            let mut errors = Vec::new();
            for account in accounts {
                if let Err(error) = store.delete_key(account) {
                    errors.push(format!("delete '{account}': {error}"));
                }
            }
            if !errors.is_empty() {
                return Err(std::io::Error::other(format!(
                    "could not clear stale keys after provider change ({} → {}): {}",
                    current.provider.as_str(),
                    settings.provider.as_str(),
                    errors.join("; ")
                )));
            }
        }
    }

    if let Some(parent) = sp.parent() {
        fs::create_dir_all(parent)?;
    }
    let settings_json = serde_json::to_string_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(&sp, settings_json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use super::super::provider::LlmProviderType;
    use super::*;

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

    fn anthropic_config(key: &str) -> LlmProviderConfig {
        LlmProviderConfig {
            settings: LlmProviderSettings {
                provider: LlmProviderType::Anthropic,
                endpoint_url: None,
                model: None,
            },
            api_key: Some(key.to_string()),
        }
    }

    fn unwrap_loaded(state: ProviderConfigLoadState) -> LlmProviderConfig {
        match state {
            ProviderConfigLoadState::Loaded(cfg) => cfg,
            other => panic!("expected Loaded, got {other:?}"),
        }
    }

    /// File-based store (mirrors debug production behavior); accepts a tempdir path.
    fn file_store(dir: &std::path::Path) -> PlaintextFileSecretStore {
        PlaintextFileSecretStore::new(dir)
    }

    /// Fails every operation so tests can prove keyless providers never touch secrets.
    struct RejectingSecretStore;

    impl ProviderSecretStore for RejectingSecretStore {
        fn read_key(&self, _provider: &str) -> Result<Option<String>, String> {
            Err("secret store must not be accessed".to_string())
        }
        fn write_key(&self, _provider: &str, _key: &str) -> Result<(), String> {
            Err("secret store must not be accessed".to_string())
        }
        fn delete_key(&self, _provider: &str) -> Result<(), String> {
            Err("secret store must not be accessed".to_string())
        }
    }

    /// Simulates a store where every write operation fails. Used for P0.4 tests.
    struct FailingWriteSecretStore;

    /// Simulates a store where every delete operation fails. Used for P1.5 tests.
    struct FailingDeleteSecretStore;

    impl ProviderSecretStore for FailingDeleteSecretStore {
        fn read_key(&self, _provider: &str) -> Result<Option<String>, String> {
            Ok(None)
        }
        fn write_key(&self, _provider: &str, _key: &str) -> Result<(), String> {
            Ok(())
        }
        fn delete_key(&self, _provider: &str) -> Result<(), String> {
            Err("simulated secure storage delete failure".to_string())
        }
    }

    impl ProviderSecretStore for FailingWriteSecretStore {
        fn read_key(&self, _provider: &str) -> Result<Option<String>, String> {
            Ok(None)
        }
        fn write_key(&self, _provider: &str, _key: &str) -> Result<(), String> {
            Err("simulated secure storage write failure".to_string())
        }
        fn delete_key(&self, _provider: &str) -> Result<(), String> {
            Ok(())
        }
    }

    /// In-memory store that tracks which accounts were deleted. Used for P0.5 tests.
    struct TrackingSecretStore {
        keys: RefCell<HashMap<String, String>>,
        deleted: RefCell<Vec<String>>,
    }

    impl TrackingSecretStore {
        fn new() -> Self {
            Self {
                keys: RefCell::new(HashMap::new()),
                deleted: RefCell::new(Vec::new()),
            }
        }

        fn with_key(self, provider: &str, key: &str) -> Self {
            self.keys
                .borrow_mut()
                .insert(provider.to_string(), key.to_string());
            self
        }

        fn deleted_accounts(&self) -> Vec<String> {
            self.deleted.borrow().clone()
        }
    }

    impl ProviderSecretStore for TrackingSecretStore {
        fn read_key(&self, provider: &str) -> Result<Option<String>, String> {
            Ok(self.keys.borrow().get(provider).cloned())
        }

        fn write_key(&self, provider: &str, key: &str) -> Result<(), String> {
            self.keys
                .borrow_mut()
                .insert(provider.to_string(), key.to_string());
            Ok(())
        }

        fn delete_key(&self, provider: &str) -> Result<(), String> {
            self.keys.borrow_mut().remove(provider);
            self.deleted.borrow_mut().push(provider.to_string());
            Ok(())
        }
    }

    fn embedded_config() -> LlmProviderConfig {
        LlmProviderConfig {
            settings: LlmProviderSettings {
                provider: LlmProviderType::EmbeddedLocal,
                endpoint_url: None,
                model: Some("/tmp/embedded.gguf".to_string()),
            },
            api_key: None,
        }
    }

    #[test]
    fn keyless_provider_save_and_load_never_access_secret_store() {
        let dir = tempfile::tempdir().unwrap();
        save_provider_config(dir.path(), Some(&embedded_config()), &RejectingSecretStore).unwrap();

        let loaded = unwrap_loaded(load_provider_config(dir.path(), &RejectingSecretStore));
        assert_eq!(loaded.settings.provider, LlmProviderType::EmbeddedLocal);
        assert!(loaded.api_key.is_none());
    }

    #[test]
    fn keyless_settings_switch_never_accesses_secret_store() {
        let dir = tempfile::tempdir().unwrap();
        save_provider_config(dir.path(), Some(&embedded_config()), &RejectingSecretStore).unwrap();
        let ollama = LlmProviderSettings {
            provider: LlmProviderType::Ollama,
            endpoint_url: None,
            model: None,
        };

        save_provider_settings_only(dir.path(), &ollama, &RejectingSecretStore).unwrap();
        let loaded = unwrap_loaded(load_provider_config(dir.path(), &RejectingSecretStore));
        assert_eq!(loaded.settings.provider, LlmProviderType::Ollama);
        assert!(loaded.api_key.is_none());
    }

    #[test]
    fn full_config_switch_to_keyless_provider_clears_old_api_key() {
        let dir = tempfile::tempdir().unwrap();
        let tracking = TrackingSecretStore::new().with_key("anthropic", "sk-ant-old");
        fs::write(
            settings_path(dir.path()),
            serde_json::to_string_pretty(&anthropic_config("unused").settings).unwrap(),
        )
        .unwrap();

        save_provider_config(dir.path(), Some(&embedded_config()), &tracking).unwrap();

        assert!(tracking.deleted_accounts().contains(&"anthropic".to_string()));
        let loaded = unwrap_loaded(load_provider_config(dir.path(), &tracking));
        assert_eq!(loaded.settings.provider, LlmProviderType::EmbeddedLocal);
        assert!(loaded.api_key.is_none());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        let cfg = anthropic_config("sk-ant-test");
        save_provider_config(dir.path(), Some(&cfg), &store).unwrap();
        let loaded = unwrap_loaded(load_provider_config(dir.path(), &store));
        assert_eq!(loaded.settings.provider, LlmProviderType::Anthropic);
        assert_eq!(loaded.api_key.as_deref(), Some("sk-ant-test"));
    }

    #[test]
    fn save_none_removes_settings_file_and_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        save_provider_config(dir.path(), Some(&anthropic_config("k")), &store).unwrap();
        save_provider_config(dir.path(), None, &store).unwrap();
        assert!(matches!(
            load_provider_config(dir.path(), &store),
            ProviderConfigLoadState::Missing
        ));
        assert!(!settings_path(dir.path()).exists());
        assert!(!key_path(dir.path()).exists());
    }

    #[test]
    fn load_nonexistent_returns_missing() {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        assert!(matches!(
            load_provider_config(dir.path(), &store),
            ProviderConfigLoadState::Missing
        ));
    }

    #[test]
    fn corrupt_json_returns_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        fs::write(settings_path(dir.path()), "not json at all").unwrap();
        assert!(matches!(
            load_provider_config(dir.path(), &store),
            ProviderConfigLoadState::InvalidJson { .. }
        ));
    }

    #[test]
    fn invalid_schema_returns_invalid_schema() {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        fs::write(settings_path(dir.path()), r#"{"unexpected":"field"}"#).unwrap();
        assert!(matches!(
            load_provider_config(dir.path(), &store),
            ProviderConfigLoadState::InvalidSchema { .. }
        ));
    }

    #[test]
    fn legacy_key_file_is_promoted_to_anthropic_config() {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        fs::write(dir.path().join(LEGACY_KEY_FILE_NAME), "sk-ant-legacy").unwrap();
        let loaded = unwrap_loaded(load_provider_config(dir.path(), &store));
        assert_eq!(loaded.settings.provider, LlmProviderType::Anthropic);
        assert_eq!(loaded.api_key.as_deref(), Some("sk-ant-legacy"));
    }

    #[test]
    fn new_settings_file_takes_precedence_over_legacy_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        fs::write(dir.path().join(LEGACY_KEY_FILE_NAME), "sk-ant-legacy").unwrap();
        let new_cfg = LlmProviderConfig {
            settings: LlmProviderSettings {
                provider: LlmProviderType::Ollama,
                endpoint_url: Some("http://localhost:11434".to_string()),
                model: None,
            },
            api_key: None,
        };
        save_provider_config(dir.path(), Some(&new_cfg), &store).unwrap();
        let loaded = unwrap_loaded(load_provider_config(dir.path(), &store));
        assert_eq!(loaded.settings.provider, LlmProviderType::Ollama);
    }

    #[test]
    fn settings_file_does_not_contain_api_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        save_provider_config(dir.path(), Some(&anthropic_config("sk-ant-secret")), &store).unwrap();
        let settings_json = fs::read_to_string(settings_path(dir.path())).unwrap();
        assert!(
            !settings_json.contains("sk-ant-secret"),
            "llm-provider.json must not contain the API key; got: {settings_json}"
        );
        assert!(
            !settings_json.contains("apiKey"),
            "llm-provider.json must not contain the apiKey field; got: {settings_json}"
        );
    }

    #[test]
    fn api_key_is_stored_in_dedicated_key_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        save_provider_config(dir.path(), Some(&anthropic_config("sk-ant-secret")), &store).unwrap();
        let key_content = fs::read_to_string(key_path(dir.path())).unwrap();
        assert_eq!(
            key_content.trim(),
            "sk-ant-secret",
            "llm-provider-key.dat must contain just the raw key"
        );
    }

    #[test]
    fn migration_from_combined_old_format_extracts_key_and_rewrites_settings() {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        // Write old combined format (settings + apiKey in one file).
        let old_json = r#"{"provider":"anthropic","apiKey":"sk-ant-migrated","endpointUrl":null,"model":null}"#;
        fs::write(settings_path(dir.path()), old_json).unwrap();

        // Load should trigger migration.
        let loaded = unwrap_loaded(load_provider_config(dir.path(), &store));
        assert_eq!(loaded.api_key.as_deref(), Some("sk-ant-migrated"));
        assert_eq!(loaded.settings.provider, LlmProviderType::Anthropic);

        // After migration: key file exists, settings file no longer contains the key.
        let settings_json = fs::read_to_string(settings_path(dir.path())).unwrap();
        assert!(
            !settings_json.contains("sk-ant-migrated"),
            "settings file should not contain the key after migration"
        );
        assert!(
            !settings_json.contains("apiKey"),
            "settings file should not contain apiKey field after migration"
        );
        assert!(
            key_path(dir.path()).exists(),
            "key file must exist after migration"
        );
        let key_raw = fs::read_to_string(key_path(dir.path())).unwrap();
        assert_eq!(key_raw.trim(), "sk-ant-migrated");
    }

    #[test]
    fn debug_repr_of_provider_config_redacts_api_key() {
        let cfg = anthropic_config("sk-ant-super-secret-key-value-1234");
        let debug_str = format!("{cfg:?}");
        assert!(
            !debug_str.contains("super-secret-key-value-1234"),
            "Debug output must not expose the full API key; got: {debug_str}"
        );
        assert!(
            debug_str.contains("redacted") || debug_str.contains("…"),
            "Debug output should indicate redaction; got: {debug_str}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn saved_key_file_has_restricted_file_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        save_provider_config(dir.path(), Some(&anthropic_config("sk-secret")), &store).unwrap();
        let meta = fs::metadata(key_path(dir.path())).unwrap();
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "key file must be owner-only readable (0600); got {mode:#o}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_key_file_returns_key_unreadable_not_missing() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        // Write valid settings and an unreadable key file (mode 000).
        save_provider_config(dir.path(), Some(&anthropic_config("sk-ant-secret")), &store).unwrap();
        let kp = key_path(dir.path());
        fs::set_permissions(&kp, fs::Permissions::from_mode(0o000)).unwrap();
        let result = load_provider_config(dir.path(), &store);
        // Restore permissions so the temp dir cleanup can remove the file.
        fs::set_permissions(&kp, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(
            matches!(result, ProviderConfigLoadState::KeyUnreadable { .. }),
            "expected KeyUnreadable, got {result:?}"
        );
    }

    #[test]
    fn missing_key_file_with_key_required_provider_loads_with_no_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        let settings = LlmProviderSettings {
            provider: LlmProviderType::Anthropic,
            endpoint_url: None,
            model: None,
        };
        save_provider_settings_only(dir.path(), &settings, &store).unwrap();
        let loaded = unwrap_loaded(load_provider_config(dir.path(), &store));
        assert_eq!(loaded.settings.provider, LlmProviderType::Anthropic);
        assert!(
            loaded.api_key.is_none(),
            "missing key file should result in no key, not an error"
        );
    }

    #[test]
    fn save_settings_only_preserves_existing_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        save_provider_config(
            dir.path(),
            Some(&anthropic_config("sk-ant-original")),
            &store,
        )
        .unwrap();

        let new_settings = LlmProviderSettings {
            provider: LlmProviderType::Anthropic,
            endpoint_url: Some("https://custom.example.com".to_string()),
            model: Some("claude-haiku-4-5-20251001".to_string()),
        };
        save_provider_settings_only(dir.path(), &new_settings, &store).unwrap();

        let loaded = unwrap_loaded(load_provider_config(dir.path(), &store));
        assert_eq!(
            loaded.settings.endpoint_url.as_deref(),
            Some("https://custom.example.com"),
            "endpoint_url should be updated"
        );
        assert_eq!(
            loaded.api_key.as_deref(),
            Some("sk-ant-original"),
            "existing API key must be preserved for same-provider edit"
        );
    }

    /// P0.3: Switching providers via save_settings_only must clear the old key so
    /// the next load does not hand the old key to the new provider.
    /// This holds for both debug (shared key file) and release (per-provider keychain).
    #[test]
    fn save_settings_only_clears_key_on_provider_type_change() {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        save_provider_config(dir.path(), Some(&anthropic_config("sk-ant-secret")), &store).unwrap();
        let before = unwrap_loaded(load_provider_config(dir.path(), &store));
        assert_eq!(before.api_key.as_deref(), Some("sk-ant-secret"));

        let openai_settings = LlmProviderSettings {
            provider: LlmProviderType::OpenAi,
            endpoint_url: None,
            model: None,
        };
        save_provider_settings_only(dir.path(), &openai_settings, &store).unwrap();

        let after = unwrap_loaded(load_provider_config(dir.path(), &store));
        assert_eq!(after.settings.provider, LlmProviderType::OpenAi);
        assert!(
            after.api_key.is_none(),
            "switching provider must not preserve the previous provider's key; got: {:?}",
            after.api_key
        );
    }

    /// P0.4: When migration to secure storage fails, the function must return a visible
    /// error and must NOT rewrite/remove the embedded key from the settings file.
    #[test]
    fn migration_failure_returns_error_and_preserves_embedded_key_in_settings() {
        let dir = tempfile::tempdir().unwrap();
        let old_json = r#"{"provider":"anthropic","apiKey":"sk-ant-precious","endpointUrl":null,"model":null}"#;
        fs::write(settings_path(dir.path()), old_json).unwrap();

        let failing_store = FailingWriteSecretStore;
        let result = load_provider_config(dir.path(), &failing_store);

        assert!(
            matches!(result, ProviderConfigLoadState::KeyUnreadable { .. }),
            "failed migration must return KeyUnreadable, got {result:?}"
        );

        // The embedded key must still be present — it must not have been erased.
        let settings_after = fs::read_to_string(settings_path(dir.path())).unwrap();
        assert!(
            settings_after.contains("sk-ant-precious"),
            "failed migration must not erase the embedded key; settings: {settings_after}"
        );
    }

    /// P0.5: Clearing provider config must delete all known API-provider secret accounts,
    /// even when settings JSON is corrupt or missing — not just the stored provider's account.
    #[test]
    fn clear_deletes_all_known_provider_accounts() {
        let dir = tempfile::tempdir().unwrap();
        let tracking = TrackingSecretStore::new()
            .with_key("anthropic", "sk-ant-key")
            .with_key("openAi", "sk-openai-key");

        save_provider_config(dir.path(), None, &tracking).unwrap();

        let deleted = tracking.deleted_accounts();
        assert!(
            deleted.contains(&"anthropic".to_string()),
            "clear must delete the 'anthropic' account; deleted: {deleted:?}"
        );
        assert!(
            deleted.contains(&"openAi".to_string()),
            "clear must delete the 'openAi' account; deleted: {deleted:?}"
        );
    }

    /// P0.5: If settings JSON is corrupt, the clear path must still attempt deletion for
    /// all known accounts rather than skipping because the provider type cannot be parsed.
    #[test]
    fn clear_with_corrupt_settings_still_deletes_all_known_accounts() {
        let dir = tempfile::tempdir().unwrap();
        // Write corrupt settings so provider type cannot be determined.
        fs::write(settings_path(dir.path()), "this is not valid json!").unwrap();

        let tracking = TrackingSecretStore::new().with_key("anthropic", "stale-key");

        save_provider_config(dir.path(), None, &tracking).unwrap();

        let deleted = tracking.deleted_accounts();
        assert!(
            deleted.contains(&"anthropic".to_string()),
            "clear with corrupt settings must still attempt to delete 'anthropic'; \
             deleted: {deleted:?}"
        );
        assert!(
            deleted.contains(&"openAi".to_string()),
            "clear with corrupt settings must still attempt to delete 'openAi'; \
             deleted: {deleted:?}"
        );
    }

    /// P0.3: Legacy key that exists but cannot be read must surface as
    /// `LegacyKeyUnreadable`, not silently fall through to `Missing`.
    #[cfg(unix)]
    #[test]
    fn unreadable_legacy_key_is_reported_not_missing() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let legacy = legacy_key_path(dir.path());
        fs::write(&legacy, "sk-ant-legacy").unwrap();
        fs::set_permissions(&legacy, fs::Permissions::from_mode(0o000)).unwrap();

        let result = load_provider_config(dir.path(), &FailingWriteSecretStore);

        // Restore permissions so tempdir cleanup can remove the file.
        fs::set_permissions(&legacy, fs::Permissions::from_mode(0o600)).unwrap();

        assert!(
            matches!(result, ProviderConfigLoadState::LegacyKeyUnreadable { .. }),
            "expected LegacyKeyUnreadable, got {result:?}"
        );
    }

    /// P0.2: Provider switch via save_settings_only must fail if the old key cannot be
    /// deleted, and the settings file must remain unchanged on failure.
    #[test]
    fn settings_only_provider_switch_fails_if_old_key_delete_fails() {
        let dir = tempfile::tempdir().unwrap();
        // Write initial Anthropic config using a normal store.
        save_provider_config(
            dir.path(),
            Some(&anthropic_config("sk-ant-old")),
            &file_store(dir.path()),
        )
        .unwrap();

        let openai_settings = LlmProviderSettings {
            provider: LlmProviderType::OpenAi,
            endpoint_url: None,
            model: None,
        };

        // Attempt the provider switch with a store whose delete always fails.
        let result =
            save_provider_settings_only(dir.path(), &openai_settings, &FailingDeleteSecretStore);

        assert!(
            result.is_err(),
            "provider switch must fail when old key cannot be deleted"
        );
        // The settings file must still reflect the original Anthropic provider.
        let persisted = fs::read_to_string(settings_path(dir.path())).unwrap();
        let current: LlmProviderSettings = serde_json::from_str(&persisted).unwrap();
        assert_eq!(
            current.provider,
            LlmProviderType::Anthropic,
            "settings file must not be updated when old-key deletion fails"
        );
    }

    /// P0.2: If the existing settings file is present but corrupt, save_settings_only
    /// must return an error and leave the file unchanged.
    #[test]
    fn settings_only_fails_on_invalid_existing_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        let sp = settings_path(dir.path());
        // Parent dir may not exist yet in a fresh tempdir; create it.
        fs::create_dir_all(sp.parent().unwrap()).unwrap();
        fs::write(&sp, "not-json").unwrap();

        let settings = LlmProviderSettings {
            provider: LlmProviderType::OpenAi,
            endpoint_url: None,
            model: None,
        };
        let result = save_provider_settings_only(dir.path(), &settings, &FailingDeleteSecretStore);

        assert!(
            result.is_err(),
            "save_settings_only must fail when existing settings file is invalid JSON"
        );
        // The corrupt file must not have been overwritten.
        assert_eq!(
            fs::read_to_string(&sp).unwrap(),
            "not-json",
            "corrupt settings file must be preserved on error"
        );
    }

    /// P0.4: save_provider_config(Some(...)) must NOT write settings if the secret write fails.
    #[test]
    fn save_provider_config_does_not_write_settings_if_secret_write_fails() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = anthropic_config("sk-ant-new");
        let result = save_provider_config(dir.path(), Some(&cfg), &FailingWriteSecretStore);
        assert!(result.is_err(), "save must fail when secret write fails");
        assert!(
            !settings_path(dir.path()).exists(),
            "settings file must not be created when secret write fails"
        );
    }

    /// P0.4: If a settings file already exists (prior save), save_provider_config(Some(...))
    /// must not overwrite it when the new secret write fails.
    #[test]
    fn save_provider_config_preserves_existing_settings_if_secret_write_fails() {
        let dir = tempfile::tempdir().unwrap();
        // First, write a valid Anthropic config using the real file store.
        save_provider_config(
            dir.path(),
            Some(&anthropic_config("sk-ant-old")),
            &file_store(dir.path()),
        )
        .unwrap();

        // Attempt to overwrite with an OpenAI config using a store that always fails writes.
        let result = save_provider_config(
            dir.path(),
            Some(&openai_config("sk-openai-new")),
            &FailingWriteSecretStore,
        );

        assert!(result.is_err(), "save must fail when secret write fails");
        // The settings file must still reflect the original Anthropic provider.
        let persisted = fs::read_to_string(settings_path(dir.path())).unwrap();
        let current: LlmProviderSettings = serde_json::from_str(&persisted).unwrap();
        assert_eq!(
            current.provider,
            LlmProviderType::Anthropic,
            "settings file must not be updated when secret write fails; got: {:?}",
            current.provider
        );
    }

    /// P1.5: A failing delete during clear must surface as an error, not be silently logged.
    #[test]
    fn clear_returns_error_when_delete_fails() {
        let dir = tempfile::tempdir().unwrap();
        let failing_store = FailingDeleteSecretStore;
        let result = save_provider_config(dir.path(), None, &failing_store);
        assert!(
            result.is_err(),
            "clear must return an error when secret deletion fails; got Ok(())"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("delete") || msg.contains("clear"),
            "error message should mention deletion/clear; got: {msg}"
        );
    }
}
