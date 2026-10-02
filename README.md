# Locus

Locus 是本地优先的 Markdown 知识工作台，使用 Rust 和 GPUI 构建原生桌面界面，服务于长期维护个人知识的开发者、创作者和一人团队。用户可以阅读与组织笔记、查看文件变化、打开笔记画布和外部终端，并通过 MCP 让 Agent 发现和维护同一套本地文件。

Markdown 文档树始终是知识内容的唯一事实来源，用户可以直接访问、编辑和版本管理。外部 Agent 可通过标准 MCP 接口定位文件后自行编辑，也可通过完整 MCP 接口直接操作笔记库；模型调用、账户、权限、远程数据策略与计费由对应客户端负责。

## 快速开始

启动应用后，在设置中选择本地 Markdown 文档库。每篇笔记由目录中的 `index.md` 表达；应用会读取外部编辑器与 Agent 对真实文件的修改。

从源码运行需要 Rust 和平台构建工具，在项目根目录执行：

```sh
cd source
cargo run
```

平台要求、构建命令与产物说明见[运行与构建](docs/build.md)。

## 笔记与画布

文档库是用户选择的本地根目录。目录层级组织笔记，每篇笔记的标题、图标、概要和归档状态来自 `index.md` 的 YAML frontmatter。`assets/` 存放附件，`assets/icon.png` 为笔记图标；`assets` 和 `canvas` 是保留目录，不作为子笔记。

通过侧栏或命令面板选择笔记，标题栏可以复制引用，笔记菜单提供编辑、打开所在目录、图标上传和归档；侧栏右键菜单也绑定对应笔记。归档项在设置中恢复。外部文件变更会更新应用的派生视图，Git 状态提示正文、画布和附件的变化。

Markdown 支持连续文本选择、全选和复制。概要和自定义 YAML 在正文头部展示，对象与数组递归展开并保留顺序、空集合与 null。实现和已知排版问题见[桌面工作区](docs/architecture/workspace.md)。

有 `canvas/index.html` 与 `canvas/main.js` 的笔记运行 HTML/JavaScript 画布，模版调用方使用互斥的 `canvas/index.json`。原生画布当前支持 macOS，标题栏和头部以下显示网页，内置 Tailwind，通过系统 WebView 读取文件。画布笔记的 `index.md` 只保留 frontmatter；普通笔记显示 Markdown 正文。

保存画布后手动刷新或重新进入笔记生效，离开时释放网页临时状态。“下载画布”导出已保存的入口、数据和引用资源；默认保存到系统下载目录，可在通用设置中修改。结果为单个 HTML 或带 `canvas.html` 和资源的目录，同名自动编号；动态资源需先加载，在线数据仍需网络，模块或 Worker 可能需要本地 HTTP 服务。编写规则见随软件发布的[画布协议](source/resources/docs/canvas.md)与[附件协议](source/resources/docs/attachments.md)。

## 外部 Agent

在终端设置中选择系统终端或 Ghostty，并选择 Codex、pi、OpenCode 或 Claude Code，默认系统终端与 Codex。需要先安装所选软件。Locus 在当前文档库根目录运行 `codex`、`pi`、`opencode` 或 `claude`；切换文档库、修改设置或退出 Locus 不结束已有外部会话。

macOS 系统终端使用 Terminal，每次创建新窗口；Ghostty 需要 1.3.0 或以上，优先在最近使用的窗口中新建标签页，没有窗口时创建窗口。启动加载 zsh 登录与交互配置，Agent 退出或找不到命令时保留 Shell。Windows 使用系统控制台。平台启动与错误处理见[文档库终端](docs/architecture/terminal.md)。

| 操作 | macOS 快捷键 |
| --- | --- |
| 命令面板 | Command+E |
| 设置 | Command+, |
| 打开 Agent | Command 与反引号 |
| 刷新当前笔记或聚焦的画布 | Command+R 或 F5 |
| 关闭窗口 | Command+W |

其他平台使用对应的 Control 组合键。MCP 在设置中启用，默认关闭、端口为 `2301`；标准 `/mcp` 与完整 `/mcp/full` 共用服务。局域网模式没有身份验证，客户端可以读写和永久删除库内内容。连接与调用方式见[MCP 使用说明](docs/mcp.md)。

## 文档

- [架构](ARCHITECTURE.md)：模块边界、数据流、存储与专题索引。
- [开发约定](AGENTS.md)：阅读顺序、工程规则与检查命令。
- [运行与构建](docs/build.md)：平台环境、版本与打包。
- [MCP 使用说明](docs/mcp.md)：客户端连接与工具契约。
- [需求索引](docs/requirements.md)：需求编号与既有实现状态。

源码位于 `source/`，Bun 开发脚本位于根目录 `scripts/`，最终应用产物位于 `dist/`。在仓库根执行 `bun run check:docs` 检查文档链接、专题索引与体积预算。
