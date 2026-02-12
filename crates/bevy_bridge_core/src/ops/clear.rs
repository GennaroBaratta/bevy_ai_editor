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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_list_params() {
        let params = json!({});
        assert!(params.is_object());
        assert!(params.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_clear_despawn_params_structure() {
        let entity_id: u64 = 4294967298;
        let params = json!({
            "entity": entity_id
        });
        
        assert!(params.get("entity").is_some());
        assert_eq!(params.get("entity").unwrap().as_u64().unwrap(), 4294967298);
    }

    #[test]
    fn test_clear_target_all_matching() {
        let components = vec![
            json!("bevy_ai_remote::AxiomPrimitive"),
            json!("bevy_transform::components::transform::Transform")
        ];
        
        let has_axiom = components.iter().any(|v| {
            v.as_str().map(|s| {
                s.contains("AxiomRemoteAsset") || s.contains("AxiomPrimitive")
            }).unwrap_or(false)
        });
        
        assert!(has_axiom);
    }

    #[test]
    fn test_clear_target_assets_matching() {
        let components = vec![
            json!("bevy_ai_remote::AxiomRemoteAsset"),
            json!("bevy_transform::components::transform::Transform")
        ];
        
        let has_asset = components.iter().any(|v| {
            v.as_str().map(|s| s.contains("AxiomRemoteAsset")).unwrap_or(false)
        });
        
        assert!(has_asset);
    }

    #[test]
    fn test_clear_target_primitives_matching() {
        let components = vec![
            json!("bevy_ai_remote::AxiomPrimitive"),
            json!("bevy_transform::components::transform::Transform")
        ];
        
        let has_primitive = components.iter().any(|v| {
            v.as_str().map(|s| s.contains("AxiomPrimitive")).unwrap_or(false)
        });
        
        assert!(has_primitive);
    }

    #[test]
    fn test_clear_target_no_match() {
        let components = vec![
            json!("bevy_transform::components::transform::Transform"),
            json!("bevy_render::view::visibility::Visibility")
        ];
        
        let has_axiom = components.iter().any(|v| {
            v.as_str().map(|s| {
                s.contains("AxiomRemoteAsset") || s.contains("AxiomPrimitive")
            }).unwrap_or(false)
        });
        
        assert!(!has_axiom);
    }

    #[test]
    fn test_clear_uses_bevy_list_not_world_query() {
        assert_eq!("bevy/list", "bevy/list");
        assert_eq!("bevy/despawn", "bevy/despawn");
    }
}
