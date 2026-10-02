# Locus 架构

本篇是当前系统的总览。修改代码前完整阅读，再按[专题](#专题)选择与任务相关的详细说明。产品介绍与运行方式见 [README.md](README.md)，可见界面的约定由桌面工作区专题维护。

Locus 是本地优先的 Markdown 知识工作台，使用 Rust 与 GPUI Kit 构建原生桌面界面。用户阅读、组织和审阅本地笔记，外部编辑器与 Agent 操作同一套文件。应用提供标准 MCP 文件发现与完整 MCP 笔记库操作两个入口。

## 核心约束

1. **文档树拥有内容事实。** 每篇笔记由目录中的 `index.md` 表达，拥有 `assets/`、`canvas/` 和子笔记；无正文目录可以组织后代。
2. **派生状态可重建。** 导航、搜索、关系、Git 状态和图像缓存来自真实文件，配置和缓存不能成为内容权威。
3. **所有权明确。** 工作区编排选择与呈现，设置实体拥有运行时偏好，共享壳拥有窗口和通用布局，文档库负责文件模型与安全边界，MCP 控制器负责服务生命周期。
4. **两个 MCP 入口共享文件与服务。** `/mcp` 返回原始路径供客户端文件工具协作；`/mcp/full` 通过逻辑地址直接维护文档库。外部客户端拥有模型调用、执行权限及会话。
5. **画布由当前笔记拥有。** HTML/JavaScript 或 JSON 模版调用在 macOS 系统 WebView 内运行，工作区管理创建释放，画布包管理网页、资源与导出。
6. **通用能力保持产品中立。** `oneo-gpui*` 接受中性配置、标签、数据和事件；Locus 产品模型、偏好与本地化留在产品包。

## 进程与数据流

```text
Locus 原生桌面进程
├─ GPUI / OS event loop
│  └─ 主窗口：文档导航 + 当前笔记 / 内嵌设置
│     └─ macOS WebKit：当前笔记画布
└─ 可选 MCP 线程：Tokio + Axum + rmcp
   ├─ /mcp：文件发现
   └─ /mcp/full：逻辑对象与内容操作

本地文档树 ← 桌面应用 / MCP / 外部编辑器与 Agent
外部终端 ← 工作区按需启动所选 Agent
```

启动加载偏好与界面状态、解析外观与语言，再创建工作区。文档库扫描生成导航与 Git 投影，递归观察文件；选择笔记后读取完整正文。当前笔记有画布时，头部以下呈现画布，否则呈现 Markdown。

文件事件合并后在后台重建投影，当前笔记相关变化重新读取正文。读取结果按库路径与代次隔离，迟到任务不能覆盖新选择。画布入口新增或删除更新页面模式，已经加载的网页通过手动刷新或重新打开读取新源码。

MCP 按调用访问同一文档库，完整操作串行执行并进行版本检查。工作区将设置映射为显式服务配置，变化时先停后启。外部 Agent 在触发时的文档库根目录启动，终端软件管理其认证、命令执行与后续会话。

## 模块边界

源码是 Rust 2024 Cargo workspace，默认运行包为 `locus`。GPUI Kit 直接依赖基线来自 `source/Cargo.toml`，解析版本由 `source/Cargo.lock` 固定；同一进程中的应用与共享组件使用同一个 GPUI 类型实例。

| 包 | 责任 |
| --- | --- |
| `locus` | 产品初始化、应用启动、菜单及工作区组装 |
| `locus-workspace` | 导航、阅读、画布编排、终端交接、界面状态与产品动作 |
| `locus-settings` | 偏好实体、设置页面、归档恢复与设置事件 |
| `locus-preferences` | 产品字段、默认值、校验、存储名称与选项标签 |
| `locus-library` | 文件模型、扫描、读写、观察、版本、链接与 Git 投影 |
| `locus-mcp` | 工具与资源契约、传输会话、监听和停止控制 |
| `locus-canvas` | WebView 宿主、资源协议、网页生命周期、主题与导出 |
| `locus-ui` | Markdown 扩展、图标与图像缓存、平台文件和终端打开 |
| `oneo-gpui` | 应用壳、窗口、布局、外观控件和 TOML 配置服务 |
| `oneo-gpui-webview` | macOS 原生网页合成、裁剪、可见性、输入与焦点 |
| `oneo-gpui-graph` / `oneo-gpui-terminal` | 独立通用组件与示例，当前 Locus 产品未接入 |
| `oneo-tool-packer` | 独立构建与应用组装 CLI |

产品依赖主线为 `locus → locus-workspace → locus-settings`。工作区还消费画布、UI、偏好、文档库和 MCP 的公开接口；设置消费 UI、偏好、文档库及 MCP。画布消费文档库、共享语言与 macOS WebView；UI 消费文档库和偏好。MCP 的产品领域依赖只有文档库；文档库不依赖 GUI。功能包不反向依赖应用二进制。

## 存储

| 位置 | 所有者与内容 |
| --- | --- |
| 用户选择的文档库 | Markdown、frontmatter、附件、画布与目录层级 |
| `~/.oneo/locus/settings.toml` | 用户偏好，包括文档库、编辑器、导出目录、外观、终端与 MCP |
| `~/.oneo/locus/state.toml` | 窗口恢复、当前选择与侧栏状态 |
| `~/.oneo/locus/sidebar-order.toml` | 按文档库及父路径保存的同级顺序 |
| `~/.oneo/locus/thumbnails/`、`link-icons/` | 可重建图像缓存 |

配置服务执行 TOML 读取与原子保存，运行时实体拥有配置变更。笔记操作使用安全路径与内容版本检查；外部编辑器参与乐观冲突检查，多个文件的操作不承诺崩溃后的事务回滚。

## 专题

每个主题维护当前机制、边界与验证方法。总览上限为 16 KiB，每篇架构专题上限为 24 KiB；在仓库根执行 `bun run check:docs` 检查体积、索引及相对链接与标题锚点。

| 文档 | 修改这些内容前阅读 |
| --- | --- |
| [文档库](docs/architecture/library.md) | 文件结构、地址、扫描、观察、Git、内容操作与安全边界 |
| [桌面工作区](docs/architecture/workspace.md) | Tree 导航、阅读与 YAML、历史、菜单、命令与焦点 |
| [画布](docs/architecture/canvas.md) | HTML/JSON、模版、资源、网页生命周期、合成与导出 |
| [Agent 接入](docs/architecture/mcp.md) | 标准/完整 MCP、传输、冲突、Host、服务与内置协议 |
| [设置与持久化](docs/architecture/settings.md) | 偏好所有权、配置 IO、外观、语言与保存调度 |
| [共享应用壳](docs/architecture/shell.md) | 初始化、窗口、布局、Debug FPS 与平台行为 |
| [文档库终端](docs/architecture/terminal.md) | 终端选择、命令、路径转义、交接与失败处理 |
| [WebView 接入](docs/architecture/webview.md) | 通用原生组件的构造、可见性、输入与合成限制 |
| [应用打包](docs/architecture/packaging.md) | CLI、目标、图标、资源、签名、运行库与产物替换 |

## 仓库布局

```text
AGENTS.md               开发与文档维护约定
ARCHITECTURE.md         系统总览与专题索引
README.md               产品介绍与快速开始
docs/architecture/      每个主题一份当前系统说明
docs/build.md           运行、构建与应用交付
docs/mcp.md             客户端连接与 MCP 使用
docs/requirements.md    LOC 需求编号与既有状态
package.json            Bun 工作区与开发命令入口
scripts/                Bun + TypeScript 开发脚本
source/Cargo.toml       工作区及直接依赖基线
source/Cargo.lock       解析后的版本
source/crates/          产品与通用 Rust 包
source/resources/docs/  编译嵌入并随应用打包的英文协议
source/packer.toml      应用身份与打包资源
.vscode/                开发与构建任务
dist/                   应用产物，Git 忽略
```

根目录 Bun 工作区管理开发脚本与 JavaScript/TypeScript 子包，Cargo 工作区管理 `source/crates/`。当前没有 JavaScript/TypeScript 子包，`package.json` 的 `workspaces` 为空；新增时登记实际包路径。应用版本继续由 `source/Cargo.toml` 维护。

## 验证与已知限制

日常检查规则见 [AGENTS.md](AGENTS.md)，运行与平台构建环境见[构建说明](docs/build.md)。需求编号及既有状态见[需求索引](docs/requirements.md)。历史检查记录不代表当前提交已重新验证。

原生画布当前支持 macOS。Windows 构建与原生行为需要 Windows 工具链和实机验证，macOS 编译不能代替；既有记录曾因缺少 `x86_64-w64-mingw32-windres` 而未完成 Windows GNU 交叉检查。中文混排和表格文字裁切的已知问题见工作区专题。退出保存受 GPUI 等待时限约束，具体边界见设置专题。
