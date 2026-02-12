# Axiom Project Progress

## Status: Agentic Capability & UI Refinement (2026-01-27)

### ✅ Completed Milestones

#### 1. Core Agent Capabilities (The "Brain")
- [x] **Vision Integration**: Implemented clipboard image paste, preview, and API transmission (Base64 encoding).
- [x] **Agentic Workflow**:
  - Implemented **Thinking Process** (Vision -> Reason -> Tool -> Verify).
  - Fixed "Path Hallucination": System Prompt optimized to prefer relative paths and avoid Linux-style (`/home/user`) guesses on Windows.
  - **Tool Chaining**: Demonstrated `write_file` -> `read_file` loop for self-verification.

#### 2. UI & Experience (The "Face")
- [x] **Image Previews**:
  - Input area: Fixed height (80px), proportional scaling.
  - Chat history: Decoding Base64 to textures, cached for performance, fixed size constraints.
- [x] **User Identity**: Renamed User role to **"Cats2333"** (mapped internally to API `user`).
- [x] **Agent Station UI**:
  - **Right Sidebar**: "Macros" for switching between different Agent Profiles (General, Bevy, Pokemon, Researcher).
  - **Bottom Control Bar**: "Fine-tuning" controls for Model, Mode, and Context.
  - **Layout**: Fixed Input box width (200px) to prevent layout shifting.
  - **Research Mode**: Dropdown to select strategy (Fast/Offline vs. Hybrid vs. Deep/Online).
  - **Context Preset**: Dropdown to inject domain expertise (General, Bevy 0.18/Future, Pokemon Master).

#### 3. System Prompt Engineering
- [x] **Dynamic Prompting**: `get_system_prompt` now accepts `mode` and `context` arguments to dynamically assemble the system instruction.
- [x] **Optimization**: Removed "Must Search" hard constraint, enabling "Common Knowledge" fast path for known facts (e.g., Bulbasaur stats).
- [x] **Context Injection**: Implemented mechanism to inject specific syntax rules (e.g., Bevy 0.18 fake syntax) to demonstrate "Knowledge Control".

### 🚧 In Progress / Planned
- [ ] **RAG / Local Docs**: Consider mounting actual local Markdown files for `read_file` based RAG instead of just prompt injection.
- [ ] **Multi-Agent Architecture**: Currently single-threaded (Single Role). Future consideration for Router/Coder/Reviewer split.
- [ ] **LSP Integration**: Still pending from initial plan.

### 📝 Architecture Notes
- **Current Pattern**: Single-Threaded Loop with Profile Switching.
- **Agent Profile**: Combines `Model` + `Research Mode` + `Context Mode`.
- **Right Sidebar**: Quick switching between Profiles.
- **Bottom Bar**: Manual override of current Profile settings.
