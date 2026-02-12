use crate::{BrpConfig, BrpError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{atomic::{AtomicU64, Ordering}, Arc};

#[derive(Debug, Clone)]
pub struct BrpClient {
    config: BrpConfig,
    http_client: reqwest::Client,
    request_id: Arc<AtomicU64>,
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    #[serde(flatten)]
    result_or_error: ResultOrError,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ResultOrError {
    Result { result: Value },
    Error { error: JsonRpcError },
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(default)]
    data: Option<Value>,
}

impl BrpClient {
    pub fn new(config: BrpConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            config,
            http_client,
            request_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub async fn send_rpc(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.request_id.fetch_add(1, Ordering::Relaxed);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            id,
            params,
        };

        tracing::debug!("Sending JSON-RPC request: method={}, id={}", method, id);

        let response = self
            .http_client
            .post(&self.config.endpoint)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(BrpError::InvalidResponse(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let json_response: JsonRpcResponse = response.json().await?;

        if json_response.id != id {
            return Err(BrpError::InvalidResponse(format!(
                "Response ID mismatch: expected {}, got {}",
                id, json_response.id
            )));
        }

        match json_response.result_or_error {
            ResultOrError::Result { result } => {
                tracing::debug!("JSON-RPC request successful: method={}, id={}", method, id);
                Ok(result)
            }
            ResultOrError::Error { error } => {
                tracing::warn!(
                    "JSON-RPC error: code={}, message={}",
                    error.code,
                    error.message
                );
                Err(BrpError::JsonRpc {
                    code: error.code,
                    message: error.message,
                    data: error.data,
                })
            }
        }
    }

    pub fn config(&self) -> &BrpConfig {
        &self.config
    }
}

impl Default for BrpClient {
    fn default() -> Self {
        Self::new(BrpConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = BrpConfig::default();
        let client = BrpClient::new(config.clone());
        assert_eq!(client.config().endpoint, config.endpoint);
        assert_eq!(client.config().timeout, config.timeout);
    }

    #[test]
    fn test_default_client() {
        let client = BrpClient::default();
        assert_eq!(client.config().endpoint, "http://127.0.0.1:15721");
    }

    #[test]
    fn test_request_id_increment() {
        let client = BrpClient::default();
        assert_eq!(client.request_id.fetch_add(1, Ordering::Relaxed), 1);
        assert_eq!(client.request_id.fetch_add(1, Ordering::Relaxed), 2);
        assert_eq!(client.request_id.fetch_add(1, Ordering::Relaxed), 3);
    }
}
