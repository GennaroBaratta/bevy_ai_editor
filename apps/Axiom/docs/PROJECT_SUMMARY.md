# Axiom Project Analysis

**Date:** January 31, 2026
**Status:** Active Analysis

## 1. Core Identity: The "Axiom" Agent
Axiom (`src/`) is not a standard web application. It is a **Rust-based AI Agentic IDE / Orchestrator**. 

-   **Type:** Native Desktop Application (Windows/Linux/Mac).
-   **Framework:** `eframe` (egui) for the GUI, `tokio` for async runtime.
-   **Purpose:** To act as an intelligent environment for software development, capable of planning, coding, and managing sub-projects.

## 2. "The Logic" (src/)
The `src/` directory contains the brain of the system:
-   **Agent Capabilities:** Defines `AgentProfile`s (e.g., "Bevy Architect", "Deep Researcher") in `src/agent/`.
-   **LLM Integration:** Direct Gemini API client (`src/llm/gemini.rs`) handling streaming and function calling.
-   **Toolbelt:** A massive suite of tools in `src/tools/` allowing the agent to:
    -   **Read/Write/Edit Code:** (`read`, `write`, `edit`, `multiedit`, `batch`)
    -   **Analyze Code:** (`ast_grep`, `lsp`, `search`)
    -   **Execute Commands:** (`shell`, `task`)
    -   **Manage Tasks:** (`todo`, `task`)
    -   **Control Bevy:** (`bevy` tools for manipulating a game engine scene).
-   **Orchestration:** `src/main.rs` handles the "Conductor" logic, multi-agent delegation, and task execution loops.

## 3. The "Managed" Workspace
The other directories in the repository (`backend/`, `frontend/`, `bevy-wasm/`, `cyber-shop-backend/`) appear to be **Outputs** or **Managed Projects** controlled by Axiom.

-   **`backend/`**: A standard Actix-web + SQLx Rust backend.
-   **`frontend/`**: A Vue 3 + Vite web frontend.
-   **`bevy-wasm/`**: A Bevy game engine project compiled to WebAssembly.
-   **`src/ws_actor.rs`, `src/App.vue`**: These files exist in `src/` but are not part of the Axiom binary. They are likely **Templates** or **Source Artifacts** used by the Agent to scaffold or update the external projects.

## 4. Architecture Summary

| Component | Role | Technology |
| :--- | :--- | :--- |
| **Axiom (Root)** | **The Architect (Controller)** | Rust, Eframe, Gemini LLM |
| `backend/` | Target Application (Server) | Rust, Actix-web, Postgres |
| `frontend/` | Target Application (Client) | Vue 3, TypeScript |
| `bevy-wasm/` | Target Visuals (3D/Sim) | Rust, Bevy, WASM |

## 5. Key Observation
The user noted that `src/` is the "true logic". This confirms that `Axiom` is a tool-building tool. The files like `src/ws_actor.rs` are likely "genetic material" that Axiom injects into the `backend/` project during generation.

## 6. Usage
To run the **Agent Interface**:
```bash
cargo run --release
```

To run the **Managed Stack** (created by the Agent):
```bash
docker-compose up --build
```
