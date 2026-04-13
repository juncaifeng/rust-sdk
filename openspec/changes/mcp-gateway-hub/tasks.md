# Tasks: MCP Gateway Hub Implementation

## 1. Project Setup and Scaffolding
- [ ] Initialize a new Rust workspace or binary crate (e.g., `cargo new mcp-gateway-hub`).
- [ ] Add dependencies to `Cargo.toml`:
  - `tokio` (async runtime)
  - `axum` (HTTP gateway)
  - `rmcp` (MCP SDK with `client` feature)
  - `serde`, `serde_json`, `serde_yaml` (configuration parsing)
  - `jsonwebtoken` (auth)

## 2. Configuration Parsing Layer
- [ ] Define struct models for `GatewayConfig`, `McpConfig`, and `DaemonConfig`.
- [ ] Implement parsing logic to load:
  - `gateway.yml`
  - `mcp.json`
  - `daemon.yml`
- [ ] Set up basic logging (`tracing` or `log`).

## 3. Daemon Module (Process Manager)
- [ ] Create `daemon.rs` module.
- [ ] Implement `DaemonManager` to loop through `DaemonConfig` servers and spawn `tokio::process::Command` processes.
- [ ] Export process streams (stdin/stdout) so they can be consumed by the MCP Client.

## 4. MCP Client Module
- [ ] Create `mcp_client.rs` module.
- [ ] Integrate the `rmcp` SDK to create clients using `TokioChildProcess`.
- [ ] Connect the MCP Clients to the running Daemon processes.
- [ ] Implement an internal tool registry that maps an exposed `tool_name` to the correct running MCP Client.
- [ ] Write logic to fetch `list_tools()` from each child process upon startup and merge them into a unified catalog.

## 5. Gateway Module (API & Auth)
- [ ] Create `gateway.rs` module using `axum`.
- [ ] Implement the `JWT` middleware to intercept requests, validate signatures, and extract token scopes.
- [ ] Build the `POST /v1/tools/call` endpoint:
  - Verify if the token scope permits calling the requested tool.
  - Forward the request payload to the MCP Client registry.
  - Wait for the MCP response and serialize it back to the client.
- [ ] Build a `GET /v1/tools` endpoint to expose the merged catalog of available tools.

## 6. Integration and Testing
- [ ] Tie the modules together in `main.rs`: Load configs -> Start Daemons -> Init MCP Clients -> Mount Gateway -> Start HTTP server.
- [ ] Create mock config files for testing.
- [ ] Write a simple mock MCP server (e.g., in python) to test end-to-end functionality.
- [ ] Perform E2E tests with `curl` to verify authentication blocking and successful tool calls.
