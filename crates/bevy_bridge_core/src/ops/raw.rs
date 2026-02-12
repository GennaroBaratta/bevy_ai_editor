use crate::{BrpClient, Result};
use serde_json::Value;

pub async fn raw(_client: &BrpClient, _method: &str, _params: Option<Value>) -> Result<Value> {
    todo!("Implementation in Task 3")
}
