# Proposal: gRPC/Protobuf Upgrade for MCP Gateway Architecture

## What is this change?
Upgrade the internal communication layer between the MCP Daemon and the Gateway Hub to use gRPC/Protobuf (via `tonic` and `prost`), while keeping the Client ↔ Daemon communication strictly JSON-RPC (MCP Standard). 

The new flow will be:
- **Client ↔ Daemon**: Local IPC using JSON-RPC (via `rmcp` SDK).
- **Daemon ↔ Gateway**: Network communication using gRPC (Protobuf).
- **Gateway ↔ External**: RESTful APIs (via `axum`) AND gRPC APIs for external consumers.

## Why are we doing this?
To scale the MCP Gateway Hub into a robust distributed system:
1. **Network Efficiency**: gRPC provides HTTP/2 multiplexing and binary serialization (Protobuf), which is significantly faster and uses less bandwidth for cross-machine communication than plain HTTP/JSON.
2. **Strict Typing (IDL)**: A `.proto` file serves as the definitive contract between the remote Gateway and the distributed Daemons, making routing and message passing type-safe.
3. **MCP Compatibility**: By keeping the Client ↔ Daemon layer as JSON-RPC, we maintain 100% compatibility with existing third-party standard MCP servers (e.g., Node.js/Python MCP tools). The Daemon acts as an intelligent sidecar proxy, translating Protobuf network packets into standard MCP UDS/Stdio JSON payloads.

## High-level Approach
1. Define the `mcp_gateway.proto` IDL containing `RegisterTools` and `CallTool` RPCs.
2. Integrate `tonic` and `prost` into the `mcp-gateway-hub` crate via a `build.rs` compilation step.
3. Refactor the Gateway to start a `tonic` gRPC server for receiving Daemon registrations.
4. Refactor the Daemon to act as a `tonic` gRPC client that connects to the Gateway, registers its locally discovered tools, and listens for remote `CallTool` requests.