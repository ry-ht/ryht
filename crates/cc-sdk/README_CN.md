# Claude Code SDK for Rust

[![Crates.io](https://img.shields.io/crates/v/cc-sdk.svg)](https://crates.io/crates/cc-sdk)
[![Documentation](https://docs.rs/cc-sdk/badge.svg)](https://docs.rs/cc-sdk)
[![License](https://img.shields.io/crates/l/cc-sdk.svg)](LICENSE)

一个用于与 Claude Code CLI 交互的 Rust SDK，提供简单查询接口和完整的交互式客户端功能。

## 功能特性

- 🚀 **简单查询接口** - 使用 `query()` 函数进行一次性查询
- 💬 **交互式客户端** - 支持有状态的对话，保持上下文
- 🔄 **流式支持** - 实时消息流
- 🛑 **中断功能** - 取消正在进行的操作
- 🔧 **完整配置** - Claude Code 的全面配置选项
- 📦 **类型安全** - 使用 serde 的强类型支持
- ⚡ **异步/等待** - 基于 Tokio 的异步操作
- 💰 **Token优化** - 内置工具最小化成本和追踪使用量（v0.1.12+）

## Token优化（v0.1.12新增）

使用内置优化工具最小化token消耗和控制成本：

```rust
use cc_sdk::{ClaudeCodeOptions, ClaudeSDKClient, PermissionMode};
use cc_sdk::token_tracker::BudgetLimit;
use cc_sdk::model_recommendation::ModelRecommendation;

// 1. 选择性价比高的模型
let recommender = ModelRecommendation::default();
let model = recommender.suggest("simple").unwrap(); // → Haiku（最便宜）

// 2. 配置最小token使用
let options = ClaudeCodeOptions::builder()
    .model(model)
    .max_turns(Some(3))              // 限制对话轮数
    .max_output_tokens(2000)          // 限制输出大小（新功能）
    .allowed_tools(vec!["Read".to_string()])  // 限制工具
    .permission_mode(PermissionMode::BypassPermissions)
    .build();

let mut client = ClaudeSDKClient::new(options);

// 3. 设置预算和告警
client.set_budget_limit(
    BudgetLimit::with_cost(5.0),      // 最多$5
    Some(|msg| eprintln!("⚠️  {}", msg))  // 80%时告警
).await;

// 4. 监控使用情况
let usage = client.get_usage_stats().await;
println!("Tokens: {}, 成本: ${:.2}", usage.total_tokens(), usage.total_cost_usd);
```

**核心功能：**
- ✅ `max_output_tokens` - 精确输出控制（1-32000，优先于环境变量）
- ✅ `TokenUsageTracker` - 实时token和成本监控
- ✅ `BudgetLimit` - 设置成本/token上限，80%预警
- ✅ `ModelRecommendation` - 智能模型选择（Haiku/Sonnet/Opus）

**模型成本对比：**
- Haiku: **1x**（基准，最便宜）
- Sonnet: **约5x**
- Opus: **约15x**

详见[Token优化指南](docs/TOKEN_OPTIMIZATION.md)获取完整策略。

## 完整功能集

此 Rust SDK 提供全面的 Claude Code 交互功能：

- ✅ **客户端方法**：`query()`、`send_message()`、`receive_response()`、`interrupt()`
- ✅ **交互式会话**：完整的有状态对话支持
- ✅ **消息流**：实时异步消息处理
- ✅ **配置选项**：系统提示、模型、权限、工具等
- ✅ **消息类型**：用户、助手、系统、结果消息
- ✅ **错误处理**：全面的错误类型和详细诊断
- ✅ **会话管理**：支持多会话和上下文隔离
- ✅ **类型安全**：充分利用 Rust 的类型系统确保代码可靠性

## 安装

在你的 `Cargo.toml` 中添加：

```toml
[dependencies]
cc-sdk = "0.2.0"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
```

## 前置要求

安装 Claude Code CLI：

```bash
npm install -g @anthropic-ai/claude-code
```

## 支持的模型（2025年）

SDK 支持 2025 年最新的 Claude 模型：

### 最新模型
- **Opus 4.1** - 最强大的模型
  - 完整名称：`"claude-opus-4-1-20250805"`
  - 别名：`"opus"`（推荐 - 使用最新 Opus）
  
- **Sonnet 4** - 平衡的性能
  - 完整名称：`"claude-sonnet-4-20250514"`
  - 别名：`"sonnet"`（推荐 - 使用最新 Sonnet）

### 上一代模型
- **Claude 3.5 Sonnet** - `"claude-3-5-sonnet-20241022"`
- **Claude 3.5 Haiku** - `"claude-3-5-haiku-20241022"`（最快）

### 在代码中使用模型

```rust
use cc_sdk::{query, ClaudeCodeOptions, Result};

// 使用 Opus 4.1（推荐使用别名）
let options = ClaudeCodeOptions::builder()
    .model("opus")  // 或 "claude-opus-4-1-20250805" 指定版本
    .build();

// 使用 Sonnet 4（推荐使用别名）
let options = ClaudeCodeOptions::builder()
    .model("sonnet")  // 或 "claude-sonnet-4-20250514" 指定版本
    .build();

let mut messages = query("你的提示", Some(options)).await?;
```

## 快速开始

### 简单查询（一次性）

```rust
use cc_sdk::{query, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let mut messages = query("2 + 2 等于多少？", None).await?;

    while let Some(msg) = messages.next().await {
        println!("{:?}", msg?);
    }

    Ok(())
}
```

### 交互式客户端

```rust
use cc_sdk::{InteractiveClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = InteractiveClient::new(ClaudeCodeOptions::default())?;
    client.connect().await?;

    // 发送消息并接收响应
    let messages = client.send_and_receive(
        "帮我写一个 Python 网络服务器".to_string()
    ).await?;

    // 处理响应
    for msg in &messages {
        match msg {
            cc_sdk::Message::Assistant { message } => {
                println!("Claude: {:?}", message);
            }
            _ => {}
        }
    }

    // 发送后续消息
    let messages = client.send_and_receive(
        "让它使用 async/await".to_string()
    ).await?;

    client.disconnect().await?;
    Ok(())
}
```

### 高级用法

```rust
use cc_sdk::{InteractiveClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = InteractiveClient::new(ClaudeCodeOptions::default())?;
    client.connect().await?;

    // 发送消息但不等待响应
    client.send_message("计算圆周率到100位".to_string()).await?;

    // 做其他工作...

    // 准备好时接收响应（在 Result 消息处停止）
    let messages = client.receive_response().await?;

    // 取消长时间运行的操作
    client.send_message("写一篇10000字的文章".to_string()).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    client.interrupt().await?;

    client.disconnect().await?;
    Ok(())
}
```

## 配置选项

```rust
use cc_sdk::{ClaudeCodeOptions, PermissionMode};

let options = ClaudeCodeOptions::builder()
    .system_prompt("你是一个有帮助的编程助手")
    .model("claude-3-5-sonnet-20241022")
    .permission_mode(PermissionMode::AcceptEdits)
    .max_turns(10)
    .max_thinking_tokens(10000)
    .allowed_tools(vec!["read_file".to_string(), "write_file".to_string()])
    .cwd("/path/to/project")
    .build();
```

### 控制协议（v0.1.12+）

新增与 Python Agent SDK 对齐的运行时控制与选项：

- `Query::set_permission_mode("acceptEdits" | "default" | "plan" | "bypassPermissions")`
- `Query::set_model(Some("sonnet"))` 或 `set_model(None)` 清空
- `ClaudeCodeOptions::builder().include_partial_messages(true)` 开启部分块
- `Query::stream_input(stream)` 结束后自动 `end_input()`

示例：

```rust
use cc_sdk::{Query, ClaudeCodeOptions};
use cc_sdk::transport::SubprocessTransport;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

# async fn demo() -> cc_sdk::Result<()> {
let options = ClaudeCodeOptions::builder()
    .model("sonnet")
    .include_partial_messages(true)
    .build();

let transport: Box<dyn cc_sdk::transport::Transport + Send> =
    Box::new(SubprocessTransport::new(options)?);
let transport = Arc::new(Mutex::new(transport));

let mut q = Query::new(transport, true, None, None, HashMap::new());
q.start().await?;
q.set_permission_mode("acceptEdits").await?;
q.set_model(Some("opus".into())).await?;

let inputs = vec![serde_json::json!("Hello"), serde_json::json!({"content":"Ping"})];
q.stream_input(futures::stream::iter(inputs)).await?;
# Ok(()) }
```

### Agent 工具与 MCP

- 工具白名单/黑名单：在 `ClaudeCodeOptions` 设置 `allowed_tools` / `disallowed_tools`
- 权限模式：`PermissionMode::{Default, AcceptEdits, Plan, BypassPermissions}`
- 运行时审批：实现 `CanUseTool`，返回 `PermissionResult::{Allow,Deny}`
- MCP 服务器：通过 `options.mcp_servers` 配置（stdio/http/sse/sdk），SDK 会打包成 `--mcp-config`

```rust
use cc_sdk::{ClaudeCodeOptions, PermissionMode, CanUseTool, ToolPermissionContext, PermissionResult,
             PermissionResultAllow, transport::{Transport, SubprocessTransport}, Query};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

struct AllowRead;
#[async_trait::async_trait]
impl CanUseTool for AllowRead {
  async fn can_use_tool(&self, tool:&str, _input:&serde_json::Value, _ctx:&ToolPermissionContext) -> PermissionResult {
    if tool == "Read" { PermissionResult::Allow(PermissionResultAllow{updated_input: None, updated_permissions: None}) }
    else { cc_sdk::PermissionResult::Deny(cc_sdk::PermissionResultDeny{ message: "Not allowed".into(), interrupt: false }) }
  }
}

# async fn demo() -> cc_sdk::Result<()> {
let mut opts = ClaudeCodeOptions::builder()
  .permission_mode(PermissionMode::AcceptEdits)
  .include_partial_messages(true)
  .build();
opts.allowed_tools = vec!["Read".into()];

let mut mcp = HashMap::new();
mcp.insert("filesystem".into(), cc_sdk::McpServerConfig::Stdio{ command: "npx".into(), args: Some(vec!["-y".into(), "@modelcontextprotocol/server-filesystem".into(), "/allowed".into()]), env: None });
opts.mcp_servers = mcp;

let transport: Box<dyn Transport + Send> = Box::new(SubprocessTransport::new(opts)?);
let transport = Arc::new(Mutex::new(transport));
let mut q = Query::new(transport, true, Some(Arc::new(AllowRead)), None, HashMap::new());
q.start().await?;
# Ok(()) }
```

## API 参考

### `query()`

用于一次性交互的简单无状态查询函数。

```rust
pub async fn query(
    prompt: impl Into<String>,
    options: Option<ClaudeCodeOptions>
) -> Result<impl Stream<Item = Result<Message>>>
```

### `InteractiveClient`

用于有状态交互式对话的主要客户端。

#### 方法

- `new(options: ClaudeCodeOptions) -> Result<Self>` - 创建新客户端
- `connect() -> Result<()>` - 连接到 Claude CLI
- `send_and_receive(prompt: String) -> Result<Vec<Message>>` - 发送消息并等待完整响应
- `send_message(prompt: String) -> Result<()>` - 发送消息但不等待
- `receive_response() -> Result<Vec<Message>>` - 接收消息直到 Result 消息
- `interrupt() -> Result<()>` - 取消正在进行的操作
- `disconnect() -> Result<()>` - 断开与 Claude CLI 的连接

## 消息类型

- `UserMessage` - 用户输入消息
- `AssistantMessage` - Claude 的响应
- `SystemMessage` - 系统通知
- `ResultMessage` - 包含时间和成本信息的操作结果

## 错误处理

SDK 提供全面的错误类型：

- `CLINotFoundError` - Claude Code CLI 未安装
- `CLIConnectionError` - 连接失败
- `ProcessError` - CLI 进程错误
- `InvalidState` - 无效的操作状态

## 示例

查看 `examples/` 目录获取更多使用示例：

- `interactive_demo.rs` - 交互式对话演示
- `query_simple.rs` - 简单查询示例
- `file_operations.rs` - 文件操作示例

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 贡献

欢迎贡献！请随时提交 Pull Request。
