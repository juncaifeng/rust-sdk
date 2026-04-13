# Proposal: MCP Gateway Hub

## What is this change?
Implement a centralized Gateway Hub for the Model Context Protocol (MCP). The system acts as a central orchestrator composed of three distinct layers:
1. **Gateway**: Exposes tools via remote APIs (e.g., REST/gRPC), providing North-South traffic routing, CRUD, and authentication.
2. **MCP Client**: Manages active MCP sessions with underlying MCP Servers (Daemons), routing `call_tool` requests and managing tool discovery.
3. **Daemon**: A process manager responsible for launching and supervising the lifecycle of actual MCP tool server subprocesses.

## Why are we doing this?
Currently, MCP Servers run as isolated processes and often require 1:1 bindings with specific AI clients (like Claude Desktop). By introducing a Gateway layer:
1. We can expose MCP tools remotely over the network with enterprise-grade authentication and authorization (RBAC/JWT).
2. We can centralize tool lifecycle management and process supervision (Daemon).
3. We decouple the external callers from the specifics of the MCP protocol via a standard Gateway.

## High-level Approach
- Use `axum` for the Gateway to handle HTTP/REST endpoints and token validation.
- Leverage the `rmcp` Rust SDK for the MCP Client to interface with underlying MCP Servers via Stdio or Streamable HTTP.
- Use `tokio::process::Command` within the Daemon module to spawn, monitor, and pipe standard IO for local MCP tools.
- Configuration will be split across `gateway.yml`, `mcp.json`, and `daemon.yml` to maintain clear boundaries of responsibility.
