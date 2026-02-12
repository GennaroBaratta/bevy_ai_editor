use crate::types::AgentProfile;

pub fn get_default_agents() -> Vec<AgentProfile> {
    vec![
        AgentProfile {
            name: "General Assistant".to_string(),
            description: "Balanced for everyday tasks.".to_string(),
            model: "gemini-2.5-flash".to_string(),
            research_mode: "Smart Hybrid".to_string(),
            context_mode: "General".to_string(),
            avatar_path: "bot.png".to_string(),
            system_prompt: "You are Axiom, a helpful AI assistant. You are capable, honest, and efficient.".to_string(),
        },
        AgentProfile {
            name: "Bevy Architect".to_string(),
            description: "Expert in Bevy 0.18 (Future).".to_string(),
            model: "gemini-2.5-pro".to_string(),
            research_mode: "Smart Hybrid".to_string(),
            context_mode: "Bevy 0.18 (Future)".to_string(),
            avatar_path: "bevy.png".to_string(),
            system_prompt: "You are a Senior Graphics Engineer specializing in Bevy Engine. You prefer ECS patterns and strict Rust type safety.".to_string(),
        },
        AgentProfile {
            name: "Pokemon Professor".to_string(),
            description: "Fast responses for pokedex queries.".to_string(),
            model: "gemini-2.5-flash".to_string(),
            research_mode: "Fast".to_string(),
            context_mode: "Pokemon Gen9".to_string(),
            avatar_path: "pokemon.png".to_string(),
            system_prompt: "You are Professor Oak. You study Pokemon and help trainers complete their Pokedex.".to_string(),
        },
        AgentProfile {
            name: "Deep Researcher".to_string(),
            description: "Thorough web search and verification.".to_string(),
            model: "gemini-2.5-pro".to_string(),
            research_mode: "Deep Research".to_string(),
            context_mode: "General".to_string(),
            avatar_path: "research.png".to_string(),
            system_prompt: "You are a Deep Research Specialist. Your goal is to find, verify, and synthesize information from multiple sources.".to_string(),
        },
    ]
}
