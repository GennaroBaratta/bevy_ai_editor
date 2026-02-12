# MCP-First + Linux Refactor Plan (Execution Handoff)

This document is a detailed implementation plan for the next agent to execute.

## Decision Snapshot

- Architecture direction: **MCP-first**.
- MCP SDK: **`rmcp`** (official Rust MCP SDK, v0.15+, `server` + `transport-io` + `macros` + `schemars` features).
- BRP HTTP client: **`reqwest`** (async) in `bevy_bridge_core`. Replaces `ureq` (sync) for bridge operations.
- Include `bevy_rpc_raw` in v1.
- `bevy_rpc_raw` is **pure pass-through**: no magic `world.query` param wrapping. Users must provide correct BRP format.
- Keep `bevy_rpc_raw` available but **disabled by default** in sample Codex/OpenCode configs (opt-in).
- `bevy_spawn_scene` is **excluded from v1** (SceneRoot component reflection is blocked; tool currently non-functional).
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
- No `bevy_spawn_scene` tool (blocked by SceneRoot Handle reflection issue).

## Current Known Issues to Fix

1. **~~Workspace path case mismatch~~ (RESOLVED)**
   - Directory renamed from `apps/Axiom` to `apps/axiom`. Matches `Cargo.toml`, `.gitignore`, README, and all runtime path references.

2. **Windows-centric agent prompt (`system_beast.md`)**
   - Lines 9-19 hardcode `OS: Windows 11` and explicitly forbid Linux-style paths (`/home/user/...`). Must be made platform-aware or platform-agnostic.

3. **Windows-only runtime assumptions**
   - Font path `C:/Windows/Fonts/msyh.ttc` in `main.rs` (hardcoded Windows font).
   - Screenshot launch command `cmd /C start ms-screenclip:` in `main.rs`.

4. **Windows-centric scripts**
   - `.cmd` scripts with hardcoded absolute paths (`D:\workspace\bevy_ai_editor`).

5. **`unsafe` proxy env var (`main.rs:737-739`)**
   - `unsafe { std::env::set_var("HTTPS_PROXY", "http://127.0.0.1:17890"); }` — hardcoded proxy, unsafe in Rust 2024 edition. Should be cfg-gated or moved to `.env`.

## Current Tool Inventory (Axiom codebase)

Reference for Phase 2 extraction. These are the actual Bevy tool structs in `apps/axiom/src/tools/bevy.rs`:

| Struct | `.name()` | Status | Notes |
|--------|-----------|--------|-------|
| `BevyUploadAssetTool` | `bevy_upload_asset` | Registered, working | Base64 asset upload + spawn via BRP |
| `BevyRpcTool` | `bevy_rpc` | Registered (Bevy profile) | Generic JSON-RPC; has `world.query` param auto-wrapping |
| `BevySpawnSceneTool` | `bevy_spawn_scene` | Registered (Bevy profile) | **Non-functional**: SceneRoot commented out |
| `BevyClearSceneTool` | `bevy_clear_scene` | Registered, working | Despawns entities with `SceneRoot` component |
| `BevySpawnPrimitiveTool` | `bevy_spawn_primitive` | **Commented out** in registry | Spawns via `AxiomPrimitive` component |

Additional tools in `video.rs` (4 FFmpeg tools) are defined but **not registered** in the tool registry.

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
                    |          bevy_mcp_server               |
                    |  tools/list + tools/call over stdio    |
                    |  Built with rmcp (async/tokio)         |
                    +--------------------+--------------------+
                                         |
                                typed async Rust API calls
                                         |
                    +--------------------v--------------------+
                    |         bevy_bridge_core               |
                    | async reqwest BRP client + typed ops   |
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

## Phase 1 — Linux Stabilization and Path Normalization

### Tasks

1. ~~Fix workspace and package path casing.~~ (DONE — directory is `apps/axiom`)
2. Replace hardcoded absolute Windows script paths with relative script logic.
3. Add Linux helper scripts for local development.
4. Remove/soften Windows-specific runtime assumptions where they break Linux:
   - Font path: use `dirs` crate to locate system fonts, or skip font loading gracefully on non-Windows.
   - Screenshot: cfg-gate `ms-screenclip` to Windows; provide no-op or `xdg-open`-based fallback on Linux.
5. Fix `system_beast.md` environment context block to be platform-aware (detect OS at runtime and inject, or remove hardcoded `OS: Windows 11`).
6. Address `unsafe set_var` for proxy: move to `.env` file loading (already uses `dotenv`) or cfg-gate.

### File-level candidates

- `run_all.cmd` / `run_editor.cmd` / `run_game.cmd` (add `.sh` equivalents)
- `apps/axiom/src/main.rs` (font path, screenshot, unsafe proxy)
- `apps/axiom/src/ui/file_tree.rs` (verify path assumptions)
- `apps/axiom/src/prompts/system_beast.md` (OS context block)
- `apps/axiom/src/prompts/road_engineer.md` (verify — currently clean)
- `apps/axiom/src/prompts/road_engineer_backup.md` (verify — currently clean)
- `apps/axiom/src/tools/video.rs` (audit for Windows-specific assumptions)
- `README.md` (add Linux launch instructions)

### New files (expected)

- `run_all.sh`
- `run_editor.sh`
- `run_game.sh`

### Acceptance criteria

- `cargo metadata --no-deps` succeeds at repo root.
- `cargo check --workspace` succeeds on Linux.
- Scripts no longer require drive-letter absolute paths.
- `grep -ri 'apps/Axiom' --include='*.rs' --include='*.toml' --include='*.md'` returns zero matches (excluding this plan document).
- Axiom binary starts without panicking on Linux (font/path errors handled gracefully).
- `system_beast.md` no longer hardcodes Windows OS context.

## Phase 2 — Extract Shared Bridge Core

### Tasks

1. Create a new crate: `crates/bevy_bridge_core`.
2. Move BRP request/response plumbing from Axiom `bevy.rs` tool implementation into shared modules.
3. Use **`reqwest`** (async) as the HTTP client. This replaces `ureq` for bridge operations.
4. Implement typed operations:
   - `upload_asset` — **extracted** from `BevyUploadAssetTool` (file read + Base64 encode + `world.spawn_entity`)
   - `rpc_raw` — **extracted** from `BevyRpcTool`, **stripped of `world.query` param auto-wrapping** (pure pass-through)
   - `spawn_primitive` — **extracted** from `BevySpawnPrimitiveTool` (`AxiomPrimitive` component spawn)
   - `clear_scene` — **extracted** from `BevyClearSceneTool` (list + filter `SceneRoot` + despawn)
   - `ping` — **net new implementation** (simple BRP connectivity check, e.g. call `bevy/list` or similar)
   - `query` — **net new implementation** (typed wrapper around `world.query` with proper Bevy 0.18 `data` key format)
5. Centralize endpoint + timeout config in one config struct.
6. Refactor Axiom to call shared crate wrappers (thin adapter only).
   - Axiom tools execute synchronously. The adapter will use `tokio::runtime::Handle::block_on` or `spawn_blocking` to call async bridge core functions from the sync tool `execute()` method.

### Note: `bevy_spawn_scene` is excluded

`BevySpawnSceneTool` has its `SceneRoot` component commented out (`bevy.rs:381-388`) with a note about incorrect Handle<Scene> JSON format. It currently spawns a ghost entity (Transform-only). This tool is **excluded from bridge core and MCP v1** until the SceneRoot reflection issue is resolved.

### Suggested internal module layout

```
crates/bevy_bridge_core/src/
  lib.rs
  config.rs          # BrpConfig { endpoint, timeout }
  error.rs           # BrpError enum
  client.rs          # async reqwest-based BRP JSON-RPC client
  ops/
    ping.rs          # NEW — connectivity check
    query.rs         # NEW — typed world.query wrapper
    spawn.rs         # Extracted from BevySpawnPrimitiveTool
    upload.rs        # Extracted from BevyUploadAssetTool
    clear.rs         # Extracted from BevyClearSceneTool
    raw.rs           # Extracted from BevyRpcTool (pass-through only)
  types/
    requests.rs
    responses.rs
```

### Dependencies

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["rt", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
base64 = "0.22"
thiserror = "2"
tracing = "0.1"
```

### Acceptance criteria

- Axiom compiles/runs with unchanged user-facing behavior.
- Shared bridge logic is no longer tied to Axiom UI structs.
- Unit tests cover serialization and error mapping for core ops.
- `ping` and `query` ops work against a running Bevy BRP host.

## Phase 3 — Build MCP Server (stdio-first)

### Tasks

1. Create crate: `crates/bevy_mcp_server` (binary).
2. Implement MCP server using **`rmcp`** with stdio transport.
3. Wire each tool to `bevy_bridge_core` async operations.
4. Add robust input validation and explicit timeout handling.
5. Implement structured error model with stable error codes.

### Dependencies

```toml
[dependencies]
rmcp = { version = "0.15", features = ["server", "transport-io", "macros", "schemars"] }
bevy_bridge_core = { path = "../bevy_bridge_core" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
```

### v1 tool surface

| MCP Tool Name | Bridge Core Op | Source |
|---------------|---------------|--------|
| `bevy_ping` | `ops::ping` | Net new |
| `bevy_query` | `ops::query` | Net new |
| `bevy_spawn_primitive` | `ops::spawn` | Extracted |
| `bevy_upload_asset` | `ops::upload` | Extracted |
| `bevy_clear_scene` | `ops::clear` | Extracted |
| `bevy_rpc_raw` | `ops::raw` | Extracted (pass-through) |

**Excluded from v1:** `bevy_spawn_scene` (blocked by SceneRoot reflection issue).

### `bevy_rpc_raw` requirements

- Input schema:
  - `method` (string, required)
  - `params` (object/array/null, optional)
  - `timeout_ms` (integer, optional)
- Output schema:
  - `ok` (bool)
  - `result` (json, optional)
  - `error` (object, optional)
- **Pure pass-through**: no hidden param transformation. No `world.query` auto-wrapping. Users provide the exact BRP format.
- Log method name and duration for observability.
- Do not add hidden allow/deny behavior in v1; policy is controlled by MCP client config.

### Implementation pattern (rmcp)

```rust
#[derive(Clone)]
pub struct BevyMcpServer {
    bridge: BrpClient,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl BevyMcpServer {
    #[tool(description = "Check if the Bevy BRP endpoint is reachable")]
    async fn bevy_ping(&self) -> Result<CallToolResult, McpError> {
        // calls bridge.ping().await
    }

    #[tool(description = "Query Bevy ECS world for entities matching component filters")]
    async fn bevy_query(&self, #[tool(param)] params: Parameters<QueryRequest>)
        -> Result<CallToolResult, McpError> {
        // calls bridge.query(params).await
    }
    // ... etc
}

#[tool_handler]
impl ServerHandler for BevyMcpServer { ... }

#[tokio::main]
async fn main() {
    let service = BevyMcpServer::new(config)
        .serve(rmcp::transport::stdio())
        .await?;
    service.waiting().await?;
}
```

### Acceptance criteria

- `tools/list` returns all 6 expected tools with JSON schemas.
- Each tool successfully executes against a local Bevy BRP host.
- Server handles malformed requests without crashing.
- Concurrent requests are handled correctly (rmcp + reqwest are both async).

## Phase 4 — Codex/OpenCode Integration Docs + Configs

### Tasks

1. Add setup doc for Codex and OpenCode MCP configs.
2. Provide copy/paste snippets with env vars and timeouts.
3. Ship secure default snippets where `bevy_rpc_raw` is disabled initially.
4. Add "enable raw" section with explicit caveats.

### Suggested doc files

- `docs/MCP_SETUP_CODEX_OPENCODE.md`
- `docs/MCP_TOOL_REFERENCE.md`

### OpenCode config snippet (verified against docs)

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "bevy": {
      "type": "local",
      "command": ["./target/debug/bevy_mcp_server"],
      "environment": {
        "BRP_ENDPOINT": "http://127.0.0.1:15721",
        "RUST_LOG": "off"
      }
    }
  },
  "tools": {
    "bevy_bevy_rpc_raw": false
  }
}
```

Notes:
- OpenCode uses `"type": "local"`, `"command"` (array), `"environment"` (object).
- Tool disabling uses the `"tools"` config section. MCP tool names are prefixed with the server name, so `bevy_rpc_raw` from the `bevy` server becomes `bevy_bevy_rpc_raw`.
- No `enabledTools` field exists — instead, disable specific tools you don't want.

### Codex config snippet (NEEDS VERIFICATION)

> **Warning**: The Codex MCP configuration format below needs verification against current Codex documentation before publishing. The TOML format and field names (`startup_timeout_sec`, `tool_timeout_sec`, `enabled_tools`) are assumed.

```toml
[mcp_servers.bevy]
command = "./target/debug/bevy_mcp_server"
cwd = "/absolute/path/to/repo"
startup_timeout_sec = 20
tool_timeout_sec = 60
enabled_tools = [
  "bevy_ping",
  "bevy_query",
  "bevy_spawn_primitive",
  "bevy_upload_asset",
  "bevy_clear_scene"
]
```

### Acceptance criteria

- Both clients can start server and discover tools.
- Docs include troubleshooting for startup timeout and BRP connectivity.
- Config snippets are tested and verified against actual client documentation.

## Phase 5 — End-to-End Validation Matrix

### Local functional test matrix

1. Start Bevy sample host (`examples/simple_game`) with `bevy_ai_remote` enabled.
2. MCP `bevy_ping` succeeds.
3. MCP `bevy_spawn_primitive` creates visible entity.
4. MCP `bevy_upload_asset` accepts payload and writes asset.
5. MCP `bevy_query` returns expected scene data.
6. MCP `bevy_clear_scene` removes managed entities.
7. MCP `bevy_rpc_raw` can call a known safe BRP method.

### Stability checks

- Invalid payloads return structured errors (not panics).
- BRP-down scenario returns non-panicking, actionable error.
- Timeout boundaries behave predictably.
- Concurrent tool calls don't interfere with each other.

### Acceptance criteria

- Documented runbook reproduces all checks on Linux.

## Work Breakdown for Successive Agent (Ordered Task List)

1. **Unblock workspace on Linux** — remaining: scripts, runtime portability fixes, `system_beast.md` OS fix, `unsafe set_var` cleanup.
2. **Create `bevy_bridge_core`** with async `reqwest` client — extract `upload_asset`, `rpc_raw`, `spawn_primitive`, `clear_scene`; implement new `ping` and `query` ops.
3. **Refactor Axiom adapter** to use the new core crate (sync→async bridge via `block_on`).
4. **Create `bevy_mcp_server`** using `rmcp` with v1 tool surface (6 tools, excluding `spawn_scene`).
5. **Add tool schemas and robust error mapping**.
6. **Write setup/reference docs** for Codex and OpenCode (verify config formats first).
7. **Run validation matrix** and capture command outputs in docs.

## Suggested Milestone Commits

1. `fix(linux): normalize workspace paths and add unix run scripts`
2. `fix(linux): make runtime portable (fonts, screenshot, proxy, system prompt)`
3. `refactor(bridge): extract shared async BRP client into bevy_bridge_core`
4. `refactor(axiom): use bevy_bridge_core for bevy tool operations`
5. `feat(mcp): add bevy_mcp_server with core bevy tools via rmcp`
6. `feat(mcp): add bevy_rpc_raw tool and structured error model`
7. `docs(mcp): add codex/opencode setup and tool reference`

## Risks and Mitigations

1. **~~Path-case regressions~~ (Resolved)**
   - Directory is now `apps/axiom`. Mitigation: grep for `apps/Axiom` in CI and fail if found.

2. **`ureq`→`reqwest` migration in bridge core**
   - Bridge core uses async `reqwest`. Axiom executes tools synchronously.
   - Risk: `block_on` called from within an async context can panic.
   - Mitigation: Axiom already has a tokio runtime. Use `spawn_blocking` + `block_on` pattern, or refactor tool execution to be async.

3. **Large payload upload timeouts**
   - Mitigation: configurable timeouts via `BrpConfig` and clear error messages.

4. **Raw RPC misuse**
   - Mitigation: disabled by default in config examples + explicit opt-in docs.

5. **Axiom behavior drift during extraction**
   - Mitigation: keep adapter thin and validate previous Bevy operations still pass.

6. **`Cargo.lock` policy**
   - `Cargo.lock` is currently gitignored. The MCP server is a binary crate — convention is to commit `Cargo.lock` for reproducible builds.
   - Decision: un-ignore `Cargo.lock` after MCP server crate is added, or scope the ignore to library crates only.

7. **`BevySpawnSceneTool` is blocked**
   - `SceneRoot` component commented out due to `Handle<Scene>` JSON serialization issue.
   - Mitigation: excluded from v1. Tracked as future work. Can be re-added once the Bevy-side reflection issue is resolved.

## Definition of Done

- Linux build and checks pass at workspace level.
- Reusable bridge core crate exists and is consumed by Axiom + MCP server.
- MCP server works with Codex and OpenCode via documented configs.
- `bevy_rpc_raw` is implemented and documented with opt-in posture.
- End-to-end validation steps are reproducible from repo docs.
- No hardcoded Windows paths remain in runtime code (cfg-gated where needed).

## Handoff Notes for Next Agent

- Phase 1 path normalization is mostly done (directory renamed). Remaining work: scripts, runtime portability, prompt fixes.
- `ping` and `query` are **net new implementations**, not extraction from existing code.
- `clear_scene` is the correct tool name (not `clear_generated`).
- Use `rmcp` crate with `#[tool]` macro and `stdio()` transport for the MCP server.
- Use `reqwest` (async) in bridge core, not `ureq` (sync).
- `bevy_rpc_raw` must be pure pass-through — no auto-wrapping of `world.query` params.
- `BevySpawnPrimitiveTool` exists in code but is commented out in the tool registry. It needs to be re-enabled in bridge core extraction.
- `video.rs` tools (4 FFmpeg wrappers) exist but are not registered. Audit for portability but do not include in MCP v1.
- Keep functionality unchanged while extracting core logic.
- Prefer small, reviewable commits per milestone.
- If a compatibility tradeoff appears, preserve MCP protocol stability over internal implementation details.
