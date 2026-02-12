use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub entity_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnResponse {
    pub entity_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearResponse {
    pub entities_removed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub entities: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResponse {
    pub alive: bool,
    pub methods: Value,
}
