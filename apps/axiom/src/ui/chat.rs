use crate::agent::AgentProfile;
use crate::llm::MessageContent;
// use crate::types::{Plan, PlanStatus}; // Removed
use base64::prelude::*;
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;

pub enum ChatAction {
    None,
}

pub fn render_chat(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    chat_history: &Vec<(String, MessageContent)>,
    available_profiles: &[AgentProfile],
    image_textures: &mut HashMap<(usize, usize), egui::TextureHandle>,
) -> ChatAction {
    let action = ChatAction::None;

    ui.vertical(|ui| {
        ui.add_space(10.0);

        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        for (msg_idx, (role, content)) in chat_history.iter().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    // Avatar Logic
                    let role_lower = role.to_lowercase();
                    let avatar_path = if role_lower.contains("cats2333") {
                        "assets/avatars/cat.png".to_string()
                    } else if role_lower.contains("system") || role_lower.contains("error") {
                        "assets/avatars/system.png".to_string()
                    } else if role_lower.contains("brock") || role_lower.contains("小刚") {
                        "assets/avatars/Brock.png".to_string()
                    } else if role_lower.contains("misty") || role_lower.contains("小霞") {
                        "assets/avatars/Misty.png".to_string()
                    } else if role_lower.contains("surge") || role_lower.contains("马志士") {
                        "assets/avatars/Surge.png".to_string()
                    } else if role_lower.contains("erika") || role_lower.contains("莉佳") {
                        "assets/avatars/Erika.png".to_string()
                    } else if role_lower.contains("koga") || role_lower.contains("阿桔") {
                        "assets/avatars/Koga.png".to_string()
                    } else if role_lower.contains("sabrina") || role_lower.contains("娜姿") {
                        "assets/avatars/Sabrina.png".to_string()
                    } else if role_lower.contains("blaine") || role_lower.contains("夏伯") {
                        "assets/avatars/Blaine.png".to_string()
                    } else if role_lower.contains("giovanni") || role_lower.contains("坂木") {
                        "assets/avatars/Giovanni.png".to_string()
                    } else {
                        // Dynamic lookup for Agent profiles
                        if let Some(profile) = available_profiles.iter().find(|p| p.name == *role) {
                            format!("assets/avatars/{}", profile.avatar_path)
                        } else {
                            format!("assets/avatars/{}.png", role)
                        }
                    };

                    let abs_path = current_dir.join(&avatar_path);

                    if abs_path.exists() {
                        let uri = format!(
                            "file://{}",
                            abs_path.display().to_string().replace("\\", "/")
                        );
                        ui.add(
                            egui::Image::from_uri(uri)
                                .fit_to_exact_size(egui::vec2(32.0, 32.0))
                                .rounding(16.0),
                        ); // Circle
                    } else {
                        // Fallback avatar (Bot icon)
                        if role != "System" && role != "Error" {
                            let bot_path = current_dir.join("assets/avatars/bot.png");
                            let uri = format!(
                                "file://{}",
                                bot_path.display().to_string().replace("\\", "/")
                            );
                            ui.add(
                                egui::Image::from_uri(uri)
                                    .fit_to_exact_size(egui::vec2(32.0, 32.0))
                                    .rounding(16.0),
                            );
                        }
                    }

                    // Role Name Colors
                    let role_lower_color = role.to_lowercase();
                    let color = if role_lower_color.contains("cats2333") {
                        egui::Color32::LIGHT_BLUE
                    } else if role_lower_color.contains("system")
                        || role_lower_color.contains("error")
                    {
                        egui::Color32::RED
                    } else if role_lower_color.contains("brock")
                        || role_lower_color.contains("小刚")
                    {
                        egui::Color32::from_rgb(168, 168, 120) // Rock Gray/Brown
                    } else if role_lower_color.contains("misty")
                        || role_lower_color.contains("小霞")
                    {
                        egui::Color32::from_rgb(104, 144, 240) // Water Blue
                    } else if role_lower_color.contains("surge")
                        || role_lower_color.contains("马志士")
                    {
                        egui::Color32::from_rgb(248, 208, 48) // Electric Yellow
                    } else if role_lower_color.contains("erika")
                        || role_lower_color.contains("莉佳")
                    {
                        egui::Color32::from_rgb(120, 200, 80) // Grass Green
                    } else if role_lower_color.contains("koga") || role_lower_color.contains("阿桔")
                    {
                        egui::Color32::from_rgb(160, 64, 160) // Poison Purple
                    } else if role_lower_color.contains("sabrina")
                        || role_lower_color.contains("娜姿")
                    {
                        egui::Color32::from_rgb(248, 88, 136) // Psychic Pink
                    } else if role_lower_color.contains("blaine")
                        || role_lower_color.contains("夏伯")
                    {
                        egui::Color32::from_rgb(240, 128, 48) // Fire Orange
                    } else if role_lower_color.contains("giovanni")
                        || role_lower_color.contains("坂木")
                    {
                        egui::Color32::from_rgb(224, 192, 104) // Ground Brown
                    } else {
                        egui::Color32::from_rgb(255, 105, 180) // Hot Pink for others
                    };
                    ui.label(egui::RichText::new(role).strong().color(color));
                });

                match content {
                    MessageContent::Text(text) => {
                        if role == "System" && text.starts_with("Executing tool: ") {
                            ui.horizontal_wrapped(|ui| {
                                let parts: Vec<&str> = text.splitn(2, " args: ").collect();
                                let name_part = parts[0].trim_start_matches("Executing tool: ");
                                let args_part = if parts.len() > 1 { parts[1] } else { "" };

                                ui.label("Executing tool: ");
                                ui.label(
                                    egui::RichText::new(name_part)
                                        .strong()
                                        .color(egui::Color32::GOLD),
                                );
                                ui.label(" args: ");
                                ui.label(
                                    egui::RichText::new(args_part)
                                        .monospace()
                                        .color(egui::Color32::LIGHT_BLUE),
                                );
                            });
                        } else {
                            ui.label(text);
                        }
                    }
                    MessageContent::Parts(parts) => {
                        for (part_idx, part) in parts.iter().enumerate() {
                            if let Some(text) = &part.text {
                                ui.label(text);
                            }
                            if let Some(image_url) = &part.image_url {
                                let texture_key = (msg_idx, part_idx);

                                // Load texture if not in cache
                                if !image_textures.contains_key(&texture_key) {
                                    // Attempt to decode base64 image
                                    let clean_url = image_url.url.trim();
                                    let base64_data = if let Some(data) =
                                        clean_url.strip_prefix("data:image/png;base64,")
                                    {
                                        Some(data)
                                    } else if let Some(data) =
                                        clean_url.strip_prefix("data:image/jpeg;base64,")
                                    {
                                        Some(data)
                                    } else {
                                        None
                                    };

                                    if let Some(data) = base64_data {
                                        let clean_data: String =
                                            data.chars().filter(|c| !c.is_whitespace()).collect();

                                        if let Ok(bytes) = BASE64_STANDARD.decode(&clean_data) {
                                            if let Ok(img) = image::load_from_memory(&bytes) {
                                                let size =
                                                    [img.width() as usize, img.height() as usize];
                                                let image_buffer = img.to_rgba8();
                                                let pixels = image_buffer.as_flat_samples();
                                                let color_image =
                                                    egui::ColorImage::from_rgba_unmultiplied(
                                                        size,
                                                        pixels.as_slice(),
                                                    );

                                                let texture = ctx.load_texture(
                                                    format!("chat_img_{}_{}", msg_idx, part_idx),
                                                    color_image,
                                                    egui::TextureOptions::default(),
                                                );
                                                image_textures.insert(texture_key, texture);
                                            }
                                        }
                                    }
                                }

                                // Display texture
                                if let Some(texture) = image_textures.get(&texture_key) {
                                    let size = texture.size_vec2();
                                    // Fixed height 80px
                                    let fixed_height = 80.0;
                                    let scale = fixed_height / size.y;
                                    let display_size = size * scale;

                                    ui.add(
                                        egui::Image::new((texture.id(), display_size))
                                            .rounding(5.0),
                                    );
                                } else {
                                    ui.colored_label(egui::Color32::RED, "🖼️ [Image Error]");
                                }
                            }
                        }
                    }
                }
            });
            ui.add_space(5.0);
        }

        // --- Render Active Plan Card (Removed in Single-Agent Mode) ---
    });

    action
}
