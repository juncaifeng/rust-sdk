use crate::config::ServerProcessConfig;
use crate::mcp_client::McpClientManager;
use rmcp::transport::TokioChildProcess;
use tokio::process::Command;
use tracing::{info, warn, error};
use tokio_stream::wrappers::ReceiverStream;
use crate::pb::pb::{mcp_gateway_client::McpGatewayClient, DaemonMessage, RegisterRequest, ToolInfo};
use crate::pb::pb::daemon_message::Payload;

pub struct DaemonManager;

impl DaemonManager {
    pub fn spawn(config: &ServerProcessConfig) -> anyhow::Result<TokioChildProcess> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);
        
        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        let process = TokioChildProcess::new(cmd)?;
        Ok(process)
    }

    pub async fn run_daemon(
        gateway_url: String, 
        daemon_id: String, 
        mcp_client_manager: McpClientManager
    ) -> anyhow::Result<()> {
        info!("Connecting to Gateway at {} as {}", gateway_url, daemon_id);
        
        let mut client = match McpGatewayClient::connect(gateway_url).await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to connect to gateway: {}", e);
                return Err(e.into());
            }
        };

        // Collect tools
        let tools = mcp_client_manager.list_all_tools().await;
        let mut tool_infos = Vec::new();
        for t in tools {
            let schema_json = serde_json::to_string(&t.input_schema).unwrap_or_else(|_| "{}".to_string());
            tool_infos.push(ToolInfo {
                name: t.name.to_string(),
                description: t.description.unwrap_or_default().to_string(),
                input_schema_json: schema_json,
            });
        }

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Send initial registration
        let reg_msg = DaemonMessage {
            payload: Some(Payload::Register(RegisterRequest {
                daemon_id: daemon_id.clone(),
                tools: tool_infos,
            })),
        };
        tx.send(reg_msg).await?;

        let request_stream = ReceiverStream::new(rx);
        let mut response_stream = client.connect_daemon(request_stream).await?.into_inner();

        info!("Connected to Gateway. Listening for CallTool requests...");

        // Listen to requests from gateway
        while let Some(msg) = response_stream.message().await? {
            if let Some(crate::pb::pb::gateway_message::Payload::CallRequest(req)) = msg.payload {
                info!("Received call request for tool: {}", req.tool_name);
                
                let mcp_manager = mcp_client_manager.clone();
                let tx_clone = tx.clone();
                
                tokio::spawn(async move {
                    let mut params = rmcp::model::CallToolRequestParams::new(req.tool_name.clone());
                    if !req.arguments_json.is_empty() {
                        if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(&req.arguments_json) {
                            params.arguments = Some(map);
                        }
                    }

                    let (success, result_json, error_message) = match mcp_manager.call_tool(params).await {
                        Ok(res) => (true, serde_json::to_string(&res).unwrap_or_default(), String::new()),
                        Err(e) => (false, String::new(), e.to_string()),
                    };

                    let resp_msg = DaemonMessage {
                        payload: Some(Payload::CallResponse(crate::pb::pb::CallToolResponse {
                            request_id: req.request_id,
                            success,
                            result_json,
                            error_message,
                        })),
                    };

                    if let Err(e) = tx_clone.send(resp_msg).await {
                        warn!("Failed to send CallToolResponse to gateway: {}", e);
                    }
                });
            } else if let Some(crate::pb::pb::gateway_message::Payload::RegisterAck(ack)) = msg.payload {
                info!("Gateway Registration ACK: success={}, message={}", ack.success, ack.message);
            }
        }

        info!("Gateway connection closed.");
        Ok(())
    }
}
