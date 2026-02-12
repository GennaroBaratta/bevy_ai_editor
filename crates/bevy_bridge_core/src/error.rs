use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BrpError {
    #[error("Connection error: {0}")]
    Connection(#[from] reqwest::Error),

    #[error("Request timeout after {0:?}")]
    Timeout(Duration),

    #[error("JSON-RPC error: {code} - {message}")]
    JsonRpc {
        code: i32,
        message: String,
        data: Option<serde_json::Value>,
    },

    #[error("Deserialization error: {0}")]
    Deserialize(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

impl BrpError {
    pub fn json_rpc(code: i32, message: impl Into<String>) -> Self {
        Self::JsonRpc {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn json_rpc_with_data(
        code: i32,
        message: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self::JsonRpc {
            code,
            message: message.into(),
            data: Some(data),
        }
    }
}
