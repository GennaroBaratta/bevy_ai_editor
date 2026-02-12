# Axiom Architecture

## Overview
Axiom is an agentic AI coding assistant built in Rust, designed to run locally with high performance and reliability. It leverages the `opencode` methodology, focusing on robust tool use and structured interaction with Large Language Models (LLMs).

## Components

### Antigravity Gateway
The core communication layer for LLM interaction is the "Antigravity Gateway". 

- **Address**: `127.0.0.1:8045`
- **Protocol**: OpenAI API Compatible
- **Model**: Gemini (via proxy)

**Why OpenAI Protocol?**
While the backend model is Gemini, we utilize the OpenAI API protocol for the gateway. This standardizes the request/response format, making the client implementation in Axiom agnostic to the specific provider quirks. The gateway handles the translation to the Gemini API, ensuring a consistent interface for the application.

### System Prompts
Axiom employs a dual-mode system prompt strategy to balance user experience and task execution efficiency:

1.  **Conversational Mode**: Used for general interaction, clarifying requirements, and planning. It creates a more natural dialogue flow.
2.  **Beast Mode**: Activated for heavy-duty coding tasks. This mode uses a rigorous, instruction-heavy prompt (derived from `opencode`'s Sisyphus agent) that enforces strict adherence to protocols, file manipulation safety, and step-by-step reasoning.

### Event Loop
The application runs on a Tokio-based asynchronous event loop. This manages:
- **GUI Rendering**: Powered by `egui` for immediate mode responsiveness.
- **LLM Interaction**: Non-blocking network requests to the Antigravity Gateway.
- **Tool Execution**: Handling file system operations (read, write) and command execution (run) without freezing the UI.

### Internal Tools (MCP Client)
Instead of a complex external MCP (Model Context Protocol) server setup, v0.1.1 implements core tools directly as internal functions, acting as a streamlined MCP client.
- **Read**: File reading capabilities.
- **Write**: Safe file writing.
- **Run**: Command execution.
