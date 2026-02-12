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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encoding() {
        let bytes = b"test data";
        let encoded = BASE64.encode(bytes);
        assert_eq!(encoded, "dGVzdCBkYXRh");
    }

    #[test]
    fn test_base64_encoding_empty() {
        let bytes = b"";
        let encoded = BASE64.encode(bytes);
        assert_eq!(encoded, "");
    }

    #[test]
    fn test_base64_encoding_binary() {
        let bytes = vec![0x00, 0x01, 0x02, 0xFF, 0xFE];
        let encoded = BASE64.encode(&bytes);
        assert_eq!(encoded, "AAEC//4=");
    }

    #[test]
    fn test_upload_params_structure() {
        let b64_data = "dGVzdCBkYXRh";
        let params = json!({
            "components": {
                "bevy_ai_remote::AxiomRemoteAsset": {
                    "filename": "test.glb",
                    "data_base64": b64_data,
                    "subdir": "models"
                },
                "bevy_transform::components::transform::Transform": {
                    "translation": [0.0, 0.0, 0.0],
                    "rotation": [0.0, 0.0, 0.0, 1.0],
                    "scale": [1.0, 1.0, 1.0]
                }
            }
        });
        
        assert!(params.get("components").is_some());
        assert!(params.get("components").unwrap().get("bevy_ai_remote::AxiomRemoteAsset").is_some());
        assert!(params.get("components").unwrap().get("bevy_transform::components::transform::Transform").is_some());
    }

    #[test]
    fn test_upload_axiom_remote_asset_component() {
        let params = json!({
            "components": {
                "bevy_ai_remote::AxiomRemoteAsset": {
                    "filename": "model.glb",
                    "data_base64": "abc123",
                    "subdir": "uploads"
                }
            }
        });
        
        let asset = params.get("components").unwrap().get("bevy_ai_remote::AxiomRemoteAsset").unwrap();
        assert_eq!(asset.get("filename").unwrap(), "model.glb");
        assert_eq!(asset.get("data_base64").unwrap(), "abc123");
        assert_eq!(asset.get("subdir").unwrap(), "uploads");
    }

    #[test]
    fn test_upload_with_none_subdir() {
        let params = json!({
            "components": {
                "bevy_ai_remote::AxiomRemoteAsset": {
                    "filename": "test.png",
                    "data_base64": "base64data",
                    "subdir": None::<String>
                }
            }
        });
        
        let asset = params.get("components").unwrap().get("bevy_ai_remote::AxiomRemoteAsset").unwrap();
        assert!(asset.get("subdir").unwrap().is_null());
    }

    #[test]
    fn test_upload_scale_always_one() {
        let params = json!({
            "components": {
                "bevy_transform::components::transform::Transform": {
                    "translation": [1.0, 2.0, 3.0],
                    "rotation": [0.0, 0.0, 0.0, 1.0],
                    "scale": [1.0, 1.0, 1.0]
                }
            }
        });
        
        let transform = params.get("components").unwrap()
            .get("bevy_transform::components::transform::Transform").unwrap();
        
        assert_eq!(transform.get("scale").unwrap(), &json!([1.0, 1.0, 1.0]));
    }
}
