use crate::agent::AgentProfile;
use crate::types::ChannelState;
use eframe::egui;
use std::collections::HashMap;

pub enum SidebarAction {
    SelectProfile(AgentProfile),
    CopyLog,
    None,
}

pub fn render_sidebar(
    ui: &mut egui::Ui,
    available_profiles: &[AgentProfile],
    current_profile: &AgentProfile,
    active_channel_id: &str,
    channels: &HashMap<String, ChannelState>,
) -> SidebarAction {
    let mut action = SidebarAction::None;

    ui.add_space(10.0);
    ui.heading("🤖 Agents");
    ui.add_space(10.0);

    ui.vertical(|ui| {
        // Get allowed agents for current channel
        let allowed_agents = if let Some(channel) = channels.get(active_channel_id) {
            channel.assigned_agents.clone()
        } else {
            Vec::new()
        };

        for profile in available_profiles {
            // Filter: Only show agents assigned to this channel
            if !allowed_agents.contains(&profile.name) {
                continue;
            }

            let is_selected = current_profile.name == profile.name;

            let btn = ui.add_enabled(
                true,
                egui::Button::new(
                    egui::RichText::new(format!(
                        "{} {}",
                        profile.avatar_path.replace(".png", ""),
                        profile.name
                    ))
                    .strong()
                    .size(14.0),
                )
                .min_size(egui::vec2(ui.available_width(), 40.0))
                .fill(if is_selected {
                    egui::Color32::from_gray(60)
                } else {
                    egui::Color32::TRANSPARENT
                }),
            );

            if btn.clicked() {
                action = SidebarAction::SelectProfile(profile.clone());
            }

            // Tooltip description
            btn.on_hover_text(&profile.description);

            ui.add_space(5.0);
        }
    });

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.label(egui::RichText::new("Axiom v0.1").weak().size(10.0));
        ui.add_space(5.0);

        // Debug: Copy Log Button
        if ui.button("📋 Copy Log").clicked() {
            action = SidebarAction::CopyLog;
        }

        ui.add_space(5.0);
    });

    action
}
