# Debugger MCP Setup — CodeLLDB Provisioning (Linux MVP)

This guide explains how to configure the **Debugger MCP Server** for OpenCode, including CodeLLDB adapter provisioning and Linux ptrace prerequisites for attach workflows.

## Prerequisites

- **Rust** (latest stable) and **Cargo** installed
- **OpenCode** (or another MCP-compatible assistant using local stdio MCP servers)
- **CodeLLDB adapter binary** available locally (typically `adapter/codelldb` from the VSCode CodeLLDB extension)
- Linux host for attach support (current MVP scope)

## Build the Debugger MCP Server

Build the server binary before wiring it into OpenCode:

```bash
cargo build -p debugger_mcp_server
```

The binary path used by OpenCode is:

- `./target/debug/debugger_mcp_server`

Use the built binary directly instead of `cargo run` so build/progress output does not interfere with MCP stdio protocol traffic.

## Provision CodeLLDB Adapter

The debugger server needs an explicit CodeLLDB adapter path via `CODELLDB_ADAPTER_PATH`.

Typical adapter locations:

- VSCode extension unpacked folder: `.../vadimcn.vscode-lldb-*/adapter/codelldb`
- Locally installed standalone adapter binary: `.../codelldb`

Set the environment variable in OpenCode MCP config:

- `CODELLDB_ADAPTER_PATH=/absolute/path/to/codelldb`

CodeLLDB can be used over stdio (`codelldb`) or socket mode (`--port` / `--connect`). For this MCP integration, point to the adapter executable and let the server orchestrate sessions.

## Linux Attach Prerequisites (ptrace)

Attach operations rely on `ptrace`, so Linux process attach may fail unless these conditions are met:

- The debugger and target process run as the **same user**
- Kernel ptrace policy allows attach (`/proc/sys/kernel/yama/ptrace_scope`)

Common symptom when blocked:

- Attach fails with **EPERM** / permission denied (often surfaced by LLDB as attach failure)

To check current ptrace policy:

```bash
cat /proc/sys/kernel/yama/ptrace_scope
```

Lower values are less restrictive; restrictive values can block non-child attach even when user IDs match.

## Scope and Security Notes

Current scope:

- **Linux-only** attach behavior documented here
- **Local-only** MVP deployment model (no remote hardened transport)

Security warning:

- The debugger MCP server currently exposes highly privileged debugger capabilities (raw console/eval/memory reads)
- Treat it as a trusted local developer tool only
- Do not expose this server to untrusted clients or multi-tenant environments

## Troubleshooting

### Server starts but debugger actions fail immediately

- Verify `CODELLDB_ADAPTER_PATH` points to an executable file
- Rebuild binary after updates: `cargo build -p debugger_mcp_server`

### Attach returns permission errors

- Confirm debugger and target run under the same Linux user
- Check `ptrace_scope` and adjust host policy if your workflow requires non-child attach
