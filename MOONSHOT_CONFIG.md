# Moonshot API 配置修正

## 当前配置问题

你的配置（`~/.rust_harness/settings.json`）：
```json
{
  "base_url": "https://api.moonshot.cn/anthropic",
  "api_format": "anthropic"
}
```

问题：Moonshot K2.5 模型使用 **OpenAI 兼容 API**，不是 Anthropic 兼容 API。

## 修正后的配置

### 选项 1：使用 OpenAI 格式（推荐）

修改 `~/.rust_harness/settings.json`：

```json
{
  "api_key": "sk-ctv5yzCJV7l1JYPj5W7RXVx48Cy05VxqyfELFCzEVU0PsCj3",
  "model": "kimi-k2.5",
  "max_tokens": 4096,
  "base_url": "https://api.moonshot.cn/v1",
  "api_format": "openai",
  "system_prompt": "you are a useful agent!"
}
```

### 选项 2：使用标准 Moonshot 端点

如果上面的端点不工作，尝试：

```json
{
  "base_url": "https://api.moonshot.cn/v1",
  "api_format": "openai"
}
```

## 关键变更

| 配置项 | 原值 | 修正值 | 原因 |
|--------|------|--------|------|
| `base_url` | `/anthropic` | `/v1` | Moonshot 使用 OpenAI 兼容端点 |
| `api_format` | `anthropic` | `openai` | 使用正确的 API 协议格式 |

## 测试命令

```bash
cargo run -- -p "Hello"
```

## 当前实现状态

代码已更新支持 Moonshot API：

1. **自动检测** - 通过以下方式检测 OpenAI 兼容 API：
   - `base_url` 包含 "moonshot" 或 "openai"
   - `api_key` 以 "sk-" 开头

2. **URL 构建** - 避免路径重复：
   - 检查 `base_url` 是否已包含 `/v1`
   - 正确构建：`https://api.moonshot.cn/v1/chat/completions`

3. **认证处理** - 使用 Bearer Token：
   - OpenAI 格式：`Authorization: Bearer sk-xxx`
   - Anthropic 格式：`x-api-key: sk-xxx`

4. **响应解析** - 支持两种格式：
   - Anthropic: `content[]` 数组
   - OpenAI: `choices[].message.content`

## 测试报告

运行测试验证实现：

```bash
cargo test api::client
```

参见 `MOONSHOT_TEST_REPORT.md` 获取详细测试结果。

## Moonshot API 参考

- 文档：https://platform.moonshot.cn/docs/api
- 端点：`https://api.moonshot.cn/v1/chat/completions`
- 认证：`Authorization: Bearer sk-xxx`
- 支持模型：`kimi-k2.5`, `kimi-plus`, 等
