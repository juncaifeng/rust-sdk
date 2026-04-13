# Tasks: gRPC/Protobuf Upgrade for MCP Gateway Architecture

## 1. Setup Protobuf and Tonic
- [ ] Add `tonic`, `prost`, `tonic-build` to `Cargo.toml`.
- [ ] Create `proto/mcp_gateway.proto` using the definition in `design.md`.
- [ ] Write `build.rs` to compile the `.proto` file into Rust structs.

## 2. Refactor MCP Client (Daemon Side)
- [ ] Modify `McpClientManager` to run within the Daemon context instead of the Gateway context.
- [ ] Implement a Tonic Client in the Daemon that connects to the Gateway's gRPC address (configured in `daemon.yml`).
- [ ] Upon startup, iterate over local spawned processes, fetch `list_tools()`, and send a `RegisterRequest` to the Gateway over the gRPC stream.
- [ ] Listen on the gRPC stream for incoming `CallToolRequest` messages, forward them to the local `rmcp` Client, and return the `CallToolResponse` over the stream.

## 3. Refactor Gateway (Server Side)
- [ ] Implement a `McpGateway` gRPC service using Tonic.
- [ ] Create a global, thread-safe `ToolRegistry` in the Gateway that maps registered `tool_name`s to the specific Daemon's gRPC stream sender channel.
- [ ] Handle incoming `ConnectDaemon` streams, storing the sender channels for active Daemons.
- [ ] Expose an internal method `call_remote_tool` that takes a tool name and arguments, looks up the corresponding Daemon channel, sends the `CallToolRequest`, and awaits a one-shot response channel.

## 4. Integrate HTTP API with gRPC Gateway
- [ ] Update `gateway.rs` (Axum) to use the new `call_remote_tool` method from the gRPC Gateway service instead of the local `McpClientManager`.
- [ ] Update `list_tools` endpoint to query the Gateway's internal `ToolRegistry` instead of fetching locally.
- [ ] Ensure JWT and RBAC scope validation logic remains intact.

## 5. Integration and Testing
- [ ] Update `main.rs` to conditionally run as a Gateway or a Daemon based on CLI arguments or environment variables (e.g., `cargo run --bin mcp-gateway-hub -- --mode gateway`).
- [ ] Write tests verifying that a Daemon can register tools and that the Gateway can route a `/v1/tools/call` HTTP request through gRPC to the Daemon and back.
