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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_wraps_params_in_data_key() {
        let components = vec!["Component1".to_string(), "Component2".to_string()];
        
        let expected = json!({
            "data": {
                "components": ["Component1", "Component2"]
            }
        });
        
        let params = json!({
            "data": {
                "components": components
            }
        });
        
        assert_eq!(params, expected);
        assert!(params.get("data").is_some());
        assert!(params.get("data").unwrap().get("components").is_some());
    }

    #[test]
    fn test_query_data_structure() {
        let components = vec!["Transform".to_string(), "Name".to_string(), "GlobalTransform".to_string()];
        
        let params = json!({
            "data": {
                "components": components
            }
        });
        
        let data_obj = params.get("data").unwrap();
        assert!(data_obj.is_object());
        
        let components_array = data_obj.get("components").unwrap();
        assert!(components_array.is_array());
        assert_eq!(components_array.as_array().unwrap().len(), 3);
        assert_eq!(components_array[0], "Transform");
        assert_eq!(components_array[1], "Name");
        assert_eq!(components_array[2], "GlobalTransform");
    }

    #[test]
    fn test_query_opposite_of_raw() {
        let params_with_data = json!({
            "data": {
                "components": ["Test"]
            }
        });
        
        let params_raw = json!({
            "components": ["Test"]
        });
        
        assert!(params_with_data.get("data").is_some());
        assert!(params_raw.get("data").is_none());
        assert_ne!(params_with_data, params_raw);
    }
}
