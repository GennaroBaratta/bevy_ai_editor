# Axiom Development Learnings & Decisions

## Configuration
- **Proxy**: All network requests (LLM APIs, etc.) MUST go through `http://127.0.0.1:8045`.
- **Target Architecture**: "Opencode" clone in Rust.
- **Prompt Strategy**: 
  - Base: `system_beast.md` (Behavior/Identity).
  - Domain Specifics: Injected via RAG/Context, NOT hardcoded in system prompt.

## Technology Stack
- Language: Rust 2024
- LLM Provider: Google Gemini (inferred from conversation) / Compatible APIs.
- Core: Event Loop -> LLM -> Tool -> Event Loop.
