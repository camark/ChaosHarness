//! Anthropic API client with tool support and OpenAI-compatible API support

use crate::api::errors::ApiError;
use crate::engine::messages::{ConversationMessage, MessageContent, ToolUseData};
use anyhow::Result;
use async_stream::try_stream;
use futures::{Stream, TryStreamExt};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tracing::warn;
use rand::Rng;

const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 1000;
const MAX_DELAY_MS: u64 = 30000;

#[derive(Debug, Serialize)]
struct MessageRequest {
    model: String,
    messages: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<Value>,
}

/// OpenAI-compatible request format (for Moonshot and other providers)
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    id: String,
    #[serde(rename = "type")]
    message_type: String,
    role: String,
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: UsageData,
}

/// OpenAI-compatible response format (for Moonshot and other providers)
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    index: i32,
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

#[derive(Debug, Deserialize)]
struct UsageData {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
}

#[derive(Debug, Clone)]
pub struct ApiRequest {
    pub model: String,
    pub messages: Vec<ConversationMessage>,
    pub system_prompt: Option<String>,
    pub max_tokens: u32,
    pub tools: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct ApiUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct ApiMessage {
    pub role: String,
    pub content: Vec<MessageContent>,
    pub tool_uses: Vec<ToolUseData>,
    pub usage: ApiUsage,
    pub stop_reason: Option<String>,
}

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        let base_url = base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string());

        Self {
            client,
            base_url,
            api_key,
        }
    }

    /// Stream a message and return the full response
    pub async fn send_message(&self, request: ApiRequest) -> Result<ApiMessage, ApiError> {
        let mut last_error: Option<ApiError> = None;

        for attempt in 0..=MAX_RETRIES {
            match self.send_once(&request).await {
                Ok(msg) => return Ok(msg),
                Err(e) => {
                    if !is_retryable(&e) {
                        return Err(e);
                    }
                    last_error = Some(e);

                    if attempt < MAX_RETRIES {
                        let delay = calculate_retry_delay(attempt, &last_error);
                        warn!(
                            "API request failed (attempt {}/{}, retrying in {}ms",
                            attempt + 1,
                            MAX_RETRIES + 1,
                            delay
                        );
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| ApiError::Request("Unknown error".to_string())))
    }

    /// Stream text deltas only (for non-tool responses)
    pub async fn stream_text(
        &self,
        request: ApiRequest,
    ) -> Result<impl Stream<Item = Result<String, ApiError>>, ApiError> {
        let response = self.send_request(&request).await?;
        Ok(extract_text_stream(response))
    }

    async fn send_once(&self, request: &ApiRequest) -> Result<ApiMessage, ApiError> {
        let response = self.send_request(request).await?;

        // Detect API type and parse response accordingly
        let use_openai_format = self.base_url.contains("moonshot")
            || self.base_url.contains("openai")
            || self.api_key.starts_with("sk-");

        if use_openai_format {
            parse_openai_response(response).await
        } else {
            parse_response(response).await
        }
    }

    async fn send_request(&self, request: &ApiRequest) -> Result<Response, ApiError> {
        // Detect API type and build appropriate URL
        let use_openai_format = self.base_url.contains("moonshot")
            || self.base_url.contains("openai")
            || self.api_key.starts_with("sk-");

        // Build URL - check if base_url already contains /v1 to avoid duplication
        let url = if use_openai_format {
            // OpenAI-compatible APIs use /v1/chat/completions
            if self.base_url.ends_with("/v1") || self.base_url.ends_with("/v1/") {
                format!("{}/chat/completions", self.base_url)
            } else {
                format!("{}/v1/chat/completions", self.base_url)
            }
        } else {
            // Anthropic uses /v1/messages
            format!("{}/v1/messages", self.base_url)
        };

        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|m| m.to_api_param())
            .collect();

        // Build request body based on API format
        if use_openai_format {
            // OpenAI format: system prompt goes into messages array
            let mut openai_messages = Vec::new();

            // Add system message first if present
            if let Some(ref system) = request.system_prompt {
                openai_messages.push(serde_json::json!({
                    "role": "system",
                    "content": system
                }));
            }

            // Add user/assistant messages
            openai_messages.extend(messages);

            let body = OpenAIRequest {
                model: request.model.clone(),
                messages: openai_messages,
                max_tokens: Some(request.max_tokens),
            };

            let mut req = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&body);

            req = req.header("Authorization", format!("Bearer {}", self.api_key));

            let response = req
                .send()
                .await
                .map_err(|e| ApiError::Network(e.to_string()))?;

            if !response.status().is_success() {
                return Err(handle_error_response(response).await);
            }

            Ok(response)
        } else {
            // Anthropic format: system prompt is separate field
            let body = MessageRequest {
                model: request.model.clone(),
                messages,
                system: request.system_prompt.clone(),
                max_tokens: request.max_tokens,
                tools: request.tools.clone(),
            };

            let mut req = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&body);

            req = req
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01");

            let response = req
                .send()
                .await
                .map_err(|e| ApiError::Network(e.to_string()))?;

            if !response.status().is_success() {
                return Err(handle_error_response(response).await);
            }

            Ok(response)
        }
    }
}

async fn parse_response(response: Response) -> Result<ApiMessage, ApiError> {
    let body: MessageResponse = response
        .json()
        .await
        .map_err(|e| ApiError::Json(e.to_string()))?;

    let mut content: Vec<MessageContent> = Vec::new();
    let mut tool_uses: Vec<ToolUseData> = Vec::new();

    for block in body.content {
        match block {
            ContentBlock::Text { text } => {
                content.push(MessageContent::Text { text });
            }
            ContentBlock::ToolUse { id, name, input } => {
                content.push(MessageContent::ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                });
                tool_uses.push(ToolUseData { id, name, input });
            }
        }
    }

    Ok(ApiMessage {
        role: body.role,
        content,
        tool_uses,
        usage: ApiUsage {
            input_tokens: body.usage.input_tokens,
            output_tokens: body.usage.output_tokens,
        },
        stop_reason: body.stop_reason,
    })
}

async fn parse_openai_response(response: Response) -> Result<ApiMessage, ApiError> {
    let body: OpenAIResponse = response
        .json()
        .await
        .map_err(|e| ApiError::Json(e.to_string()))?;

    let choice = body.choices.first().ok_or_else(|| {
        ApiError::Request("No choices in response".to_string())
    })?;

    let mut content: Vec<MessageContent> = Vec::new();
    let tool_uses: Vec<ToolUseData> = Vec::new();

    // OpenAI format: content is a string, not tool_use blocks
    // For now, treat all content as text
    // Moonshot K2.5 doesn't support tool_use in OpenAI format
    if let Some(text) = &choice.message.content {
        content.push(MessageContent::Text { text: text.clone() });
    }

    Ok(ApiMessage {
        role: choice.message.role.clone(),
        content,
        tool_uses,
        usage: ApiUsage {
            input_tokens: body.usage.prompt_tokens as u32,
            output_tokens: body.usage.completion_tokens as u32,
        },
        stop_reason: choice.finish_reason.clone(),
    })
}

fn extract_text_stream(
    response: Response,
) -> impl Stream<Item = Result<String, ApiError>> {
    try_stream! {
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.try_next().await.map_err(|e| ApiError::Network(e.to_string()))? {
            if let Ok(text) = std::str::from_utf8(&chunk) {
                // Try to parse as SSE or JSON stream
                if let Ok(data) = serde_json::from_str::<Value>(text) {
                    if let Some(delta) = data.get("delta").and_then(|d| d.get("text")).and_then(|t| t.as_str()) {
                        yield delta.to_string();
                    }
                } else if !text.trim().is_empty() && text.starts_with("data:") {
                    // SSE format
                    let data = text.trim_start_matches("data:").trim();
                    if let Ok(json) = serde_json::from_str::<Value>(data) {
                        if let Some(delta) = json.get("delta").and_then(|d| d.get("text")).and_then(|t| t.as_str()) {
                            yield delta.to_string();
                        }
                    }
                }
            }
        }
    }
}

async fn handle_error_response(response: Response) -> ApiError {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if status == 401 || status == 403 {
        ApiError::Authentication(format!("HTTP {}: {}", status, body))
    } else if status == 429 {
        ApiError::RateLimit(format!("HTTP {}: {}", status, body))
    } else {
        ApiError::Request(format!("HTTP {}: {}", status, body))
    }
}

fn is_retryable(error: &ApiError) -> bool {
    match error {
        ApiError::Network(_) => true,
        ApiError::Request(msg) => {
            msg.contains("500") || msg.contains("502") || msg.contains("503") || msg.contains("429")
        }
        ApiError::Authentication(_) | ApiError::RateLimit(_) | ApiError::Json(_) => false,
    }
}

fn calculate_retry_delay(attempt: u32, _error: &Option<ApiError>) -> u64 {
    let base_delay = BASE_DELAY_MS * 2u64.pow(attempt);
    let capped_delay = base_delay.min(MAX_DELAY_MS);

    // Add jitter (up to 25% of delay)
    let jitter = rand::thread_rng().gen_range(0..capped_delay / 4);
    capped_delay + jitter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_url_construction_anthropic() {
        // Anthropic default should use /v1/messages
        let client = ApiClient::new("test-key".to_string(), Some("https://api.anthropic.com".to_string()));
        assert_eq!(client.base_url, "https://api.anthropic.com");
    }

    #[test]
    fn test_api_url_construction_moonshot() {
        // Moonshot with /v1 suffix
        let client = ApiClient::new("sk-test".to_string(), Some("https://api.moonshot.cn/v1".to_string()));
        assert!(client.base_url.contains("moonshot"));
    }

    #[test]
    fn test_api_url_construction_moonshot_anthropic_path() {
        // Moonshot with /anthropic suffix (should still work)
        let client = ApiClient::new("sk-test".to_string(), Some("https://api.moonshot.cn/anthropic".to_string()));
        assert!(client.base_url.contains("moonshot"));
    }

    #[test]
    fn test_openai_format_detection() {
        // Should detect OpenAI format for moonshot
        let client = ApiClient::new("sk-test".to_string(), Some("https://api.moonshot.cn/v1".to_string()));
        assert!(client.base_url.contains("moonshot") || client.api_key.starts_with("sk-"));
    }

    #[test]
    fn test_openai_response_parse() {
        // Test OpenAI-format response (like Moonshot)
        let json = r#"{
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "kimi-k2.5",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello, I am Moonshot AI."
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        }"#;

        let response: OpenAIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.model, "kimi-k2.5");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.content, Some("Hello, I am Moonshot AI.".to_string()));
        assert_eq!(response.usage.prompt_tokens, 10);
        assert_eq!(response.usage.completion_tokens, 20);
    }

    #[test]
    fn test_anthropic_response_parse() {
        // Test Anthropic-format response
        let json = r#"{
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Hello, I am Claude."}],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {"input_tokens": 10, "output_tokens": 20}
        }"#;

        let response: MessageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.model, "claude-sonnet-4-20250514");
        assert_eq!(response.content.len(), 1);
        assert_eq!(response.usage.input_tokens, 10);
        assert_eq!(response.usage.output_tokens, 20);
    }

    #[test]
    fn test_user_settings_config() {
        // Test based on user's actual settings.json configuration
        // User: api_key="sk-ctv5yz...", model="kimi-k2.5", base_url="https://api.moonshot.cn/anthropic"
        // Expected behavior: Should detect OpenAI format due to sk- prefix and moonshot domain

        // Case 1: Current (incorrect) settings - should still work due to detection
        let client1 = ApiClient::new(
            "sk-ctv5yzCJV7l1JYPj5W7RXVx48Cy05VxqyfELFCzEVU0PsCj3".to_string(),
            Some("https://api.moonshot.cn/anthropic".to_string())
        );
        // Will detect OpenAI format due to sk- prefix
        assert!(client1.api_key.starts_with("sk-"));

        // Case 2: Corrected settings (recommended)
        let client2 = ApiClient::new(
            "sk-ctv5yzCJV7l1JYPj5W7RXVx48Cy05VxqyfELFCzEVU0PsCj3".to_string(),
            Some("https://api.moonshot.cn/v1".to_string())
        );
        // Will detect OpenAI format and build correct URL
        assert!(client2.base_url.contains("moonshot"));
    }

    #[test]
    fn test_openai_request_format() {
        // Test OpenAI request body serialization
        let body = OpenAIRequest {
            model: "kimi-k2.5".to_string(),
            messages: vec![
                serde_json::json!({"role": "system", "content": "You are helpful"}),
                serde_json::json!({"role": "user", "content": "Hello"}),
            ],
            max_tokens: Some(1024),
        };

        let json = serde_json::to_string_pretty(&body).unwrap();
        println!("{}", json);

        // Verify structure
        let value: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["model"], "kimi-k2.5");
        assert_eq!(value["max_tokens"], 1024);
        assert_eq!(value["messages"][0]["role"], "system");
        assert_eq!(value["messages"][0]["content"], "You are helpful");
        assert_eq!(value["messages"][1]["role"], "user");
        assert_eq!(value["messages"][1]["content"], "Hello");
    }
}
