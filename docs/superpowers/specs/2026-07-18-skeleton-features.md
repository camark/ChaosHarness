# Skeleton Features 待实现功能清单

**Date:** 2026-07-18
**Status:** Completed ✅

## Tools (返回占位符，无真实逻辑)

- [x] `tools/task_list.rs` — 列出后台任务（返回 "No tasks found"）
- [x] `tools/task_get.rs` — 获取任务详情（返回 "not found"）
- [x] `tools/task_output.rs` — 读取任务输出（返回 "(no output)"）
- [x] `tools/task_create.rs` — 创建后台任务（部分实现，agent/shell 任务为 stub）
- [x] `tools/task_update.rs` — 更新任务状态（占位符）
- [x] `tools/task_stop.rs` — 停止任务（占位符）
- [x] `tools/cron_create.rs` — 创建定时任务（占位符）
- [x] `tools/cron_delete.rs` — 删除定时任务（占位符）
- [x] `tools/cron_toggle.rs` — 启用/禁用定时任务（占位符）
- [x] `tools/remote_trigger.rs` — 触发定时任务（返回 "not implemented"）
- [x] `tools/team_create.rs` — 创建团队（占位符）
- [x] `tools/team_delete.rs` — 删除团队（占位符）
- [x] `tools/lsp.rs` — LSP 代码智能（基于正则的符号提取）
- [x] `tools/config.rs` — 配置管理（save 操作已实现）
- [x] `tools/mcp_auth.rs` — MCP 认证（已接入全局 MCP 管理器）
- [x] `tools/read_mcp_resource.rs` — 读取 MCP 资源（已接入全局 MCP 管理器）
- [x] `tools/list_mcp_resources.rs` — 列出 MCP 资源（已接入全局 MCP 管理器）

## Services

- [x] `services/cron.rs` — Cron 任务管理（已实现持久化）
- [x] `ui/repl.rs:207` — `run_resume_session` 已实现会话恢复
- [x] `prompts/context.rs` — 上下文构建（已实现真实上下文）

## 模块级 skeleton

- [x] `multi_agent/` — 已接入 query loop，支持 swarm 创建和任务执行

## 实现顺序

1. Task tools (task_list, task_get, task_output, task_create, task_update, task_stop)
2. Cron tools (cron_create, cron_delete, cron_toggle, remote_trigger)
3. Team tools (team_create, team_delete)
4. Other tools (config, lsp, mcp_auth, read_mcp_resource, list_mcp_resources)
5. Services (cron, session resume)
