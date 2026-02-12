use crate::tools::Tool;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::fs;

pub struct MultiEditTool;

impl Tool for MultiEditTool {
    fn name(&self) -> String {
        "multi_edit".to_string()
    }

    fn description(&self) -> String {
        "Perform multiple string replacements in a single file atomically.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "multi_edit",
                "description": "Perform multiple string replacements in a single file atomically. If any edit fails, none are applied.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The absolute path to the file to modify"
                        },
                        "edits": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "old_string": { "type": "string", "description": "The exact string to find" },
                                    "new_string": { "type": "string", "description": "The string to replace it with" },
                                    "replace_all": { "type": "boolean", "description": "Whether to replace all occurrences (default: false)" }
                                },
                                "required": ["old_string", "new_string"]
                            }
                        }
                    },
                    "required": ["path", "edits"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'path'"))?;

        let edits = args
            .get("edits")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Missing 'edits'"))?;

        // Acquire lock before reading and writing
        let _guard = crate::tools::locks::acquire_lock(path)?;

        let mut content = fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read file '{}': {}", path, e))?;

        // Apply edits in memory first
        for (i, edit) in edits.iter().enumerate() {
            let old_str = edit
                .get("old_string")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Edit #{}: Missing 'old_string'", i))?;

            let new_str = edit
                .get("new_string")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Edit #{}: Missing 'new_string'", i))?;

            let replace_all = edit
                .get("replace_all")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !content.contains(old_str) {
                return Err(anyhow!("Edit #{}: 'old_string' not found in content", i));
            }

            if replace_all {
                content = content.replace(old_str, new_str);
            } else {
                content = content.replacen(old_str, new_str, 1);
            }
        }

        // Write back only if all succeeded
        fs::write(path, content).map_err(|e| anyhow!("Failed to write file '{}': {}", path, e))?;

        Ok(format!(
            "Successfully applied {} edits to {}",
            edits.len(),
            path
        ))
    }
}
