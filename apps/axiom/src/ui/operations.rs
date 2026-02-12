use crate::types::SubAgentState;
use eframe::egui;
use std::collections::HashMap;

pub fn render_operations_panel(
    ctx: &egui::Context,
    active_sub_agents: &mut HashMap<String, SubAgentState>,
) {
    if active_sub_agents.is_empty() {
        return;
    }

    let mut to_remove = Vec::new();

    egui::TopBottomPanel::bottom("ops_panel")
        .min_height(100.0)
        .max_height(300.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("🚀 Mission Control")
                        .size(14.0)
                        .strong(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🗑 Clear All").clicked() {
                        to_remove.extend(active_sub_agents.keys().cloned());
                    }

                    // Add a button to clear only finished tasks
                    if ui.button("✨ Clear Finished").clicked() {
                        for (id, agent) in active_sub_agents.iter() {
                            if agent.status == "Finished" {
                                to_remove.push(id.clone());
                            }
                        }
                    }
                });
            });
            ui.separator();

            egui::ScrollArea::horizontal()
                .id_salt("ops_scroll_horizontal")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(5.0);
                        // Sort by key to keep order stable
                        let mut keys: Vec<_> = active_sub_agents.keys().cloned().collect();
                        keys.sort();

                        let current_time = ctx.input(|i| i.time);
                        const AUTO_CLEAR_DELAY: f64 = 8.0; // Seconds to keep finished tasks before auto-removing

                        for key in keys {
                            if let Some(agent) = active_sub_agents.get(&key) {
                                // Auto-clear logic:
                                // If finished AND enough time has passed since last update (completion time)
                                if agent.status == "Finished" {
                                    let time_since_finish = current_time - agent.last_update;

                                    // Optional: You could check if the user is hovering to pause the timer
                                    // But for now, simple timeout
                                    if time_since_finish > AUTO_CLEAR_DELAY {
                                        to_remove.push(key.clone());
                                        continue; // Skip rendering if we're removing
                                    }
                                }

                                egui::Frame::group(ui.style())
                                    .fill(egui::Color32::from_gray(20)) // Darker background for "hacker" feel
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                                    .rounding(5.0)
                                    .inner_margin(8.0)
                                    .show(ui, |ui| {
                                        ui.set_width(180.0); // Narrower width (was 250.0)

                                        ui.vertical(|ui| {
                                            // Header
                                            ui.horizontal(|ui| {
                                                // Try to load avatar image first
                                                let avatar_path = format!(
                                                    "assets/avatars/{}.png",
                                                    agent.agent_type.to_lowercase()
                                                );
                                                let abs_path = std::env::current_dir()
                                                    .unwrap_or_default()
                                                    .join(&avatar_path);

                                                let mut showed_avatar = false;
                                                if abs_path.exists() {
                                                    let uri = format!(
                                                        "file://{}",
                                                        abs_path
                                                            .display()
                                                            .to_string()
                                                            .replace("\\", "/")
                                                    );
                                                    ui.add(
                                                        egui::Image::from_uri(uri)
                                                            .fit_to_exact_size(egui::vec2(
                                                                20.0, 20.0,
                                                            )) // Smaller avatar
                                                            .rounding(10.0),
                                                    );
                                                    showed_avatar = true;
                                                }

                                                if !showed_avatar {
                                                    // Fallback to Emoji
                                                    let icon = match agent.agent_type.as_str() {
                                                        "researcher" => "🔍",
                                                        "coder" => "💻",
                                                        "reviewer" => "👀",
                                                        "planner" => "📝",
                                                        _ => "🤖",
                                                    };
                                                    ui.label(egui::RichText::new(icon).size(14.0));
                                                }

                                                ui.label(
                                                    egui::RichText::new(&agent.name)
                                                        .strong()
                                                        .size(12.0),
                                                );

                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        // Close Button for individual agent
                                                        if ui.small_button("❌").clicked() {
                                                            to_remove.push(key.clone());
                                                        }

                                                        if agent.status == "Running" {
                                                            ui.spinner();
                                                        } else {
                                                            ui.label(
                                                                egui::RichText::new("✓")
                                                                    .color(egui::Color32::GREEN),
                                                            );
                                                        }
                                                    },
                                                );
                                            });

                                            ui.separator();

                                            // Log Area - The "Matrix" Scroll
                                            egui::ScrollArea::vertical()
                                                .id_salt(format!("log_{}", key))
                                                .stick_to_bottom(true) // The magic auto-scroll
                                                .max_height(80.0) // Compact fixed height
                                                .show(ui, |ui| {
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(&agent.log)
                                                                .family(egui::FontFamily::Monospace)
                                                                .size(10.0) // Small terminal font
                                                                .color(egui::Color32::from_rgb(
                                                                    0, 255, 100,
                                                                )), // Matrix Green!
                                                        )
                                                        .wrap(),
                                                    );
                                                });

                                            // Status Footer
                                            ui.horizontal(|ui| {
                                                if agent.status == "Running" {
                                                    ui.label(
                                                        egui::RichText::new("Active")
                                                            .italics()
                                                            .size(9.0)
                                                            .color(egui::Color32::from_rgb(
                                                                0, 200, 255,
                                                            )),
                                                    );
                                                } else {
                                                    ui.label(
                                                        egui::RichText::new("Completed")
                                                            .strong()
                                                            .color(egui::Color32::GRAY)
                                                            .size(9.0),
                                                    );
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(5.0);
                            }
                        }
                    });
                });
            ui.add_space(5.0);
        });

    // Perform removal outside the UI definition to satisfy borrow checker if needed,
    // though here we are just modifying the HashMap which is passed as mut ref.
    for key in to_remove {
        active_sub_agents.remove(&key);
    }
}
