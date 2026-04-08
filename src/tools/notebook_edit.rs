//! NotebookEdit tool - Edit Jupyter notebook cells

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::{json, Value};
use std::path::PathBuf;

/// Input schema for notebook edit tool
pub fn notebook_edit_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "notebook_path": {
                "type": "string",
                "description": "Path to the Jupyter notebook file (.ipynb)"
            },
            "cell_id": {
                "type": "string",
                "description": "ID of the cell to edit (optional, creates new cell if not provided)"
            },
            "new_source": {
                "type": "string",
                "description": "New source code for the cell"
            },
            "cell_type": {
                "type": "string",
                "enum": ["code", "markdown"],
                "description": "Type of the cell (code or markdown)"
            },
            "edit_mode": {
                "type": "string",
                "enum": ["replace", "insert", "delete"],
                "description": "Type of edit operation",
                "default": "replace"
            }
        },
        "required": ["notebook_path", "new_source"]
    })
}

/// NotebookEdit tool
pub struct NotebookEditTool;

#[async_trait::async_trait]
impl Tool for NotebookEditTool {
    fn name(&self) -> &'static str {
        "notebook_edit"
    }

    fn description(&self) -> &'static str {
        "Edit Jupyter notebook (.ipynb) cells - replace, insert, or delete cells."
    }

    fn input_schema(&self) -> Value {
        notebook_edit_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, context: ToolExecutionContext) -> Result<ToolResult> {
        let notebook_path_str = input["notebook_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'notebook_path' field"))?;

        let new_source = input["new_source"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_source' field"))?;

        let cell_id = input["cell_id"].as_str().map(String::from);
        let cell_type = input["cell_type"].as_str().unwrap_or("code");
        let edit_mode = input["edit_mode"].as_str().unwrap_or("replace");

        let notebook_path = resolve_path(&context.cwd, notebook_path_str);

        if !notebook_path.exists() {
            return Ok(ToolResult::error(format!(
                "Notebook not found: {}",
                notebook_path.display()
            )));
        }

        // Read notebook
        let notebook_content = match tokio::fs::read_to_string(&notebook_path).await {
            Ok(content) => content,
            Err(e) => return Ok(ToolResult::error(format!("Failed to read notebook: {}", e))),
        };

        let mut notebook: Value = match serde_json::from_str(&notebook_content) {
            Ok(nb) => nb,
            Err(e) => return Ok(ToolResult::error(format!("Invalid notebook JSON: {}", e))),
        };

        let cells = notebook["cells"]
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("Invalid notebook structure: missing cells array"))?;

        match edit_mode {
            "replace" => {
                if let Some(ref id) = cell_id {
                    // Find and replace existing cell
                    let mut found = false;
                    for cell in cells.iter_mut() {
                        if cell.get("id").and_then(|v| v.as_str()) == Some(id) {
                            cell["source"] = json!([new_source]);
                            cell["cell_type"] = json!(cell_type);
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Ok(ToolResult::error(format!(
                            "Cell with id '{}' not found",
                            id
                        )));
                    }
                } else {
                    // Replace first cell or create new
                    if cells.is_empty() {
                        cells.push(create_cell(new_source, cell_type, "1"));
                    } else {
                        cells[0]["source"] = json!([new_source]);
                        cells[0]["cell_type"] = json!(cell_type);
                    }
                }
            }
            "insert" => {
                let new_cell = create_cell(new_source, cell_type, &generate_cell_id(cells));
                if let Some(ref id) = cell_id {
                    // Insert after specified cell
                    let mut insert_pos = cells.len();
                    for (i, cell) in cells.iter().enumerate() {
                        if cell.get("id").and_then(|v| v.as_str()) == Some(id) {
                            insert_pos = i + 1;
                            break;
                        }
                    }
                    cells.insert(insert_pos, new_cell);
                } else {
                    // Insert at end
                    cells.push(new_cell);
                }
            }
            "delete" => {
                if let Some(ref id) = cell_id {
                    let original_len = cells.len();
                    cells.retain(|cell| {
                        cell.get("id").and_then(|v| v.as_str()) != Some(id)
                    });
                    if cells.len() == original_len {
                        return Ok(ToolResult::error(format!(
                            "Cell with id '{}' not found",
                            id
                        )));
                    }
                } else {
                    return Ok(ToolResult::error(
                        "cell_id is required for delete mode".to_string(),
                    ));
                }
            }
            _ => {
                return Ok(ToolResult::error(format!(
                    "Unknown edit_mode: {}",
                    edit_mode
                )))
            }
        }

        // Write updated notebook
        let updated_content = serde_json::to_string_pretty(&notebook)?;
        tokio::fs::write(&notebook_path, updated_content.as_bytes()).await?;

        Ok(ToolResult::success(format!(
            "Updated notebook: {}",
            notebook_path.display()
        )))
    }
}

fn create_cell(source: &str, cell_type: &str, id: &str) -> Value {
    json!({
        "cell_type": cell_type,
        "source": [source],
        "metadata": {},
        "id": id
    })
}

fn generate_cell_id(cells: &Vec<Value>) -> String {
    let max_id = cells
        .iter()
        .filter_map(|c| c.get("id").and_then(|v| v.as_str()))
        .filter_map(|s| s.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    (max_id + 1).to_string()
}

fn resolve_path(base: &PathBuf, candidate: &str) -> PathBuf {
    let path = PathBuf::from(candidate);

    // Expand ~ to home directory
    if candidate.starts_with("~/") || candidate == "~" {
        if let Some(home_dir) = dirs::home_dir() {
            let remainder = if candidate == "~" { "" } else { &candidate[2..] };

            // Special handling for Desktop - use OS-specific desktop directory
            if remainder == "Desktop" {
                if let Some(desktop_dir) = dirs::desktop_dir() {
                    return desktop_dir;
                }
                return home_dir.join("Desktop");
            }

            return home_dir.join(remainder);
        }
    }

    // Use dirs::desktop_dir() for automatic OS-specific Desktop detection
    if candidate == "Desktop" || candidate.ends_with("/Desktop") {
        if let Some(desktop_dir) = dirs::desktop_dir() {
            return desktop_dir;
        }
    }

    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::io::Write;

    #[tokio::test]
    async fn test_notebook_edit_replace() {
        let dir = tempdir().unwrap();
        let notebook_path = dir.path().join("test.ipynb");

        // Create a simple notebook
        let initial_notebook = json!({
            "cells": [
                {
                    "cell_type": "code",
                    "source": ["old code"],
                    "metadata": {},
                    "id": "1"
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 4
        });

        let mut file = std::fs::File::create(&notebook_path).unwrap();
        write!(file, "{}", initial_notebook).unwrap();

        let tool = NotebookEditTool;
        let input = json!({
            "notebook_path": notebook_path.to_str().unwrap(),
            "cell_id": "1",
            "new_source": "new code"
        });
        let context = ToolExecutionContext::new(dir.path().to_path_buf());
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Updated"));

        // Verify the change
        let content = tokio::fs::read_to_string(&notebook_path).await.unwrap();
        let notebook: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(notebook["cells"][0]["source"], json!(["new code"]));
    }
}
