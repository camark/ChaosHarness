//! Lsp tool - Code intelligence operations
//!
//! Provides basic code intelligence using regex pattern matching.
//! For full LSP support, connect an external LSP server via MCP.

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Input schema for lsp tool
pub fn lsp_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {
                "type": "string",
                "description": "The code intelligence operation to perform",
                "enum": [
                    "document_symbol",
                    "workspace_symbol",
                    "go_to_definition",
                    "find_references",
                    "hover"
                ]
            },
            "file_path": {
                "type": "string",
                "description": "Path to the source file for file-based operations"
            },
            "symbol": {
                "type": "string",
                "description": "Explicit symbol name to look up"
            },
            "line": {
                "type": "integer",
                "description": "1-based line number for position-based lookups",
                "minimum": 1
            },
            "character": {
                "type": "integer",
                "description": "1-based character offset for position-based lookups",
                "minimum": 1
            },
            "query": {
                "type": "string",
                "description": "Substring query for workspace_symbol"
            }
        },
        "required": ["operation"]
    })
}

/// Lsp tool
pub struct LspTool;

#[async_trait::async_trait]
impl Tool for LspTool {
    fn name(&self) -> &'static str {
        "lsp"
    }

    fn description(&self) -> &'static str {
        "Inspect code symbols, definitions, references, and hover information across the workspace."
    }

    fn input_schema(&self) -> Value {
        lsp_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let operation = input["operation"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'operation' field"))?;

        let file_path = input["file_path"].as_str();
        let symbol = input["symbol"].as_str();
        let line = input["line"].as_i64();
        let _character = input["character"].as_i64();
        let query = input["query"].as_str();

        match operation {
            "document_symbol" => {
                let path = file_path.ok_or_else(|| anyhow::anyhow!("document_symbol requires file_path"))?;
                match extract_document_symbols(path) {
                    Ok(symbols) => {
                        if symbols.is_empty() {
                            Ok(ToolResult::success(format!("No symbols found in {}", path)))
                        } else {
                            let output = symbols.iter()
                                .map(|s| format!("[{}] {} (line {})", s.kind, s.name, s.line))
                                .collect::<Vec<_>>()
                                .join("\n");
                            Ok(ToolResult::success(output))
                        }
                    }
                    Err(e) => Ok(ToolResult::error(e.to_string())),
                }
            }
            "workspace_symbol" => {
                let q = query.ok_or_else(|| anyhow::anyhow!("workspace_symbol requires query"))?;
                let context = &_context;
                let cwd = &context.cwd;
                match search_workspace_symbols(cwd, q) {
                    Ok(results) => {
                        if results.is_empty() {
                            Ok(ToolResult::success(format!("No symbols matching '{}'", q)))
                        } else {
                            let output = results.iter()
                                .map(|s| format!("[{}] {} - {} (line {})", s.kind, s.name, s.file, s.line))
                                .collect::<Vec<_>>()
                                .join("\n");
                            Ok(ToolResult::success(output))
                        }
                    }
                    Err(e) => Ok(ToolResult::error(e.to_string())),
                }
            }
            "go_to_definition" => {
                let path = file_path.ok_or_else(|| anyhow::anyhow!("go_to_definition requires file_path"))?;
                let sym = symbol.ok_or_else(|| anyhow::anyhow!("go_to_definition requires symbol"))?;
                match find_definition(path, sym) {
                    Ok(results) => {
                        if results.is_empty() {
                            Ok(ToolResult::error(format!("Definition not found for '{}'", sym)))
                        } else {
                            Ok(ToolResult::success(results.join("\n")))
                        }
                    }
                    Err(e) => Ok(ToolResult::error(e.to_string())),
                }
            }
            "find_references" => {
                let path = file_path.ok_or_else(|| anyhow::anyhow!("find_references requires file_path"))?;
                let sym = symbol.ok_or_else(|| anyhow::anyhow!("find_references requires symbol"))?;
                let context = &_context;
                let cwd = &context.cwd;
                match find_references(cwd, sym, Some(path)) {
                    Ok(results) => {
                        if results.is_empty() {
                            Ok(ToolResult::success(format!("No references found for '{}'", sym)))
                        } else {
                            Ok(ToolResult::success(results.join("\n")))
                        }
                    }
                    Err(e) => Ok(ToolResult::error(e.to_string())),
                }
            }
            "hover" => {
                let path = file_path.ok_or_else(|| anyhow::anyhow!("hover requires file_path"))?;
                let line_num = line.ok_or_else(|| anyhow::anyhow!("hover requires line"))? as usize;
                match get_hover_info(path, line_num) {
                    Ok(info) => Ok(ToolResult::success(info)),
                    Err(e) => Ok(ToolResult::error(e.to_string())),
                }
            }
            _ => Ok(ToolResult::error(format!("Unknown LSP operation: {}", operation))),
        }
    }
}

/// Extracted symbol information
struct SymbolInfo {
    name: String,
    kind: String,
    line: usize,
    file: String,
}

/// Extract symbols from a document using regex patterns
fn extract_document_symbols(file_path: &str) -> Result<Vec<SymbolInfo>> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(anyhow::anyhow!("File not found: {}", file_path));
    }

    let content = fs::read_to_string(path)?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut symbols = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Rust patterns
        if ext == "rs" {
            if let Some(name) = extract_pattern(trimmed, &["fn ", "pub fn ", "async fn ", "pub async fn "]) {
                symbols.push(SymbolInfo {
                    name: name.to_string(),
                    kind: "function".to_string(),
                    line: line_num + 1,
                    file: file_path.to_string(),
                });
            } else if let Some(name) = extract_pattern(trimmed, &["struct ", "pub struct "]) {
                symbols.push(SymbolInfo {
                    name: name.to_string(),
                    kind: "struct".to_string(),
                    line: line_num + 1,
                    file: file_path.to_string(),
                });
            } else if let Some(name) = extract_pattern(trimmed, &["enum ", "pub enum "]) {
                symbols.push(SymbolInfo {
                    name: name.to_string(),
                    kind: "enum".to_string(),
                    line: line_num + 1,
                    file: file_path.to_string(),
                });
            } else if let Some(name) = extract_pattern(trimmed, &["trait ", "pub trait "]) {
                symbols.push(SymbolInfo {
                    name: name.to_string(),
                    kind: "trait".to_string(),
                    line: line_num + 1,
                    file: file_path.to_string(),
                });
            } else if let Some(name) = extract_pattern(trimmed, &["impl ", "impl<"]) {
                symbols.push(SymbolInfo {
                    name: name.to_string(),
                    kind: "impl".to_string(),
                    line: line_num + 1,
                    file: file_path.to_string(),
                });
            } else if let Some(name) = extract_pattern(trimmed, &["type ", "pub type "]) {
                symbols.push(SymbolInfo {
                    name: name.to_string(),
                    kind: "type".to_string(),
                    line: line_num + 1,
                    file: file_path.to_string(),
                });
            } else if let Some(name) = extract_pattern(trimmed, &["const ", "pub const ", "static ", "pub static "]) {
                symbols.push(SymbolInfo {
                    name: name.to_string(),
                    kind: "constant".to_string(),
                    line: line_num + 1,
                    file: file_path.to_string(),
                });
            }
        }
        // Python patterns
        else if ext == "py" {
            if let Some(name) = extract_pattern(trimmed, &["def ", "async def "]) {
                symbols.push(SymbolInfo {
                    name: name.to_string(),
                    kind: "function".to_string(),
                    line: line_num + 1,
                    file: file_path.to_string(),
                });
            } else if let Some(name) = extract_pattern(trimmed, &["class "]) {
                symbols.push(SymbolInfo {
                    name: name.to_string(),
                    kind: "class".to_string(),
                    line: line_num + 1,
                    file: file_path.to_string(),
                });
            }
        }
        // JavaScript/TypeScript patterns
        else if ext == "js" || ext == "ts" || ext == "jsx" || ext == "tsx" {
            if let Some(name) = extract_pattern(trimmed, &["function ", "async function "]) {
                symbols.push(SymbolInfo {
                    name: name.to_string(),
                    kind: "function".to_string(),
                    line: line_num + 1,
                    file: file_path.to_string(),
                });
            } else if let Some(name) = extract_pattern(trimmed, &["class "]) {
                symbols.push(SymbolInfo {
                    name: name.to_string(),
                    kind: "class".to_string(),
                    line: line_num + 1,
                    file: file_path.to_string(),
                });
            } else if let Some(name) = extract_pattern(trimmed, &["export interface ", "interface "]) {
                symbols.push(SymbolInfo {
                    name: name.to_string(),
                    kind: "interface".to_string(),
                    line: line_num + 1,
                    file: file_path.to_string(),
                });
            } else if let Some(name) = extract_pattern(trimmed, &["export type ", "type "]) {
                symbols.push(SymbolInfo {
                    name: name.to_string(),
                    kind: "type".to_string(),
                    line: line_num + 1,
                    file: file_path.to_string(),
                });
            }
        }
    }

    Ok(symbols)
}

/// Extract a name from a line if it starts with one of the prefixes
fn extract_pattern<'a>(line: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    for prefix in prefixes {
        if line.starts_with(prefix) {
            let rest = &line[prefix.len()..];
            // Extract until ( for functions, { or : for structs/enums, < for generics
            let end = rest.find(|c: char| c == '(' || c == '{' || c == ':' || c == '<')
                .unwrap_or(rest.len());
            let name = rest[..end].trim();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

/// Search for symbols in the workspace
fn search_workspace_symbols(cwd: &Path, query: &str) -> Result<Vec<SymbolInfo>> {
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();

    // Search in common source directories
    let search_dirs = vec![
        cwd.join("src"),
        cwd.join("lib"),
        cwd.join("app"),
        cwd.to_path_buf(),
    ];

    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }
        search_dir(&dir, &query_lower, &mut results, 3)?;
    }

    Ok(results)
}

/// Recursively search a directory for symbols
fn search_dir(dir: &Path, query: &str, results: &mut Vec<SymbolInfo>, depth: usize) -> Result<()> {
    if depth == 0 || results.len() >= 50 {
        return Ok(());
    }

    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
            if dir_name.starts_with('.') || dir_name == "node_modules" || dir_name == "target" {
                continue;
            }
            search_dir(&path, query, results, depth - 1)?;
        } else if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ["rs", "py", "js", "ts", "jsx", "tsx"].contains(&ext) {
                if let Ok(symbols) = extract_document_symbols(&path.to_string_lossy()) {
                    for sym in symbols {
                        if sym.name.to_lowercase().contains(query) {
                            results.push(sym);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Find definition of a symbol
fn find_definition(file_path: &str, symbol: &str) -> Result<Vec<String>> {
    let path = Path::new(file_path);
    let content = fs::read_to_string(path)?;
    let mut results = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        // Look for definition patterns
        if (trimmed.contains(&format!("fn {}", symbol))
            || trimmed.contains(&format!("fn {}(", symbol))
            || trimmed.contains(&format!("struct {}", symbol))
            || trimmed.contains(&format!("enum {}", symbol))
            || trimmed.contains(&format!("trait {}", symbol))
            || trimmed.contains(&format!("type {}", symbol))
            || trimmed.contains(&format!("const {}", symbol))
            || trimmed.contains(&format!("let {}", symbol)))
            && !trimmed.starts_with("//")
        {
            results.push(format!("{}:{}: {}", file_path, line_num + 1, trimmed));
        }
    }

    Ok(results)
}

/// Find references to a symbol in the workspace
fn find_references(cwd: &Path, symbol: &str, exclude_file: Option<&str>) -> Result<Vec<String>> {
    let mut results = Vec::new();
    let search_dirs = vec![cwd.join("src"), cwd.join("lib"), cwd.to_path_buf()];

    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }
        search_references_in_dir(&dir, symbol, exclude_file, &mut results, 3)?;
    }

    Ok(results)
}

/// Recursively search for references
fn search_references_in_dir(
    dir: &Path,
    symbol: &str,
    exclude_file: Option<&str>,
    results: &mut Vec<String>,
    depth: usize,
) -> Result<()> {
    if depth == 0 || results.len() >= 100 {
        return Ok(());
    }

    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
            if dir_name.starts_with('.') || dir_name == "node_modules" || dir_name == "target" {
                continue;
            }
            search_references_in_dir(&path, symbol, exclude_file, results, depth - 1)?;
        } else if path.is_file() {
            let path_str = path.to_string_lossy();
            if Some(path_str.as_ref()) == exclude_file {
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ["rs", "py", "js", "ts", "jsx", "tsx"].contains(&ext) {
                if let Ok(content) = fs::read_to_string(&path) {
                    for (line_num, line) in content.lines().enumerate() {
                        if line.contains(symbol) && !line.trim().starts_with("//") {
                            results.push(format!("{}:{}: {}", path_str, line_num + 1, line.trim()));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Get hover information for a line
fn get_hover_info(file_path: &str, line: usize) -> Result<String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(anyhow::anyhow!("File not found: {}", file_path));
    }

    let content = fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();

    if line == 0 || line > lines.len() {
        return Err(anyhow::anyhow!("Line {} out of range (1-{})", line, lines.len()));
    }

    let target_line = lines[line - 1];
    let mut info = String::new();

    // Check for comments above the line
    let mut comments = Vec::new();
    if line >= 2 {
        let prev_line = lines[line - 2].trim();
        if prev_line.starts_with("//") || prev_line.starts_with("///") || prev_line.starts_with("#") {
            comments.push(prev_line);
        }
    }

    info.push_str(&format!("Line {}: {}", line, target_line.trim()));

    if !comments.is_empty() {
        info.push_str(&format!("\n\nDocumentation:\n{}", comments.join("\n")));
    }

    // Add context about what's on this line
    let trimmed = target_line.trim();
    if trimmed.contains("fn ") {
        info.push_str("\n\nType: Function definition");
    } else if trimmed.contains("struct ") {
        info.push_str("\n\nType: Struct definition");
    } else if trimmed.contains("enum ") {
        info.push_str("\n\nType: Enum definition");
    } else if trimmed.contains("trait ") {
        info.push_str("\n\nType: Trait definition");
    } else if trimmed.contains("let ") || trimmed.contains("const ") {
        info.push_str("\n\nType: Variable/constant binding");
    } else if trimmed.contains("impl ") {
        info.push_str("\n\nType: Implementation block");
    }

    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_lsp_workspace_symbol_missing_query() {
        let tool = LspTool;
        let input = serde_json::json!({
            "operation": "workspace_symbol"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        // Should return Err for missing required field
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires query"));
    }

    #[tokio::test]
    async fn test_lsp_file_operation_missing_path() {
        let tool = LspTool;
        let input = serde_json::json!({
            "operation": "document_symbol"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        // Should return Err for missing required field
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires file_path"));
    }

    #[tokio::test]
    async fn test_lsp_document_symbol() {
        let tool = LspTool;
        let input = serde_json::json!({
            "operation": "document_symbol",
            "file_path": "src/tools/lsp.rs"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        // Should find at least the LspTool struct and some functions
        assert!(result.output.contains("struct") || result.output.contains("function"));
    }

    #[tokio::test]
    async fn test_lsp_document_symbol_not_found() {
        let tool = LspTool;
        let input = serde_json::json!({
            "operation": "document_symbol",
            "file_path": "nonexistent.rs"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("not found") || result.output.contains("File not found"));
    }

    #[tokio::test]
    async fn test_lsp_workspace_symbol() {
        let tool = LspTool;
        let input = serde_json::json!({
            "operation": "workspace_symbol",
            "query": "LspTool"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("LspTool"));
    }

    #[tokio::test]
    async fn test_lsp_hover() {
        let tool = LspTool;
        let input = serde_json::json!({
            "operation": "hover",
            "file_path": "src/tools/lsp.rs",
            "line": 1
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Line 1:"));
    }

    #[tokio::test]
    async fn test_lsp_hover_out_of_range() {
        let tool = LspTool;
        let input = serde_json::json!({
            "operation": "hover",
            "file_path": "src/tools/lsp.rs",
            "line": 99999
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("out of range"));
    }

    #[tokio::test]
    async fn test_lsp_unknown_operation() {
        let tool = LspTool;
        let input = serde_json::json!({
            "operation": "unknown_op"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("Unknown LSP operation"));
    }

    #[test]
    fn test_extract_pattern() {
        assert_eq!(extract_pattern("fn hello()", &["fn "]), Some("hello"));
        assert_eq!(extract_pattern("pub fn world(x: i32)", &["pub fn "]), Some("world"));
        assert_eq!(extract_pattern("struct Foo {", &["struct "]), Some("Foo"));
        assert_eq!(extract_pattern("let x = 5", &["fn "]), None);
        assert_eq!(extract_pattern("impl MyStruct", &["impl "]), Some("MyStruct"));
    }
}
