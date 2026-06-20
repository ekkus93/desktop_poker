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
}

/// Full configuration for one LLM provider.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProviderConfig {
    pub provider: LlmProviderType,
    /// API key — required for Anthropic and OpenAI, ignored for Ollama/llama-server.
    pub api_key: Option<String>,
    /// Override the base URL (default is provider-specific).
    pub endpoint_url: Option<String>,
    /// Override the model name (default is provider-specific).
    pub model: Option<String>,
}

impl std::fmt::Debug for LlmProviderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmProviderConfig")
            .field("provider", &self.provider)
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
            .field("endpoint_url", &self.endpoint_url)
            .field("model", &self.model)
            .finish()
    }
}

impl LlmProviderConfig {
    /// True when the config has enough information to make a request.
    pub fn is_usable(&self) -> bool {
        match self.provider {
            LlmProviderType::Anthropic | LlmProviderType::OpenAi => {
                self.api_key.as_deref().is_some_and(|k| !k.is_empty())
            }
            LlmProviderType::Ollama | LlmProviderType::LlamaServer => true,
        }
    }

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
        }
    }

    /// Human-readable label for the provider (for UI / logs).
    pub fn provider_label(&self) -> &'static str {
        match self.provider {
            LlmProviderType::Anthropic => "Anthropic (Claude)",
            LlmProviderType::OpenAi => "OpenAI",
            LlmProviderType::Ollama => "Ollama",
            LlmProviderType::LlamaServer => "llama-server",
        }
    }

    /// Construct a minimal Anthropic config from a raw API key (migration path).
    pub fn from_anthropic_key(key: String) -> Self {
        Self {
            provider: LlmProviderType::Anthropic,
            api_key: Some(key),
            endpoint_url: None,
            model: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anthropic(key: &str) -> LlmProviderConfig {
        LlmProviderConfig {
            provider: LlmProviderType::Anthropic,
            api_key: Some(key.to_string()),
            endpoint_url: None,
            model: None,
        }
    }

    fn ollama() -> LlmProviderConfig {
        LlmProviderConfig {
            provider: LlmProviderType::Ollama,
            api_key: None,
            endpoint_url: None,
            model: None,
        }
    }

    #[test]
    fn anthropic_without_key_is_not_usable() {
        let cfg = LlmProviderConfig {
            provider: LlmProviderType::Anthropic,
            api_key: None,
            endpoint_url: None,
            model: None,
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
    fn llama_server_is_usable_without_key() {
        let cfg = LlmProviderConfig {
            provider: LlmProviderType::LlamaServer,
            api_key: None,
            endpoint_url: None,
            model: None,
        };
        assert!(cfg.is_usable());
    }

    #[test]
    fn resolved_model_uses_override_when_set() {
        let mut cfg = anthropic("k");
        cfg.model = Some("claude-opus-4-8".to_string());
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
        cfg.endpoint_url = Some("http://my-server:11434".to_string());
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
}
