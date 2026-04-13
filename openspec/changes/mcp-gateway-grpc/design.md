# Design: gRPC/Protobuf Upgrade for MCP Gateway Architecture

## Architecture

The system is now distributed, with the Daemon and Gateway acting as separate logical nodes (even if run on the same machine) communicating via gRPC.

1. **Daemon (Sidecar/Proxy)**
   - Runs on the target machine where MCP tools are hosted.
   - Spawns local MCP Server processes (Node.js, Python, Rust) using `TokioChildProcess` via stdio.
   - Creates an `rmcp` Client to connect to these local servers and fetch their tools (`list_tools`).
   - Connects to the Gateway via gRPC (`tonic::Client`).
   - Calls the `RegisterDaemon` RPC to announce itself and its available tools.
   - Listens on a bi-directional gRPC stream for incoming `CallTool` requests from the Gateway.

2. **Gateway (Orchestrator/API Gateway)**
   - Exposes a gRPC Server (`tonic::Server`) for Daemons to connect and register.
   - Maintains a global `ToolRegistry` mapping `tool_name` -> `Daemon_ID` (or stream channel).
   - Exposes an HTTP/REST API (`axum`) for external clients (with JWT Auth).
   - When an external client calls `POST /v1/tools/call`, the Gateway looks up the tool, forwards the `CallToolRequest` over the gRPC stream to the corresponding Daemon, waits for the response, and returns it to the HTTP client.

## Protobuf Definition (`mcp_gateway.proto`)

```protobuf
syntax = "proto3";
package mcp.gateway;

service McpGateway {
  // Daemon registers itself and its tools, then opens a bi-directional stream
  // Gateway pushes CallToolRequest, Daemon responds with CallToolResponse
  rpc ConnectDaemon(stream DaemonMessage) returns (stream GatewayMessage);
}

message DaemonMessage {
  oneof payload {
    RegisterRequest register = 1;
    CallToolResponse call_response = 2;
  }
}

message GatewayMessage {
  oneof payload {
    RegisterResponse register_ack = 1;
    CallToolRequest call_request = 2;
  }
}

message RegisterRequest {
  string daemon_id = 1;
  repeated ToolInfo tools = 2;
}

message ToolInfo {
  string name = 1;
  string description = 2;
  string input_schema_json = 3;
}

message RegisterResponse {
  bool success = 1;
  string message = 2;
}

message CallToolRequest {
  string request_id = 1;
  string tool_name = 2;
  string arguments_json = 3; // JSON payload for tool arguments
}

message CallToolResponse {
  string request_id = 1;
  bool success = 2;
  string result_json = 3; // JSON payload for tool execution result
  string error_message = 4;
}
```

## Data Flow Diagram

```text
[External Client]
       │
       │ (HTTP POST /v1/tools/call { "name": "echo", "args": {...} })
       ▼
┌─────────────────────────┐
│ Gateway (Axum)          │
│ - Validates JWT & RBAC  │
│ - Looks up Tool in Hash │
│ - Creates gRPC Msg      │
└─────────┬───────────────┘
          │
          │ (gRPC Stream: CallToolRequest over HTTP/2)
          ▼
┌─────────────────────────┐
│ Daemon (Tonic Client)   │
│ - Parses gRPC Msg       │
│ - Creates rmcp Request  │
└─────────┬───────────────┘
          │
          │ (JSON-RPC over Stdio)
          ▼
[Local MCP Server (Python/Node)]
```
