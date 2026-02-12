use crate::{BrpClient, Result};
use serde_json::Value;

pub async fn raw(client: &BrpClient, method: &str, params: Option<Value>) -> Result<Value> {
    client.send_rpc(method, params).await
}
