use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use crate::tools::Tool;

// Global persistent state for the shell
static SHELL_STATE: OnceLock<Mutex<ShellState>> = OnceLock::new();

struct ShellState {
    cwd: PathBuf,
    env_vars: HashMap<String, String>,
}

impl ShellState {
    fn new() -> Self {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let env_vars = env::vars().collect();
        Self { cwd, env_vars }
    }
}

// Helper to get the global state
fn get_state() -> &'static Mutex<ShellState> {
    SHELL_STATE.get_or_init(|| Mutex::new(ShellState::new()))
}

pub struct ShellTool;

impl Tool for ShellTool {
    fn name(&self) -> String {
        "run_command".to_string()
    }

    fn description(&self) -> String {
        "Executes shell commands in a persistent session. Maintains current working directory and environment variables across calls. IMPORTANT: To change directory, run 'cd path' as a stand-alone command. 'cd' inside a chain (e.g. 'mkdir foo && cd foo') will NOT persist.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "run_command",
                "description": "Executes shell commands in a persistent session.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to execute (e.g., 'ls -la', 'cd ./src', 'export VAR=value')."
                        }
                    },
                    "required": ["command"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let command_str = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'command' argument"))?;

        let command_str = command_str.trim();

        // Access global state
        let mut state = get_state()
            .lock()
            .map_err(|e| anyhow!("Failed to lock shell state: {}", e))?;

        // 1. Handle 'cd' command internally
        if command_str.starts_with("cd ") || command_str == "cd" {
            let path_str = if command_str == "cd" {
                "~"
            } else {
                command_str.strip_prefix("cd ").unwrap().trim()
            };

            // Handle ~ or empty as home (simplification, though explicit path is preferred by agents)
            // But usually agents will pass explicit paths.
            // Let's support relative and absolute paths.

            let new_path = if path_str == "~" {
                dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?
            } else {
                // Resolve relative path against current cwd
                let path = Path::new(path_str);
                if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    state.cwd.join(path)
                }
            };

            // Canonicalize to resolve .. and symlinks, and check existence
            match new_path.canonicalize() {
                Ok(path) => {
                    if path.is_dir() {
                        state.cwd = path;
                        return Ok(format!("Changed directory to: {}", state.cwd.display()));
                    } else {
                        return Err(anyhow!("Path is not a directory: {}", new_path.display()));
                    }
                }
                Err(e) => {
                    // Try to see if it works without canonicalize (sometimes needed for new dirs not yet fully flushed or other issues, but generally safe to require existence)
                    // But canonicalize fails if file doesn't exist.
                    return Err(anyhow!(
                        "Directory not found: {} ({})",
                        new_path.display(),
                        e
                    ));
                }
            }
        }

        // 2. Execute other commands
        let output_result = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", command_str])
                .current_dir(&state.cwd)
                .envs(&state.env_vars)
                .output()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(command_str)
                .current_dir(&state.cwd)
                .envs(&state.env_vars)
                .output()
        };

        match output_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                // Naive environment variable capture for export commands
                // Real persistence for 'export' in a shell session usually requires sourcing or parsing output.
                // Since `sh -c` runs in a subshell, `export` won't stick unless we do something clever.
                // For now, the requirement says "Maintain ... environment variables".
                // If the user runs "export FOO=bar", it won't persist because the child process exits.
                // TO DO: To make env vars persist, we might need to parse them from the command or run a wrapper that outputs env.
                // But for a simple "ShellTool" as requested, usually `cd` persistence is the main one.
                // If "environment variables" state is strictly required to be updatable via command, we'd need to parse exports.
                // Let's stick to the requirement "Maintain ... env_vars" which implies we keep the map.
                // But how does the user UPDATE it?
                // The requirements say: "Initialize `cwd`... `env_vars` (HashMap)... Inject environment variables..."
                // It doesn't explicitly say "Capture new env vars from export commands".
                // However, "Maintain a global/shared state... for the shell session" usually implies stateful env changes.
                //
                // Let's try to parse simple `export KEY=VALUE` lines if possible?
                // Or maybe just sticking to the base requirement of maintaining the map (which starts with system envs)
                // and maybe allowing manual updates if the command is purely `export K=V`.
                //
                // Given the constraints and simplicity, I'll implement a basic heuristic:
                // If command starts with "export " (Linux) or "set " (Windows), try to parse it.
                // But complex commands like "build && export A=1" won't work easily.
                //
                // Let's re-read carefully: "Maintain a global/shared state... State includes... env_vars".
                // "Input Schema: command".
                // "Execution Logic: ... Inject environment variables...".
                // It doesn't explicitly say "UPDATE env vars from command".
                // But `cd` is explicitly mentioned.
                // I will add a basic handler for `export/set` to be helpful, strictly for top-level commands.

                if output.status.success() {
                    // Try to update env vars if it was a simple export/set command
                    update_env_from_command(command_str, &mut state);
                }

                if output.status.success() {
                    Ok(stdout.to_string())
                } else {
                    Ok(format!(
                        "Command failed with status: {}\nStdout: {}\nStderr: {}",
                        output.status, stdout, stderr
                    ))
                }
            }
            Err(e) => Err(anyhow!("Failed to execute command: {}", e)),
        }
    }
}

// Simple heuristic to update env vars for straightforward commands.
// Not a full shell parser.
fn update_env_from_command(command: &str, state: &mut ShellState) {
    // Linux/Mac: export KEY=VALUE
    if command.starts_with("export ") {
        let remainder = command.trim_start_matches("export ").trim();
        if let Some((key, value)) = remainder.split_once('=') {
            let key = key.trim().to_string();
            // Remove surrounding quotes if present
            let value = value.trim();
            let value = if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                &value[1..value.len() - 1]
            } else {
                value
            };
            state.env_vars.insert(key, value.to_string());
        }
    }
    // Windows: set KEY=VALUE
    else if cfg!(target_os = "windows") && command.to_lowercase().starts_with("set ") {
        let remainder = command[4..].trim(); // skip "set "
        if let Some((key, value)) = remainder.split_once('=') {
            let key = key.trim().to_string();
            state.env_vars.insert(key, value.trim().to_string());
        }
    }
}
