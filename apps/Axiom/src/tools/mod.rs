use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::fs;

pub mod ast_grep;
pub mod batch;
pub mod bevy;
pub mod locks;
pub mod lsp;
pub mod multiedit;
pub mod search;
pub mod shell;
pub mod todo;

use crate::types::AsyncMessage;
use std::sync::mpsc::Sender;

pub trait Tool {
    fn name(&self) -> String;
    #[allow(dead_code)]
    fn description(&self) -> String;
    fn schema(&self) -> Value;
    fn execute(&self, args: Value) -> Result<String>;
}

pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn name(&self) -> String {
        "read_file".to_string()
    }

    fn description(&self) -> String {
        "Reads a file from the local filesystem.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Reads a file from the local filesystem.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The path to the file to read (relative to current working directory or absolute)"
                        }
                    },
                    "required": ["path"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'path' argument"))?;

        match fs::read_to_string(path) {
            Ok(content) => Ok(content),
            Err(e) => Err(anyhow!("Failed to read file '{}': {}", path, e)),
        }
    }
}

pub struct WriteFileTool;

impl Tool for WriteFileTool {
    fn name(&self) -> String {
        "write_file".to_string()
    }

    fn description(&self) -> String {
        "Write content to a file. Overwrites if exists.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file. Overwrites if exists.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The path to the file to write. Defaults to current working directory. PREFER relative paths (e.g. 'file.txt'). Do not guess /home/user paths."
                        },
                        "content": {
                            "type": "string",
                            "description": "The content to write to the file"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'path' argument"))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'content' argument"))?;

        // Acquire lock before writing
        let _guard = locks::acquire_lock(path)?;

        match fs::write(path, content) {
            Ok(_) => Ok(format!("File written to {}", path)),
            Err(e) => Err(anyhow!("Failed to write file '{}': {}", path, e)),
        }
    }
}

pub struct EditFileTool;

impl Tool for EditFileTool {
    fn name(&self) -> String {
        "edit_file".to_string()
    }

    fn description(&self) -> String {
        "Replace a specific string in a file with a new string. Useful for partial edits."
            .to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "edit_file",
                "description": "Replace a specific string in a file with a new string. Useful for partial edits.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The path to the file to edit"
                        },
                        "old_string": {
                            "type": "string",
                            "description": "The exact string content to find and replace"
                        },
                        "new_string": {
                            "type": "string",
                            "description": "The new content to replace with"
                        }
                    },
                    "required": ["path", "old_string", "new_string"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'path' argument"))?;

        let old_string = args
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'old_string' argument"))?;

        let new_string = args
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'new_string' argument"))?;

        // Acquire lock before reading and writing
        let _guard = locks::acquire_lock(path)?;

        let content = fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read file '{}': {}", path, e))?;

        if !content.contains(old_string) {
            return Err(anyhow!("old_string not found in file content"));
        }

        let new_content = content.replace(old_string, new_string);

        fs::write(path, new_content)
            .map_err(|e| anyhow!("Failed to write file '{}': {}", path, e))?;

        Ok(format!("Successfully edited file {}", path))
    }
}

pub fn get_tools_for_profile(profile_name: &str, tx: Sender<AsyncMessage>) -> Vec<Box<dyn Tool>> {
    let mut tools: Vec<Box<dyn Tool>> = vec![
        Box::new(ReadFileTool),
        Box::new(WriteFileTool),
        Box::new(EditFileTool),
        Box::new(search::GlobTool),
        Box::new(todo::TodoReadTool),
        Box::new(todo::TodoWriteTool),
        Box::new(ast_grep::AstGrepTool),
        Box::new(batch::BatchTool::new(tx.clone())),
        Box::new(multiedit::MultiEditTool),
        Box::new(lsp::LspTool),
        Box::new(shell::ShellTool),
        Box::new(bevy::BevySpawnPrimitiveTool),
    ];

    if profile_name == "Bevy Editor Companion" {
        tools.push(Box::new(bevy::BevyRpcTool));
        tools.push(Box::new(bevy::BevySpawnSceneTool));
    }

    tools
}

pub fn get_all_tools(tx: Sender<AsyncMessage>) -> Vec<Box<dyn Tool>> {
    get_tools_for_profile("General", tx)
}
