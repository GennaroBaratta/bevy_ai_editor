use anyhow::{anyhow, Result};
use glob::glob;
use serde_json::{json, Value};

use crate::tools::Tool;

pub struct GlobTool;

impl Tool for GlobTool {
    fn name(&self) -> String {
        "glob".to_string()
    }

    fn description(&self) -> String {
        "Find files matching a glob pattern. Safer than 'run_command' for listing files."
            .to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "glob",
                "description": "Find files matching a glob pattern. Limit 50 results to prevent token overflow.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "The glob pattern (e.g., '**/*.rs', 'src/**/*.toml')"
                        }
                    },
                    "required": ["pattern"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'pattern' argument"))?;

        let paths: Result<Vec<_>, _> = glob(pattern)
            .map_err(|e| anyhow!("Failed to read glob pattern: {}", e))?
            .collect();

        let paths = paths.map_err(|e| anyhow!("Glob error: {}", e))?;

        // Limit results
        let limit = 50;
        let count = paths.len();
        let truncated = count > limit;

        let mut result = paths
            .into_iter()
            .take(limit)
            .map(|p| p.display().to_string())
            .collect::<Vec<String>>()
            .join("\n");

        if truncated {
            result.push_str(&format!(
                "\n\n(Truncated. Showing {} of {} results. Refine your pattern.)",
                limit, count
            ));
        } else if count == 0 {
            result = "No files found matching the pattern.".to_string();
        }

        Ok(result)
    }
}
