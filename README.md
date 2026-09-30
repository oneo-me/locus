# Locus

Locus 是本地优先的 Markdown 知识工作台，使用 Rust 和 GPUI 构建原生桌面界面。用户可以阅读与组织笔记、查看文件变化、打开笔记画布和文档库终端，并通过 MCP 让外部 Agent 发现和维护同一套本地文件。

Markdown 文档树始终是知识内容的唯一事实来源。外部 Agent 可通过标准 MCP 接口定位文件后自行编辑，也可通过完整 MCP 接口直接操作笔记库；其执行和模型配置由对应客户端负责。

## 快速开始

启动应用后，在设置中选择本地 Markdown 文档库。每篇笔记由目录中的 `index.md` 表达；应用会读取外部编辑器与 Agent 对真实文件的修改。

从源码运行需要 Rust 和平台构建工具，在项目根目录执行：

```sh
cd source
cargo run
```

平台要求、构建命令与产物说明见[运行与构建](docs/usage/build.md)。

## 文档

- [项目文档](docs/README.md)：产品定义、视觉规范与完整导航。
- [使用说明](docs/usage/README.md)：笔记、画布与外部 Agent。
- [MCP 使用说明](docs/usage/mcp.md)：客户端连接与工具契约。
- [需求](docs/requirements.md)：需求标识与实现状态。
- [工程](docs/engineering.md)：目录结构、模块边界与验证。

源码位于 `source/`，文档位于 `docs/`，最终应用产物位于 `dist/`。本地 `AGENTS.md` 与构建产物由 Git 忽略。
