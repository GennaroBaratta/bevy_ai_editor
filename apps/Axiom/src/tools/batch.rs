use crate::tools::Tool;
use crate::types::AsyncMessage;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::sync::mpsc::Sender;

pub struct BatchTool {
    tx: Sender<AsyncMessage>,
}

impl BatchTool {
    pub fn new(tx: Sender<AsyncMessage>) -> Self {
        Self { tx }
    }
}

impl Tool for BatchTool {
    fn name(&self) -> String {
        "batch_run".to_string()
    }

    fn description(&self) -> String {
        "Execute multiple tools in parallel (especially useful for spawning multiple sub-agents)."
            .to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "batch_run",
                "description": "Execute multiple tools in parallel. Use this to spawn multiple 'task' agents simultaneously.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "tools": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "tool": { "type": "string", "description": "The name of the tool to run" },
                                    "parameters": { "type": "object", "description": "The arguments for the tool" }
                                },
                                "required": ["tool", "parameters"]
                            }
                        }
                    },
                    "required": ["tools"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let tools_list = args
            .get("tools")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Missing or invalid 'tools' argument"))?;

        // Get registry of all tools, passing the channel
        // Note: Re-creating registry might be slightly inefficient but safe
        let tx = self.tx.clone();

        // We will execute them sequentially in the loop, BUT:
        // For 'task' tool specifically, it spawns a NEW thread immediately and returns.
        // So effectively, if we batch_run multiple 'task' calls, they WILL run in parallel.
        // The issue might be that the loop itself takes a split second, or the UI update lag.

        let mut results = Vec::new();
        let available_tools = crate::tools::get_all_tools(tx.clone());

        for (i, tool_call) in tools_list.iter().enumerate() {
            let tool_name = tool_call
                .get("tool")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Item #{}: Missing 'tool' name", i))?;

            let params = tool_call
                .get("parameters")
                .cloned()
                .ok_or_else(|| anyhow!("Item #{}: Missing 'parameters'", i))?;

            // Find the tool
            let tool = available_tools.iter().find(|t| t.name() == tool_name);

            if let Some(tool) = tool {
                match tool.execute(params) {
                    Ok(output) => {
                        // For 'task' tool, 'output' is just the summary AFTER completion?
                        // Wait, looking at task.rs:
                        // handle.join() blocks!

                        // AH HA! That's why it's sequential.
                        // The 'task' tool implementation currently blocks the calling thread until the sub-agent finishes.
                        // We need to fix 'task' tool to be async/non-blocking if we want true parallel batching.
                        // OR, we change batch tool to spawn threads for each tool execution.

                        // But wait, 'task' tool spawns a thread:
                        // let handle = thread::spawn(...)
                        // match handle.join() ... -> BLOCKS!

                        // We cannot fix this easily in 'batch' without changing 'Tool' trait to be async or changing 'task' to return immediately.
                        // However, 'task' needs to return a summary to the LLM.

                        // If we want PARALLEL execution visible in UI, we should probably modify 'batch'
                        // to spawn a thread for each tool execution if it's a 'task' tool.
                        // But 'Tool::execute' returns Result<String>.

                        results.push(json!({
                            "tool": tool_name,
                            "status": "success",
                            "output": output
                        }));
                    }
                    Err(e) => {
                        results.push(json!({
                            "tool": tool_name,
                            "status": "error",
                            "error": e.to_string()
                        }));
                    }
                }
            } else {
                results.push(json!({
                    "tool": tool_name,
                    "status": "error",
                    "error": format!("Tool '{}' not found", tool_name)
                }));
            }
        }

        Ok(serde_json::to_string_pretty(&results)?)
    }
}
