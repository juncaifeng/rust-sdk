# RMCP (Rust Model Context Protocol) Code Wiki

本文档旨在分析和说明 `rmcp` 项目的整体架构、主要模块职责、关键类与函数说明、依赖关系以及项目运行方式。`rmcp` 是一个官方的 Rust 语言实现的 Model Context Protocol (MCP) SDK，基于 Tokio 异步运行时构建。

## 1. 项目整体架构

`rmcp` 项目采用了典型的工作空间 (Workspace) 结构，包含核心 SDK 和过程宏支持，以简化 MCP 协议的客户端和服务端开发。

项目的整体架构分为以下几个主要部分：
- **核心协议实现 (`crates/rmcp`)**：提供了 MCP 协议的底层抽象、数据模型、JSON-RPC 消息处理逻辑、生命周期管理以及多种底层通信传输层 (Transport) 的实现。
- **过程宏 (`crates/rmcp-macros`)**：提供声明式开发体验。开发者可以通过给函数和结构体添加注解 (如 `#[tool]`, `#[prompt]`)，自动生成路由逻辑和参数的 JSON Schema 描述。
- **传输层抽象 (Transport Abstraction)**：将底层的数据流 (如标准输入输出 `stdio`、子进程管道 `child_process`、HTTP、SSE 等) 统一抽象为异步的消息收发接口。
- **扩展与示例 (`examples/`, `conformance/`)**：提供了丰富的客户端与服务端实现示例，以及传输层对接示例。

## 2. 主要模块职责

核心逻辑主要位于 `crates/rmcp` 下，划分为以下核心模块：

- **`rmcp::model`**：定义了 MCP 规范中的所有基础数据类型。如 `Tool`, `Prompt`, `Resource`, `SamplingMessage` 等，并提供了针对这些数据结构的序列化与反序列化实现。
- **`rmcp::transport`**：负责底层的网络与 I/O 通信。实现了协议要求的基础传输通道：
  - `stdio`：标准输入输出传输层，常用于本地进程间的 MCP 服务端通信。
  - `child_process`：启动子进程并通过其标准输入输出通信，常用于 MCP 客户端。
  - `streamable_http_client` / `streamable_http_server`：基于 HTTP 和 SSE 的流式传输协议。
  - `async_rw` / `worker` / `sink_stream`：异步读写流等辅助传输层的实现。
- **`rmcp::service`**：提供 JSON-RPC 消息生命周期管理，定义了代表连接对端的 `Peer` 结构，以及代表当前运行会话的 `Service` 结构。它负责处理握手、消息路由与会话状态管理。
- **`rmcp::handler`**：提供用于处理业务逻辑的核心 Trait：
  - `ClientHandler`：供客户端实现，用于处理来自服务端的 Sampling 请求、资源更新通知等。
  - `ServerHandler`：供服务端实现，用于处理来自客户端的工具调用 (`call_tool`)、资源读取 (`read_resource`)、提示词获取 (`get_prompt`) 等。
- **`rmcp-macros` (过程宏)**：
  - 将复杂的底层 API 包装为易于使用的声明式宏，如通过 `#[tool]` 自动生成工具参数的 JSON Schema，通过 `#[tool_router]` 汇总路由。

## 3. 关键类与函数说明

### 3.1 核心 Trait

- **`ServerHandler`**：
  服务端业务逻辑的核心接口。包含的方法如 `get_info` (获取服务器能力)、`call_tool` (调用工具)、`list_tools` (获取工具列表)、`read_resource` (读取资源) 等。
- **`ClientHandler`**：
  客户端业务逻辑的核心接口。包含的方法如 `create_message` (处理服务端的 Sampling LLM 请求)、`list_roots` (返回客户端的工作空间根目录) 等。
- **`Transport`**：
  定义了底层的通信契约，包含 `send` 和 `receive` 方法，用于异步收发 `TxJsonRpcMessage` 和 `RxJsonRpcMessage`。

### 3.2 核心结构体

- **`Service`**：代表一个活跃的 MCP 连接会话。它负责维持后台的任务循环，并对外提供控制会话的接口，例如 `waiting()` 等待会话结束，或 `cancel()` 取消会话。
- **`Peer`**：代表连接的另一端。在请求上下文 (`RequestContext`) 中，可以通过 `context.peer` 主动向对方发送请求或通知 (例如发送 `notify_progress` 进度通知)。
- **`RequestContext<Role>`**：封装了每次请求的上下文信息，包括请求元数据和与 `Peer` 的连接引用。

### 3.3 关键过程宏 (Macros)

- **`#[tool]`**：标记一个函数为 MCP 工具。支持自定义名称、描述，并自动解析函数参数生成 JSON Schema。
- **`#[tool_router]`**：标记在一个 `impl` 块上，自动搜集内部所有的 `#[tool]` 函数，生成一个统一的工具路由 (`ToolRouter`)。
- **`#[tool_handler]`**：为结构体自动生成 `ServerHandler` 的工具相关方法 (`call_tool`, `list_tools` 等)，极大地减少了样板代码。
- **`#[prompt]` / `#[prompt_router]` / `#[prompt_handler]`**：与工具宏类似，用于声明和自动路由 Prompt (提示词) 请求。

### 3.4 关键扩展函数

- **`ServiceExt::serve`**：为实现了 `ServerHandler` 或 `ClientHandler` 的类型提供的扩展方法，用于将处理器与特定的 Transport 绑定并启动服务。例如：`().serve(stdio()).await?`。

## 4. 依赖关系

该项目建立在现代 Rust 异步生态之上，主要依赖如下：

- **`tokio`**：核心异步运行时，用于处理异步 I/O、并发任务调度和通道 (channels) 通信。
- **`serde` & `serde_json`**：用于所有 MCP 协议中定义的数据模型、JSON-RPC 请求与响应的序列化和反序列化。
- **`schemars`**：用于将 Rust 的强类型结构体自动转换为 JSON Schema (2020-12 规范)，这是 MCP 工具参数描述所必需的。
- **`thiserror`**：用于定义和处理底层的各类错误类型 (如 `RmcpError`, `DynamicTransportError` 等)。
- **`tower` & `hyper`** (可选)：在 HTTP 传输层功能中用于构建健壮的网络服务。

## 5. 项目运行方式

### 5.1 作为依赖引入

在你的 Rust 项目 `Cargo.toml` 中引入：

```toml
[dependencies]
# 引入服务端功能
rmcp = { version = "1.4.0", features = ["server"] }
```

### 5.2 运行示例服务端

项目在 `examples/servers` 下提供了多个示例。可以使用以下命令直接运行标准输入输出 (stdio) 的计算器服务端示例：

```bash
cargo build --release -p mcp-server-examples --example servers_calculator_stdio
```

或者直接运行开发版本并查看日志：
```bash
cargo run -p mcp-server-examples --example servers_calculator_stdio
```

### 5.3 使用 MCP Inspector 进行调试

你可以使用官方的 `@modelcontextprotocol/inspector` 工具直接启动并调试基于 stdio 传输的示例服务：

```bash
npx @modelcontextprotocol/inspector cargo run -p mcp-server-examples --example servers_calculator_stdio
```
执行后，将会在本地启动一个 Web UI，可以在浏览器中直接测试服务端暴露的 Tools 和 Prompts。

### 5.4 在 Claude Desktop 中配置使用

编译发布版后，可以将其配置为 Claude Desktop 的可用服务器。修改 Claude Desktop 的配置文件 (`claude_desktop_config.json`)：

```json
{
  "mcpServers": {
    "calculator": {
      "command": "/绝对路径/至/rust-sdk/target/release/examples/servers_calculator_stdio",
      "args": []
    }
  }
}
```
重启 Claude Desktop 后，即可在对话中直接使用该 MCP 服务端提供的功能。
