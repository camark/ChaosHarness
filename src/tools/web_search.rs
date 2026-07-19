//! WebSearch tool - Search the web

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::{json, Value};
use std::time::Duration;

/// Input schema for web search tool
pub fn web_search_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Search query"
            },
            "num_results": {
                "type": "integer",
                "description": "Number of results to return",
                "default": 10,
                "minimum": 1,
                "maximum": 50
            },
            "timeout_seconds": {
                "type": "integer",
                "description": "Request timeout in seconds",
                "default": 30,
                "minimum": 1,
                "maximum": 120
            }
        },
        "required": ["query"]
    })
}

/// WebSearch tool
pub struct WebSearchTool;

#[async_trait::async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn description(&self) -> &'static str {
        "Search the web and return search results with titles, URLs, and snippets."
    }

    fn input_schema(&self) -> Value {
        web_search_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let query = input["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' field"))?;

        let num_results = input["num_results"]
            .as_u64()
            .unwrap_or(10)
            .clamp(1, 50) as usize;

        let timeout_seconds = input["timeout_seconds"]
            .as_u64()
            .unwrap_or(30)
            .clamp(1, 120);

        // Use DuckDuckGo HTML search (no API key required)
        let encoded_query = urlencoding::encode(query);
        let search_url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            encoded_query
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .user_agent("Mozilla/5.0 (compatible; RustHarness/1.0)")
            .build()?;

        let response = tokio::time::timeout(
            Duration::from_secs(timeout_seconds),
            client.get(&search_url).send(),
        )
        .await;

        match response {
            Ok(Ok(resp)) => {
                let status = resp.status();
                if !status.is_success() {
                    return Ok(ToolResult::error(format!(
                        "Search failed: HTTP {}",
                        status
                    )));
                }

                let html = match resp.text().await {
                    Ok(t) => t,
                    Err(e) => {
                        return Ok(ToolResult::error(format!(
                            "Failed to read response: {}",
                            e
                        )))
                    }
                };

                // Parse results from HTML
                let results = parse_search_results(&html, num_results);

                if results.is_empty() {
                    return Ok(ToolResult::success(
                        "(no search results found)".to_string(),
                    ));
                }

                let formatted = results
                    .iter()
                    .enumerate()
                    .map(|(i, r)| format!("{}. {}\n   URL: {}\n   {}", i + 1, r.title, r.url, r.snippet))
                    .collect::<Vec<_>>()
                    .join("\n\n");

                Ok(ToolResult::success(formatted))
            }
            Ok(Err(e)) => Ok(ToolResult::error(format!("Search request failed: {}", e))),
            Err(_) => Ok(ToolResult::error(format!(
                "Search timed out after {} seconds",
                timeout_seconds
            ))),
        }
    }
}

#[derive(Debug, Clone)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

fn parse_search_results(html: &str, limit: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // Simple HTML parsing using string matching
    // DuckDuckGo HTML search results have this structure:
    // <a class="result__a" href="...">Title</a>
    // <a class="result__snippet">Snippet</a>

    let mut current_title = String::new();
    let mut current_url = String::new();

    for line in html.lines() {
        let line = line.trim();

        // Extract title and URL from result links
        if line.contains(r#"class="result__a""#) {
            if let Some(href_start) = line.find("href=\"") {
                let href_rest = &line[href_start + 6..];
                if let Some(href_end) = href_rest.find('"') {
                    current_url = html_unescape(&href_rest[..href_end]);
                }
            }

            // Extract title text between > and <
            if let Some(title_start) = line.find(">") {
                let title_rest = &line[title_start + 1..];
                if let Some(title_end) = title_rest.find("</a>") {
                    current_title = html_unescape(&title_rest[..title_end]);
                }
            }
        }

        // Extract snippet
        if line.contains(r#"class="result__snippet""#) {
            if let Some(snippet_start) = line.find(">") {
                let snippet_rest = &line[snippet_start + 1..];
                if let Some(snippet_end) = snippet_rest.find("</a>") {
                    let snippet = html_unescape(&snippet_rest[..snippet_end]);

                    if !current_title.is_empty() && !current_url.is_empty() {
                        results.push(SearchResult {
                            title: current_title.clone(),
                            url: current_url.clone(),
                            snippet: snippet.clone(),
                        });

                        if results.len() >= limit {
                            break;
                        }
                    }
                }
            }
        }
    }

    results
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_web_search_empty_query() {
        let tool = WebSearchTool;
        let input = json!({"query": ""});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        // Empty query might return results or error, just verify it completes
        assert!(result.output.len() > 0);
    }

    #[tokio::test]
    async fn test_web_search_missing_query() {
        let tool = WebSearchTool;
        let input = json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        // Should return error
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'query'"));
    }
}
