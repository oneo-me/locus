# Locus

Locus 是本地优先的 Markdown 知识工作台，使用 Rust 和 GPUI 构建原生桌面界面。用户可以阅读与组织笔记、查看文件变化、打开笔记画布和文档库终端，并通过 MCP 让外部 Agent 发现和维护同一套本地文件。

Markdown 文档树始终是知识内容的唯一事实来源。外部 Agent 可通过标准 MCP 接口定位文件后自行编辑，也可通过完整 MCP 接口直接操作笔记库；其执行和模型配置由对应客户端负责。

## 使用

启动后在设置中选择本地文档库。每篇笔记由目录中的 `index.md` 表达，目录可以嵌套；标题、图标、概要和归档状态使用 YAML frontmatter。`assets/` 存放附件，`assets/icon.png` 是专用图标，`canvas/package.json` 声明 React 组件包，`exports` 指向默认导出页面组件的 TSX/JSX 源码；`assets` 和 `canvas` 为保留目录，不作为子笔记。画布目录可包含脚本、样式、图片及嵌套资源。

通过侧栏或命令面板选择笔记，使用标题栏操作复制引用、打开外部编辑器或显示所属目录。归档笔记可在设置中恢复。文件由外部工具修改后，应用重新读取变化；Git 状态用于提示正文、画布和附件的变化。

普通笔记页面在正文头部展示概要和自定义 YAML 数据。

笔记存在 `canvas/package.json` 时，标题栏和头部元数据以下显示画布。Locus 使用电脑已安装的 Node.js 与 npm 启动本地 Vite 服务，提供 React、Tailwind、跨笔记组件导入及保存后的热更新；不捆绑 Node.js/npm。需要 Node.js 20.19+ 或 22.12+（或更新的受支持主版本）；首次安装依赖需要 npm 注册表访问。没有画布包时显示 Markdown 正文，已有画布文件不会自动转换或删除。组件包格式、复用方式和 Agent 工作流程见[画布编写规范](resources/docs/canvas.md)，可参考[组件模板](crates/locus-canvas/web/template)。

在“设置 → 终端”选择系统终端或 Ghostty，并选择 Codex、pi、OpenCode 或 Claude Code。默认为系统终端与 Codex。打开 Agent 时，应用调用外部终端，在当前笔记库根目录运行对应的 `codex`、`pi`、`opencode` 或 `claude` 命令；需要先安装所选终端和 Agent。外部会话由终端软件管理，切换笔记库或退出 Locus 不结束会话，设置变化只影响后续启动。

macOS 系统终端使用 Terminal，Ghostty 使用其 [AppleScript 接口](https://ghostty.org/docs/features/applescript)（需要 1.3.0 或以上）；两者均先后台启动，再创建 Agent 会话并激活，复用已运行的应用。Terminal 创建新窗口；Ghostty 优先在最近使用的已有窗口中新建标签页，没有窗口时才创建新窗口。启动时加载 zsh 登录与交互配置，Agent 退出或找不到命令时保留 Shell，便于查看错误。Windows 使用系统控制台，不提供 Ghostty 选项。以下为 macOS 快捷键：

| 操作 | 快捷键 |
| --- | --- |
| 命令面板 | Command+E |
| 设置 | Command+, |
| 打开 Agent | Command 与反引号组合键 |
| 刷新当前笔记或获得焦点的画布 | Command+R 或 F5 |
| 关闭窗口 | Command+W |

设置中可启停 MCP、配置局域网访问和端口，并查看运行状态；侧边栏左下角也保留只读状态。应用不提供连接引导、地址展示或复制功能。标准接口 `/mcp` 与完整接口 `/mcp/full` 同时提供，共用服务、端口与局域网设置；服务默认关闭，默认端口为 `2301`，客户端连接方式见下方使用说明。标准接口保留三个发现工具及原始文件路径；完整接口通过逻辑地址读取、编辑、上传、下载、移动和删除笔记、附件、图标及画布，不返回服务器文件路径。局域网模式没有身份验证，网络客户端可以读写和永久删除库内内容。工具、传输和冲突规则见 [MCP 使用说明](crates/locus-mcp/README.md)。

## 从源码运行

需要 Rust 工具链及对应平台的构建环境：

- macOS：Xcode 与 Command Line Tools，最低系统版本为 macOS 13。
- Windows：Visual Studio C++ 工具链与 Windows SDK。
- 使用画布还需安装前文所述版本的 Node.js 与 npm。

在项目目录运行：

```sh
cargo run
```

## 构建应用

安装构建工具后，在项目目录执行：

```sh
cargo install --path crates/oneo-tool-packer --locked
oneo-tool-packer
```

构建配置位于 [packer.toml](packer.toml)。默认构建当前平台的 Release 版本，产物保存到 `.build/<target>/`：macOS 为 `.app`，Windows 为包含可执行文件与运行依赖的目录。

macOS 默认使用 ad-hoc 签名。构建 Windows 版本需要在 Windows 环境中运行。
