# Changelog

## [v0.1.1] - 2026-01-27

### Added
- **GUI**: Integrated `egui` interface for chat and tool visualization.
- **Tool Loop**: Implemented the core feedback loop for LLM tool use.
- **Core Tools**: Added native support for:
    - `read`: Read file contents.
    - `write`: Write to files.
    - `run`: Execute shell commands.
- **Chinese Font Support**: Added `Microsoft YaHei` (and fallbacks) to support CJK characters in the UI.
- **Log Highlighting**: improved visual distinction for logs and tool outputs in the chat interface.

### Changed
- **System Prompt**: Refined prompt strategy to switch between conversational and "Beast Mode" based on context.
