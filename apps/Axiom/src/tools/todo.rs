use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

use crate::tools::Tool;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: String,   // pending, in_progress, completed, cancelled
    pub priority: String, // high, medium, low
}

pub struct TodoReadTool;

impl Tool for TodoReadTool {
    fn name(&self) -> String {
        "todoread".to_string()
    }

    fn description(&self) -> String {
        "Read the current todo list.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "todoread",
                "description": "Read the current todo list.",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        })
    }

    fn execute(&self, _args: Value) -> Result<String> {
        let path = "todos.json";
        if !Path::new(path).exists() {
            return Ok("No todo list found. Use 'todowrite' to create one.".to_string());
        }

        let content =
            fs::read_to_string(path).map_err(|e| anyhow!("Failed to read todos.json: {}", e))?;

        Ok(content)
    }
}

pub struct TodoWriteTool;

impl Tool for TodoWriteTool {
    fn name(&self) -> String {
        "todowrite".to_string()
    }

    fn description(&self) -> String {
        "Overwrite the todo list with new items.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "todowrite",
                "description": "Overwrite the todo list with new items. This replaces the entire list.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "todos": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "string" },
                                    "content": { "type": "string" },
                                    "status": { "type": "string", "enum": ["pending", "in_progress", "completed", "cancelled"] },
                                    "priority": { "type": "string", "enum": ["high", "medium", "low"] }
                                },
                                "required": ["id", "content", "status", "priority"]
                            }
                        }
                    },
                    "required": ["todos"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let todos_val = args
            .get("todos")
            .ok_or_else(|| anyhow!("Missing 'todos' argument"))?;

        let todos: Vec<TodoItem> = serde_json::from_value(todos_val.clone())
            .map_err(|e| anyhow!("Invalid todo format: {}", e))?;

        let json_output = serde_json::to_string_pretty(&todos)?;
        fs::write("todos.json", json_output)?;

        let active_count = todos
            .iter()
            .filter(|t| t.status != "completed" && t.status != "cancelled")
            .count();
        Ok(format!(
            "Todo list updated. {} active tasks remaining.",
            active_count
        ))
    }
}
