use std::{
    fs,
    path::{Path, PathBuf},
};

use super::provider::LlmProviderConfig;

const PROVIDER_FILE_NAME: &str = "llm-provider.json";
// Legacy file written by Phase 2; read once for migration.
const LEGACY_KEY_FILE_NAME: &str = "claude-api-key.txt";

fn provider_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(PROVIDER_FILE_NAME)
}

fn legacy_key_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(LEGACY_KEY_FILE_NAME)
}

/// Load the provider config from disk.
///
/// Migration: if `llm-provider.json` does not exist but the legacy
/// `claude-api-key.txt` does, the key is promoted to an Anthropic config and
/// the legacy file is left in place.
pub fn load_provider_config(app_data_dir: &Path) -> Option<LlmProviderConfig> {
    let path = provider_path(app_data_dir);

    // Try reading the new JSON file first.
    if path.exists() {
        let text = fs::read_to_string(&path).ok()?;
        let cfg: LlmProviderConfig = serde_json::from_str(&text).ok()?;
        return Some(cfg);
    }

    // Fall back to the legacy plain-text API key.
    let legacy = legacy_key_path(app_data_dir);
    if legacy.exists() {
        let key = fs::read_to_string(&legacy)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())?;
        return Some(LlmProviderConfig::from_anthropic_key(key));
    }

    None
}

/// Write the provider config to `{app_data_dir}/llm-provider.json`.
///
/// If `config` is `None`, the file is deleted (clears all provider config).
pub fn save_provider_config(
    app_data_dir: &Path,
    config: Option<&LlmProviderConfig>,
) -> Result<(), std::io::Error> {
    let path = provider_path(app_data_dir);

    match config {
        None => {
            if path.exists() {
                fs::remove_file(&path)?;
            }
        }
        Some(cfg) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(cfg)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            fs::write(&path, json)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::provider::LlmProviderType;
    use super::*;

    fn anthropic_config(key: &str) -> LlmProviderConfig {
        LlmProviderConfig {
            provider: LlmProviderType::Anthropic,
            api_key: Some(key.to_string()),
            endpoint_url: None,
            model: None,
        }
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = anthropic_config("sk-ant-test");
        save_provider_config(dir.path(), Some(&cfg)).unwrap();
        let loaded = load_provider_config(dir.path()).unwrap();
        assert_eq!(loaded.provider, LlmProviderType::Anthropic);
        assert_eq!(loaded.api_key.as_deref(), Some("sk-ant-test"));
    }

    #[test]
    fn save_none_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        save_provider_config(dir.path(), Some(&anthropic_config("k"))).unwrap();
        save_provider_config(dir.path(), None).unwrap();
        assert!(load_provider_config(dir.path()).is_none());
        assert!(!provider_path(dir.path()).exists());
    }

    #[test]
    fn load_nonexistent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_provider_config(dir.path()).is_none());
    }

    #[test]
    fn legacy_key_file_is_promoted_to_anthropic_config() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(LEGACY_KEY_FILE_NAME), "sk-ant-legacy").unwrap();
        let loaded = load_provider_config(dir.path()).unwrap();
        assert_eq!(loaded.provider, LlmProviderType::Anthropic);
        assert_eq!(loaded.api_key.as_deref(), Some("sk-ant-legacy"));
    }

    #[test]
    fn new_json_takes_precedence_over_legacy_key() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(LEGACY_KEY_FILE_NAME), "sk-ant-legacy").unwrap();
        let new_cfg = LlmProviderConfig {
            provider: LlmProviderType::Ollama,
            api_key: None,
            endpoint_url: Some("http://localhost:11434".to_string()),
            model: None,
        };
        save_provider_config(dir.path(), Some(&new_cfg)).unwrap();
        let loaded = load_provider_config(dir.path()).unwrap();
        assert_eq!(loaded.provider, LlmProviderType::Ollama);
    }
}
