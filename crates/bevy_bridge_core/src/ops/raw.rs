use crate::{BrpClient, Result};
use serde_json::Value;

pub async fn raw(client: &BrpClient, method: &str, params: Option<Value>) -> Result<Value> {
    client.send_rpc(method, params).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_raw_does_not_wrap_params() {
        let params = json!({"foo": "bar", "baz": 123});
        
        assert_eq!(params.get("foo").unwrap(), "bar");
        assert_eq!(params.get("baz").unwrap(), 123);
        assert!(params.get("data").is_none());
    }

    #[test]
    fn test_raw_params_passthrough_contract() {
        let nested_params = json!({
            "entity": "0v1#4294967298",
            "components": ["Transform", "Name"]
        });
        
        assert!(nested_params.is_object());
        assert_eq!(nested_params.get("entity").unwrap(), "0v1#4294967298");
        assert!(nested_params.get("components").unwrap().is_array());
    }
}
