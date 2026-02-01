use bevy::prelude::*;
use bevy_remote::{http::RemoteHttpPlugin, RemotePlugin}; // Assuming bevy_remote is re-exported or available

/// Add this plugin to your Bevy app to enable remote control via Axiom.
///
/// # Example
/// ```rust
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(BevyAiRemotePlugin)
///     .run();
/// ```
pub struct BevyAiRemotePlugin;

impl Plugin for BevyAiRemotePlugin {
    fn build(&self, app: &mut App) {
        // Ensure RemotePlugin is added if not already
        if !app.is_plugin_added::<RemotePlugin>() {
            app.add_plugins(RemotePlugin::default());
        }
        // Ensure HTTP transport is enabled with correct config
        if !app.is_plugin_added::<RemoteHttpPlugin>() {
            app.add_plugins(
                RemoteHttpPlugin::default()
                    .with_address("127.0.0.1".parse().unwrap())
                    .with_port(15721),
            );
        }

        info!("Bevy AI Remote Plugin initialized on port 15721");

        // TODO: Register `editor/spawn` method here when API is stable.
        // Currently relies on `world.spawn_entity` via BRP.
    }
}
