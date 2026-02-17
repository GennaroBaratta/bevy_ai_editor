use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use bevy::prelude::*;
use bevy_remote::{http::RemoteHttpPlugin, RemotePlugin};
use serde::{Deserialize, Serialize};
#[cfg(feature = "debug_probe")]
use std::cell::UnsafeCell;
use std::fs::File;
use std::io::Write;
use std::path::Path;
#[cfg(feature = "debug_probe")]
use std::sync::atomic::{compiler_fence, AtomicU64, AtomicUsize, Ordering};

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
    // Optional sub-path relative to _remote_cache (e.g., "Textures")
    pub subdir: Option<String>,
}

/// Unified marker for all entities spawned by the Axiom editor.
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component)]
pub struct AxiomSpawned;

#[cfg(feature = "debug_probe")]
pub const AXIOM_DEBUG_SNAPSHOT_CAPACITY: usize = 4096;

#[cfg(feature = "debug_probe")]
#[repr(C)]
pub struct AxiomDebugProbeState {
    pub frame_counter: AtomicU64,
    pub snapshot_len: AtomicUsize,
    pub snapshot_bytes: UnsafeCell<[u8; AXIOM_DEBUG_SNAPSHOT_CAPACITY]>,
}

#[cfg(feature = "debug_probe")]
impl AxiomDebugProbeState {
    const fn new() -> Self {
        Self {
            frame_counter: AtomicU64::new(0),
            snapshot_len: AtomicUsize::new(0),
            snapshot_bytes: UnsafeCell::new([0; AXIOM_DEBUG_SNAPSHOT_CAPACITY]),
        }
    }
}

#[cfg(feature = "debug_probe")]
unsafe impl Sync for AxiomDebugProbeState {}

#[cfg(feature = "debug_probe")]
#[no_mangle]
pub static AXIOM_DEBUG_PROBE_STATE: AxiomDebugProbeState = AxiomDebugProbeState::new();

#[cfg(all(feature = "debug_probe", debug_assertions))]
#[inline(never)]
#[no_mangle]
pub extern "C" fn axiom_debug_safe_point(frame_index: u64, entity_count: u64, snapshot_len: usize) {
    let _ = (frame_index, entity_count, snapshot_len);
    compiler_fence(Ordering::SeqCst);
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
        app.register_type::<AxiomSpawned>();

        // Add systems
        app.add_systems(Update, (spawn_primitives, handle_remote_assets));

        #[cfg(feature = "debug_probe")]
        app.add_systems(Update, debug_probe_safe_point_anchor);

        info!("Bevy AI Remote Plugin initialized on port 15721");
    }
}

#[cfg(feature = "debug_probe")]
fn debug_probe_safe_point_anchor(world: &mut World) {
    let frame_index = AXIOM_DEBUG_PROBE_STATE
        .frame_counter
        .fetch_add(1, Ordering::Relaxed)
        + 1;
    let entity_count = world.entities().len();
    let snapshot = format!(
        "{{\"frame_index\":{},\"entity_count\":{},\"resource_summaries\":[],\"warnings\":[\"resource summaries unavailable in debug probe\"]}}",
        frame_index, entity_count
    );
    let snapshot_len = write_debug_probe_snapshot(snapshot.as_bytes());

    #[cfg(debug_assertions)]
    axiom_debug_safe_point(frame_index, entity_count as u64, snapshot_len);
}

#[cfg(feature = "debug_probe")]
fn write_debug_probe_snapshot(snapshot: &[u8]) -> usize {
    let snapshot_len = snapshot.len().min(AXIOM_DEBUG_SNAPSHOT_CAPACITY);
    unsafe {
        let output = &mut *AXIOM_DEBUG_PROBE_STATE.snapshot_bytes.get();
        output[..snapshot_len].copy_from_slice(&snapshot[..snapshot_len]);
        if snapshot_len < output.len() {
            output[snapshot_len] = 0;
        }
    }
    AXIOM_DEBUG_PROBE_STATE
        .snapshot_len
        .store(snapshot_len, Ordering::Release);
    snapshot_len
}

fn spawn_primitives(
    mut commands: Commands,
    query: Query<(Entity, &AxiomPrimitive), Added<AxiomPrimitive>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, primitive) in query.iter() {
        info!("Hydrating primitive: {:?}", primitive.primitive_type);
        match primitive.primitive_type.to_lowercase().as_str() {
            "cube" => {
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Cuboid::default())),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                    AxiomSpawned,
                ));
            }
            "sphere" => {
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Sphere::default())),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                    AxiomSpawned,
                ));
            }
            "capsule" => {
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Capsule3d::default())),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                    AxiomSpawned,
                ));
            }
            "cylinder" => {
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Cylinder::default())),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                    AxiomSpawned,
                ));
            }
            "cone" => {
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Cone::default())),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                    AxiomSpawned,
                ));
            }
            "torus" => {
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Torus::default())),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                    AxiomSpawned,
                ));
            }
            "plane" => {
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                    AxiomSpawned,
                ));
            }
            "tetrahedron" => {
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Tetrahedron::default())),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                    AxiomSpawned,
                ));
            }
            "cuboid" => {
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(Cuboid::default())),
                    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
                    AxiomSpawned,
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
        let mut cache_dir = Path::new("assets/_remote_cache").to_path_buf();

        // Handle subdirectory if provided
        if let Some(sub) = &asset.subdir {
            if !sub.is_empty() {
                cache_dir = cache_dir.join(sub);
            }
        }

        if !cache_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&cache_dir) {
                error!("Failed to create cache dir {:?}: {}", cache_dir, e);
                continue;
            }
        }

        // 3. Write file to disk
        let file_path = cache_dir.join(&asset.filename);

        // Prevent redundant writes / race conditions for same content
        let mut should_write = true;
        if file_path.exists() {
            if let Ok(existing_bytes) = std::fs::read(&file_path) {
                if existing_bytes == decoded {
                    info!(
                        "File {:?} already exists and matches content. Skipping write.",
                        file_path
                    );
                    should_write = false;
                }
            }
        }

        if should_write {
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
        } else {
            // Touch the file to ensure asset server notices if it's hot reloading?
            // Actually, if content is same, we don't want to trigger reload.
        }

        // 4. Load the asset using AssetServer
        // Note: AssetServer paths are relative to "assets" folder
        // We need to construct the path relative to "assets"
        let mut relative_path_str = "_remote_cache".to_string();
        if let Some(sub) = &asset.subdir {
            if !sub.is_empty() {
                relative_path_str = format!("{}/{}", relative_path_str, sub);
            }
        }
        relative_path_str = format!("{}/{}", relative_path_str, asset.filename);

        // Only load as Scene if it's a model file. If it's a texture, we just write it and stop.
        if asset.filename.ends_with(".glb") || asset.filename.ends_with(".gltf") {
            let scene_path = format!("{}#Scene0", relative_path_str);
            info!("Loading scene from: {}", scene_path);
            let scene_handle: Handle<Scene> = asset_server.load(scene_path);
            // 5. Attach SceneRoot to the entity
            commands
                .entity(entity)
                .insert((SceneRoot(scene_handle), AxiomSpawned));
        } else {
            info!("Saved auxiliary asset (texture/bin), not spawning SceneRoot.");
            // Just cleanup the component so it doesn't stay on the entity forever
            commands.entity(entity).insert(AxiomSpawned);
            commands.entity(entity).remove::<AxiomRemoteAsset>();
            // Also despawn the entity itself if it has no other components, to keep hierarchy clean
            // commands.entity(entity).despawn();
        }
    }
}
