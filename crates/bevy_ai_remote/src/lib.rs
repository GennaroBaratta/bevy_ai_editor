use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use bevy::prelude::*;
use bevy_remote::{http::RemoteHttpPlugin, RemotePlugin};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Component to tag entities that should be rendered as a primitive shape.
#[derive(Component, Reflect, Default, Debug, Serialize, Deserialize)]
#[reflect(Component)]
pub struct AxiomPrimitive {
    pub primitive_type: String,
}

/// Component to receive a Base64 encoded asset file from the Editor.
/// usage: spawn an entity with this component. The system will write the file
/// to `assets/_remote_cache/` and then attach a SceneRoot to the entity.
#[derive(Component, Reflect, Default, Debug, Serialize, Deserialize)]
#[reflect(Component)]
pub struct AxiomRemoteAsset {
    pub filename: String,
    pub data_base64: String,
}

/// Add this plugin to your Bevy app to enable remote control via Axiom.
pub struct BevyAiRemotePlugin;

impl Plugin for BevyAiRemotePlugin {
    fn build(&self, app: &mut App) {
        // Ensure RemotePlugin is added if not already
        if !app.is_plugin_added::<RemotePlugin>() {
            app.add_plugins(RemotePlugin::default());
        }

        use std::net::IpAddr;

        // Ensure HTTP transport is enabled with correct config
        if !app.is_plugin_added::<RemoteHttpPlugin>() {
            app.add_plugins(
                RemoteHttpPlugin::default()
                    .with_address("127.0.0.1".parse::<IpAddr>().unwrap())
                    .with_port(15721),
            );
        }

        // Register our custom components
        app.register_type::<AxiomPrimitive>();
        app.register_type::<AxiomRemoteAsset>();

        // Add systems
        app.add_systems(Update, (spawn_primitives, handle_remote_assets));

        info!("Bevy AI Remote Plugin initialized on port 15721");
    }
}

fn spawn_primitives(
    mut commands: Commands,
    query: Query<(Entity, &AxiomPrimitive), Added<AxiomPrimitive>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, primitive) in query.iter() {
        info!("Hydrating primitive: {:?}", primitive.primitive_type);
        match primitive.primitive_type.as_str() {
            "cube" => {
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Cuboid::default())),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                ));
            }
            "sphere" => {
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Sphere::default())),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                ));
            }
            _ => {
                warn!("Unknown primitive type: {}", primitive.primitive_type);
            }
        }
    }
}

fn handle_remote_assets(
    mut commands: Commands,
    query: Query<(Entity, &AxiomRemoteAsset), Added<AxiomRemoteAsset>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, asset) in query.iter() {
        info!("Receiving remote asset: {}", asset.filename);

        // 1. Decode Base64
        let decoded = match BASE64.decode(&asset.data_base64) {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to decode base64 for {}: {}", asset.filename, e);
                continue;
            }
        };

        // 2. Ensure cache directory exists
        let cache_dir = Path::new("assets/_remote_cache");
        if !cache_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(cache_dir) {
                error!("Failed to create cache dir: {}", e);
                continue;
            }
        }

        // 3. Write file to disk
        let file_path = cache_dir.join(&asset.filename);
        let mut file = match File::create(&file_path) {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to create file {:?}: {}", file_path, e);
                continue;
            }
        };

        if let Err(e) = file.write_all(&decoded) {
            error!("Failed to write file {:?}: {}", file_path, e);
            continue;
        }

        info!("Saved remote asset to {:?}", file_path);

        // 4. Load the asset using AssetServer
        // Note: AssetServer paths are relative to "assets" folder
        let relative_path = format!("_remote_cache/{}#Scene0", asset.filename);
        let scene_handle: Handle<Scene> = asset_server.load(relative_path);

        // 5. Attach SceneRoot to the entity
        commands.entity(entity).insert(SceneRoot(scene_handle));

        // Optional: Remove the raw data component to save memory, or keep it for debugging
        // commands.entity(entity).remove::<AxiomRemoteAsset>();
    }
}
