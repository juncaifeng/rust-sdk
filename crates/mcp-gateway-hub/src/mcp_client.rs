use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use rmcp::{
    ServiceExt, 
    model::{CallToolRequestParams, Tool}, 
    transport::TokioChildProcess,
    RoleClient,
    Service,
};

pub type ClientService = rmcp::service::RunningService<RoleClient, ()>;

#[derive(Clone)]
pub struct McpClientManager {
    clients: Arc<RwLock<Vec<Arc<ClientService>>>>,
    tool_registry: Arc<RwLock<HashMap<String, Arc<ClientService>>>>,
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(Vec::new())),
            tool_registry: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_client(&self, process: TokioChildProcess) -> anyhow::Result<()> {
        let client: ClientService = ().serve(process).await?;
        let client = Arc::new(client);
        
        let tools = client.list_all_tools().await?;
        
        let mut registry = self.tool_registry.write().await;
        for tool in tools {
            info!("Registered tool: {}", tool.name);
            registry.insert(tool.name.to_string(), client.clone());
        }
        
        self.clients.write().await.push(client);
        Ok(())
    }
    
    pub async fn call_tool(&self, params: CallToolRequestParams) -> anyhow::Result<serde_json::Value> {
        let registry = self.tool_registry.read().await;
        if let Some(client) = registry.get(params.name.as_ref()) {
            let result = client.call_tool(params).await?;
            Ok(serde_json::to_value(result)?)
        } else {
            Err(anyhow::anyhow!("Tool not found: {}", params.name))
        }
    }
    
    pub async fn list_all_tools(&self) -> Vec<Tool> {
        let mut all_tools = Vec::new();
        let clients = self.clients.read().await;
        for client in clients.iter() {
            if let Ok(tools) = client.list_all_tools().await {
                all_tools.extend(tools);
            }
        }
        all_tools
    }
}
