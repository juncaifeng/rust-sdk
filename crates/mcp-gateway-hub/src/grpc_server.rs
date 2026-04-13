use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};
use tonic::{Request, Response, Status, Streaming};
use tracing::{info, warn, error};

use crate::pb::pb::mcp_gateway_server::McpGateway;
use crate::pb::pb::{DaemonMessage, GatewayMessage, RegisterResponse, CallToolRequest, ToolInfo};
use crate::pb::pb::gateway_message::Payload as GatewayPayload;

type DaemonSender = mpsc::Sender<Result<GatewayMessage, Status>>;
type ResponseChannel = oneshot::Sender<Result<crate::pb::pb::CallToolResponse, Status>>;

#[derive(Clone)]
pub struct ToolRegistry {
    // tool_name -> (daemon_id, ToolInfo)
    pub tools: Arc<RwLock<HashMap<String, (String, ToolInfo)>>>,
    // daemon_id -> Sender channel to gRPC stream
    pub daemons: Arc<RwLock<HashMap<String, DaemonSender>>>,
    // request_id -> oneshot sender for waiting responses
    pub pending_calls: Arc<RwLock<HashMap<String, ResponseChannel>>>,
    // request_id -> oneshot sender for StartServerResponse
    pub pending_starts: Arc<RwLock<HashMap<String, oneshot::Sender<Result<crate::pb::pb::StartServerResponse, Status>>>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            daemons: Arc::new(RwLock::new(HashMap::new())),
            pending_calls: Arc::new(RwLock::new(HashMap::new())),
            pending_starts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_all_tools(&self) -> Vec<serde_json::Value> {
        let tools_map = self.tools.read().await;
        let mut res = Vec::new();
        for (_, (daemon_id, info)) in tools_map.iter() {
            let mut val = serde_json::json!({
                "name": info.name,
                "description": info.description,
                "daemon_id": daemon_id,
            });
            if let Ok(schema) = serde_json::from_str::<serde_json::Value>(&info.input_schema_json) {
                val.as_object_mut().unwrap().insert("inputSchema".to_string(), schema);
            }
            res.push(val);
        }
        res
    }

    pub async fn get_all_daemons(&self) -> Vec<serde_json::Value> {
        let daemons_map = self.daemons.read().await;
        let tools_map = self.tools.read().await;
        
        let mut res = Vec::new();
        for daemon_id in daemons_map.keys() {
            let mut tools = Vec::new();
            for (_, (d_id, info)) in tools_map.iter() {
                if d_id == daemon_id {
                    tools.push(info.name.clone());
                }
            }
            res.push(serde_json::json!({
                "id": daemon_id,
                "status": "connected",
                "tools_count": tools.len(),
                "tools": tools,
            }));
        }
        res
    }

    pub async fn call_remote_tool(&self, tool_name: String, arguments_json: String) -> anyhow::Result<serde_json::Value> {
        let daemon_id = {
            let tools_map = self.tools.read().await;
            tools_map.get(&tool_name).map(|(id, _)| id.clone())
        };

        let daemon_id = daemon_id.ok_or_else(|| anyhow::anyhow!("Tool {} not found", tool_name))?;

        let sender = {
            let daemons_map = self.daemons.read().await;
            daemons_map.get(&daemon_id).cloned()
        };

        let sender = sender.ok_or_else(|| anyhow::anyhow!("Daemon {} disconnected", daemon_id))?;

        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        self.pending_calls.write().await.insert(request_id.clone(), tx);

        let req = GatewayMessage {
            payload: Some(GatewayPayload::CallRequest(CallToolRequest {
                request_id: request_id.clone(),
                tool_name: tool_name.clone(),
                arguments_json,
            })),
        };

        if let Err(e) = sender.send(Ok(req)).await {
            self.pending_calls.write().await.remove(&request_id);
            return Err(anyhow::anyhow!("Failed to send request to daemon: {}", e));
        }

        // Wait for response
        let result = tokio::time::timeout(std::time::Duration::from_secs(30), rx).await;
        self.pending_calls.write().await.remove(&request_id);

        match result {
            Ok(Ok(Ok(resp))) => {
                if resp.success {
                    Ok(serde_json::from_str(&resp.result_json).unwrap_or_else(|_| serde_json::json!({})))
                } else {
                    Err(anyhow::anyhow!("Tool execution failed: {}", resp.error_message))
                }
            }
            Ok(Ok(Err(s))) => Err(anyhow::anyhow!("gRPC error: {}", s)),
            Ok(Err(_)) => Err(anyhow::anyhow!("Internal channel closed")),
            Err(_) => Err(anyhow::anyhow!("Timeout waiting for daemon response")),
        }
    }

    pub async fn start_remote_server(
        &self,
        daemon_id: String,
        server_name: String,
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    ) -> anyhow::Result<()> {
        let sender = {
            let daemons_map = self.daemons.read().await;
            daemons_map.get(&daemon_id).cloned()
        };
        let sender = sender.ok_or_else(|| anyhow::anyhow!("Daemon {} disconnected", daemon_id))?;

        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.pending_starts.write().await.insert(request_id.clone(), tx);

        let req = GatewayMessage {
            payload: Some(GatewayPayload::StartServer(crate::pb::pb::StartServerRequest {
                request_id: request_id.clone(),
                server_name,
                command,
                args,
                env,
            })),
        };

        sender.send(Ok(req)).await.map_err(|e| anyhow::anyhow!("Failed to send StartServer: {}", e))?;

        let result = tokio::time::timeout(std::time::Duration::from_secs(30), rx).await;
        self.pending_starts.write().await.remove(&request_id);

        match result {
            Ok(Ok(Ok(resp))) => {
                if resp.success {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Server start failed: {}", resp.message))
                }
            }
            Ok(Ok(Err(s))) => Err(anyhow::anyhow!("gRPC error: {}", s)),
            Ok(Err(_)) => Err(anyhow::anyhow!("Internal channel closed")),
            Err(_) => Err(anyhow::anyhow!("Timeout waiting for daemon response")),
        }
    }
}

pub struct GatewayGrpcService {
    registry: ToolRegistry,
}

impl GatewayGrpcService {
    pub fn new(registry: ToolRegistry) -> Self {
        Self { registry }
    }
}

#[tonic::async_trait]
impl McpGateway for GatewayGrpcService {
    type ConnectDaemonStream = tokio_stream::wrappers::ReceiverStream<Result<GatewayMessage, Status>>;

    async fn connect_daemon(
        &self,
        request: Request<Streaming<DaemonMessage>>,
    ) -> Result<Response<Self::ConnectDaemonStream>, Status> {
        let mut in_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(100);
        let registry = self.registry.clone();

        tokio::spawn(async move {
            let mut current_daemon_id = None;

            while let Ok(Some(msg)) = in_stream.message().await {
                match msg.payload {
                    Some(crate::pb::pb::daemon_message::Payload::Register(req)) => {
                        info!("Daemon {} registered with {} tools", req.daemon_id, req.tools.len());
                        current_daemon_id = Some(req.daemon_id.clone());
                        
                        // Register daemon channel
                        registry.daemons.write().await.insert(req.daemon_id.clone(), tx.clone());
                        
                        // Register tools
                        let mut tools_map = registry.tools.write().await;
                        for tool in req.tools {
                            tools_map.insert(tool.name.clone(), (req.daemon_id.clone(), tool));
                        }

                        // Send Ack
                        let ack = GatewayMessage {
                            payload: Some(GatewayPayload::RegisterAck(RegisterResponse {
                                success: true,
                                message: "Registered successfully".to_string(),
                            })),
                        };
                        let _ = tx.send(Ok(ack)).await;
                    }
                    Some(crate::pb::pb::daemon_message::Payload::CallResponse(resp)) => {
                        if let Some(chan) = registry.pending_calls.write().await.remove(&resp.request_id) {
                            let _ = chan.send(Ok(resp));
                        } else {
                            warn!("Received response for unknown request_id: {}", resp.request_id);
                        }
                    }
                    Some(crate::pb::pb::daemon_message::Payload::StartServerResponse(resp)) => {
                        if let Some(chan) = registry.pending_starts.write().await.remove(&resp.request_id) {
                            let _ = chan.send(Ok(resp));
                        } else {
                            warn!("Received StartServerResponse for unknown request_id: {}", resp.request_id);
                        }
                    }
                    None => {}
                }
            }

            if let Some(id) = current_daemon_id {
                info!("Daemon {} disconnected", id);
                registry.daemons.write().await.remove(&id);
                // We could also clean up tools for this daemon here
                let mut tools_map = registry.tools.write().await;
                tools_map.retain(|_, (d_id, _)| d_id != &id);
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }
}
