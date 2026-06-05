use serde::{Deserialize, Serialize};

const DEFAULT_MODEL: &str = "claude-haiku-4-5-20251001";
const DEFAULT_TIMEOUT_SECS: u64 = 5;
const DEFAULT_MAX_TOKENS: u32 = 128;
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
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

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeMessage>,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
}

/// HTTP client for the Claude Messages API.
pub struct LlmClient {
    client: reqwest::blocking::Client,
    api_key: String,
    model: String,
    timeout_secs: u64,
    base_url: String,
}

impl LlmClient {
    /// Create a new client with default settings (5s timeout, haiku model).
    pub fn new(api_key: String) -> Self {
        Self::with_options(api_key, DEFAULT_MODEL.to_string(), DEFAULT_TIMEOUT_SECS)
    }

    pub fn with_options(api_key: String, model: String, timeout_secs: u64) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_default();

        Self {
            client,
            api_key,
            model,
            timeout_secs,
            base_url: ANTHROPIC_API_URL.to_string(),
        }
    }

    /// Override the API base URL (used in tests with a mock server).
    #[cfg(test)]
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Send a request to the Claude API and return the text from the first content block.
    pub fn complete(&self, system: &str, user: &str) -> Result<String, LlmError> {
        let body = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: DEFAULT_MAX_TOKENS,
            system: system.to_string(),
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: user.to_string(),
            }],
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout
                } else {
                    LlmError::Network(e.to_string())
                }
            })?;

        let status = response.status();

        if !status.is_success() {
            let body_text = response.text().unwrap_or_default();
            return Err(LlmError::Api(status.as_u16(), body_text));
        }

        let claude_resp: ClaudeResponse = response
            .json()
            .map_err(|e| LlmError::Parse(e.to_string()))?;

        claude_resp
            .content
            .into_iter()
            .find(|b| b.block_type == "text")
            .and_then(|b| b.text)
            .ok_or_else(|| LlmError::Parse("no text content block in response".to_string()))
    }

    pub fn timeout_secs(&self) -> u64 {
        self.timeout_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    fn mock_success_body(text: &str) -> serde_json::Value {
        serde_json::json!({
            "content": [{"type": "text", "text": text}],
            "model": DEFAULT_MODEL,
            "stop_reason": "end_turn"
        })
    }

    #[test]
    fn complete_returns_text_from_content_block() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(mock_success_body(r#"{"action":"call","amount":null}"#));
        });

        let client = LlmClient::new("test-key".to_string())
            .with_base_url(format!("{}/v1/messages", server.base_url()));

        let result = client.complete("system", "user").unwrap();
        assert_eq!(result, r#"{"action":"call","amount":null}"#);
    }

    #[test]
    fn non_2xx_response_returns_api_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(401)
                .header("content-type", "application/json")
                .body(r#"{"error":"invalid key"}"#);
        });

        let client = LlmClient::new("bad-key".to_string())
            .with_base_url(format!("{}/v1/messages", server.base_url()));

        let err = client.complete("system", "user").unwrap_err();
        assert!(matches!(err, LlmError::Api(401, _)));
    }

    #[test]
    fn malformed_json_response_returns_parse_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(200)
                .header("content-type", "application/json")
                .body("not valid json at all");
        });

        let client = LlmClient::new("test-key".to_string())
            .with_base_url(format!("{}/v1/messages", server.base_url()));

        let err = client.complete("system", "user").unwrap_err();
        assert!(matches!(err, LlmError::Parse(_)));
    }

    #[test]
    fn request_timeout_returns_timeout_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            // Delay longer than the 1-second timeout we'll set.
            then.status(200)
                .delay(std::time::Duration::from_secs(3))
                .json_body(mock_success_body("{}"));
        });

        let client = LlmClient::with_options(
            "test-key".to_string(),
            DEFAULT_MODEL.to_string(),
            1, // 1-second timeout
        )
        .with_base_url(format!("{}/v1/messages", server.base_url()));

        let err = client.complete("system", "user").unwrap_err();
        assert!(matches!(err, LlmError::Timeout | LlmError::Network(_)));
    }
}
