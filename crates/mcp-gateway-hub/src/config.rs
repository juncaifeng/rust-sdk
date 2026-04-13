use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct GatewayConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub require_auth: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct McpConfig {
    pub client_info: ClientInfo,
    pub capabilities: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct DaemonConfig {
    pub gateway_url: String,
    pub daemon_id: String,
    pub servers: Vec<ServerProcessConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerProcessConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

pub fn load_gateway_config<P: AsRef<Path>>(path: P) -> anyhow::Result<GatewayConfig> {
    let content = fs::read_to_string(path)?;
    let config: GatewayConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}

pub fn load_mcp_config<P: AsRef<Path>>(path: P) -> anyhow::Result<McpConfig> {
    let content = fs::read_to_string(path)?;
    let config: McpConfig = serde_json::from_str(&content)?;
    Ok(config)
}

pub fn load_daemon_config<P: AsRef<Path>>(path: P) -> anyhow::Result<DaemonConfig> {
    let content = fs::read_to_string(path)?;
    let config: DaemonConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}
