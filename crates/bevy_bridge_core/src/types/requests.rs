use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    pub components: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadAssetRequest {
    pub filename: String,
    pub data_base64: String,
    pub subdir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawRpcRequest {
    pub method: String,
    pub params: Option<Value>,
}
