use crate::{BrpClient, Result};
use crate::types::UploadResponse;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::json;

pub async fn upload(
    client: &BrpClient,
    filename: &str,
    bytes: &[u8],
    subdir: Option<&str>,
    translation: [f32; 3],
    rotation: [f32; 4],
) -> Result<UploadResponse> {
    let b64_data = BASE64.encode(bytes);
    
    let params = json!({
        "components": {
            "bevy_ai_remote::AxiomRemoteAsset": {
                "filename": filename,
                "data_base64": b64_data,
                "subdir": subdir
            },
            "bevy_transform::components::transform::Transform": {
                "translation": translation,
                "rotation": rotation,
                "scale": [1.0, 1.0, 1.0]
            }
        }
    });
    
    let result = client.send_rpc("world.spawn_entity", Some(params)).await?;
    
    let entity_id = result
        .as_str()
        .ok_or_else(|| crate::BrpError::InvalidResponse("Expected entity ID as string".into()))?
        .to_string();
    
    Ok(UploadResponse { entity_id })
}
