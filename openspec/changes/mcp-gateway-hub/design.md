# Design: MCP Gateway Hub

## Architecture

The MCP Gateway Hub is a unified Rust application comprising three distinct modules:

1. **Gateway Module (`gateway`)**
   - Built on `axum`.
   - Parses `gateway.yml` to configure the HTTP listener, JWT authentication middleware, and API routes.
   - Provides REST endpoints for CRUD operations and `/v1/tools/call` for invoking MCP tools.
   - Extracts the Bearer token, validates claims, checks tool-level permissions (RBAC), and proxies authorized calls to the internal MCP Client.

2. **MCP Client Module (`mcp_client`)**
   - Powered by the `rmcp` SDK.
   - Parses `mcp.json` to define client capabilities and available routing for tools.
   - Maintains an active pool/registry of connections to the Daemon processes.
   - Routes `call_tool` requests, handling the MCP serialization/deserialization.

3. **Daemon Module (`daemon`)**
   - Built with `tokio::process`.
   - Parses `daemon.yml` which defines an array of MCP Server executables (e.g., node, python scripts) and their environment variables.
   - Supervises processes, automatically restarting them on failure.
   - Pipes stdio streams directly into the `rmcp` SDK `TokioChildProcess` transport.

## Configuration Schemas

### `gateway.yml`
```yaml
server:
  host: 0.0.0.0
  port: 8080
auth:
  jwt_secret: "super_secret"
  require_auth: true
```

### `mcp.json`
```json
{
  "client_info": {
    "name": "mcp-gateway-hub",
    "version": "1.0.0"
  },
  "capabilities": {
    "tools": {}
  }
}
```

### `daemon.yml`
```yaml
servers:
  - name: weather-tool
    command: "python"
    args: ["weather_mcp.py"]
    env:
      API_KEY: "weather_api"
  - name: db-tool
    command: "node"
    args: ["db_mcp.js"]
```

## Authentication & Authorization Flow
1. **External Request**: Client sends `POST /v1/tools/call` with `{"name": "weather-tool", "args": {...}}` and `Authorization: Bearer <token>`.
2. **Gateway**: 
   - `axum` middleware validates JWT.
   - Extracts `scopes` from claims. Checks if scope allows invoking `weather-tool`.
   - If unauthorized, returns HTTP 403.
3. **Internal Routing**:
   - Authorized request is passed via an internal `mpsc` channel or async function call to the MCP Client.
4. **Execution**:
   - MCP Client looks up `weather-tool` in its registry.
   - Sends `CallToolRequestParams` to the matching Daemon process via `rmcp`'s Stdio Transport.
   - Returns result back up the chain.
