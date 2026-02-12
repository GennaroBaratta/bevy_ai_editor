# MCP-First + Linux Refactor Plan (Execution Handoff)

This document is a detailed implementation plan for the next agent to execute.

## Decision Snapshot

- Architecture direction: **MCP-first**.
- Include `bevy_rpc_raw` in v1.
- Keep `bevy_rpc_raw` available but **disabled by default** in sample Codex/OpenCode configs (opt-in).
- Preserve Axiom app behavior while extracting reusable bridge logic.

## Goals

1. Make the repository build and run cleanly on Linux.
2. Extract Bevy BRP bridge logic out of Axiom UI/tool glue into a reusable core crate.
3. Expose a standalone MCP server consumable by both Codex and OpenCode.
4. Ship docs and configs so users can connect in minutes.

## Non-goals (v1)

- No full redesign of Axiom UI.
- No deep rework of BRP protocol semantics.
- No broad cross-platform screenshot UX redesign beyond Linux-safe behavior.
- No remote deployment infra; local `stdio` MCP server is first-class target.

## Current Known Issues to Fix

1. **Workspace path case mismatch (Linux breaker)**
   - `Cargo.toml` references `apps/axiom`, folder is `apps/Axiom`.
2. **Mixed path casing in code/docs/prompts**
   - Examples: `.gitignore`, README text, prompt files, runtime path assumptions.
3. **Windows-only runtime assumptions**
   - Font path and screenshot launch command (`ms-screenclip`) in Axiom runtime.
4. **Windows-centric scripts**
   - `.cmd` scripts with hardcoded absolute paths.

## High-Level Architecture (Target)

```
                           +-----------------------------+
                           |      Codex / OpenCode       |
                           |    (MCP client, stdio)      |
                           +-------------+---------------+
                                         |
                                   MCP JSON-RPC
                                         |
                    +--------------------v--------------------+
                    |           bevy_mcp_server              |
                    |  tools/list + tools/call over stdio    |
                    +--------------------+--------------------+
                                         |
                                typed Rust API calls
                                         |
                    +--------------------v--------------------+
                    |          bevy_bridge_core              |
                    | BRP client + typed ops + validations   |
                    +--------------------+--------------------+
                                         |
                                HTTP JSON-RPC (BRP)
                                         |
                    +--------------------v--------------------+
                    |      Bevy app + bevy_ai_remote         |
                    |        (BRP endpoint :15721)           |
                    +-----------------------------------------+
```

## Phase-by-Phase Execution Plan

## Phase 1 - Linux Stabilization and Path Normalization

### Tasks

1. Fix workspace and package path casing.
2. Normalize all string references to one canonical path (`apps/Axiom` on disk).
3. Replace hardcoded absolute Windows script paths with relative script logic.
4. Add Linux helper scripts for local development.
5. Remove/soften Windows-specific runtime assumptions where they break Linux.

### File-level candidates

- `Cargo.toml`
- `.gitignore`
- `README.md`
- `run_all.cmd`
- `run_editor.cmd`
- `run_game.cmd`
- `apps/Axiom/src/main.rs`
- `apps/Axiom/src/ui/file_tree.rs`
- `apps/Axiom/src/prompts/system_beast.md`
- `apps/Axiom/src/prompts/road_engineer.md`

### New files (expected)

- `run_all.sh`
- `run_editor.sh`
- `run_game.sh`

### Acceptance criteria

- `cargo metadata --no-deps` succeeds at repo root.
- `cargo check --workspace` succeeds on Linux.
- Scripts no longer require drive-letter absolute paths.

## Phase 2 - Extract Shared Bridge Core

### Tasks

1. Create a new crate: `crates/bevy_bridge_core`.
2. Move BRP request/response plumbing from Axiom Bevy tool implementation into shared modules.
3. Implement typed operations used by both Axiom and MCP server:
   - `ping`
   - `query`
   - `spawn_primitive`
   - `upload_asset`
   - `clear_generated`
   - `rpc_raw`
4. Centralize endpoint + timeout config in one config struct.
5. Refactor Axiom to call shared crate wrappers (thin adapter only).

### Suggested internal module layout

```
crates/bevy_bridge_core/src/
  lib.rs
  config.rs
  error.rs
  brp_client.rs
  ops/
    ping.rs
    query.rs
    spawn.rs
    upload.rs
    clear.rs
    raw.rs
  types/
    requests.rs
    responses.rs
```

### Acceptance criteria

- Axiom compiles/runs with unchanged user-facing behavior.
- Shared bridge logic is no longer tied to Axiom UI structs.
- Unit tests cover serialization and error mapping for core ops.

## Phase 3 - Build MCP Server (stdio-first)

### Tasks

1. Create crate: `crates/bevy_mcp_server` (binary).
2. Implement MCP server over `stdio` with tool discovery and tool execution.
3. Wire each tool to `bevy_bridge_core`.
4. Add robust input validation and explicit timeout handling.
5. Implement structured error model with stable error codes.

### v1 tool surface

- `bevy_ping`
- `bevy_query`
- `bevy_spawn_primitive`
- `bevy_upload_asset`
- `bevy_clear_generated`
- `bevy_rpc_raw`

### `bevy_rpc_raw` requirements

- Input schema:
  - `method` (string, required)
  - `params` (object/array/null, optional)
  - `timeout_ms` (integer, optional)
- Output schema:
  - `ok` (bool)
  - `result` (json, optional)
  - `error` (object, optional)
- Log method name and duration for observability.
- Do not add hidden allow/deny behavior in v1; policy is controlled by MCP client config.

### Acceptance criteria

- `tools/list` returns all expected tools with schemas.
- Each tool successfully executes against local Bevy BRP host.
- Server handles malformed requests without crashing.

## Phase 4 - Codex/OpenCode Integration Docs + Configs

### Tasks

1. Add setup doc for Codex and OpenCode MCP configs.
2. Provide copy/paste snippets with env vars and timeouts.
3. Ship secure default snippets where `bevy_rpc_raw` is disabled initially.
4. Add "enable raw" section with explicit caveats.

### Suggested doc files

- `docs/MCP_SETUP_CODEX_OPENCODE.md`
- `docs/MCP_TOOL_REFERENCE.md`

### Example Codex snippet (target doc content)

```toml
[mcp_servers.bevy]
command = "cargo"
args = ["run", "-p", "bevy_mcp_server"]
cwd = "/absolute/path/to/repo"
startup_timeout_sec = 20
tool_timeout_sec = 60
enabled_tools = [
  "bevy_ping",
  "bevy_query",
  "bevy_spawn_primitive",
  "bevy_upload_asset",
  "bevy_clear_generated"
]
```

### Example OpenCode snippet (target doc content)

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "bevy": {
      "type": "local",
      "command": ["cargo", "run", "-p", "bevy_mcp_server"],
      "enabledTools": [
        "bevy_ping",
        "bevy_query",
        "bevy_spawn_primitive",
        "bevy_upload_asset",
        "bevy_clear_generated"
      ]
    }
  }
}
```

### Acceptance criteria

- Both clients can start server and discover tools.
- Docs include troubleshooting for startup timeout and BRP connectivity.

## Phase 5 - End-to-End Validation Matrix

### Local functional test matrix

1. Start Bevy sample host with `bevy_ai_remote` enabled.
2. MCP `bevy_ping` succeeds.
3. MCP `bevy_spawn_primitive` creates visible entity.
4. MCP `bevy_upload_asset` accepts payload and writes asset.
5. MCP `bevy_query` returns expected scene data.
6. MCP `bevy_clear_generated` removes managed entities.
7. MCP `bevy_rpc_raw` can call a known safe BRP method.

### Stability checks

- Invalid payloads return structured errors.
- BRP-down scenario returns non-panicking, actionable error.
- Timeout boundaries behave predictably.

### Acceptance criteria

- Documented runbook reproduces all checks on Linux.

## Work Breakdown for Successive Agent (Ordered Task List)

1. **Unblock workspace on Linux** (path/case + scripts + minimal runtime portability fixes).
2. **Create `bevy_bridge_core`** and migrate logic from Axiom Bevy tool.
3. **Refactor Axiom adapter** to use the new core crate.
4. **Create `bevy_mcp_server`** with full v1 tool surface including `bevy_rpc_raw`.
5. **Add tool schemas and robust error mapping**.
6. **Write setup/reference docs** for Codex and OpenCode.
7. **Run validation matrix** and capture command outputs in docs.

## Suggested Milestone Commits

1. `fix(linux): normalize workspace paths and add unix run scripts`
2. `refactor(bridge): extract shared BRP client into bevy_bridge_core`
3. `refactor(axiom): use bevy_bridge_core for bevy tool operations`
4. `feat(mcp): add bevy_mcp_server with core bevy tools`
5. `feat(mcp): add bevy_rpc_raw tool and structured error model`
6. `docs(mcp): add codex/opencode setup and tool reference`

## Risks and Mitigations

1. **Path-case regressions**
   - Mitigation: grep for `apps/axiom` and fail CI if reintroduced.
2. **Large payload upload timeouts**
   - Mitigation: configurable timeouts and clear error messages.
3. **Raw RPC misuse**
   - Mitigation: disabled by default in config examples + explicit opt-in docs.
4. **Axiom behavior drift during extraction**
   - Mitigation: keep adapter thin and validate previous Bevy operations still pass.

## Definition of Done

- Linux build and checks pass at workspace level.
- Reusable bridge core crate exists and is consumed by Axiom + MCP server.
- MCP server works with Codex and OpenCode via documented configs.
- `bevy_rpc_raw` is implemented and documented with opt-in posture.
- End-to-end validation steps are reproducible from repo docs.

## Handoff Notes for Next Agent

- Start with Linux path normalization; do not begin MCP implementation before workspace is healthy.
- Keep functionality unchanged while extracting core logic.
- Prefer small, reviewable commits per milestone.
- If a compatibility tradeoff appears, preserve MCP protocol stability over internal implementation details.
