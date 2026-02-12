# MCP Setup — Codex & OpenCode Integration

This guide explains how to integrate the **Bevy MCP Server** with AI coding assistants that support the Model Context Protocol (MCP).

## Prerequisites

- **Rust** (latest stable) and **Cargo** installed
- A **Bevy game** running with `bevy_ai_remote` plugin (listening on `http://127.0.0.1:15721`)
- **OpenCode** or **Codex** (or another MCP-compatible AI assistant)

## Starting the MCP Server

The Bevy MCP Server runs as a stdio-based MCP server. From the repository root:

```bash
cargo run -p bevy_mcp_server
```

The server will:
- Connect to the Bevy BRP endpoint (default: `http://127.0.0.1:15721`)
- Expose 6 MCP tools for scene manipulation
- Communicate via JSON-RPC over stdin/stdout

## OpenCode Configuration

Add this to your OpenCode configuration file (typically `.opencode/config.json` or similar):

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "bevy": {
      "type": "local",
      "command": ["cargo", "run", "-p", "bevy_mcp_server"],
      "environment": {
        "BRP_ENDPOINT": "http://127.0.0.1:15721"
      }
    }
  },
  "tools": {
    "bevy_bevy_rpc_raw": false
  }
}
```

**Notes**:
- `command`: Path to run the MCP server binary
- `environment.BRP_ENDPOINT`: Override default Bevy Remote Protocol endpoint
- `tools.bevy_bevy_rpc_raw: false`: Disables the raw BRP tool by default (see below for why)

## Codex Configuration

Add this to your Codex MCP configuration:

```toml
[mcp.servers.bevy]
command = ["cargo", "run", "-p", "bevy_mcp_server"]
type = "local"

[mcp.servers.bevy.environment]
BRP_ENDPOINT = "http://127.0.0.1:15721"

[tools]
bevy_bevy_rpc_raw = false  # Disabled by default for safety
```

**Notes**:
- Adjust `command` if using a pre-built binary instead of `cargo run`
- Set `BRP_ENDPOINT` if your Bevy game uses a different port
- `bevy_bevy_rpc_raw` is disabled to prevent unsafe raw BRP access (see Advanced Usage below)

## Troubleshooting

### Startup Timeout
If the MCP server fails to start within the expected time:
- **Cause**: Cargo compile time on first run
- **Solution**: Pre-build the binary with `cargo build -p bevy_mcp_server` before configuring MCP

### BRP Connection Errors
If tools fail with "Connection refused" or "Ping failed":
1. Verify your Bevy game is running: `curl http://127.0.0.1:15721`
2. Check the BRP endpoint in your game's plugin initialization
3. Confirm the `BRP_ENDPOINT` environment variable matches your game's port

### Common Error Messages
- `"Bridge error: Connection failed"` → Bevy game not running or wrong endpoint
- `"Ping failed: connection refused"` → BRP server not listening on expected port
- `"Invalid base64"` → Asset upload data incorrectly encoded (check your input)

## Advanced Usage: Enabling `bevy_rpc_raw`

The `bevy_rpc_raw` tool allows direct, unfiltered access to the Bevy Remote Protocol. **It is disabled by default for safety.**

**⚠️ Use with Caution:**
- No parameter validation or transformation
- Can execute ANY BRP method (including potentially destructive operations)
- Requires knowledge of Bevy 0.18 BRP internals
- Intended for advanced users debugging BRP or prototyping new operations

**To enable**:
1. Remove the `"bevy_bevy_rpc_raw": false` line from your config
2. OR set `"bevy_bevy_rpc_raw": true` explicitly

**Example raw usage** (after enabling):
```json
{
  "method": "world.spawn_entity",
  "params": {
    "components": {
      "bevy_transform::components::transform::Transform": {
        "translation": [0.0, 1.0, 0.0],
        "rotation": [0.0, 0.0, 0.0, 1.0],
        "scale": [1.0, 1.0, 1.0]
      }
    }
  }
}
```

**Prefer the typed tools** (`bevy_spawn_primitive`, `bevy_upload_asset`, etc.) for normal usage — they provide validation, defaults, and safer abstractions.

## Next Steps

- See [MCP_TOOL_REFERENCE.md](./MCP_TOOL_REFERENCE.md) for detailed tool schemas and examples
- Run `bevy_ping` to verify connectivity before using other tools
- Use `bevy_query` to inspect what's currently in your scene
