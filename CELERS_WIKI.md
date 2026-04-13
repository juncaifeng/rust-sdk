# CeleRS 项目 Code Wiki

CeleRS 是一个使用 Rust 编写的、生产级别的企业分布式任务队列库。它在提供与 Python Celery 二进制级别协议兼容性的同时，提供了卓越的性能、类型安全性和可靠性。

---

## 1. 项目整体架构

CeleRS 采用了与 Python Celery 相似的**分层架构设计**，由上至下可以分为四层，包含 18 个完全实现的 Crate：

- **应用层 (Application Layer)**：用户定义任务代码的层级，包括任务宏处理 (`celers-macros`) 和命令行工具 (`celers-cli`)。
- **运行时与工作流层 (Runtime & Workflow Layer)**：负责任务的实际执行、并发控制和工作流编排。核心模块包含工作节点 (`celers-worker`)、工作流原语 (`celers-canvas`) 和定时调度器 (`celers-beat`)。
- **消息中间件层 (Messaging Layer)**：提供与各类 Broker 和 Backend 的通信抽象。类似于 Python 的 Kombu，负责路由和消息交互。
- **协议层 (Protocol Layer)**：实现 Celery 兼容的二进制消息协议（支持 v2/v5 协议），确保跨语言（如 Python、JavaScript）的互操作性。

**核心能力**：
- **兼容性与多语言互通**：可直接替换 Python Celery 的 worker，实现无缝集成。
- **类型安全**：在编译期验证任务签名。
- **高性能**：吞吐量可达 Python Celery 的 10 倍以上（单 Worker 目标 10,000 tasks/sec）。
- **多种 Broker 与 Backend**：支持 Redis、PostgreSQL、MySQL、RabbitMQ (AMQP) 和 AWS SQS 作为消息代理；支持 Redis、Database 和 gRPC 作为结果存储后端。

---

## 2. 主要模块职责

CeleRS 基于 Cargo Workspace 组织了 18 个不同的 Crate，各司其职：

**核心与协议 (Core & Protocol)**
- `celers`：主门面 (Facade) Crate，提供统一的高级 API。
- `celers-core`：定义系统的核心 Trait 和类型（如 `Task`、`Broker`、`ResultBackend`）。
- `celers-protocol`：实现 Celery Protocol v2/v5 的消息序列化和反序列化格式。
- `celers-kombu`：提供 Kombu 风格的消息传递抽象。

**消息代理层 (Broker Layer)**
- `celers-broker-redis`：基于 Redis 实现，使用 Lua 脚本和 Pipelining 提供高吞吐量。
- `celers-broker-postgres` / `celers-broker-sql`：基于数据库实现，利用 `FOR UPDATE SKIP LOCKED` 提供 ACID 保证。
- `celers-broker-amqp`：基于 RabbitMQ 实现企业级消息路由和交换机。
- `celers-broker-sqs`：集成 AWS SQS 的云原生无服务器队列。

**结果后端层 (Result Backend Layer)**
- `celers-backend-redis`：基于 Redis 的快速内存存储，支持 TTL 和 Chord（分布式屏障）同步。
- `celers-backend-db`：提供 PostgreSQL/MySQL 的结果持久化和 SQL 分析。
- `celers-backend-rpc`：面向微服务架构的 gRPC 结果存储后端。

**运行时与工作流 (Runtime & Workflow)**
- `celers-worker`：任务执行的运行时核心，提供并发控制、重试逻辑、优雅停机和死信队列 (DLQ) 处理。
- `celers-canvas`：提供复杂工作流原语（Chain 串行、Group 并行、Chord Map-Reduce、Map/Starmap）。
- `celers-beat`：周期性任务调度器（支持 Cron、Interval、Solar）。

**开发与运维工具 (Utilities)**
- `celers-macros`：提供过程宏（如 `#[celers::task]`）以自动注册和生成类型安全的任务。
- `celers-cli`：用于 Worker 管理、队列检查和 DLQ 操作的命令行工具。
- `celers-metrics`：集成 Prometheus 指标和 OpenTelemetry 分布式链路追踪。

---

## 3. 关键类与函数说明

- **`#[celers::task]`**
  这是一个过程宏，用于将普通的异步函数转换为 CeleRS 任务。它会自动处理参数的序列化/反序列化，并生成类型安全的签名，允许你在编译时捕捉参数错误。

- **`Broker` Trait**
  所有消息队列后端必须实现的核心 Trait。它定义了任务队列的基础操作规范，包括：
  - `enqueue`：将序列化任务推送至队列。
  - `dequeue`：以阻塞/等待方式从队列拉取任务。
  - `ack`：成功处理后确认消息。
  - `reject`：拒绝任务并可选择重新入队。

- **`Worker` 结构体**
  任务消费与执行的引擎。它组合了 `Broker`、任务注册表 (`TaskRegistry`) 以及各类中间件（如熔断器 `CircuitBreaker`、死信队列处理器 `DlqHandler` 等）。使用 `WorkerConfig` 来配置并发数、重试策略和超时时间。

- **`SerializedTask` 结构体**
  任务在系统中传输的标准信封结构。它封装了任务的具体负载（Payload）和元数据（如 `TaskId`、优先级、重试次数、过期时间等）。

- **工作流原语 (Canvas Primitives)**
  - `Chain`：顺序执行多个任务，将前一个任务的输出作为后一个任务的输入。
  - `Group`：并行执行多个独立任务。
  - `Chord`：执行一组并行任务，并在所有任务完成后触发一个回调任务。

---

## 4. 依赖关系

项目构建于健壮的 Rust 异步生态之上：

- **异步与并发**：依赖 `tokio` 作为底层异步运行时。
- **序列化**：重度依赖 `serde` (JSON, MessagePack, YAML 等) 处理跨语言数据交换。
- **数据库与中间件驱动**：
  - Redis：使用 `redis` (异步支持和集群支持)。
  - 关系型数据库：使用 `sqlx` 驱动 Postgres/MySQL。
  - AMQP：使用 `lapin` 与 RabbitMQ 交互。
  - AWS：集成 `aws-sdk-sqs`。
- **可观测性**：使用 `tracing` 进行结构化日志记录，`prometheus` 导出运行指标，`opentelemetry` 实现分布式追踪。
- **CLI 工具**：利用 `clap` 构建命令行接口，`ratatui` 和 `crossterm` 用于终端 UI 渲染。

---

## 5. 项目运行方式

### 5.1 基础任务定义与运行

**定义任务：**
```rust
use celers::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct AddArgs { x: i32, y: i32 }

#[celers::task]
async fn add(args: AddArgs) -> Result<i32, Box<dyn std::error::Error>> {
    Ok(args.x + args.y)
}
```

**启动 Worker：**
```rust
use celers_broker_redis::RedisBroker;
use celers_worker::{Worker, WorkerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 初始化 Broker
    let broker = RedisBroker::new("redis://localhost:6379", "celers")?;

    // 2. 配置 Worker 参数 (并发数、重试次数等)
    let config = WorkerConfig {
        concurrency: 4,
        max_retries: 3,
        ..Default::default()
    };

    // 3. 注册任务并启动运行
    let mut worker = Worker::new(broker, config);
    worker.register_task("add", add);
    worker.run().await?;

    Ok(())
}
```

**发布任务 (Enqueue)：**
```rust
let task = add::new(AddArgs { x: 5, y: 3 }).with_priority(9); // 赋予高优先级
broker.enqueue(task).await?;
```

### 5.2 命令行工具 (CLI) 操作

CeleRS 提供了开箱即用的运维 CLI 工具，用于日常的管理与监控：

```bash
# 启动 Worker 进程
celers worker --broker redis://localhost:6379 --concurrency 8

# 查看当前队列和系统的健康状态
celers status

# 检查死信队列 (Dead Letter Queue) 中的失败任务
celers dlq inspect

# 重新执行由于异常进入死信队列的特定任务
celers dlq replay <task-id>

# 生成默认的配置文件模板
celers init > celers.toml
```

### 5.3 测试与基准评估

由于 CeleRS 注重性能，项目内置了完善的测试和基准测试套件：
```bash
# 运行所有单元与集成测试
cargo test --all-features

# 运行性能基准测试（如序列化性能、队列吞吐量评估）
cargo bench --bench queue_operations
```