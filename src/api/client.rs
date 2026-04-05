//! Anthropic API client with retry logic and tool support

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
        parse_response(response).await
    }

    async fn send_request(&self, request: &ApiRequest) -> Result<Response, ApiError> {
        let url = format!("{}/v1/messages", self.base_url);

        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|m| m.to_api_param())
            .collect();

        let body = MessageRequest {
            model: request.model.clone(),
            messages,
            system: request.system_prompt.clone(),
            max_tokens: request.max_tokens,
            tools: request.tools.clone(),
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(handle_error_response(response).await);
        }

        Ok(response)
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
