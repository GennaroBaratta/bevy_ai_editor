use serde::{Deserialize, Serialize};

/// Request to upload an asset with base64-encoded data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadRequest {
    pub asset_id: String,
    pub bytes: Vec<u8>,
}

/// Request to spawn a primitive object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    pub primitive: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

/// Target for clear operation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ClearTarget {
    All,
    Assets,
    Primitives,
}

/// Request to clear entities from the scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearRequest {
    pub target: ClearTarget,
}

/// Request to query entities by component types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub components: Vec<String>,
}
