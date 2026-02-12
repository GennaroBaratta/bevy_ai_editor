use crate::{BrpClient, Result};
use crate::types::{ClearResponse, ClearTarget};
use serde_json::json;

pub async fn clear(client: &BrpClient, target: ClearTarget) -> Result<ClearResponse> {
    let list_result = client.send_rpc("bevy/list", Some(json!({}))).await?;
    
    let entities = list_result
        .as_array()
        .ok_or_else(|| crate::BrpError::InvalidResponse("Expected array from bevy/list".into()))?;
    
    let mut count = 0;
    
    for entity_info in entities {
        if let Some(entity_id) = entity_info.get("entity").and_then(|e| e.as_u64()) {
            let components = entity_info.get("components").and_then(|c| c.as_array());
            
            let should_despawn = match target {
                ClearTarget::All => {
                    components.map(|c| {
                        c.iter().any(|v| {
                            v.as_str().map(|s| {
                                s.contains("AxiomRemoteAsset") || s.contains("AxiomPrimitive")
                            }).unwrap_or(false)
                        })
                    }).unwrap_or(false)
                }
                ClearTarget::Assets => {
                    components.map(|c| {
                        c.iter().any(|v| {
                            v.as_str().map(|s| s.contains("AxiomRemoteAsset")).unwrap_or(false)
                        })
                    }).unwrap_or(false)
                }
                ClearTarget::Primitives => {
                    components.map(|c| {
                        c.iter().any(|v| {
                            v.as_str().map(|s| s.contains("AxiomPrimitive")).unwrap_or(false)
                        })
                    }).unwrap_or(false)
                }
            };
            
            if should_despawn {
                let despawn_params = json!({
                    "entity": entity_id
                });
                
                let _ = client.send_rpc("bevy/despawn", Some(despawn_params)).await;
                count += 1;
            }
        }
    }
    
    Ok(ClearResponse { entities_removed: count })
}
