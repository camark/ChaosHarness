# /skills 命令使用指南

## 概述

`/skills` 命令用于管理 AI 技能，支持从 SkillsMP 市场或 GitHub 搜索、下载、安装和管理技能。

## 子命令

### `/skills list`

列出所有已安装的技能。

```bash
/skills list
```

**输出示例：**
```
Installed skills (3 total):
  claude-api: Interact with Anthropic's API
  web-searcher: Search the web for information
  code-reviewer: Review code for best practices
```

### `/skills show <name>`

显示指定技能的内容。

```bash
/skills show claude-api
```

### `/skills install <name|url>`

安装技能。支持两种格式：

1. **按名称安装** - 从 SkillsMP/GitHub 搜索并安装
2. **按 URL 安装** - 直接从 GitHub URL 安装

```bash
# 按名称安装（从 GitHub 搜索）
/skills install claude-api

# 从 GitHub URL 安装
/skills install https://github.com/user/repo/blob/main/skill.md
```

**输出示例：**
```
Installed skill 'claude-api' from anthropics:
  ~/.rust_harness/skills/claude-api.md
```

### `/skills search <query>`

搜索 SkillsMP 市场上的技能。

```bash
/skills search code review
```

**输出示例：**
```
Found 5 skills for 'code review':
  - code-reviewer by anthropics: Review code for best practices
  - rust-review by heilcheng: Rust code review and optimization
  - python-linter by sanjay3290: Python linting and formatting
```

### `/skills remove <name>`

删除已安装的技能。

```bash
/skills remove claude-api
```

**输出示例：**
```
Removed skill: claude-api
```

## 技能来源

### SkillsMP

SkillsMP (https://skillsmp.com) 是一个聚合了 700,000+ AI 技能的市场平台。它从 GitHub 自动索引包含 `SKILL.md` 文件的仓库。

### GitHub

任何 GitHub 仓库中包含的 `.md` 或 `SKILL.md` 文件都可以作为技能安装。

## 技能格式

技能文件使用 Markdown 格式，可选的 YAML frontmatter：

```markdown
---
name: example-skill
description: An example skill
---

# Example Skill

This skill provides example functionality.

## Instructions

1. First step
2. Second step
```

## 技能目录

- **用户技能**: `~/.rust_harness/skills/`
- **项目技能**: `<project>/.rust_harness/skills/`

## 环境变量

安装技能时可以配置以下环境变量：

- `GITHUB_TOKEN` - GitHub API token（提高搜索限制）

## 示例

### 安装 Anthropic API 技能

```bash
/skills install claude-api
```

### 搜索代码审查技能

```bash
/skills search code review
```

### 安装特定 GitHub 技能

```bash
/skills install https://github.com/anthropics/skills/blob/main/skills/claude-api/SKILL.md
```

### 查看已安装的技能

```bash
/skills list
```

### 删除技能

```bash
/skills remove claude-api
```

## 故障排除

### 技能未找到

- 检查技能名称是否正确
- 尝试使用 `/skills search` 搜索类似技能

### 下载失败

- 检查网络连接
- GitHub URL 是否正确
- 如果是私有仓库，需要设置 `GITHUB_TOKEN` 环境变量

### 技能不工作

- 检查技能文件格式是否正确
- 确保技能文件在正确的目录中
