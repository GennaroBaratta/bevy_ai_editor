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
    
    let entity_id = result
        .as_str()
        .ok_or_else(|| crate::BrpError::InvalidResponse("Expected entity ID as string".into()))?
        .to_string();
    
    Ok(SpawnResponse { entity_id })
}
