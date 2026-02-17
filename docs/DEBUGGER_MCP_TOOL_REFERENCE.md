# MCP Tool Reference — Debugger MCP Server

This document describes the debugger MCP tools exposed by `crates/debugger_mcp_server`.

---

## 1. `debugger_attach`

**Description**: Start a debugger session and attach CodeLLDB to a target PID.

**Input**:
```json
{
  "pid": 12345,
  "program": "/home/user/game/target/debug/simple_game",
  "adapter_path": "/home/user/.vscode/extensions/vadimcn.vscode-lldb/adapter/codelldb"
}
```

**Output**:
```json
{
  "ok": true,
  "state": "attached",
  "pid": 12345,
  "log_path": ".sisyphus/evidence/dap_session_12345_1730000000000.jsonl"
}
```

---

## 2. `debugger_detach`

**Description**: Disconnect and tear down the current debugger session.

**Input**:
```json
{
  "terminate_debuggee": false
}
```

**Output**:
```json
{
  "ok": true,
  "state": "detached"
}
```

---

## 3. `debugger_set_breakpoints`

**Description**: Set source breakpoints for a file and optional function breakpoints.

**Input**:
```json
{
  "source_path": "/home/user/project/src/main.rs",
  "breakpoints": [
    { "line": 42 },
    { "line": 77, "condition": "counter > 10", "log_message": "counter hit" }
  ],
  "function_breakpoints": ["my_crate::systems::tick", "my_crate::debug::safe_point"]
}
```

**Output**:
```json
{
  "ok": true,
  "state": "stopped",
  "stop": {
    "reason": "breakpoint",
    "thread_id": 1
  },
  "configuration_done_sent": true,
  "source_breakpoints": [{ "verified": true, "line": 42 }],
  "function_breakpoints": [{ "verified": true, "message": null }]
}
```

---

## 4. `debugger_continue`

**Description**: Continue execution of a stopped thread.

**Input**:
```json
{
  "thread_id": 1
}
```

**Output**:
```json
{
  "ok": true,
  "state": "running",
  "thread_id": 1,
  "last_stop": {
    "reason": "breakpoint",
    "thread_id": 1
  }
}
```

---

## 5. `debugger_step_over`

**Description**: Step to the next line in the current frame.

**Input**:
```json
{
  "thread_id": 1
}
```

**Output**:
```json
{
  "ok": true,
  "state": "stopped",
  "thread_id": 1,
  "stop": {
    "reason": "step",
    "thread_id": 1
  }
}
```

---

## 6. `debugger_step_in`

**Description**: Step into the next function call.

**Input**:
```json
{
  "thread_id": 1
}
```

**Output**:
```json
{
  "ok": true,
  "state": "stopped",
  "thread_id": 1,
  "stop": {
    "reason": "step",
    "thread_id": 1
  }
}
```

---

## 7. `debugger_step_out`

**Description**: Step out of the current function.

**Input**:
```json
{
  "thread_id": 1
}
```

**Output**:
```json
{
  "ok": true,
  "state": "stopped",
  "thread_id": 1,
  "stop": {
    "reason": "step",
    "thread_id": 1
  }
}
```

---

## 8. `debugger_variables`

**Description**: Read variables from a DAP `variablesReference`.

**Input**:
```json
{
  "variables_reference": 123,
  "start": 0,
  "count": 50
}
```

**Output**:
```json
{
  "ok": true,
  "variables": [
    {
      "name": "frame_counter",
      "value": "1024",
      "type": "u64",
      "variablesReference": 0
    }
  ],
  "raw": { "type": "response", "command": "variables" }
}
```

---

## 9. `debugger_evaluate`

**Description**: Evaluate an expression in the current debugger context.

**Input**:
```json
{
  "expression": "player.position.x",
  "frame_id": 1001,
  "context": "watch"
}
```

**Output**:
```json
{
  "ok": true,
  "result": "12.5",
  "type": "f32",
  "variables_reference": 0,
  "memory_reference": "0x00007ffd12340000",
  "raw": { "type": "response", "command": "evaluate" }
}
```

---

## 10. `debugger_read_memory`

**Description**: Read memory from a DAP `memoryReference`.

**Input**:
```json
{
  "memory_reference": "0x00007ffd12340000",
  "offset": 0,
  "count": 32
}
```

**Output**:
```json
{
  "ok": true,
  "address": "0x00007ffd12340000",
  "count": 32,
  "data_base64": "AQIDBAUGBwgJAA==",
  "unreadable_bytes": 0,
  "raw": { "type": "response", "command": "readMemory" }
}
```

**Notes**:
- `count` max is `65536` bytes per call

---

## 11. `debugger_console`

**Description**: Execute a debugger console command (REPL evaluate).

**Input**:
```json
{
  "command": "p/x &AXIOM_DEBUG_PROBE_STATE",
  "frame_id": 1001,
  "context": "repl",
  "arguments": { "format": "hex" }
}
```

**Output**:
```json
{
  "ok": true,
  "result": "$0 = 0x00007ffd12340000",
  "type": "&AxiomDebugProbeState",
  "variables_reference": 0,
  "memory_reference": "0x00007ffd12340000",
  "raw": { "type": "response", "command": "evaluate" }
}
```

**Notes**:
- MCP input supports `context` and `arguments`, but this server always sends DAP evaluate with `context: "repl"`

---

## 12. `bevy_debug_snapshot`

**Description**: Capture structured runtime snapshot from `AXIOM_DEBUG_PROBE_STATE` when stopped at a safe point.

**Input**:
```json
{
  "include_entities": true,
  "include_components": true,
  "include_resources": false
}
```

**Output** (supported case):
```json
{
  "ok": true,
  "supported": true,
  "frame_counter": 2048,
  "snapshot_len": 512,
  "snapshot": {
    "entities": [],
    "resources": {}
  },
  "raw": {
    "stackTrace": { "type": "response" },
    "evaluate": { "primary": {}, "fallback": null },
    "reads": { "frame_counter": {}, "snapshot_len": {}, "snapshot": {} }
  }
}
```

**Output** (unsupported case):
```json
{
  "ok": true,
  "supported": false,
  "reason": "Debugger is not currently stopped",
  "stop": null
}
```

---

## Troubleshooting

- **Missing `CODELLDB_ADAPTER_PATH`**: Set env var or pass `adapter_path` in `debugger_attach`; otherwise attach fails before adapter spawn.
- **Attach fails with ptrace EPERM**: On Linux/containerized setups, grant ptrace capability or relax ptrace scope (`/proc/sys/kernel/yama/ptrace_scope`) as appropriate.
- **"not stopped" / missing `threadId`**: Step/continue tools require a stopped event context; pass explicit `thread_id` or break first so stopped event includes `threadId`.
- **`readMemory` returns `unreadableBytes`**: Treat the read as partial/invalid for typed decoding; re-read a smaller range or verify the memory reference is valid.
- **Safe point required for `bevy_debug_snapshot`**: Top frame must contain `axiom_debug_safe_point`; if not, snapshot returns `supported: false`.

---

## Recommended Flow

1. `debugger_attach`
2. `debugger_set_breakpoints` (source + `function_breakpoints`)
3. `debugger_continue` / `debugger_step_*`
4. `debugger_variables`, `debugger_evaluate`, `debugger_read_memory`, `debugger_console`
5. `bevy_debug_snapshot` at `axiom_debug_safe_point`
6. `debugger_detach`
