use rmcp::{
    ErrorData as McpError,
    model::*,
    tool, tool_handler, tool_router,
    handler::server::{tool::ToolRouter, ServerHandler, wrapper::Parameters},
    transport,
    ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use bevy_bridge_core::{BrpClient, BrpConfig, ops, types};
use base64::Engine;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct PingParams {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct QueryParams {
    components: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SpawnPrimitiveParams {
    primitive_type: String,
    position: [f32; 3],
    #[serde(default = "default_rotation")]
    rotation: [f32; 4],
    #[serde(default = "default_scale")]
    scale: [f32; 3],
}

fn default_rotation() -> [f32; 4] { [0.0, 0.0, 0.0, 1.0] }
fn default_scale() -> [f32; 3] { [1.0, 1.0, 1.0] }

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct UploadAssetParams {
    filename: String,
    data_base64: String,
    subdir: Option<String>,
    #[serde(default)]
    translation: [f32; 3],
    #[serde(default = "default_rotation")]
    rotation: [f32; 4],
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ClearSceneParams {
    #[serde(default = "default_target")]
    target: String,
}

fn default_target() -> String { "all".to_string() }

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct RpcRawParams {
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Clone)]
struct BevyMcpServer {
    tool_router: ToolRouter<Self>,
    client: BrpClient,
}

#[tool_router]
impl BevyMcpServer {
    fn new() -> Self {
        let config = BrpConfig::from_env();
        let client = BrpClient::new(config);
        
        Self {
            tool_router: Self::tool_router(),
            client,
        }
    }

    #[tool(description = "Check connectivity to Bevy BRP server")]
    async fn bevy_ping(&self, _params: Parameters<PingParams>) -> Result<CallToolResult, McpError> {
        let response = ops::ping::ping(&self.client).await
            .map_err(|e| McpError::internal_error(format!("Ping failed: {}", e), None))?;
        
        Ok(CallToolResult::structured(serde_json::json!({
            "alive": response.alive,
            "methods": response.methods
        })))
    }

    #[tool(description = "Query entities by component types")]
    async fn bevy_query(&self, params: Parameters<QueryParams>) -> Result<CallToolResult, McpError> {
        let response = ops::query::query(&self.client, params.0.components.clone()).await
            .map_err(|e| McpError::internal_error(format!("Query failed: {}", e), None))?;
        
        Ok(CallToolResult::structured(serde_json::json!({
            "entities": response.entities
        })))
    }

    #[tool(description = "Spawn a primitive object in the Bevy scene")]
    async fn bevy_spawn_primitive(&self, params: Parameters<SpawnPrimitiveParams>) -> Result<CallToolResult, McpError> {
        let response = ops::spawn::spawn(
            &self.client,
            &params.0.primitive_type,
            params.0.position,
            params.0.rotation,
            params.0.scale,
        ).await
            .map_err(|e| McpError::internal_error(format!("Spawn failed: {}", e), None))?;
        
        Ok(CallToolResult::structured(serde_json::json!({
            "entity_id": response.entity_id
        })))
    }

    #[tool(description = "Upload an asset (GLB, texture) to the Bevy runtime")]
    async fn bevy_upload_asset(&self, params: Parameters<UploadAssetParams>) -> Result<CallToolResult, McpError> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&params.0.data_base64)
            .map_err(|e| McpError::invalid_params(format!("Invalid base64: {}", e), None))?;
        
        let response = ops::upload::upload(
            &self.client,
            &params.0.filename,
            &bytes,
            params.0.subdir.as_deref(),
            params.0.translation,
            params.0.rotation,
        ).await
            .map_err(|e| McpError::internal_error(format!("Upload failed: {}", e), None))?;
        
        Ok(CallToolResult::structured(serde_json::json!({
            "entity_id": response.entity_id
        })))
    }

    #[tool(description = "Clear scene entities (all, assets, or primitives)")]
    async fn bevy_clear_scene(&self, params: Parameters<ClearSceneParams>) -> Result<CallToolResult, McpError> {
        let target = match params.0.target.as_str() {
            "assets" => types::ClearTarget::Assets,
            "primitives" => types::ClearTarget::Primitives,
            _ => types::ClearTarget::All,
        };
        
        let response = ops::clear::clear(&self.client, target).await
            .map_err(|e| McpError::internal_error(format!("Clear failed: {}", e), None))?;
        
        Ok(CallToolResult::structured(serde_json::json!({
            "entities_removed": response.entities_removed
        })))
    }

    #[tool(description = "Raw BRP RPC call (advanced users only - no parameter wrapping)")]
    async fn bevy_rpc_raw(&self, params: Parameters<RpcRawParams>) -> Result<CallToolResult, McpError> {
        let result = ops::raw::raw(&self.client, &params.0.method, params.0.params.clone()).await
            .map_err(|e| McpError::internal_error(format!("RPC failed: {}", e), None))?;
        
        Ok(CallToolResult::structured(result))
    }
}

#[tool_handler]
impl ServerHandler for BevyMcpServer {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    let server = BevyMcpServer::new();
    let transport = transport::stdio();
    
    tracing::info!("Starting Bevy MCP Server on stdio...");
    
    server.serve(transport).await?.waiting().await?;
    
    Ok(())
}
