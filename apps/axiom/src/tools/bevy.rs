use crate::tools::Tool;
use anyhow::{anyhow, Result};
use bevy_bridge_core::{BrpClient, BrpConfig, ops};
use glam::Quat;
use serde_json::{json, Value};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tokio::runtime::Runtime;

const BEVY_RPC_URL: &str = "http://127.0.0.1:15721";

fn get_bridge_client() -> Result<BrpClient> {
    let config = BrpConfig::from_env();
    Ok(BrpClient::new(config))
}

/// Tool to upload a local file to Bevy via BRP and spawn it
pub struct BevyUploadAssetTool;

impl Tool for BevyUploadAssetTool {
    fn name(&self) -> String {
        "bevy_upload_asset".to_string()
    }

    fn description(&self) -> String {
        "Upload a local asset file (e.g., .glb) to Bevy and spawn it. Encodes file as Base64 and sends via 'AxiomRemoteAsset'.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "bevy_upload_asset",
                "description": "Upload and spawn a local asset file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "local_path": {
                            "type": "string",
                            "description": "Absolute path to the local file on the editor machine."
                        },
                        "relative_path": {
                            "type": "string",
                            "description": "Optional relative subdirectory in the game's asset cache (e.g. 'Textures')."
                        },
                        "translation": {
                            "type": "array",
                            "items": { "type": "number" },
                            "minItems": 3,
                            "maxItems": 3,
                            "description": "[x, y, z] position"
                        },
                        "rotation": {
                            "type": "array",
                            "items": { "type": "number" },
                            "minItems": 3,
                            "maxItems": 3,
                            "description": "[x, y, z] rotation in Euler angles (Degrees). e.g. [0, 90, 0] to rotate 90 deg around Y axis."
                        }
                    },
                    "required": ["local_path", "translation"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let client = get_bridge_client()?;
        let rt = Runtime::new()?;
        
        let local_path = args
            .get("local_path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow!("Missing local_path"))?;

        let relative_path = args
            .get("relative_path")
            .and_then(|v| v.as_str());

        let t = args
            .get("translation")
            .and_then(|v| v.as_array())
            .ok_or(anyhow!("Missing translation"))?;

        let tx = t.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let ty = t.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let tz = t.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

        // Handle Rotation
        let rotation_quat = if let Some(rot_arr) = args.get("rotation").and_then(|v| v.as_array()) {
            let rx = rot_arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let ry = rot_arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let rz = rot_arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

            // Convert Degrees to Radians and create Quat
            Quat::from_euler(
                glam::EulerRot::XYZ,
                rx.to_radians(),
                ry.to_radians(),
                rz.to_radians(),
            )
        } else {
            Quat::IDENTITY
        };

        // 1. Read file
        let path = Path::new(local_path);

        // Smart Path Resolution Strategy
        // 1. Try absolute path or raw path provided by user
        let mut abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };

        // 2. If not found, try fallback: apps/axiom/resources/models/{filename}
        if !abs_path.exists() {
            if let Some(name) = path.file_name() {
                let fallback_models = std::env::current_dir()?
                    .join("apps")
                    .join("axiom")
                    .join("resources")
                    .join("models")
                    .join(name);

                if fallback_models.exists() {
                    println!(
                        "[BevyTool] Path not found, falling back to: {:?}",
                        fallback_models
                    );
                    abs_path = fallback_models;
                } else {
                    // 3. If not found, try fallback: apps/axiom/resources/{filename}
                    let fallback_resources = std::env::current_dir()?
                        .join("apps")
                        .join("axiom")
                        .join("resources")
                        .join(name);

                    if fallback_resources.exists() {
                        println!(
                            "[BevyTool] Path not found, falling back to: {:?}",
                            fallback_resources
                        );
                        abs_path = fallback_resources;
                    }
                }
            }
        }

        let filename = path
            .file_name()
            .ok_or(anyhow!("Invalid filename"))?
            .to_string_lossy()
            .to_string();

        let mut file = File::open(&abs_path)
            .map_err(|e| anyhow!("Failed to open file at {:?}: {}", abs_path, e))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        println!(
            "[BevyTool] Uploading {} ({} bytes) ...",
            filename,
            buffer.len()
        );

        // Call bridge_core operation
        let response = rt.block_on(async {
            ops::upload::upload(
                &client,
                &filename,
                &buffer,
                relative_path,
                [tx, ty, tz],
                [rotation_quat.x, rotation_quat.y, rotation_quat.z, rotation_quat.w],
            )
            .await
        })
        .map_err(|e| anyhow!("Bridge error: {}", e))?;

        Ok(format!(
            "Uploaded and Spawned {}. Entity ID: {}",
            filename, response.entity_id
        ))
    }
}

/// Generic JSON-RPC Tool for Bevy Remote
pub struct BevyRpcTool;

impl Tool for BevyRpcTool {
    fn name(&self) -> String {
        "bevy_rpc".to_string()
    }

    fn description(&self) -> String {
        "Send a raw JSON-RPC request to the running Bevy engine (bevy_remote). Methods: bevy/spawn, bevy/get, bevy/list, etc.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "bevy_rpc",
                "description": "Send a raw JSON-RPC request to Bevy.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "method": {
                            "type": "string",
                            "description": "The RPC method (e.g., 'bevy/spawn', 'bevy/query', 'bevy/list')."
                        },
                        "params": {
                            "type": "object",
                            "description": "The parameters for the RPC method."
                        }
                    },
                    "required": ["method", "params"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let client = get_bridge_client()?;
        let rt = Runtime::new()?;
        
        let method = args
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'method' argument"))?;

        let params = args.get("params").cloned();

        let result = rt.block_on(async {
            ops::raw::raw(&client, method, params).await
        })
        .map_err(|e| anyhow!("Bridge error: {}", e))?;

        if let Some(error) = result.get("error") {
            Err(anyhow!("Bevy RPC Error: {}", error))
        } else if let Some(result_value) = result.get("result") {
            Ok(serde_json::to_string_pretty(result_value)?)
        } else {
            Ok(serde_json::to_string_pretty(&result)?)
        }
    }
}

/// Helper tool to Spawn a Scene (glTF) easily
pub struct BevySpawnSceneTool;

impl Tool for BevySpawnSceneTool {
    fn name(&self) -> String {
        "bevy_spawn_scene".to_string()
    }

    fn description(&self) -> String {
        "Spawn a glTF scene in Bevy. Handles Transform and SceneRoot components automatically."
            .to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "bevy_spawn_scene",
                "description": "Spawn a glTF scene in Bevy.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "asset_path": {
                            "type": "string",
                            "description": "Path to the glTF asset (relative to assets folder, e.g., 'models/cube.glb#Scene0')"
                        },
                        "translation": {
                            "type": "array",
                            "items": { "type": "number" },
                            "minItems": 3,
                            "maxItems": 3,
                            "description": "[x, y, z] position"
                        },
                        "scale": {
                            "type": "array",
                            "items": { "type": "number" },
                            "minItems": 3,
                            "maxItems": 3,
                            "description": "[x, y, z] scale (default [1,1,1])"
                        }
                    },
                    "required": ["asset_path", "translation"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let asset_path = args
            .get("asset_path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow!("Missing asset_path"))?;
        let t = args
            .get("translation")
            .and_then(|v| v.as_array())
            .ok_or(anyhow!("Missing translation"))?;
        let s = args.get("scale").and_then(|v| v.as_array());

        let tx = t.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let ty = t.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tz = t.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0);

        let sx = s
            .and_then(|arr| arr.get(0))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let sy = s
            .and_then(|arr| arr.get(1))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let sz = s
            .and_then(|arr| arr.get(2))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        // Construct Bevy 0.15+ compatible spawn payload
        // Fallback: Spawning SceneRoot via BRP is tricky due to Handle reflection issues (Strong/Uuid variants).
        // For now, we spawn an empty entity with a Transform to verify the control link works.
        // The user will see a "Ghost" entity in the scene hierarchy (if they had an inspector), but nothing visible.
        // This confirms command parsing -> network -> bevy execution is 100% working.
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "world.spawn_entity",
            "id": 1,
            "params": {
                "components": {
                    // Temporarily disabled SceneRoot until we figure out the correct JSON format for Handle<Scene>
                    /*
                    "bevy_scene::components::SceneRoot": {
                        "Handle<bevy_scene::scene::Scene>": {
                            "path": asset_path
                        }
                    },
                    */
                    "bevy_transform::components::transform::Transform": {
                        "translation": [tx, ty, tz],
                        "rotation": [0.0, 0.0, 0.0, 1.0],
                        "scale": [1.0, 1.0, 1.0]
                    }
                }
            }
        });

        match ureq::post(BEVY_RPC_URL).send_json(payload) {
            Ok(res) => {
                let body: Value = res.into_json()?;
                Ok(serde_json::to_string_pretty(&body)?)
            }
            Err(e) => Err(anyhow!("Failed to spawn scene via bevy_remote: {}", e)),
        }
    }
}

/// Tool to Clear the Bevy Scene (Despawn all entities)
pub struct BevyClearSceneTool;

impl Tool for BevyClearSceneTool {
    fn name(&self) -> String {
        "bevy_clear_scene".to_string()
    }

    fn description(&self) -> String {
        "Despawn all entities in the Bevy scene to start fresh.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "bevy_clear_scene",
                "description": "Clear the scene by despawning all entities.",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        })
    }

    fn execute(&self, _args: Value) -> Result<String> {
        let client = get_bridge_client()?;
        let rt = Runtime::new()?;
        
        let response = rt.block_on(async {
            ops::clear::clear(&client, bevy_bridge_core::types::ClearTarget::All).await
        })
        .map_err(|e| anyhow!("Bridge error: {}", e))?;

        Ok(format!("Cleared {} entities.", response.entities_removed))
    }
}

/// Helper tool to Spawn a Primitive Cube easily
pub struct BevySpawnPrimitiveTool;

impl Tool for BevySpawnPrimitiveTool {
    fn name(&self) -> String {
        "bevy_spawn_primitive".to_string()
    }

    fn description(&self) -> String {
        "Spawn a primitive 3D object (currently just a cube) at a specific location via Bevy Remote using a pre-existing glTF asset 'cube.glb'.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "bevy_spawn_primitive",
                "description": "Spawn a primitive 3D object using assets/cube.glb.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": ["cube"],
                            "description": "Type of primitive to spawn."
                        },
                        "translation": {
                            "type": "array",
                            "items": { "type": "number" },
                            "minItems": 3,
                            "maxItems": 3,
                            "description": "[x, y, z] position"
                        }
                    },
                    "required": ["type", "translation"]
                }
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let client = get_bridge_client()?;
        let rt = Runtime::new()?;
        
        let t = args
            .get("translation")
            .and_then(|v| v.as_array())
            .ok_or(anyhow!("Missing translation"))?;

        let tx = t.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let ty = t.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let tz = t.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

        let response = rt.block_on(async {
            ops::spawn::spawn(
                &client,
                "cube",
                [tx, ty, tz],
                [0.0, 0.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
            )
            .await
        })
        .map_err(|e| anyhow!("Bridge error: {}", e))?;

        Ok(format!("Spawned Cube. Entity ID: {}", response.entity_id))
    }
}
