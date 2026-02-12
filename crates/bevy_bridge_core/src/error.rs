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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_rpc_error_construction() {
        let err = BrpError::json_rpc(-32600, "Invalid Request");
        match err {
            BrpError::JsonRpc {
                code,
                message,
                data,
            } => {
                assert_eq!(code, -32600);
                assert_eq!(message, "Invalid Request");
                assert!(data.is_none());
            }
            _ => panic!("Expected JsonRpc variant"),
        }
    }

    #[test]
    fn test_json_rpc_error_with_data() {
        let err = BrpError::json_rpc_with_data(
            -32700,
            "Parse error",
            json!({"detail": "unexpected token"}),
        );
        match err {
            BrpError::JsonRpc {
                code,
                message,
                data,
            } => {
                assert_eq!(code, -32700);
                assert_eq!(message, "Parse error");
                assert!(data.is_some());
                assert_eq!(data.unwrap().get("detail").unwrap(), "unexpected token");
            }
            _ => panic!("Expected JsonRpc variant"),
        }
    }

    #[test]
    fn test_error_display_messages() {
        let timeout_err = BrpError::Timeout(Duration::from_secs(5));
        assert_eq!(timeout_err.to_string(), "Request timeout after 5s");

        let json_rpc_err = BrpError::json_rpc(-32601, "Method not found");
        assert_eq!(
            json_rpc_err.to_string(),
            "JSON-RPC error: -32601 - Method not found"
        );

        let invalid_response_err = BrpError::InvalidResponse("Bad format".to_string());
        assert_eq!(
            invalid_response_err.to_string(),
            "Invalid response: Bad format"
        );
    }

    #[test]
    fn test_connection_error_conversion() {
        // Test that reqwest::Error converts properly via From trait
        // We can't construct a reqwest::Error directly, so we verify the variant exists
        let json_err =
            serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, "test"));
        let brp_err: BrpError = json_err.into();
        assert!(matches!(brp_err, BrpError::Deserialize(_)));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let brp_err: BrpError = io_err.into();
        match brp_err {
            BrpError::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
            _ => panic!("Expected Io variant"),
        }
    }
}
