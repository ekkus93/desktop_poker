use serde::{Deserialize, Serialize};

use super::provider::{LlmProviderConfig, LlmProviderType};

const DEFAULT_TIMEOUT_SECS: u64 = 5;
const DEFAULT_MAX_TOKENS: u32 = 128;
const ANTHROPIC_VERSION: &str = "2023-06-01";

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

// ── Anthropic wire types ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<AnthropicMessage>,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
}

// ── OpenAI-compatible wire types (OpenAI / Ollama / llama-server) ─────────────

#[derive(Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<OpenAiMessage>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiChoiceMessage,
}

#[derive(Deserialize)]
struct OpenAiChoiceMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

// ── Client ────────────────────────────────────────────────────────────────────

/// Multi-provider LLM client.
pub struct LlmClient {
    client: reqwest::blocking::Client,
    config: LlmProviderConfig,
    /// Overrides the resolved endpoint (used in tests with a mock server).
    endpoint_override: Option<String>,
}

impl LlmClient {
    pub fn new(config: LlmProviderConfig) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();
        Self {
            client,
            config,
            endpoint_override: None,
        }
    }

    /// Override the API base URL (used in tests with a mock server).
    #[cfg(test)]
    pub fn with_endpoint_override(mut self, url: String) -> Self {
        self.endpoint_override = Some(url);
        self
    }

    /// Construct a client with a custom timeout (for tests that verify timeout behaviour).
    #[cfg(test)]
    pub fn with_timeout_secs(
        config: LlmProviderConfig,
        timeout_secs: u64,
        endpoint: String,
    ) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_default();
        Self {
            client,
            config,
            endpoint_override: Some(endpoint),
        }
    }

    fn base_url(&self) -> &str {
        self.endpoint_override
            .as_deref()
            .unwrap_or_else(|| self.config.resolved_endpoint())
    }

    /// Send a request and return the assistant's text response.
    pub fn complete(&self, system: &str, user: &str) -> Result<String, LlmError> {
        match self.config.provider {
            LlmProviderType::Anthropic => self.complete_anthropic(system, user),
            LlmProviderType::OpenAi | LlmProviderType::Ollama | LlmProviderType::LlamaServer => {
                self.complete_openai_compatible(system, user)
            }
        }
    }

    fn complete_anthropic(&self, system: &str, user: &str) -> Result<String, LlmError> {
        let api_key = self.config.api_key.as_deref().unwrap_or_default();

        let url = format!("{}/v1/messages", self.base_url());

        let body = AnthropicRequest {
            model: self.config.resolved_model().to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            system: system.to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: user.to_string(),
            }],
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(map_reqwest_err)?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().unwrap_or_default();
            return Err(LlmError::Api(status.as_u16(), body_text));
        }

        let resp: AnthropicResponse = response
            .json()
            .map_err(|e| LlmError::Parse(e.to_string()))?;

        resp.content
            .into_iter()
            .find(|b| b.block_type == "text")
            .and_then(|b| b.text)
            .ok_or_else(|| LlmError::Parse("no text content block in response".to_string()))
    }

    fn complete_openai_compatible(&self, system: &str, user: &str) -> Result<String, LlmError> {
        let url = format!("{}/v1/chat/completions", self.base_url());

        let body = OpenAiRequest {
            model: self.config.resolved_model().to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            messages: vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: user.to_string(),
                },
            ],
        };

        let mut req = self
            .client
            .post(&url)
            .header("content-type", "application/json");

        // Add Bearer token only when an API key is configured (OpenAI requires it;
        // Ollama and llama-server accept it but don't require it).
        if let Some(key) = self.config.api_key.as_deref().filter(|k| !k.is_empty()) {
            req = req.header("authorization", format!("Bearer {key}"));
        }

        let response = req.json(&body).send().map_err(map_reqwest_err)?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().unwrap_or_default();
            return Err(LlmError::Api(status.as_u16(), body_text));
        }

        let resp: OpenAiResponse = response
            .json()
            .map_err(|e| LlmError::Parse(e.to_string()))?;

        resp.choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| LlmError::Parse("no content in first choice".to_string()))
    }
}

fn map_reqwest_err(e: reqwest::Error) -> LlmError {
    if e.is_timeout() {
        LlmError::Timeout
    } else {
        LlmError::Network(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npc::provider::LlmProviderType;
    use httpmock::prelude::*;

    fn anthropic_client(server: &MockServer) -> LlmClient {
        let cfg = LlmProviderConfig {
            provider: LlmProviderType::Anthropic,
            api_key: Some("test-key".to_string()),
            endpoint_url: None,
            model: None,
        };
        LlmClient::new(cfg).with_endpoint_override(server.base_url())
    }

    fn openai_client(server: &MockServer) -> LlmClient {
        let cfg = LlmProviderConfig {
            provider: LlmProviderType::OpenAi,
            api_key: Some("sk-test".to_string()),
            endpoint_url: None,
            model: None,
        };
        LlmClient::new(cfg).with_endpoint_override(server.base_url())
    }

    fn ollama_client(server: &MockServer) -> LlmClient {
        let cfg = LlmProviderConfig {
            provider: LlmProviderType::Ollama,
            api_key: None,
            endpoint_url: None,
            model: Some("llama3.2".to_string()),
        };
        LlmClient::new(cfg).with_endpoint_override(server.base_url())
    }

    fn anthropic_response(text: &str) -> serde_json::Value {
        serde_json::json!({
            "content": [{"type": "text", "text": text}],
            "model": "claude-haiku-4-5-20251001",
            "stop_reason": "end_turn"
        })
    }

    fn openai_response(text: &str) -> serde_json::Value {
        serde_json::json!({
            "choices": [{"message": {"role": "assistant", "content": text}}],
            "model": "gpt-4o-mini"
        })
    }

    // ── Anthropic ─────────────────────────────────────────────────────────────

    #[test]
    fn anthropic_complete_returns_text_from_content_block() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(200)
                .json_body(anthropic_response(r#"{"action":"call","amount":null}"#));
        });
        let result = anthropic_client(&server)
            .complete("system", "user")
            .unwrap();
        assert_eq!(result, r#"{"action":"call","amount":null}"#);
    }

    #[test]
    fn anthropic_non_2xx_returns_api_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(401).body(r#"{"error":"invalid key"}"#);
        });
        let err = anthropic_client(&server)
            .complete("system", "user")
            .unwrap_err();
        assert!(matches!(err, LlmError::Api(401, _)));
    }

    #[test]
    fn anthropic_malformed_json_returns_parse_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(200).body("not valid json");
        });
        let err = anthropic_client(&server)
            .complete("system", "user")
            .unwrap_err();
        assert!(matches!(err, LlmError::Parse(_)));
    }

    // ── OpenAI-compatible ─────────────────────────────────────────────────────

    #[test]
    fn openai_complete_returns_choice_content() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/chat/completions");
            then.status(200)
                .json_body(openai_response(r#"{"action":"fold","amount":null}"#));
        });
        let result = openai_client(&server).complete("system", "user").unwrap();
        assert_eq!(result, r#"{"action":"fold","amount":null}"#);
    }

    #[test]
    fn ollama_complete_uses_chat_completions_path() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/chat/completions");
            then.status(200)
                .json_body(openai_response(r#"{"action":"check","amount":null}"#));
        });
        let result = ollama_client(&server).complete("system", "user").unwrap();
        assert_eq!(result, r#"{"action":"check","amount":null}"#);
    }

    #[test]
    fn openai_non_2xx_returns_api_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/chat/completions");
            then.status(429).body(r#"{"error":"rate limited"}"#);
        });
        let err = openai_client(&server)
            .complete("system", "user")
            .unwrap_err();
        assert!(matches!(err, LlmError::Api(429, _)));
    }

    #[test]
    fn timeout_returns_timeout_or_network_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(200)
                .delay(std::time::Duration::from_secs(3))
                .json_body(anthropic_response("{}"));
        });
        let cfg = LlmProviderConfig {
            provider: LlmProviderType::Anthropic,
            api_key: Some("k".to_string()),
            endpoint_url: None,
            model: None,
        };
        let client = LlmClient::with_timeout_secs(cfg, 1, server.base_url());
        let err = client.complete("system", "user").unwrap_err();
        assert!(matches!(err, LlmError::Timeout | LlmError::Network(_)));
    }
}
