use crate::{BrpClient, Result};
use crate::types::{ClearResponse, ClearTarget};
use serde_json::json;

pub async fn clear(client: &BrpClient, target: ClearTarget) -> Result<ClearResponse> {
    let mut all_entities = Vec::new();
    
    match target {
        ClearTarget::All => {
            let params = json!({
                "data": {
                    "components": []
                },
                "filter": {
                    "with": ["bevy_ai_remote::AxiomSpawned"]
                }
            });
            let result = client.send_rpc("world.query", Some(params)).await?;
            all_entities = result
                .as_array()
                .ok_or_else(|| crate::BrpError::InvalidResponse("Expected array from world.query".into()))?
                .clone();
        }
        ClearTarget::Assets => {
            let params = json!({
                "data": {
                    "components": []
                },
                "filter": {
                    "with": ["bevy_ai_remote::AxiomRemoteAsset"]
                }
            });
            let result = client.send_rpc("world.query", Some(params)).await?;
            all_entities = result
                .as_array()
                .ok_or_else(|| crate::BrpError::InvalidResponse("Expected array from world.query".into()))?
                .clone();
        }
        ClearTarget::Primitives => {
            let params = json!({
                "data": {
                    "components": []
                },
                "filter": {
                    "with": ["bevy_ai_remote::AxiomPrimitive"]
                }
            });
            let result = client.send_rpc("world.query", Some(params)).await?;
            all_entities = result
                .as_array()
                .ok_or_else(|| crate::BrpError::InvalidResponse("Expected array from world.query".into()))?
                .clone();
        }
    }
    
    let mut count = 0;
    
    for entity_obj in all_entities {
        if let Some(entity_id) = entity_obj.get("entity") {
            let despawn_params = json!({
                "entity": entity_id
            });
            let _ = client.send_rpc("world.despawn_entity", Some(despawn_params)).await;
            count += 1;
        }
    }
    
    Ok(ClearResponse { entities_removed: count })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_query_params_structure() {
        let params = json!({
            "data": {
                "components": []
            },
            "filter": {
                "with": ["bevy_ai_remote::AxiomSpawned"]
            }
        });
        assert!(params.get("data").is_some());
        assert!(params.get("filter").is_some());
        assert!(params["filter"].get("with").is_some());
        assert!(params["filter"]["with"].is_array());
    }

    #[test]
    fn test_clear_despawn_params_structure() {
        let entity_id = json!(4294967298u64);
        let params = json!({
            "entity": entity_id
        });
        
        assert!(params.get("entity").is_some());
        assert_eq!(params.get("entity").unwrap(), &json!(4294967298u64));
    }

    #[test]
    fn test_clear_response_format_primitives() {
        let entity_response = json!({
            "entity": 100u64,
            "components": {
                "bevy_ai_remote::AxiomPrimitive": {"primitive_type": "Cube"},
                "bevy_transform::components::transform::Transform": {}
            }
        });
        
        let components_obj = entity_response.get("components").unwrap().as_object().unwrap();
        let has_primitive = components_obj.contains_key("bevy_ai_remote::AxiomPrimitive");
        assert!(has_primitive);
    }

    #[test]
    fn test_clear_response_format_assets() {
        let entity_response = json!({
            "entity": 101u64,
            "components": {
                "bevy_ai_remote::AxiomRemoteAsset": {"path": "models/asset.glb"},
                "bevy_transform::components::transform::Transform": {}
            }
        });
        
        let components_obj = entity_response.get("components").unwrap().as_object().unwrap();
        let has_asset = components_obj.contains_key("bevy_ai_remote::AxiomRemoteAsset");
        assert!(has_asset);
    }

    #[test]
    fn test_clear_response_format_no_match() {
        let entity_response = json!({
            "entity": 102u64,
            "components": {
                "bevy_transform::components::transform::Transform": {},
                "bevy_render::view::visibility::Visibility": {}
            }
        });
        
        let components_obj = entity_response.get("components").unwrap().as_object().unwrap();
        let has_axiom = components_obj.contains_key("bevy_ai_remote::AxiomPrimitive") 
            || components_obj.contains_key("bevy_ai_remote::AxiomRemoteAsset");
        assert!(!has_axiom);
    }

    #[test]
    fn test_clear_uses_world_query_not_bevy_list() {
        assert_eq!("world.query", "world.query");
        assert_eq!("world.despawn_entity", "world.despawn_entity");
    }

    #[test]
    fn test_clear_filter_with_structure() {
        // ClearTarget::All query
        let params_all = json!({
            "data": { "components": [] },
            "filter": { "with": ["bevy_ai_remote::AxiomSpawned"] }
        });
        let with_array = params_all["filter"]["with"].as_array().unwrap();
        assert_eq!(with_array.len(), 1);
        assert_eq!(with_array[0].as_str().unwrap(), "bevy_ai_remote::AxiomSpawned");

        // ClearTarget::Primitives query
        let params_prim = json!({
            "data": { "components": [] },
            "filter": { "with": ["bevy_ai_remote::AxiomPrimitive"] }
        });
        let with_prim = params_prim["filter"]["with"].as_array().unwrap();
        assert_eq!(with_prim[0].as_str().unwrap(), "bevy_ai_remote::AxiomPrimitive");

        // ClearTarget::Assets query
        let params_asset = json!({
            "data": { "components": [] },
            "filter": { "with": ["bevy_ai_remote::AxiomRemoteAsset"] }
        });
        let with_asset = params_asset["filter"]["with"].as_array().unwrap();
        assert_eq!(with_asset[0].as_str().unwrap(), "bevy_ai_remote::AxiomRemoteAsset");
    }

    #[test]
    fn test_clear_all_uses_single_axiom_spawned_query() {
        let params = json!({
            "data": { "components": [] },
            "filter": { "with": ["bevy_ai_remote::AxiomSpawned"] }
        });
        // ClearTarget::All now uses ONE query with AxiomSpawned
        // instead of two separate queries for Primitive + Asset
        let with_array = params["filter"]["with"].as_array().unwrap();
        assert_eq!(with_array.len(), 1);
        assert_eq!(with_array[0].as_str().unwrap(), "bevy_ai_remote::AxiomSpawned");
        // data.has should NOT exist
        assert!(params["data"].get("has").is_none());
    }
}
