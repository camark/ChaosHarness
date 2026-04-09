# ACP (Agent Communication Protocol) 支持

## 概述

RustHarness 现已支持 ACP 协议，这是一个基于 REST 的 AI 代理通信协议标准。ACP 允许不同 AI 代理之间进行互操作和协作。

## 什么是 ACP？

ACP (Agent Communication Protocol) 是一个开放的 AI 代理通信标准，由 IBM 等公司推动。它定义了：

- **AgentCard**: 代理的"名片"，描述代理的能力、端点和元数据
- **任务管理**: 创建、查询和管理 AI 任务的标准 API
- **消息传递**: 代理间通信的消息格式
- **能力发现**: 发现和协商代理能力

详细规格参考：[https://github.com/i-am-bee/acp](https://github.com/i-am-bee/acp)

## 启动 ACP 服务器

### 命令行

```bash
# 在端口 8080 上启动 ACP 服务器
cargo run -- --acp-server 8080

# 或者使用 release 模式
cargo run --release -- --acp-server 8080
```

### 服务端点

启动后，ACP 服务器提供以下端点：

| 端点 | 方法 | 描述 |
|------|------|------|
| `/.well-known/agent.json` | GET | AgentCard 发现端点（标准位置） |
| `/acp` | GET | AgentCard 端点 |
| `/tasks` | POST | 创建新任务 |
| `/tasks/{id}` | GET | 获取任务状态 |
| `/tasks/{id}/send` | POST | 发送消息到任务 |
| `/tasks/{id}/artifacts` | GET | 获取任务产出物 |
| `/tasks/{id}/cancel` | POST | 取消任务 |
| `/tasks/{id}/input` | POST | 提交输入到任务 |

## AgentCard

RustHarness 的 AgentCard 包含以下信息：

- **名称**: RustHarness
- **描述**: AI-powered coding assistant harness
- **能力**: 
  - 工具使用 (tool_use)
  - 内存/上下文 (memory)
  - 多轮对话 (multi_turn)
  - 文件 I/O (file_io)
  - Web 访问 (web_access)
  - 代码执行 (code_execution)

### 可用技能 (Skills)

RustHarness 通过 ACP 暴露以下技能：

1. **bash** - 执行 shell 命令
2. **read_file** - 读取文本文件
3. **write_file** - 创建或覆盖文件
4. **edit_file** - 编辑文件内容
5. **glob** -  glob 模式匹配文件
6. **grep** - 正则搜索文件内容
7. **web_fetch** - 获取 URL 内容
8. **web_search** - 网络搜索
9. **ask_user** - 交互式用户提问

## 使用示例

### 获取 AgentCard

```bash
curl http://localhost:8080/.well-known/agent.json
# 或
curl http://localhost:8080/acp
```

### 创建任务

```bash
curl -X POST http://localhost:8080/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "description": "帮我分析这个项目的结构",
    "message": {
      "role": "user",
      "content": [
        {"type": "text", "text": "请列出 src 目录下的所有 Rust 文件"}
      ]
    }
  }'
```

### 发送消息到任务

```bash
curl -X POST http://localhost:8080/tasks/{task-id}/send \
  -H "Content-Type: application/json" \
  -d '{
    "message": {
      "role": "user",
      "content": [
        {"type": "text", "text": "继续分析"}
      ]
    }
  }'
```

### 获取任务状态

```bash
curl http://localhost:8080/tasks/{task-id}
```

### 获取任务产出物

```bash
curl http://localhost:8080/tasks/{task-id}/artifacts
```

## ACP 客户端

RustHarness 提供了 ACP 客户端用于连接远程代理：

```rust
use rust_harness::acp::AcpClient;

// 创建客户端
let mut client = AcpClient::new("http://remote-agent:8080");

// 发现远程代理的 AgentCard
let agent_card = client.discover().await?;
println!("Connected to: {}", agent_card.name);

// 创建任务
let task = client.create_task(
    "分析代码库",
    Some(MessageBuilder::user()
        .add_text("请帮我分析这个项目的架构")
        .build())
).await?;

// 发送消息
let response = client.send_message(
    &task.id,
    MessageBuilder::user()
        .add_text("能详细说明吗？")
        .build()
).await?;
```

## 与 MCP 的比较

| 特性 | ACP | MCP |
|------|-----|-----|
| 主要用途 | 代理间通信 | 工具/数据源连接 |
| 传输协议 | REST/HTTP | stdio/SSE |
| 发现机制 | AgentCard | 服务器配置 |
| 任务管理 | 内置支持 | 无 |
| 标准化程度 | 开放标准 (Linux Foundation) | Anthropic 主导 |

## 架构集成

ACP 模块位于 `src/acp/`:

```
src/acp/
├── mod.rs          # 模块导出
├── types.rs        # 类型定义 (AgentCard, Task, Message 等)
├── agent_card.rs   # AgentCard 构建和序列化
├── server.rs       # ACP 服务器实现
├── client.rs       # ACP 客户端实现
└── handlers.rs     # HTTP 请求处理器
```

## 配置

目前 ACP 服务配置通过命令行参数 `--acp-server <PORT>` 进行。

未来将在 `settings.json` 中添加：

```json
{
  "acp": {
    "enable_server": true,
    "server_port": 8080,
    "remote_agents": [
      "http://agent1:8080",
      "http://agent2:8080"
    ],
    "api_key": "optional-auth-key"
  }
}
```

## 参考资料

- [ACP Specification](https://github.com/i-am-bee/acp)
- [AgentCard Schema](https://a2a.plus/docs/json-specification)
- [IBM ACP Documentation](https://www.ibm.com/think/topics/agent-communication-protocol)
- [A2A Protocol Comparison](https://agent-network-protocol.com/blogs/posts/agent-communication-protocols-comparison.html)
