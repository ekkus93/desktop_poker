use serde::{Deserialize, Serialize};

const ANTHROPIC_DEFAULT_MODEL: &str = "claude-haiku-4-5-20251001";
const OPENAI_DEFAULT_MODEL: &str = "gpt-4o-mini";
const OLLAMA_DEFAULT_MODEL: &str = "llama3.2:3b";
const LLAMA_SERVER_DEFAULT_MODEL: &str = "default";

const ANTHROPIC_DEFAULT_ENDPOINT: &str = "https://api.anthropic.com";
const OPENAI_DEFAULT_ENDPOINT: &str = "https://api.openai.com";
const OLLAMA_DEFAULT_ENDPOINT: &str = "http://localhost:11434";
const LLAMA_SERVER_DEFAULT_ENDPOINT: &str = "http://localhost:8080";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LlmProviderType {
    Anthropic,
    OpenAi,
    Ollama,
    LlamaServer,
    EmbeddedLocal,
}

impl LlmProviderType {
    /// Stable lowercase ASCII identifier for this provider, used as the
    /// keychain account name in release builds.
    pub fn as_str(&self) -> &'static str {
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
}

/// Non-secret provider settings: safe to log, display in debug state, and
/// include in bug reports. Does NOT contain the API key.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProviderSettings {
    pub provider: LlmProviderType,
    /// Override the base URL (default is provider-specific).
    pub endpoint_url: Option<String>,
    /// Override the model name (default is provider-specific).
    pub model: Option<String>,
}

impl LlmProviderSettings {
    /// Resolved model name (falls back to provider default).
    pub fn resolved_model(&self) -> &str {
        if let Some(m) = self.model.as_deref().filter(|s| !s.is_empty()) {
            return m;
        }
        match self.provider {
            LlmProviderType::Anthropic => ANTHROPIC_DEFAULT_MODEL,
            LlmProviderType::OpenAi => OPENAI_DEFAULT_MODEL,
            LlmProviderType::Ollama => OLLAMA_DEFAULT_MODEL,
            LlmProviderType::LlamaServer => LLAMA_SERVER_DEFAULT_MODEL,
            LlmProviderType::EmbeddedLocal => "",
        }
    }

    /// Resolved base URL (falls back to provider default).
    pub fn resolved_endpoint(&self) -> &str {
        if let Some(url) = self.endpoint_url.as_deref().filter(|s| !s.is_empty()) {
            return url;
        }
        match self.provider {
            LlmProviderType::Anthropic => ANTHROPIC_DEFAULT_ENDPOINT,
            LlmProviderType::OpenAi => OPENAI_DEFAULT_ENDPOINT,
            LlmProviderType::Ollama => OLLAMA_DEFAULT_ENDPOINT,
            LlmProviderType::LlamaServer => LLAMA_SERVER_DEFAULT_ENDPOINT,
            LlmProviderType::EmbeddedLocal => "embedded://local",
        }
    }

    /// Human-readable label for the provider (for UI / logs).
    pub fn provider_label(&self) -> &'static str {
        match self.provider {
            LlmProviderType::Anthropic => "Anthropic (Claude)",
            LlmProviderType::OpenAi => "OpenAI",
            LlmProviderType::Ollama => "Ollama",
            LlmProviderType::LlamaServer => "llama-server",
            LlmProviderType::EmbeddedLocal => "Embedded local GGUF",
        }
    }
}

/// Combined runtime config: non-secret settings + secret API key.
///
/// Serializes flat (`#[serde(flatten)]` on `settings`) so the wire/command
/// format is unchanged: `{provider, apiKey, endpointUrl, model}`.
/// On disk the secret is stored separately — see `provider_storage`.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProviderConfig {
    #[serde(flatten)]
    pub settings: LlmProviderSettings,
    /// API key — required for Anthropic and OpenAI; ignored by local providers.
    pub api_key: Option<String>,
}

impl std::fmt::Debug for LlmProviderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmProviderConfig")
            .field("settings", &self.settings)
            .field(
                "api_key",
                &self.api_key.as_deref().map(|k| {
                    if k.len() > 8 {
                        format!("{}…[redacted]", &k[..4])
                    } else if k.is_empty() {
                        "<empty>".to_string()
                    } else {
                        "<set>".to_string()
                    }
                }),
            )
            .finish()
    }
}

impl LlmProviderConfig {
    /// True when the config has enough information to make a request.
    pub fn is_usable(&self) -> bool {
        match self.settings.provider {
            LlmProviderType::Anthropic | LlmProviderType::OpenAi => {
                self.api_key.as_deref().is_some_and(|k| !k.is_empty())
            }
            LlmProviderType::Ollama | LlmProviderType::LlamaServer => true,
            LlmProviderType::EmbeddedLocal => self
                .settings
                .model
                .as_deref()
                .is_some_and(|path| !path.trim().is_empty()),
        }
    }

    // Delegate helpers so callers don't need to go through `.settings.`.

    pub fn resolved_model(&self) -> &str {
        self.settings.resolved_model()
    }

    pub fn resolved_endpoint(&self) -> &str {
        self.settings.resolved_endpoint()
    }

    pub fn provider_label(&self) -> &'static str {
        self.settings.provider_label()
    }

    /// Construct a minimal Anthropic config from a raw API key (migration path).
    pub fn from_anthropic_key(key: String) -> Self {
        Self {
            settings: LlmProviderSettings {
                provider: LlmProviderType::Anthropic,
                endpoint_url: None,
                model: None,
            },
            api_key: Some(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anthropic(key: &str) -> LlmProviderConfig {
        LlmProviderConfig {
            settings: LlmProviderSettings {
                provider: LlmProviderType::Anthropic,
                endpoint_url: None,
                model: None,
            },
            api_key: Some(key.to_string()),
        }
    }

    fn ollama() -> LlmProviderConfig {
        LlmProviderConfig {
            settings: LlmProviderSettings {
                provider: LlmProviderType::Ollama,
                endpoint_url: None,
                model: None,
            },
            api_key: None,
        }
    }

    #[test]
    fn only_remote_api_providers_use_secret_storage() {
        assert!(LlmProviderType::Anthropic.uses_api_key());
        assert!(LlmProviderType::OpenAi.uses_api_key());
        assert!(!LlmProviderType::Ollama.uses_api_key());
        assert!(!LlmProviderType::LlamaServer.uses_api_key());
        assert!(!LlmProviderType::EmbeddedLocal.uses_api_key());
    }

    #[test]
    fn anthropic_without_key_is_not_usable() {
        let cfg = LlmProviderConfig {
            settings: LlmProviderSettings {
                provider: LlmProviderType::Anthropic,
                endpoint_url: None,
                model: None,
            },
            api_key: None,
        };
        assert!(!cfg.is_usable());
    }

    #[test]
    fn anthropic_with_key_is_usable() {
        assert!(anthropic("sk-ant-test").is_usable());
    }

    #[test]
    fn ollama_is_usable_without_key() {
        assert!(ollama().is_usable());
    }

    #[test]
    fn embedded_local_is_usable_with_model_path() {
        let cfg = LlmProviderConfig {
            settings: LlmProviderSettings {
                provider: LlmProviderType::EmbeddedLocal,
                endpoint_url: None,
                model: Some("/tmp/tiny-model.gguf".to_string()),
            },
            api_key: None,
        };
        assert!(cfg.is_usable());
    }

    #[test]
    fn llama_server_is_usable_without_key() {
        let cfg = LlmProviderConfig {
            settings: LlmProviderSettings {
                provider: LlmProviderType::LlamaServer,
                endpoint_url: None,
                model: None,
            },
            api_key: None,
        };
        assert!(cfg.is_usable());
    }

    #[test]
    fn resolved_model_uses_override_when_set() {
        let mut cfg = anthropic("k");
        cfg.settings.model = Some("claude-opus-4-8".to_string());
        assert_eq!(cfg.resolved_model(), "claude-opus-4-8");
    }

    #[test]
    fn resolved_model_falls_back_to_provider_default() {
        assert_eq!(anthropic("k").resolved_model(), ANTHROPIC_DEFAULT_MODEL);
        assert_eq!(ollama().resolved_model(), OLLAMA_DEFAULT_MODEL);
    }

    #[test]
    fn resolved_endpoint_uses_override_when_set() {
        let mut cfg = ollama();
        cfg.settings.endpoint_url = Some("http://my-server:11434".to_string());
        assert_eq!(cfg.resolved_endpoint(), "http://my-server:11434");
    }

    #[test]
    fn resolved_endpoint_falls_back_to_provider_default() {
        assert_eq!(
            anthropic("k").resolved_endpoint(),
            ANTHROPIC_DEFAULT_ENDPOINT
        );
        assert_eq!(ollama().resolved_endpoint(), OLLAMA_DEFAULT_ENDPOINT);
    }

    #[test]
    fn settings_debug_does_not_contain_api_key() {
        let cfg = anthropic("sk-ant-super-secret");
        let debug = format!("{:?}", cfg.settings);
        assert!(
            !debug.contains("super-secret"),
            "LlmProviderSettings Debug must not expose the API key; got: {debug}"
        );
    }

    #[test]
    fn flat_serde_roundtrip_preserves_every_provider_variant() {
        let cases = [
            (
                LlmProviderType::Anthropic,
                Some("sk-ant-abc"),
                Some("https://my.anthropic.proxy"),
                Some("claude-opus-4-8"),
            ),
            (
                LlmProviderType::OpenAi,
                Some("sk-openai-abc"),
                Some("https://my.openai.proxy"),
                Some("gpt-4o-mini"),
            ),
            (
                LlmProviderType::Ollama,
                None,
                Some("http://localhost:11434"),
                Some("llama3.2:3b"),
            ),
            (
                LlmProviderType::LlamaServer,
                None,
                Some("http://localhost:8080"),
                Some("default"),
            ),
            (
                LlmProviderType::EmbeddedLocal,
                None,
                None,
                Some("/tmp/tiny-model.gguf"),
            ),
        ];

        for (provider, api_key, endpoint_url, model) in cases {
            let expected_provider = provider.clone();
            let cfg = LlmProviderConfig {
                settings: LlmProviderSettings {
                    provider,
                    endpoint_url: endpoint_url.map(str::to_string),
                    model: model.map(str::to_string),
                },
                api_key: api_key.map(str::to_string),
            };

            let json = serde_json::to_string(&cfg).expect("provider config should serialize");
            assert!(json.contains("\"provider\""));
            assert!(json.contains("\"apiKey\""));
            assert!(json.contains("\"endpointUrl\""));
            assert!(json.contains("\"model\""));

            let back: LlmProviderConfig =
                serde_json::from_str(&json).expect("provider config should deserialize");
            assert_eq!(back.settings.provider, expected_provider);
            assert_eq!(back.api_key.as_deref(), api_key);
            assert_eq!(back.settings.endpoint_url.as_deref(), endpoint_url);
            assert_eq!(back.settings.model.as_deref(), model);
        }
    }
}
