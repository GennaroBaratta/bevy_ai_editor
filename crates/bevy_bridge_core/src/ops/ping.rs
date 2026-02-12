use crate::{BrpClient, Result};
use crate::types::PingResponse;

pub async fn ping(client: &BrpClient) -> Result<PingResponse> {
    let result = client.send_rpc("rpc.discover", None).await?;
    
    Ok(PingResponse {
        alive: true,
        methods: result,
    })
}
