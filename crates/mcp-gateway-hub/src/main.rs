use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use std::sync::Arc;

pub mod config;
pub mod daemon;
pub mod gateway;
pub mod mcp_client;
pub mod pb;
pub mod grpc_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args: Vec<String> = std::env::args().collect();
    let mode = if args.len() > 1 && args[1] == "--mode" && args.len() > 2 {
        args[2].clone()
    } else {
        "all".to_string() // Default to running both for testing convenience
    };

    let daemon_config_path = if args.len() > 4 && args[3] == "--config" {
        args[4].clone()
    } else {
        "daemon.yml".to_string()
    };

    if mode == "daemon" || mode == "all" {
        info!("Starting MCP Daemon...");
        let daemon_config = config::load_daemon_config(&daemon_config_path).unwrap_or_else(|e| {
            warn!("Could not load daemon.yml, using empty config: {}", e);
            config::DaemonConfig { gateway_url: "http://127.0.0.1:50051".to_string(), daemon_id: "default-daemon".to_string(), servers: vec![] }
        });

        let mcp_client_manager = mcp_client::McpClientManager::new();

        // Start Daemons and register them to local MCP Client Manager
        for server_config in &daemon_config.servers {
            info!("Spawning local MCP server process: {}", server_config.name);
            match daemon::DaemonManager::spawn(server_config) {
                Ok(process) => {
                    if let Err(e) = mcp_client_manager.add_client(process).await {
                        warn!("Failed to initialize MCP client for {}: {}", server_config.name, e);
                    } else {
                        info!("Successfully connected to local MCP server: {}", server_config.name);
                    }
                }
                Err(e) => {
                    warn!("Failed to spawn local MCP server process {}: {}", server_config.name, e);
                }
            }
        }

        let daemon_id = daemon_config.daemon_id.clone();
        let gateway_url = daemon_config.gateway_url.clone();
        
        if mode == "daemon" {
            // Run daemon logic blocking
            daemon::DaemonManager::run_daemon(gateway_url, daemon_id, mcp_client_manager).await?;
            return Ok(());
        } else {
            // Run daemon logic in background for "all" mode
            // wait a sec for gateway to start
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                if let Err(e) = daemon::DaemonManager::run_daemon(gateway_url, daemon_id, mcp_client_manager).await {
                    error!("Daemon failed: {}", e);
                }
            });
        }
    }

    if mode == "gateway" || mode == "all" {
        info!("Starting MCP Gateway Hub...");
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

        let tool_registry = grpc_server::ToolRegistry::new();

        // Start gRPC Server
        let grpc_service = grpc_server::GatewayGrpcService::new(tool_registry.clone());
        let grpc_addr = "0.0.0.0:50051".parse()?;
        
        info!("Gateway gRPC listening on {}", grpc_addr);
        tokio::spawn(async move {
            if let Err(e) = tonic::transport::Server::builder()
                .add_service(pb::pb::mcp_gateway_server::McpGatewayServer::new(grpc_service))
                .serve(grpc_addr)
                .await {
                error!("gRPC Server failed: {}", e);
            }
        });

        // Start Axum HTTP Gateway
        let app_state = gateway::AppState {
            config: Arc::new(gateway_config.clone()),
            tool_registry,
        };

        let router = gateway::create_router(app_state);
        
        let addr = format!("{}:{}", gateway_config.server.host, gateway_config.server.port);
        info!("Gateway HTTP API listening on {}", addr);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, router).await?;
    }

    Ok(())
}
