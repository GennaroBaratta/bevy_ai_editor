use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters, ServerHandler},
    model::*,
    tool, tool_handler, tool_router, transport, ErrorData as McpError, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio::{
    fs::OpenOptions,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::{oneshot, Mutex, Notify},
    task::JoinHandle,
    time::{sleep, timeout},
};

const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(5);
const ATTACH_TIMEOUT: Duration = Duration::from_secs(10);
const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const CONFIGURATION_DONE_TIMEOUT: Duration = Duration::from_secs(5);
const INITIALIZED_EVENT_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const WAIT_FOR_STOPPED_TIMEOUT: Duration = Duration::from_secs(10);
const STOPPED_POLL_INTERVAL: Duration = Duration::from_millis(50);
const OUTPUT_EVENT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const OUTPUT_EVENT_WAIT_TIMEOUT: Duration = Duration::from_millis(300);
const MAX_RECENT_OUTPUT_EVENTS: usize = 1024;
const READ_MEMORY_MAX_COUNT: u32 = 64 * 1024;
const AXIOM_DEBUG_PROBE_SNAPSHOT_CAPACITY: usize = 4096;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DebuggerAttachParams {
    pid: u32,
    #[serde(default)]
    program: Option<String>,
    #[serde(default)]
    adapter_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DebuggerDetachParams {
    #[serde(default)]
    terminate_debuggee: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BreakpointSpec {
    line: u32,
    #[serde(default)]
    column: Option<u32>,
    #[serde(default)]
    condition: Option<String>,
    #[serde(default)]
    hit_condition: Option<String>,
    #[serde(default)]
    log_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DebuggerSetBreakpointsParams {
    source_path: String,
    breakpoints: Vec<BreakpointSpec>,
    #[serde(default)]
    function_breakpoints: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DebuggerContinueParams {
    #[serde(default)]
    thread_id: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DebuggerStepOverParams {
    #[serde(default)]
    thread_id: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DebuggerStepInParams {
    #[serde(default)]
    thread_id: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DebuggerStepOutParams {
    #[serde(default)]
    thread_id: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DebuggerVariablesParams {
    variables_reference: u64,
    #[serde(default)]
    start: Option<u32>,
    #[serde(default)]
    count: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DebuggerEvaluateParams {
    expression: String,
    #[serde(default)]
    frame_id: Option<u64>,
    #[serde(default)]
    context: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DebuggerReadMemoryParams {
    memory_reference: String,
    #[serde(default)]
    offset: i64,
    count: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DebuggerConsoleParams {
    command: String,
    #[serde(default)]
    frame_id: Option<u64>,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    arguments: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BevyDebugSnapshotParams {
    #[serde(default = "default_true")]
    include_entities: bool,
    #[serde(default = "default_true")]
    include_components: bool,
    #[serde(default)]
    include_resources: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum SessionState {
    Detached,
    Attached,
}

struct AuditLogger {
    path: PathBuf,
    file: Mutex<tokio::fs::File>,
}

impl AuditLogger {
    async fn new(pid: u32) -> Result<Self, String> {
        let evidence_dir = PathBuf::from(".sisyphus/evidence");
        tokio::fs::create_dir_all(&evidence_dir)
            .await
            .map_err(|e| format!("Failed to create evidence directory: {e}"))?;

        let ts = timestamp_millis();
        let filename = format!("dap_session_{pid}_{ts}.jsonl");
        let path = evidence_dir.join(filename);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| format!("Failed to open audit log file: {e}"))?;

        Ok(Self {
            path,
            file: Mutex::new(file),
        })
    }

    async fn log(&self, direction: &str, payload: &Value) -> Result<(), String> {
        let kind = classify_dap_message(payload);
        let envelope = json!({
            "ts_ms": timestamp_millis(),
            "direction": direction,
            "kind": kind,
            "payload": payload,
        });
        let mut line = serde_json::to_vec(&envelope)
            .map_err(|e| format!("Failed to serialize audit line: {e}"))?;
        line.push(b'\n');

        let mut file = self.file.lock().await;
        file.write_all(&line)
            .await
            .map_err(|e| format!("Failed to write audit log line: {e}"))?;
        file.flush()
            .await
            .map_err(|e| format!("Failed to flush audit log file: {e}"))?;

        Ok(())
    }
}

struct DapSession {
    child: Child,
    writer: Arc<Mutex<ChildStdin>>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    last_stopped_event: Arc<Mutex<Option<Value>>>,
    stopped_seq: Arc<AtomicU64>,
    recent_output_events: Arc<Mutex<VecDeque<(u64, String)>>>,
    initialized_seen: Arc<Mutex<bool>>,
    initialized_notify: Arc<Notify>,
    next_seq: u64,
    attached_pid: u32,
    configuration_done_sent: bool,
    reader_task: JoinHandle<()>,
    audit: Arc<AuditLogger>,
}

impl DapSession {
    async fn send_request_begin(
        &mut self,
        command: &str,
        arguments: Value,
    ) -> Result<(u64, oneshot::Receiver<Value>), String> {
        self.next_seq += 1;
        let seq = self.next_seq;
        let request = json!({
            "seq": seq,
            "type": "request",
            "command": command,
            "arguments": arguments,
        });

        self.audit.log("outbound", &request).await?;

        let body = serde_json::to_string(&request)
            .map_err(|e| format!("Failed to encode DAP request for {command}: {e}"))?;
        let framed = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(seq, tx);
        }

        {
            let mut writer = self.writer.lock().await;
            if let Err(e) = writer.write_all(framed.as_bytes()).await {
                let mut pending = self.pending.lock().await;
                pending.remove(&seq);
                return Err(format!(
                    "Failed to send DAP request '{command}' to adapter stdin: {e}"
                ));
            }
            if let Err(e) = writer.flush().await {
                let mut pending = self.pending.lock().await;
                pending.remove(&seq);
                return Err(format!(
                    "Failed to flush DAP request '{command}' to adapter stdin: {e}"
                ));
            }
        }

        Ok((seq, rx))
    }

    async fn await_response(
        &self,
        command: &str,
        seq: u64,
        rx: oneshot::Receiver<Value>,
        wait_timeout: Duration,
    ) -> Result<Value, String> {

        let response = match timeout(wait_timeout, rx).await {
            Ok(Ok(value)) => value,
            Ok(Err(_)) => {
                return Err(format!(
                    "Adapter response channel closed while waiting for '{command}'"
                ));
            }
            Err(_) => {
                let mut pending = self.pending.lock().await;
                pending.remove(&seq);
                return Err(format!(
                    "Timeout while waiting for DAP response to '{command}'"
                ));
            }
        };

        let success = response
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        if !success {
            let message = response
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown adapter error");
            return Err(format!("DAP request '{command}' failed: {message}"));
        }

        Ok(response)
    }

    async fn send_request(
        &mut self,
        command: &str,
        arguments: Value,
        wait_timeout: Duration,
    ) -> Result<Value, String> {
        let (seq, rx) = self.send_request_begin(command, arguments).await?;
        self.await_response(command, seq, rx, wait_timeout).await
    }

    async fn shutdown(mut self) {
        self.reader_task.abort();
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }

    async fn stop_info(&self) -> Option<Value> {
        let stopped = self.last_stopped_event.lock().await;
        stopped.as_ref().map(stopped_summary)
    }

    async fn wait_for_stopped_event_after_seq(
        &self,
        before_seq: u64,
        wait_timeout: Duration,
    ) -> Result<Value, String> {
        wait_for_stopped_event_after_seq(
            &self.last_stopped_event,
            &self.stopped_seq,
            before_seq,
            wait_timeout,
        )
        .await
    }

    async fn wait_for_initialized_event(&self, wait_timeout: Duration) -> bool {
        {
            let initialized = self.initialized_seen.lock().await;
            if *initialized {
                return true;
            }
        }

        let notified = self.initialized_notify.notified();
        let _ = timeout(wait_timeout, notified).await;

        let initialized = self.initialized_seen.lock().await;
        *initialized
    }
}

struct SessionManager {
    state: SessionState,
    session: Option<DapSession>,
}

impl SessionManager {
    fn new() -> Self {
        Self {
            state: SessionState::Detached,
            session: None,
        }
    }
}

#[derive(Clone)]
struct DebuggerMcpServer {
    tool_router: ToolRouter<Self>,
    session: Arc<Mutex<SessionManager>>,
}

async fn reader_loop(
    stdout: ChildStdout,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    audit: Arc<AuditLogger>,
    last_stopped_event: Arc<Mutex<Option<Value>>>,
    stopped_seq: Arc<AtomicU64>,
    recent_output_events: Arc<Mutex<VecDeque<(u64, String)>>>,
    initialized_seen: Arc<Mutex<bool>>,
    initialized_notify: Arc<Notify>,
) {
    let mut reader = BufReader::new(stdout);
    let mut output_event_seq = 0_u64;
    loop {
        let message = match read_dap_message(&mut reader).await {
            Ok(value) => value,
            Err(e) => {
                let _ = audit
                    .log(
                        "internal",
                        &json!({
                            "type": "reader_error",
                            "message": e.to_string(),
                        }),
                    )
                    .await;
                break;
            }
        };

        let _ = audit.log("inbound", &message).await;

        match message.get("type").and_then(Value::as_str) {
            Some("response") => {
                if let Some(request_seq) = message.get("request_seq").and_then(Value::as_u64) {
                    let mut pending_map = pending.lock().await;
                    if let Some(tx) = pending_map.remove(&request_seq) {
                        let _ = tx.send(message);
                    }
                }
            }
            Some("event") => {
                let event_name = message
                    .get("event")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if event_name == "stopped" {
                    let mut stopped = last_stopped_event.lock().await;
                    *stopped = Some(message);
                    stopped_seq.fetch_add(1, Ordering::SeqCst);
                } else if event_name == "output" {
                    if let Some(output) = message
                        .get("body")
                        .and_then(Value::as_object)
                        .and_then(|body| body.get("output"))
                        .and_then(Value::as_str)
                    {
                        let mut events = recent_output_events.lock().await;
                        let seq = output_event_seq;
                        output_event_seq = output_event_seq.saturating_add(1);
                        push_recent_output_event(&mut events, seq, output.to_string());
                    }
                } else if event_name == "initialized" {
                    {
                        let mut initialized = initialized_seen.lock().await;
                        *initialized = true;
                    }
                    initialized_notify.notify_waiters();
                }
            }
            _ => {}
        }
    }
}

fn push_recent_output_event(events: &mut VecDeque<(u64, String)>, seq: u64, output: String) {
    events.push_back((seq, output));
    while events.len() > MAX_RECENT_OUTPUT_EVENTS {
        events.pop_front();
    }
}

async fn read_dap_message(reader: &mut BufReader<ChildStdout>) -> std::io::Result<Value> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "adapter stdout closed while reading DAP headers",
            ));
        }

        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
        if trimmed.is_empty() {
            break;
        }

        if let Some((key, value)) = trimmed.split_once(':') {
            if key.trim().eq_ignore_ascii_case("content-length") {
                let parsed = value.trim().parse::<usize>().map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid Content-Length value: {e}"),
                    )
                })?;
                content_length = Some(parsed);
            }
        }
    }

    let length = content_length.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "missing required Content-Length header",
        )
    })?;

    let mut body = vec![0_u8; length];
    reader.read_exact(&mut body).await?;
    serde_json::from_slice::<Value>(&body).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid DAP JSON payload: {e}"),
        )
    })
}

fn classify_dap_message(payload: &Value) -> &'static str {
    match payload.get("type").and_then(Value::as_str) {
        Some("request") => "request",
        Some("response") => "response",
        Some("event") => "event",
        _ => "other",
    }
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn to_mcp_error(message: impl Into<String>) -> McpError {
    McpError::internal_error(message.into(), None)
}

fn map_attach_error(msg: String) -> String {
    let lower = msg.to_lowercase();
    if lower.contains("eperm")
        || lower.contains("ptrace")
        || lower.contains("operation not permitted")
    {
        return format!(
            "Attach failed: ptrace permission denied (EPERM). Check ptrace scope/container privileges. Adapter error: {msg}"
        );
    }
    msg
}

fn detached_session_error(tool_name: &str) -> McpError {
    to_mcp_error(format!(
        "{tool_name} requires an attached debugger session. Call debugger_attach first."
    ))
}

fn stopped_summary(stopped_event: &Value) -> Value {
    let body = stopped_event
        .get("body")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    json!({
        "reason": body.get("reason").and_then(Value::as_str),
        "description": body.get("description").and_then(Value::as_str),
        "text": body.get("text").and_then(Value::as_str),
        "thread_id": body.get("threadId").and_then(Value::as_u64),
        "all_threads_stopped": body.get("allThreadsStopped").and_then(Value::as_bool),
        "hit_breakpoint_ids": body.get("hitBreakpointIds"),
    })
}

fn resolved_state(stopped: &Option<Value>) -> &'static str {
    if stopped.is_some() {
        "stopped"
    } else {
        "running"
    }
}

fn snapshot_unsupported(reason: impl Into<String>, stopped_event: Option<&Value>) -> CallToolResult {
    let stop = stopped_event.map(stopped_summary).unwrap_or(Value::Null);
    CallToolResult::structured(json!({
        "ok": true,
        "supported": false,
        "reason": reason.into(),
        "stop": stop,
    }))
}

fn parse_hex_address(input: &str) -> Option<String> {
    let start = input.find("0x")?;
    let hex = input[start + 2..]
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect::<String>();
    if hex.is_empty() {
        return None;
    }
    Some(format!("0x{hex}"))
}

fn parse_hex_address_from_output_event(message: &Value) -> Option<String> {
    let output = message
        .get("body")
        .and_then(Value::as_object)
        .and_then(|body| body.get("output"))
        .and_then(Value::as_str)?;
    parse_hex_address(output)
}

async fn wait_for_output_event_address(
    recent_output_events: &Arc<Mutex<VecDeque<(u64, String)>>>,
    start_seq: u64,
    wait_timeout: Duration,
) -> Option<String> {
    let started_at = Instant::now();
    loop {
        {
            let events = recent_output_events.lock().await;
            for (seq, output) in events.iter() {
                if *seq < start_seq {
                    continue;
                }
                if let Some(address) = parse_hex_address(output) {
                    return Some(address);
                }
            }
        }

        if started_at.elapsed() >= wait_timeout {
            return None;
        }

        sleep(OUTPUT_EVENT_POLL_INTERVAL).await;
    }
}

async fn wait_for_stopped_event_after_seq(
    last_stopped_event: &Arc<Mutex<Option<Value>>>,
    stopped_seq: &Arc<AtomicU64>,
    before_seq: u64,
    wait_timeout: Duration,
) -> Result<Value, String> {
    let started_at = Instant::now();
    loop {
        if stopped_seq.load(Ordering::SeqCst) > before_seq {
            let stopped = last_stopped_event.lock().await;
            if let Some(event) = &*stopped {
                return Ok(event.clone());
            }
        }

        if started_at.elapsed() >= wait_timeout {
            return Err("Timed out waiting for next DAP 'stopped' event".to_string());
        }

        sleep(STOPPED_POLL_INTERVAL).await;
    }
}

fn read_u64_le(bytes: &[u8]) -> Result<u64, String> {
    if bytes.len() < 8 {
        return Err(format!(
            "Expected at least 8 bytes, received {} bytes",
            bytes.len()
        ));
    }
    let mut array = [0_u8; 8];
    array.copy_from_slice(&bytes[..8]);
    Ok(u64::from_le_bytes(array))
}

fn read_memory_data_bytes(read_memory_response: &Value, expected_min_len: usize) -> Result<Vec<u8>, String> {
    let body = read_memory_response
        .get("body")
        .and_then(Value::as_object)
        .ok_or_else(|| "readMemory response missing object body".to_string())?;

    let unreadable = body
        .get("unreadableBytes")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if unreadable > 0 {
        return Err(format!(
            "readMemory returned unreadableBytes={unreadable}"
        ));
    }

    let encoded = body
        .get("data")
        .and_then(Value::as_str)
        .ok_or_else(|| "readMemory response missing base64 data".to_string())?;

    let bytes = BASE64_STANDARD
        .decode(encoded)
        .map_err(|e| format!("Failed to decode readMemory base64 data: {e}"))?;

    if bytes.len() < expected_min_len {
        return Err(format!(
            "Decoded memory payload is too short: expected at least {expected_min_len} bytes, got {}",
            bytes.len()
        ));
    }

    Ok(bytes)
}

async fn resolve_thread_id(
    session: &DapSession,
    explicit_thread_id: Option<u64>,
) -> Result<u64, String> {
    if let Some(thread_id) = explicit_thread_id {
        return Ok(thread_id);
    }

    let stopped = session.last_stopped_event.lock().await;
    stopped
        .as_ref()
        .and_then(|event| event.get("body"))
        .and_then(|body| body.get("threadId"))
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            "Missing threadId: provide thread_id or wait for a stopped event with threadId"
                .to_string()
        })
}

async fn ensure_configuration_done(session: &mut DapSession) -> Result<bool, String> {
    if session.configuration_done_sent {
        return Ok(false);
    }

    session
        .send_request("configurationDone", json!({}), CONFIGURATION_DONE_TIMEOUT)
        .await?;
    session.configuration_done_sent = true;
    Ok(true)
}

async fn perform_step_with_stop_restore(
    session: &mut DapSession,
    command: &str,
    thread_id: u64,
) -> Result<Value, McpError> {
    let before_seq = session.stopped_seq.load(Ordering::SeqCst);

    session
        .send_request(command, json!({ "threadId": thread_id }), ATTACH_TIMEOUT)
        .await
        .map_err(to_mcp_error)?;

    session
        .wait_for_stopped_event_after_seq(before_seq, WAIT_FOR_STOPPED_TIMEOUT)
        .await
        .map_err(to_mcp_error)
}

fn initialize_args() -> Value {
    json!({
        "adapterID": "codelldb",
        "clientID": "debugger_mcp_server",
        "clientName": "debugger_mcp_server",
        "locale": "en-US",
        "pathFormat": "path",
        "linesStartAt1": true,
        "columnsStartAt1": true,
        "supportsVariableType": true,
        "supportsVariablePaging": true,
        "supportsRunInTerminalRequest": false,
    })
}

fn attach_args(pid: u32, program: Option<String>) -> Value {
    let mut args = Map::new();
    args.insert("pid".to_string(), json!(pid));
    args.insert("stopOnEntry".to_string(), json!(true));
    args.insert("sourceLanguages".to_string(), json!(["rust"]));
    if let Some(program) = program {
        args.insert("program".to_string(), json!(program));
    }
    Value::Object(args)
}

fn probe_adapter_startup(child: &mut Child) -> Result<Option<std::process::ExitStatus>, std::io::Error> {
    child.try_wait()
}

#[tool_router]
impl DebuggerMcpServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            session: Arc::new(Mutex::new(SessionManager::new())),
        }
    }

    #[tool(description = "Attach debugger session to a target runtime")]
    async fn debugger_attach(
        &self,
        params: Parameters<DebuggerAttachParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let mut manager = self.session.lock().await;

        if manager.session.is_some() {
            return Err(to_mcp_error(
                "A debugger session is already attached. Detach before attaching again.",
            ));
        }

        let adapter_path = params
            .adapter_path
            .clone()
            .or_else(|| std::env::var("CODELLDB_ADAPTER_PATH").ok())
            .ok_or_else(|| {
                to_mcp_error(
                    "Missing CodeLLDB adapter path. Set CODELLDB_ADAPTER_PATH or pass adapter_path.",
                )
            })?;

        let mut child = Command::new(&adapter_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| {
                to_mcp_error(format!(
                    "Failed to spawn CodeLLDB adapter at '{adapter_path}': {e}"
                ))
            })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            to_mcp_error("Adapter spawn failed: missing stdin pipe for CodeLLDB process")
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            to_mcp_error("Adapter spawn failed: missing stdout pipe for CodeLLDB process")
        })?;

        let pending = Arc::new(Mutex::new(HashMap::new()));
        let last_stopped_event = Arc::new(Mutex::new(None));
        let stopped_seq = Arc::new(AtomicU64::new(0));
        let recent_output_events = Arc::new(Mutex::new(VecDeque::new()));
        let initialized_seen = Arc::new(Mutex::new(false));
        let initialized_notify = Arc::new(Notify::new());
        let audit = Arc::new(AuditLogger::new(params.pid).await.map_err(to_mcp_error)?);
        let reader_task = tokio::spawn(reader_loop(
            stdout,
            pending.clone(),
            audit.clone(),
            last_stopped_event.clone(),
            stopped_seq.clone(),
            recent_output_events.clone(),
            initialized_seen.clone(),
            initialized_notify.clone(),
        ));

        let mut session = DapSession {
            child,
            writer: Arc::new(Mutex::new(stdin)),
            pending,
            last_stopped_event,
            stopped_seq,
            recent_output_events,
            initialized_seen,
            initialized_notify,
            next_seq: 0,
            attached_pid: params.pid,
            configuration_done_sent: false,
            reader_task,
            audit: audit.clone(),
        };

        match probe_adapter_startup(&mut session.child) {
            Ok(Some(status)) => {
                session.shutdown().await;
                return Err(to_mcp_error(format!(
                    "CodeLLDB adapter exited during startup with status: {status}"
                )));
            }
            Ok(None) => {
                let _ = session
                    .audit
                    .log(
                        "internal",
                        &json!({"type": "startup", "message": "adapter process running"}),
                    )
                    .await;
            }
            Err(e) => {
                session.shutdown().await;
                return Err(to_mcp_error(format!(
                    "Failed while probing adapter startup state: {e}"
                )));
            }
        }

        let init_result = session
            .send_request("initialize", initialize_args(), INITIALIZE_TIMEOUT)
            .await;
        if let Err(e) = init_result {
            session.shutdown().await;
            return Err(to_mcp_error(format!(
                "Failed DAP initialize handshake with adapter: {e}"
            )));
        }

        let (attach_seq, attach_rx) = match session
            .send_request_begin("attach", attach_args(params.pid, params.program.clone()))
            .await
        {
            Ok(value) => value,
            Err(e) => {
                session.shutdown().await;
                return Err(to_mcp_error(map_attach_error(e)));
            }
        };

        if !session
            .wait_for_initialized_event(INITIALIZED_EVENT_WAIT_TIMEOUT)
            .await
        {
            let _ = session
                .audit
                .log(
                    "internal",
                    &json!({
                        "type": "initialized_wait_timeout",
                        "message": "Timed out waiting for DAP initialized event before configurationDone",
                    }),
                )
                .await;
        }

        if let Err(e) = ensure_configuration_done(&mut session).await {
            session.shutdown().await;
            return Err(to_mcp_error(format!(
                "Failed to send DAP configurationDone during attach: {e}"
            )));
        }

        let attach_result = session
            .await_response("attach", attach_seq, attach_rx, ATTACH_TIMEOUT)
            .await;
        if let Err(e) = attach_result {
            session.shutdown().await;
            return Err(to_mcp_error(map_attach_error(e)));
        }

        manager.state = SessionState::Attached;
        let log_path = session.audit.path.to_string_lossy().to_string();
        let pid = session.attached_pid;
        manager.session = Some(session);

        Ok(CallToolResult::structured(json!({
            "ok": true,
            "state": "attached",
            "pid": pid,
            "log_path": log_path,
        })))
    }

    #[tool(description = "Detach current debugger session")]
    async fn debugger_detach(
        &self,
        params: Parameters<DebuggerDetachParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let mut manager = self.session.lock().await;

        let Some(mut session) = manager.session.take() else {
            manager.state = SessionState::Detached;
            return Ok(CallToolResult::structured(json!({
                "ok": true,
                "state": "detached",
            })));
        };

        let disconnect_result = session
            .send_request(
                "disconnect",
                json!({
                    "terminateDebuggee": params.terminate_debuggee,
                }),
                DISCONNECT_TIMEOUT,
            )
            .await;

        session.shutdown().await;
        manager.state = SessionState::Detached;

        if let Err(e) = disconnect_result {
            return Err(to_mcp_error(format!(
                "Detach failed while sending DAP disconnect: {e}"
            )));
        }

        Ok(CallToolResult::structured(json!({
            "ok": true,
            "state": "detached",
        })))
    }

    #[tool(description = "Set source breakpoints for a file")]
    async fn debugger_set_breakpoints(
        &self,
        params: Parameters<DebuggerSetBreakpointsParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let mut manager = self.session.lock().await;
        let Some(session) = manager.session.as_mut() else {
            return Err(detached_session_error("debugger_set_breakpoints"));
        };

        let source_breakpoints: Vec<Value> = params
            .breakpoints
            .iter()
            .map(|bp| {
                let mut mapped = Map::new();
                mapped.insert("line".to_string(), json!(bp.line));
                if let Some(column) = bp.column {
                    mapped.insert("column".to_string(), json!(column));
                }
                if let Some(condition) = &bp.condition {
                    mapped.insert("condition".to_string(), json!(condition));
                }
                if let Some(hit_condition) = &bp.hit_condition {
                    mapped.insert("hitCondition".to_string(), json!(hit_condition));
                }
                if let Some(log_message) = &bp.log_message {
                    mapped.insert("logMessage".to_string(), json!(log_message));
                }
                Value::Object(mapped)
            })
            .collect();

        let source_response = session
            .send_request(
                "setBreakpoints",
                json!({
                    "source": { "path": params.source_path },
                    "breakpoints": source_breakpoints,
                }),
                ATTACH_TIMEOUT,
            )
            .await
            .map_err(to_mcp_error)?;

        let fbp: Vec<Value> = params
            .function_breakpoints
            .iter()
            .map(|name| json!({ "name": name }))
            .collect();
        let function_response = session
            .send_request(
                "setFunctionBreakpoints",
                json!({ "breakpoints": fbp }),
                ATTACH_TIMEOUT,
            )
            .await
            .map_err(to_mcp_error)?;

        let configuration_done_sent_now = ensure_configuration_done(session)
            .await
            .map_err(to_mcp_error)?;
        let stop_info = session.stop_info().await;

        Ok(CallToolResult::structured(json!({
            "ok": true,
            "state": resolved_state(&stop_info),
            "stop": stop_info,
            "configuration_done_sent": configuration_done_sent_now,
            "source_breakpoints": source_response.get("body").and_then(|b| b.get("breakpoints")).cloned().unwrap_or_else(|| json!([])),
            "function_breakpoints": function_response
                .get("body")
                .and_then(|b| b.get("breakpoints"))
                .cloned()
                .unwrap_or_else(|| json!([])),
        })))
    }

    #[tool(description = "Continue execution")]
    async fn debugger_continue(
        &self,
        params: Parameters<DebuggerContinueParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let mut manager = self.session.lock().await;
        let Some(session) = manager.session.as_mut() else {
            return Err(detached_session_error("debugger_continue"));
        };

        let last_stop = session.stop_info().await;
        let thread_id = resolve_thread_id(session, params.thread_id)
            .await
            .map_err(to_mcp_error)?;

        session
            .send_request(
                "continue",
                json!({
                    "threadId": thread_id,
                }),
                ATTACH_TIMEOUT,
            )
            .await
            .map_err(to_mcp_error)?;

        {
            let mut stopped = session.last_stopped_event.lock().await;
            *stopped = None;
        }

        Ok(CallToolResult::structured(json!({
            "ok": true,
            "state": "running",
            "thread_id": thread_id,
            "last_stop": last_stop,
        })))
    }

    #[tool(description = "Step over the next line")]
    async fn debugger_step_over(
        &self,
        params: Parameters<DebuggerStepOverParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let mut manager = self.session.lock().await;
        let Some(session) = manager.session.as_mut() else {
            return Err(detached_session_error("debugger_step_over"));
        };

        let thread_id = resolve_thread_id(session, params.thread_id)
            .await
            .map_err(to_mcp_error)?;

        let stopped_event = perform_step_with_stop_restore(session, "next", thread_id).await?;
        let stop = stopped_summary(&stopped_event);

        Ok(CallToolResult::structured(json!({
            "ok": true,
            "state": "stopped",
            "thread_id": thread_id,
            "stop": stop,
        })))
    }

    #[tool(description = "Step into function call")]
    async fn debugger_step_in(
        &self,
        params: Parameters<DebuggerStepInParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let mut manager = self.session.lock().await;
        let Some(session) = manager.session.as_mut() else {
            return Err(detached_session_error("debugger_step_in"));
        };

        let thread_id = resolve_thread_id(session, params.thread_id)
            .await
            .map_err(to_mcp_error)?;

        let stopped_event = perform_step_with_stop_restore(session, "stepIn", thread_id).await?;
        let stop = stopped_summary(&stopped_event);

        Ok(CallToolResult::structured(json!({
            "ok": true,
            "state": "stopped",
            "thread_id": thread_id,
            "stop": stop,
        })))
    }

    #[tool(description = "Step out of current function")]
    async fn debugger_step_out(
        &self,
        params: Parameters<DebuggerStepOutParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let mut manager = self.session.lock().await;
        let Some(session) = manager.session.as_mut() else {
            return Err(detached_session_error("debugger_step_out"));
        };

        let thread_id = resolve_thread_id(session, params.thread_id)
            .await
            .map_err(to_mcp_error)?;

        let stopped_event = perform_step_with_stop_restore(session, "stepOut", thread_id).await?;
        let stop = stopped_summary(&stopped_event);

        Ok(CallToolResult::structured(json!({
            "ok": true,
            "state": "stopped",
            "thread_id": thread_id,
            "stop": stop,
        })))
    }

    #[tool(description = "Read variables from a variables reference")]
    async fn debugger_variables(
        &self,
        params: Parameters<DebuggerVariablesParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let mut manager = self.session.lock().await;
        let Some(session) = manager.session.as_mut() else {
            return Err(detached_session_error("debugger_variables"));
        };

        let mut arguments = Map::new();
        arguments.insert(
            "variablesReference".to_string(),
            json!(params.variables_reference),
        );
        if let Some(start) = params.start {
            arguments.insert("start".to_string(), json!(start));
        }
        if let Some(count) = params.count {
            arguments.insert("count".to_string(), json!(count));
        }

        let raw = session
            .send_request("variables", Value::Object(arguments), ATTACH_TIMEOUT)
            .await
            .map_err(to_mcp_error)?;

        let variables = raw
            .get("body")
            .and_then(|body| body.get("variables"))
            .cloned()
            .unwrap_or_else(|| json!([]));

        Ok(CallToolResult::structured(json!({
            "ok": true,
            "variables": variables,
            "raw": raw,
        })))
    }

    #[tool(description = "Evaluate expression in debugger context")]
    async fn debugger_evaluate(
        &self,
        params: Parameters<DebuggerEvaluateParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let mut manager = self.session.lock().await;
        let Some(session) = manager.session.as_mut() else {
            return Err(detached_session_error("debugger_evaluate"));
        };

        let mut arguments = Map::new();
        arguments.insert("expression".to_string(), json!(params.expression));
        arguments.insert(
            "context".to_string(),
            json!(params.context.unwrap_or_else(|| "watch".to_string())),
        );
        if let Some(frame_id) = params.frame_id {
            arguments.insert("frameId".to_string(), json!(frame_id));
        }

        let raw = session
            .send_request("evaluate", Value::Object(arguments), ATTACH_TIMEOUT)
            .await
            .map_err(to_mcp_error)?;

        let body = raw
            .get("body")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        Ok(CallToolResult::structured(json!({
            "ok": true,
            "result": body.get("result").and_then(Value::as_str),
            "type": body.get("type").and_then(Value::as_str),
            "variables_reference": body.get("variablesReference").and_then(Value::as_u64),
            "memory_reference": body.get("memoryReference").and_then(Value::as_str),
            "raw": raw,
        })))
    }

    #[tool(description = "Read memory from target runtime")]
    async fn debugger_read_memory(
        &self,
        params: Parameters<DebuggerReadMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        if params.count > READ_MEMORY_MAX_COUNT {
            return Err(to_mcp_error(format!(
                "debugger_read_memory count {} exceeds max allowed {} bytes",
                params.count, READ_MEMORY_MAX_COUNT
            )));
        }

        let mut manager = self.session.lock().await;
        let Some(session) = manager.session.as_mut() else {
            return Err(detached_session_error("debugger_read_memory"));
        };

        let raw = session
            .send_request(
                "readMemory",
                json!({
                    "memoryReference": params.memory_reference,
                    "offset": params.offset,
                    "count": params.count,
                }),
                ATTACH_TIMEOUT,
            )
            .await
            .map_err(to_mcp_error)?;

        let body = raw
            .get("body")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        Ok(CallToolResult::structured(json!({
            "ok": true,
            "address": body.get("address").and_then(Value::as_str),
            "count": params.count,
            "data_base64": body.get("data").and_then(Value::as_str).unwrap_or_default(),
            "unreadable_bytes": body.get("unreadableBytes").and_then(Value::as_u64),
            "raw": raw,
        })))
    }

    #[tool(description = "Execute debugger console command")]
    async fn debugger_console(
        &self,
        params: Parameters<DebuggerConsoleParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let mut manager = self.session.lock().await;
        let Some(session) = manager.session.as_mut() else {
            return Err(detached_session_error("debugger_console"));
        };

        let mut arguments = Map::new();
        arguments.insert("expression".to_string(), json!(params.command));
        arguments.insert("context".to_string(), json!("repl"));
        if let Some(frame_id) = params.frame_id {
            arguments.insert("frameId".to_string(), json!(frame_id));
        }

        let raw = session
            .send_request("evaluate", Value::Object(arguments), ATTACH_TIMEOUT)
            .await
            .map_err(to_mcp_error)?;

        let body = raw
            .get("body")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        Ok(CallToolResult::structured(json!({
            "ok": true,
            "result": body.get("result").and_then(Value::as_str),
            "type": body.get("type").and_then(Value::as_str),
            "variables_reference": body.get("variablesReference").and_then(Value::as_u64),
            "memory_reference": body.get("memoryReference").and_then(Value::as_str),
            "raw": raw,
        })))
    }

    #[tool(description = "Capture Bevy runtime snapshot useful for debugger UI")]
    async fn bevy_debug_snapshot(
        &self,
        params: Parameters<BevyDebugSnapshotParams>,
    ) -> Result<CallToolResult, McpError> {
        let _params = params.0;

        let mut manager = self.session.lock().await;
        let Some(session) = manager.session.as_mut() else {
            return Ok(snapshot_unsupported(
                "No attached debugger session",
                None,
            ));
        };

        let stopped_event = {
            let stopped = session.last_stopped_event.lock().await;
            stopped.clone()
        };

        let Some(stopped_event) = stopped_event else {
            return Ok(snapshot_unsupported(
                "Debugger is not currently stopped",
                None,
            ));
        };

        let thread_id = stopped_event
            .get("body")
            .and_then(Value::as_object)
            .and_then(|body| body.get("threadId"))
            .and_then(Value::as_u64);

        let Some(thread_id) = thread_id else {
            return Ok(snapshot_unsupported(
                "Stopped event does not include threadId",
                Some(&stopped_event),
            ));
        };

        let stack_trace_raw = session
            .send_request(
                "stackTrace",
                json!({
                    "threadId": thread_id,
                    "startFrame": 0,
                    "levels": 3,
                }),
                ATTACH_TIMEOUT,
            )
            .await
            .map_err(to_mcp_error)?;

        let top_frame = stack_trace_raw
            .get("body")
            .and_then(Value::as_object)
            .and_then(|body| body.get("stackFrames"))
            .and_then(Value::as_array)
            .and_then(|frames| frames.first())
            .cloned();

        let top_frame_name = top_frame
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|frame| frame.get("name"))
            .and_then(Value::as_str)
            .unwrap_or_default();

        if !top_frame_name.contains("axiom_debug_safe_point") {
            return Ok(snapshot_unsupported(
                format!(
                    "Top stack frame is not axiom_debug_safe_point (got '{}')",
                    top_frame_name
                ),
                Some(&stopped_event),
            ));
        }

        let frame_id = top_frame
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|frame| frame.get("id"))
            .and_then(Value::as_u64);

        let mut eval_args = Map::new();
        eval_args.insert("expression".to_string(), json!("&AXIOM_DEBUG_PROBE_STATE"));
        eval_args.insert("context".to_string(), json!("watch"));
        if let Some(id) = frame_id {
            eval_args.insert("frameId".to_string(), json!(id));
        }

        let primary_eval_raw = session
            .send_request("evaluate", Value::Object(eval_args), ATTACH_TIMEOUT)
            .await
            .map_err(to_mcp_error)?;

        let primary_eval_body = primary_eval_raw
            .get("body")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        let mut fallback_eval_raw: Option<Value> = None;
        let memory_reference = if let Some(memory_reference) = primary_eval_body
            .get("memoryReference")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
        {
            memory_reference.to_string()
        } else {
            let mut fallback_args = Map::new();
            fallback_args.insert(
                "expression".to_string(),
                json!("p/x &AXIOM_DEBUG_PROBE_STATE"),
            );
            fallback_args.insert("context".to_string(), json!("repl"));
            if let Some(id) = frame_id {
                fallback_args.insert("frameId".to_string(), json!(id));
            }

            let output_start_seq = {
                let events = session.recent_output_events.lock().await;
                events
                    .back()
                    .map(|(seq, _)| seq.saturating_add(1))
                    .unwrap_or(0)
            };

            let (fallback_seq, fallback_rx) = session
                .send_request_begin("evaluate", Value::Object(fallback_args))
                .await
                .map_err(to_mcp_error)?;

            let fallback = session
                .await_response("evaluate", fallback_seq, fallback_rx, ATTACH_TIMEOUT)
                .await
                .map_err(to_mcp_error)?;

            let mut address = fallback
                .get("body")
                .and_then(Value::as_object)
                .and_then(|body| body.get("result"))
                .and_then(Value::as_str)
                .and_then(parse_hex_address);

            if address.is_none() {
                address = wait_for_output_event_address(
                    &session.recent_output_events,
                    output_start_seq,
                    OUTPUT_EVENT_WAIT_TIMEOUT,
                )
                .await;
            }

            let address = address.ok_or_else(|| {
                    to_mcp_error(
                        "Failed to resolve AXIOM_DEBUG_PROBE_STATE address from evaluate fallback",
                    )
                })?;

            fallback_eval_raw = Some(fallback);
            address
        };

        let read_frame_counter_raw = session
            .send_request(
                "readMemory",
                json!({
                    "memoryReference": memory_reference,
                    "offset": 0,
                    "count": 8,
                }),
                ATTACH_TIMEOUT,
            )
            .await
            .map_err(to_mcp_error)?;
        let frame_counter_bytes = read_memory_data_bytes(&read_frame_counter_raw, 8).map_err(to_mcp_error)?;
        let frame_counter = read_u64_le(&frame_counter_bytes).map_err(to_mcp_error)?;

        let read_snapshot_len_raw = session
            .send_request(
                "readMemory",
                json!({
                    "memoryReference": memory_reference,
                    "offset": 8,
                    "count": 8,
                }),
                ATTACH_TIMEOUT,
            )
            .await
            .map_err(to_mcp_error)?;
        let snapshot_len_bytes = read_memory_data_bytes(&read_snapshot_len_raw, 8).map_err(to_mcp_error)?;
        let snapshot_len_raw = read_u64_le(&snapshot_len_bytes).map_err(to_mcp_error)?;
        let snapshot_len = usize::try_from(snapshot_len_raw)
            .unwrap_or(AXIOM_DEBUG_PROBE_SNAPSHOT_CAPACITY)
            .min(AXIOM_DEBUG_PROBE_SNAPSHOT_CAPACITY);

        let read_snapshot_bytes_raw = session
            .send_request(
                "readMemory",
                json!({
                    "memoryReference": memory_reference,
                    "offset": 16,
                    "count": snapshot_len,
                }),
                ATTACH_TIMEOUT,
            )
            .await
            .map_err(to_mcp_error)?;
        let mut snapshot_bytes =
            read_memory_data_bytes(&read_snapshot_bytes_raw, snapshot_len).map_err(to_mcp_error)?;

        while snapshot_bytes.last().copied() == Some(0) {
            snapshot_bytes.pop();
        }

        let snapshot_text = String::from_utf8(snapshot_bytes)
            .map_err(|e| to_mcp_error(format!("Snapshot bytes are not valid UTF-8: {e}")))?;
        let snapshot_json: Value = serde_json::from_str(&snapshot_text)
            .map_err(|e| to_mcp_error(format!("Snapshot bytes are not valid JSON: {e}")))?;

        Ok(CallToolResult::structured(json!({
            "ok": true,
            "supported": true,
            "frame_counter": frame_counter,
            "snapshot_len": snapshot_len,
            "snapshot": snapshot_json,
            "raw": {
                "stackTrace": stack_trace_raw,
                "evaluate": {
                    "primary": primary_eval_raw,
                    "fallback": fallback_eval_raw,
                },
                "reads": {
                    "frame_counter": read_frame_counter_raw,
                    "snapshot_len": read_snapshot_len_raw,
                    "snapshot": read_snapshot_bytes_raw,
                    "snapshot_len_raw": snapshot_len_raw,
                }
            }
        })))
    }
}

#[tool_handler]
impl ServerHandler for DebuggerMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Debugger MCP Server with single-session CodeLLDB attach/detach support".into(),
            ),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let server = DebuggerMcpServer::new();
    let transport = transport::stdio();

    tracing::info!("Starting Debugger MCP Server on stdio...");

    server.serve(transport).await?.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_output_events(entries: &[(u64, &str)]) -> Arc<Mutex<VecDeque<(u64, String)>>> {
        let mut events = VecDeque::new();
        for (seq, output) in entries {
            push_recent_output_event(&mut events, *seq, (*output).to_string());
        }
        Arc::new(Mutex::new(events))
    }

    #[test]
    fn parse_hex_address_extracts_address_from_console_output() {
        assert_eq!(
            parse_hex_address("$0 = 0x7ffee4bff5a8"),
            Some("0x7ffee4bff5a8".to_string())
        );
        assert_eq!(
            parse_hex_address("result: 0xDEADBEEF,"),
            Some("0xDEADBEEF".to_string())
        );
        assert_eq!(parse_hex_address("$0 = 42"), None);
        assert_eq!(parse_hex_address("pointer: 0x"), None);
    }

    #[test]
    fn parse_hex_address_from_output_event_extracts_pointer_result() {
        let output_event = serde_json::json!({
            "type": "event",
            "event": "output",
            "body": {
                "category": "console",
                "output": "(bevy_ai_remote::AxiomDebugProbeState *) 0x000055a48f077a08\n"
            }
        });
        assert_eq!(
            parse_hex_address_from_output_event(&output_event),
            Some("0x000055a48f077a08".to_string())
        );
    }

    #[tokio::test]
    async fn wait_for_output_event_address_returns_hex_for_entries_at_or_after_start_seq() {
        let recent_output_events = seeded_output_events(&[
            (41, "noise without pointer"),
            (42, "(ProbeState*) 0x00000000DEADBEEF"),
        ]);

        let address = wait_for_output_event_address(
            &recent_output_events,
            42,
            Duration::from_millis(1),
        )
        .await;

        assert_eq!(address, Some("0x00000000DEADBEEF".to_string()));
    }

    #[tokio::test]
    async fn wait_for_output_event_address_ignores_addresses_before_start_seq_marker() {
        let recent_output_events = seeded_output_events(&[
            (7, "(ProbeState*) 0x00000000000000AA"),
            (8, "plain output"),
        ]);

        let address = wait_for_output_event_address(&recent_output_events, 8, Duration::ZERO).await;

        assert_eq!(address, None);
    }

    #[test]
    fn push_recent_output_event_keeps_ring_buffer_bounded_and_evicts_oldest_entries() {
        let mut events = VecDeque::new();

        for seq in 0..(MAX_RECENT_OUTPUT_EVENTS as u64 + 10) {
            push_recent_output_event(&mut events, seq, format!("line-{seq}"));
        }

        assert_eq!(events.len(), MAX_RECENT_OUTPUT_EVENTS);
        assert_eq!(events.front().map(|(seq, _)| *seq), Some(10));
        assert_eq!(events.back().map(|(seq, _)| *seq), Some(MAX_RECENT_OUTPUT_EVENTS as u64 + 9));
    }

    #[test]
    fn read_u64_le_parses_little_endian_and_rejects_short_input() {
        let parsed = read_u64_le(&[0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11])
            .expect("8-byte LE input should parse");
        assert_eq!(parsed, 0x1122334455667788);

        let err = read_u64_le(&[1, 2, 3]).expect_err("short input must fail");
        assert!(
            err.contains("Expected at least 8 bytes") && err.contains("received 3 bytes"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn read_memory_data_bytes_errors_when_unreadable_bytes_present() {
        let response = serde_json::json!({
            "body": {
                "unreadableBytes": 4,
                "data": "AQIDBA=="
            }
        });

        let err = read_memory_data_bytes(&response, 1)
            .expect_err("unreadableBytes > 0 should produce an error");
        assert!(err.contains("unreadableBytes=4"), "unexpected error: {err}");
    }

    #[test]
    fn read_memory_data_bytes_errors_when_data_missing() {
        let response = serde_json::json!({
            "body": {
                "unreadableBytes": 0
            }
        });

        let err = read_memory_data_bytes(&response, 1).expect_err("missing data must fail");
        assert!(
            err.contains("missing base64 data"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn read_memory_data_bytes_decodes_base64_payload() {
        let response = serde_json::json!({
            "body": {
                "unreadableBytes": 0,
                "data": "AQIDBAU="
            }
        });

        let bytes = read_memory_data_bytes(&response, 5).expect("base64 payload should decode");
        assert_eq!(bytes, vec![1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    async fn wait_for_stopped_event_after_seq_returns_new_stop_event() {
        let last_stopped_event = Arc::new(Mutex::new(Some(serde_json::json!({
            "type": "event",
            "event": "stopped",
            "body": { "threadId": 7 }
        }))));
        let stopped_seq = Arc::new(AtomicU64::new(3));

        let stop = wait_for_stopped_event_after_seq(
            &last_stopped_event,
            &stopped_seq,
            2,
            Duration::from_millis(1),
        )
            .await
            .expect("new stop should be returned");

        assert_eq!(stop, serde_json::json!({
            "type": "event",
            "event": "stopped",
            "body": { "threadId": 7 }
        }));
    }

    #[tokio::test]
    async fn wait_for_stopped_event_after_seq_rejects_stale_stop_without_new_seq() {
        let last_stopped_event = Arc::new(Mutex::new(Some(serde_json::json!({
            "type": "event",
            "event": "stopped",
            "body": { "threadId": 9 }
        }))));
        let stopped_seq = Arc::new(AtomicU64::new(5));

        let err = wait_for_stopped_event_after_seq(
            &last_stopped_event,
            &stopped_seq,
            5,
            Duration::ZERO,
        )
        .await
        .expect_err("stale stop must not satisfy wait when sequence did not advance");

        assert!(
            err.contains("Timed out waiting for next DAP 'stopped' event"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn debugger_console_params_schema_has_no_bare_true() {
        let schema = schemars::schema_for!(DebuggerConsoleParams);
        let json = serde_json::to_string(&schema).expect("schema serialization must succeed");
        assert!(
            !json.contains("\"arguments\":true") && !json.contains("\"arguments\": true"),
            "Schema contains bare 'true' for arguments field, which OpenCode rejects:\n{}",
            serde_json::to_string_pretty(&schema)
                .expect("pretty schema serialization must succeed")
        );
    }

    #[test]
    fn debugger_set_breakpoints_params_schema_has_no_bare_true_for_function_breakpoints() {
        let schema = schemars::schema_for!(DebuggerSetBreakpointsParams);
        let json = serde_json::to_string(&schema).expect("schema serialization must succeed");
        assert!(
            !json.contains("\"function_breakpoints\":true")
                && !json.contains("\"function_breakpoints\": true"),
            "Schema contains bare 'true' for function_breakpoints field, which OpenCode rejects:\n{}",
            serde_json::to_string_pretty(&schema)
                .expect("pretty schema serialization must succeed")
        );
    }

    #[cfg(unix)]
    #[test]
    fn probe_adapter_startup_returns_quickly_for_running_process() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        runtime.block_on(async {
            let mut child = Command::new("sh")
                .args(["-c", "sleep 1"])
                .spawn()
                .expect("should spawn test process");

            let start = Instant::now();
            let probe = probe_adapter_startup(&mut child).expect("probe should succeed");
            let elapsed = start.elapsed();
            assert!(probe.is_none(), "sleeping process should still be running");
            assert!(
                elapsed < Duration::from_millis(300),
                "probe blocked too long: {:?}",
                elapsed
            );

            child.kill().await.expect("cleanup kill should succeed");
            let _ = child.wait().await;
        });
    }

    #[cfg(unix)]
    #[test]
    fn probe_adapter_startup_detects_early_exit() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should initialize");
        runtime.block_on(async {
            let mut child = Command::new("sh")
                .args(["-c", "exit 7"])
                .spawn()
                .expect("should spawn test process");

            let status = child.wait().await.expect("process should exit");
            assert_eq!(status.code(), Some(7), "sanity check exit code");

            let probe = probe_adapter_startup(&mut child).expect("probe should succeed");
            let status = probe.expect("probe should detect exited child");
            assert_eq!(status.code(), Some(7));
        });
    }
}
