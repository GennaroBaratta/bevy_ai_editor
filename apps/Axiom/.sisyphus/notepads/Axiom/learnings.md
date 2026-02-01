# Learnings
- When using `reqwest` and `serde_json`, ensure that structs matching the API response are correctly defined and public if they need to be accessed from other modules.
- Implementing a tool loop requires maintaining the conversation history state (`messages` vector) and appending both the assistant's tool call message and the tool's result message before calling the LLM again.
- Stateless clients (like `GeminiClient` in this context) require the caller to manage the full conversation history.
- `eframe`/`egui` UI updates must happen on the main thread, so async tasks must communicate back via channels (`std::sync::mpsc` or `tokio::sync::mpsc`).
- When initializing a git repo that has code but no commits, commit all files as "initial commit" to ensure a consistent state for future checkouts, while including the specific fix requested.
- Modified `AsyncMessage::Log` to include tool arguments for better user visibility during execution.
- Implemented `WriteFileTool` in `src/tools/mod.rs` to allow file creation/writing capabilities.
- Added `WriteFileTool` to `get_all_tools` registry.

# Decisions
- We chose to implement the loop limit (`MAX_TURNS`) to prevent infinite loops if the model keeps requesting tools.
- We show "Log" messages in the chat history as "System" messages for now to provide immediate feedback to the user without creating a complex UI element for logs.

# Issues
- `cargo check` revealed that `Message` and `ToolCall` were not re-exported from `llm` mod, requiring an update to `src/llm/mod.rs`.
- The `Tool` trait has a `description()` method that is currently unused in the loop logic, but kept for API completeness.

## UI Cleanup
- Removed API key input from top panel in `src/main.rs`. Replaced with "Axiom Chat" heading.
- Kept `api_key` in struct for internal use.

## Font Support
- Chinese font support in `egui`:
    - `egui` defaults do not support CJK characters.
    - Loaded `msyh.ttc` from Windows fonts directory.
    - Used `fonts.families.entry(Proportional).insert(0, ...)` to ensure it's used for UI text.
    - Note: `egui` versions vary on whether `FontData` needs to be wrapped in `Arc`. In this project (likely older egui), it does NOT use `Arc`.
