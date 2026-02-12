pub mod conductor;
pub mod contexts;

pub const SYSTEM_BEAST: &str = include_str!("system_beast.md");

pub fn get_system_prompt(research_mode: &str, context_mode: &str, profile_prompt: &str) -> String {
    let base_prompt = SYSTEM_BEAST;
    let context_prompt = contexts::get_context_prompt(context_mode);

    let mode_instruction = match research_mode {
        "Fast" => {
            r#"
# RESEARCH MODE: 🚀 FAST (OFFLINE)
- **STRICTLY FORBIDDEN** to use `webfetch`, `google_search` or `run_command` for searching information.
- **MUST** rely ONLY on your internal knowledge base.
- If you do not know something, explicitly state "I don't know" rather than hallucinating or trying to search.
- **FOCUS**: Speed and direct answers.
"#
        }
        "Deep Research" => {
            r#"
# RESEARCH MODE: 🔍 DEEP (ONLINE)
- **MANDATORY** to verify key facts using `webfetch` or search tools.
- **DO NOT** rely solely on your internal memory, even for common facts.
- **CROSS-REFERENCE** multiple sources if possible.
- **FOCUS**: Accuracy, depth, and up-to-date information.
"#
        }
        "Smart Hybrid" | _ => {
            r#"
# RESEARCH MODE: 🌐 HYBRID (AUTO)
- **Common Knowledge**: Use internal knowledge for well-known facts to save time.
- **Obscure/Recent**: Use search tools ONLY if information is likely recent, obscure, or if you are unsure.
- **FOCUS**: Balance between speed and accuracy.
"#
        }
    };

    let role_instruction = if !profile_prompt.is_empty() {
        format!("\n# AGENT ROLE & IDENTITY\n{}\n", profile_prompt)
    } else {
        String::new()
    };

    format!(
        "{}\n\n{}\n\n{}\n\n{}",
        base_prompt, mode_instruction, context_prompt, role_instruction
    )
}
