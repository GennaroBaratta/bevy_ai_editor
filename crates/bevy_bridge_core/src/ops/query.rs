use crate::{BrpClient, Result};
use serde_json::Value;

pub async fn query(_client: &BrpClient, _query: &str) -> Result<Vec<Value>> {
    todo!("Implementation in Task 3")
}
