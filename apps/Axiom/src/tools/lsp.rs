use anyhow::{anyhow, Result};
use lsp_types::{
    ClientCapabilities, Diagnostic, InitializeParams, InitializeResult, Position,
    PublishDiagnosticsParams, ServerCapabilities, TextDocumentClientCapabilities,
    TextDocumentIdentifier, TextDocumentPositionParams, TraceValue, Uri,
    WorkspaceClientCapabilities, WorkspaceFolder,
};

use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;
use url::Url;

use crate::tools::Tool;

// --- Global LSP State ---

struct SharedLspState {
    // ID -> Response Result (or Error)
    responses: HashMap<i64, Value>,
    // URI String -> Diagnostics
    diagnostics: HashMap<String, Vec<Diagnostic>>,
}

struct LspSession {
    #[allow(dead_code)]
    process: Child,
    stdin: Option<ChildStdin>, // We take this out to write
    request_id: i64,
    initialized: bool,
    capabilities: Option<ServerCapabilities>,

    // Shared state for background reader
    shared: Arc<Mutex<SharedLspState>>,
    // Notify when a response arrives
    response_cv: Arc<Condvar>,
}

// Global mutex to hold the session
static LSP_SESSION: OnceLock<Mutex<LspSession>> = OnceLock::new();

// --- Helper Functions ---

fn get_or_init_session() -> Result<std::sync::MutexGuard<'static, LspSession>> {
    // Initialize if not present
    if LSP_SESSION.get().is_none() {
        // Try to start rust-analyzer by default
        let mut process = Command::new("rust-analyzer")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| {
                anyhow!(
                    "Failed to spawn rust-analyzer. Is it installed? Error: {}",
                    e
                )
            })?;

        let stdin = process.stdin.take();
        let stdout = process
            .stdout
            .take()
            .ok_or(anyhow!("Failed to open stdout"))?;

        let shared = Arc::new(Mutex::new(SharedLspState {
            responses: HashMap::new(),
            diagnostics: HashMap::new(),
        }));
        let response_cv = Arc::new(Condvar::new());

        // Spawn background reader thread
        let shared_clone = shared.clone();
        let cv_clone = response_cv.clone();

        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_message(&mut reader) {
                    Ok((_headers, body)) => {
                        if let Ok(val) = serde_json::from_str::<Value>(&body) {
                            let mut state = shared_clone.lock().unwrap();

                            // 1. Check for Response (has "id")
                            if let Some(id) = val.get("id").and_then(|v| v.as_i64()) {
                                state.responses.insert(id, val);
                                cv_clone.notify_all();
                            }
                            // 2. Check for Notification (no "id", has "method")
                            else if let Some(method) = val.get("method").and_then(|v| v.as_str())
                            {
                                if method == "textDocument/publishDiagnostics" {
                                    if let Ok(params) =
                                        serde_json::from_value::<PublishDiagnosticsParams>(
                                            val.get("params").cloned().unwrap_or(Value::Null),
                                        )
                                    {
                                        let uri_str = params.uri.to_string();
                                        state.diagnostics.insert(uri_str, params.diagnostics);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("LSP Reader Error: {}", e);
                        // If EOF, break
                        break;
                    }
                }
            }
        });

        let session = LspSession {
            process,
            stdin,
            request_id: 0,
            initialized: false,
            capabilities: None,
            shared,
            response_cv,
        };

        let _ = LSP_SESSION.set(Mutex::new(session));
    }

    let mut session = LSP_SESSION
        .get()
        .unwrap()
        .lock()
        .map_err(|e| anyhow!("LSP Lock poisoned: {}", e))?;

    // Perform LSP initialization handshake if not done
    if !session.initialized {
        initialize_handshake(&mut session)?;
        session.initialized = true;
    }

    Ok(session)
}

fn initialize_handshake(session: &mut LspSession) -> Result<()> {
    let root_dir = std::env::current_dir()?;
    let root_url = Url::from_directory_path(&root_dir).map_err(|_| anyhow!("Invalid root path"))?;
    let root_uri: Uri = root_url
        .as_str()
        .parse()
        .map_err(|e| anyhow!("Failed to parse URI: {}", e))?;

    let params = InitializeParams {
        process_id: Some(std::process::id()),
        #[allow(deprecated)]
        root_uri: Some(root_uri.clone()),
        capabilities: ClientCapabilities {
            text_document: Some(TextDocumentClientCapabilities {
                definition: Some(lsp_types::GotoCapability {
                    dynamic_registration: Some(false),
                    link_support: Some(true),
                }),
                publish_diagnostics: Some(lsp_types::PublishDiagnosticsClientCapabilities {
                    related_information: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            workspace: Some(WorkspaceClientCapabilities {
                workspace_folders: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        },
        trace: Some(TraceValue::Off),
        workspace_folders: Some(vec![WorkspaceFolder {
            uri: root_uri,
            name: "Axiom".into(),
        }]),
        client_info: Some(lsp_types::ClientInfo {
            name: "AxiomAgent".into(),
            version: Some("0.1.0".into()),
        }),
        ..Default::default()
    };

    let result: InitializeResult = send_request(session, "initialize", json!(params))?;
    session.capabilities = Some(result.capabilities);

    // Send initialized notification
    send_notification(session, "initialized", json!({}))?;

    Ok(())
}

fn send_request<T: serde::de::DeserializeOwned>(
    session: &mut LspSession,
    method: &str,
    params: Value,
) -> Result<T> {
    session.request_id += 1;
    let id = session.request_id;

    let request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    });

    let body = serde_json::to_string(&request)?;
    let message = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

    if let Some(stdin) = &mut session.stdin {
        stdin.write_all(message.as_bytes())?;
        stdin.flush()?;
    } else {
        return Err(anyhow!("Stdin closed"));
    }

    // Wait for response
    let mut state = session.shared.lock().unwrap();
    loop {
        if let Some(val) = state.responses.remove(&id) {
            if let Some(error) = val.get("error") {
                return Err(anyhow!("LSP Error: {:?}", error));
            }
            if let Some(result) = val.get("result") {
                return serde_json::from_value(result.clone())
                    .map_err(|e| anyhow!("Failed to parse result: {}", e));
            }
            return Ok(serde_json::from_value(Value::Null)?);
        }

        // Wait on Condvar
        state = session.response_cv.wait(state).unwrap();
    }
}

fn send_notification(session: &mut LspSession, method: &str, params: Value) -> Result<()> {
    let request = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    });

    let body = serde_json::to_string(&request)?;
    let message = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

    if let Some(stdin) = &mut session.stdin {
        stdin.write_all(message.as_bytes())?;
        stdin.flush()?;
    }
    Ok(())
}

fn read_message(
    reader: &mut BufReader<std::process::ChildStdout>,
) -> Result<(HashMap<String, String>, String)> {
    let mut headers = HashMap::new();
    let mut content_length = 0;

    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let line = line.trim();

        if line.is_empty() {
            break;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_lowercase();
            let value = value.trim();
            if key == "content-length" {
                content_length = value.parse::<usize>()?;
            }
            headers.insert(key, value.to_string());
        }
    }

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;
    let body_str = String::from_utf8(body)?;

    Ok((headers, body_str))
}

// --- Tool Implementation ---

pub struct LspTool;

impl Tool for LspTool {
    fn name(&self) -> String {
        "lsp".to_string()
    }

    fn description(&self) -> String {
        "Advanced code intelligence tool (LSP). Supports diagnostics, definition, and references."
            .to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "lsp",
                "description": "Interact with Language Server Protocol (LSP). currently defaults to rust-analyzer.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "enum": ["definition", "references", "diagnostics"],
                            "description": "The LSP command to execute."
                        },
                        "path": {
                            "type": "string",
                            "description": "File path (absolute or relative)."
                        },
                        "line": {
                            "type": "integer",
                            "description": "Line number (0-based). Required for definition/references."
                        },
                        "character": {
                            "type": "integer",
                            "description": "Character/Column number (0-based). Required for definition/references."
                        }
                    },
                    "required": ["command", "path"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or(anyhow!("Missing command"))?;
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow!("Missing path"))?;

        // Optional coords
        let line = args.get("line").and_then(|v| v.as_u64()).map(|v| v as u32);
        let character = args
            .get("character")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let root_dir = std::env::current_dir()?;
        let abs_path = if Path::new(path_str).is_absolute() {
            std::path::PathBuf::from(path_str)
        } else {
            root_dir.join(path_str)
        };

        let uri_url =
            Url::from_file_path(&abs_path).map_err(|_| anyhow!("Invalid file path for URL"))?;
        let uri: Uri = uri_url
            .as_str()
            .parse()
            .map_err(|e| anyhow!("Failed to parse URI: {}", e))?;

        let mut session = get_or_init_session()?;

        // Ensure we open the document (fake didOpen) to trigger analysis if needed
        // Ideally we track open files, but sending didOpen redundantly usually is fine or ignored if already open.
        // For efficiency, we can skip if we know it's open, but for now let's just send it if it's a new path.
        // Note: rust-analyzer usually scans disk, but didOpen is safer for diagnostics.
        if let Ok(text) = std::fs::read_to_string(&abs_path) {
            let _ = send_notification(
                &mut session,
                "textDocument/didOpen",
                json!({
                    "textDocument": {
                        "uri": uri,
                        "languageId": "rust", // Assume rust for now
                        "version": 1,
                        "text": text
                    }
                }),
            );
        }

        match command {
            "diagnostics" => {
                // Wait a bit for diagnostics to arrive?
                // Diagnostics are pushed async. We just return what we have.
                // To be safe, we might want to sleep briefly if we just opened the file.
                // thread::sleep(std::time::Duration::from_millis(500));

                let state = session.shared.lock().unwrap();
                let uri_str = uri.to_string();

                if let Some(diags) = state.diagnostics.get(&uri_str) {
                    if diags.is_empty() {
                        return Ok("No diagnostics (errors/warnings) found.".to_string());
                    }
                    let mut out = String::new();
                    for d in diags {
                        out.push_str(&format!(
                            "[{:?}] Line {}: {}\n",
                            d.severity
                                .unwrap_or(lsp_types::DiagnosticSeverity::INFORMATION),
                            d.range.start.line + 1, // 1-based for humans
                            d.message
                        ));
                    }
                    Ok(out)
                } else {
                    Ok("No diagnostics info available yet.".to_string())
                }
            }
            "definition" => {
                let line = line.ok_or(anyhow!("Missing line for definition"))?;
                let character = character.ok_or(anyhow!("Missing character for definition"))?;

                let params = TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position { line, character },
                };
                let result: Option<lsp_types::GotoDefinitionResponse> =
                    send_request(&mut session, "textDocument/definition", json!(params))?;

                match result {
                    Some(lsp_types::GotoDefinitionResponse::Scalar(loc)) => Ok(format!(
                        "Definition at: {}:{}",
                        loc.uri.path(),
                        loc.range.start.line + 1
                    )),
                    Some(lsp_types::GotoDefinitionResponse::Array(locs)) => {
                        let info: Vec<String> = locs
                            .iter()
                            .map(|l| format!("{}:{}", l.uri.path(), l.range.start.line + 1))
                            .collect();
                        Ok(format!("Definitions found:\n{}", info.join("\n")))
                    }
                    Some(lsp_types::GotoDefinitionResponse::Link(links)) => {
                        let info: Vec<String> = links
                            .iter()
                            .map(|l| {
                                format!("{}:{}", l.target_uri.path(), l.target_range.start.line + 1)
                            })
                            .collect();
                        Ok(format!("Definitions found (links):\n{}", info.join("\n")))
                    }
                    None => Ok("No definition found.".to_string()),
                }
            }
            "references" => {
                let line = line.ok_or(anyhow!("Missing line for references"))?;
                let character = character.ok_or(anyhow!("Missing character for references"))?;

                let params = lsp_types::ReferenceParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri },
                        position: Position { line, character },
                    },
                    work_done_progress_params: Default::default(),
                    partial_result_params: Default::default(),
                    context: lsp_types::ReferenceContext {
                        include_declaration: true,
                    },
                };
                let result: Option<Vec<lsp_types::Location>> =
                    send_request(&mut session, "textDocument/references", json!(params))?;

                if let Some(locs) = result {
                    let info: Vec<String> = locs
                        .iter()
                        .map(|l| format!("{}:{}", l.uri.path(), l.range.start.line + 1))
                        .collect();
                    Ok(format!(
                        "Found {} references:\n{}",
                        locs.len(),
                        info.join("\n")
                    ))
                } else {
                    Ok("No references found.".to_string())
                }
            }
            _ => Err(anyhow!("Unknown LSP command: {}", command)),
        }
    }
}
