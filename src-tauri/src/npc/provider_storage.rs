use std::{
    fs,
    path::{Path, PathBuf},
};

use super::provider::{LlmProviderConfig, LlmProviderSettings};

/// Non-secret provider settings (provider type, endpoint, model).
const SETTINGS_FILE_NAME: &str = "llm-provider.json";
/// Secret API key stored separately with restricted file permissions.
const KEY_FILE_NAME: &str = "llm-provider-key.dat";
/// Legacy plain-text key written by Phase 2; read once for migration.
const LEGACY_KEY_FILE_NAME: &str = "claude-api-key.txt";

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
}

fn settings_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(SETTINGS_FILE_NAME)
}

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
/// - `llm-provider-key.dat` — raw API key string, 0600 permissions
///
/// ## Migration paths
/// 1. **Combined old format** — `llm-provider.json` contains both settings and
///    `apiKey` field (written by an older version).  On first load the key is
///    moved to `llm-provider-key.dat` and the settings file is rewritten
///    without it.
/// 2. **Legacy plain-text key** — `claude-api-key.txt` from Phase 2 is promoted
///    to an Anthropic config if neither current file exists.
pub fn load_provider_config(app_data_dir: &Path) -> ProviderConfigLoadState {
    let sp = settings_path(app_data_dir);
    let kp = key_path(app_data_dir);

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

        // Determine the API key: prefer the dedicated key file; fall back to the
        // legacy embedded `apiKey` field for one-time migration.
        let api_key = match read_key_file(&kp) {
            Err(e) => {
                // Key file exists but cannot be read — surface as an explicit error.
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
                    // Migrate immediately: write key file and rewrite settings-only JSON.
                    if let Err(e) = write_key_file(&kp, key) {
                        eprintln!("[provider-storage] migration: failed to write key file: {e}");
                    } else if let Ok(json) = serde_json::to_string_pretty(&settings) {
                        if let Err(e) = fs::write(&sp, json) {
                            eprintln!(
                                "[provider-storage] migration: failed to rewrite settings file: {e}"
                            );
                        } else {
                            eprintln!(
                                "[provider-storage] migrated API key from llm-provider.json \
                                 to llm-provider-key.dat"
                            );
                        }
                    }
                }
                embedded
            }
        };

        return ProviderConfigLoadState::Loaded(LlmProviderConfig { settings, api_key });
    }

    // Fall back to the legacy plain-text API key.
    let legacy = legacy_key_path(app_data_dir);
    if legacy.exists() {
        if let Ok(raw) = fs::read_to_string(&legacy) {
            let key = raw.trim().to_string();
            if !key.is_empty() {
                return ProviderConfigLoadState::Loaded(LlmProviderConfig::from_anthropic_key(key));
            }
        }
    }

    ProviderConfigLoadState::Missing
}

/// Write the provider config to separate settings and key files.
///
/// If `config` is `None`, both files are deleted (clears all provider config).
pub fn save_provider_config(
    app_data_dir: &Path,
    config: Option<&LlmProviderConfig>,
) -> Result<(), std::io::Error> {
    let sp = settings_path(app_data_dir);
    let kp = key_path(app_data_dir);

    match config {
        None => {
            if sp.exists() {
                fs::remove_file(&sp)?;
            }
            if kp.exists() {
                fs::remove_file(&kp)?;
            }
        }
        Some(cfg) => {
            if let Some(parent) = sp.parent() {
                fs::create_dir_all(parent)?;
            }

            // Write non-secret settings (safe to log, no key inside).
            let settings_json = serde_json::to_string_pretty(&cfg.settings)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            fs::write(&sp, settings_json)?;

            // Write API key to its own restricted file.
            let key = cfg.api_key.as_deref().unwrap_or("").trim().to_string();
            if key.is_empty() {
                if kp.exists() {
                    fs::remove_file(&kp)?;
                }
            } else {
                #[cfg(not(debug_assertions))]
                eprintln!(
                    "[security] LLM API key stored in plaintext at {}. \
                     Move to OS keychain storage before distributing a release build.",
                    kp.display()
                );
                write_key_file(&kp, &key)?;
            }
        }
    }
    Ok(())
}

/// Save only the non-secret provider settings, preserving any existing key file.
///
/// Use this when the user edits endpoint/model/provider fields without entering
/// a new API key — the existing key must not be erased.
pub fn save_provider_settings_only(
    app_data_dir: &Path,
    settings: &LlmProviderSettings,
) -> Result<(), std::io::Error> {
    let sp = settings_path(app_data_dir);
    if let Some(parent) = sp.parent() {
        fs::create_dir_all(parent)?;
    }
    let settings_json = serde_json::to_string_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(&sp, settings_json)?;
    // Key file is intentionally left untouched.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::provider::LlmProviderType;
    use super::*;

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

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = anthropic_config("sk-ant-test");
        save_provider_config(dir.path(), Some(&cfg)).unwrap();
        let loaded = unwrap_loaded(load_provider_config(dir.path()));
        assert_eq!(loaded.settings.provider, LlmProviderType::Anthropic);
        assert_eq!(loaded.api_key.as_deref(), Some("sk-ant-test"));
    }

    #[test]
    fn save_none_removes_both_files() {
        let dir = tempfile::tempdir().unwrap();
        save_provider_config(dir.path(), Some(&anthropic_config("k"))).unwrap();
        save_provider_config(dir.path(), None).unwrap();
        assert!(matches!(
            load_provider_config(dir.path()),
            ProviderConfigLoadState::Missing
        ));
        assert!(!settings_path(dir.path()).exists());
        assert!(!key_path(dir.path()).exists());
    }

    #[test]
    fn load_nonexistent_returns_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            load_provider_config(dir.path()),
            ProviderConfigLoadState::Missing
        ));
    }

    #[test]
    fn corrupt_json_returns_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(settings_path(dir.path()), "not json at all").unwrap();
        assert!(matches!(
            load_provider_config(dir.path()),
            ProviderConfigLoadState::InvalidJson { .. }
        ));
    }

    #[test]
    fn invalid_schema_returns_invalid_schema() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(settings_path(dir.path()), r#"{"unexpected":"field"}"#).unwrap();
        assert!(matches!(
            load_provider_config(dir.path()),
            ProviderConfigLoadState::InvalidSchema { .. }
        ));
    }

    #[test]
    fn legacy_key_file_is_promoted_to_anthropic_config() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(LEGACY_KEY_FILE_NAME), "sk-ant-legacy").unwrap();
        let loaded = unwrap_loaded(load_provider_config(dir.path()));
        assert_eq!(loaded.settings.provider, LlmProviderType::Anthropic);
        assert_eq!(loaded.api_key.as_deref(), Some("sk-ant-legacy"));
    }

    #[test]
    fn new_settings_file_takes_precedence_over_legacy_key() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(LEGACY_KEY_FILE_NAME), "sk-ant-legacy").unwrap();
        let new_cfg = LlmProviderConfig {
            settings: LlmProviderSettings {
                provider: LlmProviderType::Ollama,
                endpoint_url: Some("http://localhost:11434".to_string()),
                model: None,
            },
            api_key: None,
        };
        save_provider_config(dir.path(), Some(&new_cfg)).unwrap();
        let loaded = unwrap_loaded(load_provider_config(dir.path()));
        assert_eq!(loaded.settings.provider, LlmProviderType::Ollama);
    }

    #[test]
    fn settings_file_does_not_contain_api_key() {
        let dir = tempfile::tempdir().unwrap();
        save_provider_config(dir.path(), Some(&anthropic_config("sk-ant-secret"))).unwrap();
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
        save_provider_config(dir.path(), Some(&anthropic_config("sk-ant-secret"))).unwrap();
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
        // Write old combined format (settings + apiKey in one file).
        let old_json = r#"{"provider":"anthropic","apiKey":"sk-ant-migrated","endpointUrl":null,"model":null}"#;
        fs::write(settings_path(dir.path()), old_json).unwrap();

        // Load should trigger migration.
        let loaded = unwrap_loaded(load_provider_config(dir.path()));
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
        save_provider_config(dir.path(), Some(&anthropic_config("sk-secret"))).unwrap();
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
        // Write valid settings and an unreadable key file (mode 000).
        save_provider_config(dir.path(), Some(&anthropic_config("sk-ant-secret"))).unwrap();
        let kp = key_path(dir.path());
        fs::set_permissions(&kp, fs::Permissions::from_mode(0o000)).unwrap();
        let result = load_provider_config(dir.path());
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
        // Write settings only, no key file.
        let settings = LlmProviderSettings {
            provider: LlmProviderType::Anthropic,
            endpoint_url: None,
            model: None,
        };
        save_provider_settings_only(dir.path(), &settings).unwrap();
        let loaded = unwrap_loaded(load_provider_config(dir.path()));
        assert_eq!(loaded.settings.provider, LlmProviderType::Anthropic);
        assert!(
            loaded.api_key.is_none(),
            "missing key file should result in no key, not an error"
        );
    }

    #[test]
    fn save_settings_only_preserves_existing_key() {
        let dir = tempfile::tempdir().unwrap();
        save_provider_config(dir.path(), Some(&anthropic_config("sk-ant-original"))).unwrap();

        // Update only the settings.
        let new_settings = LlmProviderSettings {
            provider: LlmProviderType::Anthropic,
            endpoint_url: Some("https://custom.example.com".to_string()),
            model: Some("claude-haiku-4-5-20251001".to_string()),
        };
        save_provider_settings_only(dir.path(), &new_settings).unwrap();

        let loaded = unwrap_loaded(load_provider_config(dir.path()));
        assert_eq!(
            loaded.settings.endpoint_url.as_deref(),
            Some("https://custom.example.com"),
            "endpoint_url should be updated"
        );
        assert_eq!(
            loaded.api_key.as_deref(),
            Some("sk-ant-original"),
            "existing API key must be preserved"
        );
    }
}
