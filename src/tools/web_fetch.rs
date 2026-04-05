//! WebFetch tool - Fetch content from URLs

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::{json, Value};
use std::time::Duration;

/// Input schema for web fetch tool
pub fn web_fetch_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "url": {
                "type": "string",
                "description": "URL to fetch content from"
            },
            "timeout_seconds": {
                "type": "integer",
                "description": "Request timeout in seconds",
                "default": 30,
                "minimum": 1,
                "maximum": 120
            }
        },
        "required": ["url"]
    })
}

/// WebFetch tool
pub struct WebFetchTool;

#[async_trait::async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &'static str {
        "web_fetch"
    }

    fn description(&self) -> &'static str {
        "Fetch content from a URL and return the text content."
    }

    fn input_schema(&self) -> Value {
        web_fetch_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let url = input["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' field"))?;

        let timeout_seconds = input["timeout_seconds"]
            .as_u64()
            .unwrap_or(30)
            .max(1)
            .min(120);

        // Validate URL
        let parsed_url = match url::Url::parse(url) {
            Ok(u) => u,
            Err(e) => return Ok(ToolResult::error(format!("Invalid URL: {}", e))),
        };

        // Only allow HTTP and HTTPS
        if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
            return Ok(ToolResult::error(
                "Only HTTP and HTTPS URLs are supported".to_string(),
            ));
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .user_agent("Mozilla/5.0 (compatible; RustHarness/1.0)")
            .build()?;

        let response = tokio::time::timeout(
            Duration::from_secs(timeout_seconds),
            client.get(url).send(),
        )
        .await;

        match response {
            Ok(Ok(resp)) => {
                let status = resp.status();
                if !status.is_success() {
                    return Ok(ToolResult::error(format!(
                        "HTTP {}: {}",
                        status,
                        status.canonical_reason().unwrap_or("Unknown Error")
                    )));
                }

                let text = match resp.text().await {
                    Ok(t) => t,
                    Err(e) => return Ok(ToolResult::error(format!("Failed to read response: {}", e))),
                };

                // Truncate very long responses (at char boundary, not byte boundary)
                let truncated = if text.len() > 50000 {
                    // Find the last character boundary before 50000
                    let truncate_at = text.char_indices()
                        .take_while(|(i, _)| *i < 50000)
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(50000);
                    format!("{}...\n[truncated]", &text[..truncate_at])
                } else {
                    text
                };

                Ok(ToolResult::success(truncated))
            }
            Ok(Err(e)) => Ok(ToolResult::error(format!("Request failed: {}", e))),
            Err(_) => Ok(ToolResult::error(format!(
                "Request timed out after {} seconds",
                timeout_seconds
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_web_fetch_invalid_url() {
        let tool = WebFetchTool;
        let input = json!({"url": "not-a-valid-url"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("Invalid URL"));
    }

    #[tokio::test]
    async fn test_web_fetch_unsupported_scheme() {
        let tool = WebFetchTool;
        let input = json!({"url": "ftp://example.com"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("Only HTTP and HTTPS"));
    }
}
