use crate::{BrpClient, Result};
use crate::types::SpawnResponse;
use serde_json::json;

pub async fn spawn(
    client: &BrpClient,
    primitive_type: &str,
    position: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
) -> Result<SpawnResponse> {
    let params = json!({
        "components": {
            "bevy_ai_remote::AxiomPrimitive": {
                "primitive_type": primitive_type
            },
            "bevy_transform::components::transform::Transform": {
                "translation": position,
                "rotation": rotation,
                "scale": scale
            }
        }
    });
    
    let result = client.send_rpc("world.spawn_entity", Some(params)).await?;
    
    let entity_id = result.get("entity")
        .ok_or_else(|| crate::BrpError::InvalidResponse(
            "Missing 'entity' in spawn response".into()
        ))?
        .to_string();
    
    Ok(SpawnResponse { entity_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_params_structure() {
        let params = json!({
            "components": {
                "bevy_ai_remote::AxiomPrimitive": {
                    "primitive_type": "Cube"
                },
                "bevy_transform::components::transform::Transform": {
                    "translation": [1.0, 2.0, 3.0],
                    "rotation": [0.0, 0.0, 0.0, 1.0],
                    "scale": [1.0, 1.0, 1.0]
                }
            }
        });
        
        assert!(params.get("components").is_some());
        assert!(params.get("components").unwrap().get("bevy_ai_remote::AxiomPrimitive").is_some());
        assert!(params.get("components").unwrap().get("bevy_transform::components::transform::Transform").is_some());
    }

    #[test]
    fn test_spawn_axiom_primitive_component() {
        let params = json!({
            "components": {
                "bevy_ai_remote::AxiomPrimitive": {
                    "primitive_type": "Sphere"
                }
            }
        });
        
        let axiom_primitive = params.get("components").unwrap().get("bevy_ai_remote::AxiomPrimitive").unwrap();
        assert_eq!(axiom_primitive.get("primitive_type").unwrap(), "Sphere");
    }

    #[test]
    fn test_spawn_transform_component() {
        let params = json!({
            "components": {
                "bevy_transform::components::transform::Transform": {
                    "translation": [10.0, 20.0, 30.0],
                    "rotation": [0.0, 0.7071068, 0.0, 0.7071068],
                    "scale": [2.0, 2.0, 2.0]
                }
            }
        });
        
        let transform = params.get("components").unwrap()
            .get("bevy_transform::components::transform::Transform").unwrap();
        
        assert_eq!(transform.get("translation").unwrap(), &json!([10.0, 20.0, 30.0]));
        assert_eq!(transform.get("rotation").unwrap(), &json!([0.0, 0.7071068, 0.0, 0.7071068]));
        assert_eq!(transform.get("scale").unwrap(), &json!([2.0, 2.0, 2.0]));
    }

    #[test]
    fn test_spawn_component_keys_exact_format() {
        let params = json!({
            "components": {
                "bevy_ai_remote::AxiomPrimitive": {"primitive_type": "Plane"},
                "bevy_transform::components::transform::Transform": {
                    "translation": [0.0, 0.0, 0.0],
                    "rotation": [0.0, 0.0, 0.0, 1.0],
                    "scale": [1.0, 1.0, 1.0]
                }
            }
        });
        
        let components = params.get("components").unwrap();
        assert!(components.as_object().unwrap().contains_key("bevy_ai_remote::AxiomPrimitive"));
        assert!(components.as_object().unwrap().contains_key("bevy_transform::components::transform::Transform"));
    }
}
