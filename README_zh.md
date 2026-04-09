# Rust Harness

RustHarness 是一个 AI 驱动的编程助手。

[English](README_en.md) | 简体中文

## 快速开始

### 使用启动脚本（推荐）

**Windows:**
```bash
launch.bat
```

**Linux/macOS:**
```bash
chmod +x launch.sh
./launch.sh
```

启动脚本提供 3 种模式：
1. **REPL** - 交互式命令行模式
2. **Frontend** - React TUI 模式（需要 TTY 支持）
3. **Backend Only** - WebSocket 服务器模式

### 手动启动

**REPL 模式:**
```bash
cargo run -- <你的提示词>
```

**前端模式 (React TUI):**

Windows（需要 Windows Terminal）：
```bash
cd frontend
start.bat
```

Linux/macOS:
```bash
cd frontend
./start.sh
```

**注意:** 前端需要 TTY 支持。如果看到 "Raw mode is not supported" 错误：
- Windows: 在 Windows Terminal (`wt.exe`) 或 CMD 中运行，不要在 Git Bash 中运行
- Linux/macOS: 在原生终端中运行，不要在 VS Code 终端中运行

**后端模式 (WebSocket 服务器):**
```bash
cargo run -- --backend-only
```

**后端模式 (Stdio/OHJSON 协议):**
```bash
cargo run -- --stdio-backend
```

## 项目结构

```
src/
├── main.rs              # 入口点和 CLI
├── api/                 # AI 模型 API 客户端
│   ├── client.rs        # Anthropic API 客户端（带重试逻辑）
│   ├── errors.rs        # API 错误类型
│   └── usage.rs         # Token 使用统计
├── commands/            # 斜杠命令系统
│   ├── types.rs         # 命令类型
│   ├── registry.rs      # 命令注册表和处理程序
│   └── mod.rs           # 模块导出
├── config/              # 配置
│   ├── paths.rs         # 配置/数据目录路径解析
│   └── settings.rs      # 设置模型和加载
├── engine/              # 核心引擎
│   ├── messages.rs      # 对话消息类型
│   └── query.rs         # 带工具使用循环的查询引擎
├── hooks/               # 可扩展事件处理
│   ├── events.rs        # Hook 事件类型
│   ├── executor.rs      # 带超时的 Hook 执行
│   ├── registry.rs      # Hook 注册表
│   └── builtins.rs      # 内置 Hooks
├── mcp/                 # 模型上下文协议支持
│   ├── client.rs        # 带 stdio/sse 传输的 MCP 客户端
│   ├── config.rs        # MCP 服务器配置加载
│   └── types.rs         # MCP 类型和模式
├── memory/              # 持久上下文内存
│   ├── manager.rs       # 内存管理器
│   └── types.rs         # 内存类型
├── permissions/         # 权限检查
│   ├── modes.rs         # 权限模式
│   └── checker.rs       # 权限检查器
├── plugins/             # 插件系统
│   ├── loader.rs        # 插件加载器
│   └── installer.rs     # 插件安装器
├── services/            # 后台服务
│   ├── backend_server.rs # React TUI 的 WebSocket 服务器
│   ├── stdio_backend.rs  # 带 OHJSON 协议的 Stdio 后端
│   ├── session_storage.rs # 会话持久化
│   └── cron.rs          # Cron 调度器
├── skills/              # Skills 系统
│   ├── loader.rs        # 从 .md/.skill 文件加载
│   ├── registry.rs      # Skill 注册表
│   └── installer.rs     # 从 GitHub 下载安装 Skills
├── tools/               # AI 代理工具包
│   ├── base.rs          # 工具特征和注册表
│   ├── bash.rs          # Shell 命令执行
│   ├── file_read.rs     # 读取文件
│   ├── file_write.rs    # 写入文件
│   ├── file_edit.rs     # 编辑文件
│   ├── glob.rs          # 文件模式匹配
│   ├── grep.rs          # 内容搜索
│   ├── web_fetch.rs     # 获取 URL
│   ├── web_search.rs    # 网络搜索
│   └── mcp.rs           # MCP 工具集成
└── ui/                  # 用户界面
    └── repl.rs          # REPL（读取 - 求值-输出循环）
```

## 斜杠命令

| 命令 | 描述 |
|------|------|
| `/help` | 显示可用命令 |
| `/clear` | 清除对话历史 |
| `/exit` | 退出 REPL |
| `/status` | 显示会话状态 |
| `/usage` | 显示 token 使用统计 |
| `/skills` | 列出或显示可用 skills |
| `/skills list` | 列出所有已安装的 skills |
| `/skills show <name>` | 显示 skill 内容 |
| `/skills install <name\|url>` | 从 SkillsMP 或 GitHub URL 安装 skill |
| `/skills search <query>` | 在 SkillsMP 中搜索 skills |
| `/skills remove <name>` | 移除已安装的 skill |
| `/plugin` | 管理插件（列出/安装/卸载/启用/禁用） |
| `/hooks` | 显示配置的 hooks |
| `/mcp` | 列出配置的 MCP 服务器 |
| `/mcp list` | 列出所有配置的 MCP 服务器 |
| `/mcp query <server-name>` | 查询特定 MCP 服务器的配置详情 |
| `/config` | 显示或更新配置 |
| `/memory` | 管理项目内存（列出/显示/添加/删除） |
| `/resume <id>` | 恢复之前的会话 |
| `/sessions` | 列出所有保存的会话 |
| `/export` | 导出当前会话为 markdown |
| `/delete_session <id>` | 删除会话 |
| `/init` | 初始化默认配置 |
| `/version` | 显示版本信息 |
| `/permissions` | 更改权限模式 |
| `/plan` | 切换计划模式 |

## MCP (Model Context Protocol)

MCP 服务器可在 `~/.rust_harness/settings.json` 中配置：

```json
{
  "mcp_servers": {
    "test-server": {
      "name": "test-server",
      "command": "node",
      "args": ["/path/to/server.js"],
      "transport": "stdio",
      "enabled": true
    }
  }
}
```

支持的传输方式：
- **stdio**: 生成进程并通过 stdin/stdout 通信
- **sse**: 连接到 Server-Sent Events 端点

## ACP (Agent Communication Protocol)

RustHarness 支持 ACP 协议，用于 AI 代理间的互操作通信。

**启动 ACP 服务器：**

```bash
cargo run -- --acp-server 8080
```

**端点：**
- `GET /.well-known/agent.json` - AgentCard 发现
- `GET /acp` - AgentCard 信息
- `POST /tasks` - 创建任务
- `GET /tasks/{id}` - 获取任务状态
- `POST /tasks/{id}/send` - 发送消息
- `GET /tasks/{id}/artifacts` - 获取任务产出物

详见 [ACP.md](ACP.md) 完整文档。

## CLI 选项

| 选项 | 描述 |
|------|------|
| `-m, --model <MODEL>` | 模型别名或完整模型 ID |
| `-p, --print <PRINT>` | 打印响应后退出（非交互模式） |
| `-c, --continue` | 继续最近的对话 |
| `-r, --resume <RESUME>` | 按会话 ID 恢复对话 |
| `-s, --system-prompt` | 覆盖默认系统提示词 |
| `-k, --api-key` | API 密钥（或设置 ANTHROPIC_API_KEY 环境变量） |
| `--base-url` | 自定义 API 基础 URL |
| `--api-format` | API 格式：'anthropic' 或 'openai' |
| `--permission-mode` | 权限模式：default、plan 或 full_auto |
| `--bare` | 最小模式：跳过 hooks、plugins、MCP |
| `--backend-only` | 运行 WebSocket 服务器供 React TUI 前端使用 |
| `--stdio-backend` | 运行 stdio 后端（OHJSON 协议） |

## 环境变量

- `ANTHROPIC_API_KEY` - API 密钥
- `ANTHROPIC_MODEL` - 默认模型
- `ANTHROPIC_BASE_URL` - 自定义 API 基础 URL
- `RUST_HARNESS_CONFIG_DIR` - 自定义配置目录
- `RUST_HARNESS_MODEL` - 模型覆盖
- `RUST_HARNESS_BASE_URL` - 基础 URL 覆盖
- `RUST_HARNESS_MAX_TOKENS` - 最大 token 数覆盖
- `RUST_HARNESS_API_FORMAT` - API 格式覆盖

## 构建

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release

# 运行测试
cargo test

# 代码检查
cargo clippy -- -D warnings

# 代码格式化
cargo fmt
```

## 实现状态

### 核心功能
- ✅ CLI 带所有标志 (clap)
- ✅ API 客户端带重试逻辑（指数退避，3 次重试）
- ✅ 带工具使用循环的查询引擎
- ✅ 工具注册表和执行框架
- ✅ 权限检查器集成
- ✅ 11+ 核心工具（bash、文件 I/O、搜索、网络）
- ✅ 带 stdio/sse 传输的 MCP 客户端
- ✅ 长对话自动压缩
- ✅ Token 使用统计
- ✅ WebSocket 后端服务器 (axum)
- ✅ 带 OHJSON 协议的 Stdio 后端
- ✅ 带环境变量覆盖的配置系统
- ✅ 权限模式（Default/Plan/FullAuto）
- ✅ 带 8 个事件的 Hooks 系统
- ✅ 斜杠命令（18 个命令）
- ✅ 从 ~/.rust_harness/skills/ 加载 Skills
- ✅ 插件加载器（claude-code 兼容）
- ✅ 带 MEMORY.md 持久化的内存管理器
- ✅ 会话存储和恢复功能
- ✅ 多代理协调系统（4 种协作模式）
- ✅ ACP 协议支持

## 许可证

[查看 LICENSE 文件](LICENSE)
