use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use std::sync::Arc;

pub mod config;
pub mod daemon;
pub mod gateway;
pub mod mcp_client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting MCP Gateway Hub...");

    // Load configs
    let gateway_config = config::load_gateway_config("gateway.yml").unwrap_or_else(|e| {
        warn!("Could not load gateway.yml, using default config: {}", e);
        config::GatewayConfig {
            server: config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            auth: config::AuthConfig {
                jwt_secret: "super_secret".to_string(),
                require_auth: true,
            },
        }
    });

    let _mcp_config = config::load_mcp_config("mcp.json").unwrap_or_else(|e| {
        warn!("Could not load mcp.json, using default config: {}", e);
        config::McpConfig {
            client_info: config::ClientInfo {
                name: "mcp-gateway-hub".to_string(),
                version: "1.0.0".to_string(),
            },
            capabilities: Default::default(),
        }
    });

    let daemon_config = config::load_daemon_config("daemon.yml").unwrap_or_else(|e| {
        warn!("Could not load daemon.yml, using empty config: {}", e);
        config::DaemonConfig { servers: vec![] }
    });

    // Initialize MCP Client Manager
    let mcp_client_manager = mcp_client::McpClientManager::new();

    // Start Daemons and register them
    for server_config in daemon_config.servers {
        info!("Spawning daemon: {}", server_config.name);
        match daemon::DaemonManager::spawn(&server_config) {
            Ok(process) => {
                if let Err(e) = mcp_client_manager.add_client(process).await {
                    warn!("Failed to initialize MCP client for {}: {}", server_config.name, e);
                } else {
                    info!("Successfully connected to {}", server_config.name);
                }
            }
            Err(e) => {
                warn!("Failed to spawn daemon {}: {}", server_config.name, e);
            }
        }
    }

    // Start Gateway
    let app_state = gateway::AppState {
        config: Arc::new(gateway_config.clone()),
        mcp_client_manager,
    };

    let router = gateway::create_router(app_state);
    
    let addr = format!("{}:{}", gateway_config.server.host, gateway_config.server.port);
    info!("Gateway listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
