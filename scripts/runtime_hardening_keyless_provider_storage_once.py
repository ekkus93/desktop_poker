from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old in text:
        return text.replace(old, new, 1)
    if new in text:
        return text
    raise SystemExit(f"expected {label} anchor was not found")


provider_path = Path("src-tauri/src/npc/provider.rs")
provider = provider_path.read_text(encoding="utf-8")
provider = replace_once(
    provider,
    '''    pub fn as_str(&self) -> &'static str {
        match self {
            LlmProviderType::Anthropic => "anthropic",
            LlmProviderType::OpenAi => "openAi",
            LlmProviderType::Ollama => "ollama",
            LlmProviderType::LlamaServer => "llamaServer",
            LlmProviderType::EmbeddedLocal => "embeddedLocal",
        }
    }
}''',
    '''    pub fn as_str(&self) -> &'static str {
        match self {
            LlmProviderType::Anthropic => "anthropic",
            LlmProviderType::OpenAi => "openAi",
            LlmProviderType::Ollama => "ollama",
            LlmProviderType::LlamaServer => "llamaServer",
            LlmProviderType::EmbeddedLocal => "embeddedLocal",
        }
    }

    /// Whether this provider stores an API credential in the secret store.
    /// Local providers must remain usable when no platform keychain service exists.
    pub const fn uses_api_key(&self) -> bool {
        matches!(self, LlmProviderType::Anthropic | LlmProviderType::OpenAi)
    }
}''',
    "LlmProviderType secret-storage policy",
)
provider = replace_once(
    provider,
    '''    #[test]
    fn anthropic_without_key_is_not_usable() {''',
    '''    #[test]
    fn only_remote_api_providers_use_secret_storage() {
        assert!(LlmProviderType::Anthropic.uses_api_key());
        assert!(LlmProviderType::OpenAi.uses_api_key());
        assert!(!LlmProviderType::Ollama.uses_api_key());
        assert!(!LlmProviderType::LlamaServer.uses_api_key());
        assert!(!LlmProviderType::EmbeddedLocal.uses_api_key());
    }

    #[test]
    fn anthropic_without_key_is_not_usable() {''',
    "provider secret-storage policy test",
)
provider_path.write_text(provider, encoding="utf-8")

storage_path = Path("src-tauri/src/npc/provider_storage.rs")
storage = storage_path.read_text(encoding="utf-8")

old_load = '''        // Determine the API key via the injected store.
        // Fall back to the legacy embedded `apiKey` field for one-time migration.
        let provider_str = settings.provider.as_str();
        let api_key = match store.read_key(provider_str) {
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
                                        "[provider-storage] migration: failed to rewrite settings \\
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
        };'''
new_load = '''        // Keyless providers must not touch platform secret storage. This keeps
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
                                            "[provider-storage] migration: failed to rewrite settings \\
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
        };'''
storage = replace_once(storage, old_load, new_load, "keyless provider load policy")

old_save_some = '''        Some(cfg) => {
            if let Some(parent) = sp.parent() {
                fs::create_dir_all(parent)?;
            }

            let key = cfg.api_key.as_deref().unwrap_or("").trim().to_string();
            let provider_str = cfg.settings.provider.as_str();

            // Write/delete the secret FIRST. If the secret operation fails,
            // the settings file must not be written — leaving a new settings
            // file without a matching secret would be an inconsistent state.
            if key.is_empty() {
                store.delete_key(provider_str).map_err(|error| {
                    std::io::Error::other(format!(
                        "could not delete provider key for {provider_str}: {error}"
                    ))
                })?;
            } else {
                store.write_key(provider_str, &key).map_err(|error| {
                    std::io::Error::other(format!(
                        "could not write provider key for {provider_str}: {error}"
                    ))
                })?;
            }

            // Secret succeeded — safe to write non-secret settings now.
            let settings_json = serde_json::to_string_pretty(&cfg.settings)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            fs::write(&sp, settings_json)?;
        }'''
new_save_some = '''        Some(cfg) => {
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
                                "could not clear stale provider key for {old}: {error}; \
                                 new-key rollback: {}",
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
        }'''
storage = replace_once(storage, old_save_some, new_save_some, "keyless full-config save policy")

old_settings_switch = '''    // When the provider type changes, delete the old key first.  If deletion
    // fails the new settings must NOT be written — leaving the old key behind
    // while the UI believes the provider changed is a silent-failure violation.
    if let Some(current) = current.as_ref() {
        if current.provider != settings.provider {
            store.delete_key(current.provider.as_str()).map_err(|e| {
                std::io::Error::other(format!(
                    "could not clear stale key after provider change \\
                         ({} → {}): {e}",
                    current.provider.as_str(),
                    settings.provider.as_str()
                ))
            })?;
        }
    }'''
new_settings_switch = '''    // When the provider type changes, clear every API-provider account that
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
    }'''
storage = replace_once(
    storage,
    old_settings_switch,
    new_settings_switch,
    "keyless settings-only save policy",
)

storage = replace_once(
    storage,
    '''    /// Simulates a store where every write operation fails. Used for P0.4 tests.
    struct FailingWriteSecretStore;''',
    '''    /// Fails every operation so tests can prove keyless providers never touch secrets.
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
    struct FailingWriteSecretStore;''',
    "rejecting secret store test double",
)

storage = replace_once(
    storage,
    '''    #[test]
    fn save_and_load_roundtrip() {''',
    '''    fn embedded_config() -> LlmProviderConfig {
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
    fn save_and_load_roundtrip() {''',
    "keyless provider storage regression tests",
)

storage_path.write_text(storage, encoding="utf-8")
