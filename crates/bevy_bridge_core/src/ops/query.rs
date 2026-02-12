use crate::{BrpClient, Result};
use crate::types::QueryResponse;
use serde_json::json;

pub async fn query(client: &BrpClient, components: Vec<String>) -> Result<QueryResponse> {
    let params = json!({
        "data": {
            "components": components
        }
    });
    
    let result = client.send_rpc("world.query", Some(params)).await?;
    
    let entities = result
        .as_array()
        .ok_or_else(|| crate::BrpError::InvalidResponse("Expected array from world.query".into()))?
        .clone();
    
    Ok(QueryResponse { entities })
}
