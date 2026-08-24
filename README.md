# Locus

Locus 是本地优先的私有知识与 Agent 工作台。它把本地 Markdown 文档工作区与现成 Agent runtime 的会话控制放进同一个产品，让知识阅读、任务发起、执行观察和结果审阅形成连续流程。

Locus 不自行实现 Agent loop、模型调用或工具系统。Agent 的提示词、上下文、工具、权限、认证与执行由各自 runtime 负责；Locus 负责工作区、文档体验、会话控制、协议适配与统一展示。

## 当前状态

仓库当前提供可运行的文档工作区基线，包括层级 Markdown 浏览与搜索、文章与附件阅读、知识图谱、Git 变更视图、本机设置、文件事件更新和 MCP 文档发现。

Agent 会话、Agent runtime 适配和目标中的前后端包拆分尚未实现。现阶段以已有文档能力为工作基础，逐步扩展为完整的 Locus，不用未完成的架构替换可运行产品。

## 产品方向

- 文档工作区：浏览、搜索和阅读层级化 Markdown，查看附件、关系与本地变更。
- Agent 会话：从工作区发起和继续任务，统一展示文本、推理、工具活动、权限请求与运行状态。
- Agent 适配：连接现成 Agent CLI 或 server，把不同协议归一为稳定的控制命令与事件。
- 知识协作：Agent 通过自身文件工具访问工作区，文档变化回到同一 Markdown 事实来源。
- 本地持久化：文档、会话、设置与派生数据按职责保存在用户自己的设备或私有环境中。

## 技术方向

- 前端：React + React Router Framework Mode，目标采用 SPA 模式。
- 界面：Tailwind CSS v4 与 shadcn/ui；共享组件、主题 Tokens 和样式由独立 UI 包维护。
- 后端：Hono，负责领域 API、本地文件系统、Agent runtime 和持久化能力。
- 工程组织：pnpm workspace monorepo；应用位于 `apps/*`，领域与界面包位于 `packages/*`。
- 代码质量：Oxlint、Oxfmt 和 TypeScript。

目标包边界为 `@locus/web`、`@locus/server`、`@locus/ui`、`@locus/agent-core`、`@locus/agent-ui`、`@locus/document-ui` 和 `@locus/document-library`。当前只有 `@locus/web`，其余包会在对应能力进入实现时建立。

## 本地运行

### 环境要求

- Node.js 22 或更高版本
- pnpm 11.20 或更高版本
- Git；文档库使用 Git LFS 时还需要 Git LFS

### 安装与启动

```bash
pnpm install
ASTERWEFT_LIBRARY_PATH=/absolute/path/to/markdown-library pnpm dev
```

也可以在 `apps/web/.env` 中配置文档库路径：

```dotenv
ASTERWEFT_LIBRARY_PATH=/absolute/path/to/markdown-library
```

开发服务器默认运行在 <http://localhost:3000>。

### 文档库结构

文档库由包含 `index.md` 的文章目录组成：

```text
my-library/
└── welcome/
    └── index.md
```

```markdown
---
title: 欢迎
tags: [示例]
---

# 欢迎
```

Markdown 文件是内容的唯一事实来源；索引、图谱和 Git 状态均为可重新派生的视图。

## 常用命令

```bash
pnpm dev
pnpm build
pnpm test
pnpm typecheck
pnpm check
pnpm format
```

项目的产品、设计、架构和包边界以 `asterweft://projects/locus` 下的 Locus 笔记为准。
