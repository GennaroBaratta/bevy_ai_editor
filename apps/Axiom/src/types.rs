use crate::llm::MessageContent;

#[derive(Clone, Debug)]
pub struct ChannelState {
    pub id: String,                             // Unique ID (e.g., "global", "backend")
    pub name: String,                           // Display Name (e.g., "🌐 Global", "🦀 Backend")
    pub history: Vec<(String, MessageContent)>, // The chat history for this channel
    pub assigned_agents: Vec<String>,           // List of Agent Names assigned to this channel
}

impl Default for ChannelState {
    fn default() -> Self {
        Self {
            id: "global".to_string(),
            name: "🌐 Global".to_string(),
            history: Vec::new(),
            assigned_agents: Vec::new(), // Global usually implies all, or dynamic
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentProfile {
    pub name: String,
    pub description: String,
    pub avatar_path: String,   // e.g., "bevy.png"
    pub model: String,         // e.g., "gemini-pro"
    pub research_mode: String, // "Fast", "Smart Hybrid", "Deep Research"
    pub context_mode: String,  // "General", "Bevy", "Pokemon"
    pub system_prompt: String, // The actual prompt
}

impl Default for AgentProfile {
    fn default() -> Self {
        Self {
            name: "Axiom".to_string(),
            description: "Default AI Assistant".to_string(),
            avatar_path: "system.png".to_string(),
            model: "gemini-2.5-flash".to_string(),
            research_mode: "Smart Hybrid".to_string(),
            context_mode: "General".to_string(),
            system_prompt: "".to_string(),
        }
    }
}

#[allow(dead_code)]
pub enum AsyncMessage {
    Response(MessageContent),
    StreamText(String),
    Done,
    Log(String),
    Error(String),
}
