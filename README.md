# Locus

Locus 是本地优先的 Markdown 知识工作台，使用 Rust 和 GPUI 构建原生桌面界面。用户可以阅读与组织笔记、查看文件变化、打开笔记画布和文档库终端，并通过 MCP 让外部 Agent 发现和维护同一套本地文件。

Markdown 文档树始终是知识内容的唯一事实来源。外部 Agent 可通过标准 MCP 接口定位文件后自行编辑，也可通过完整 MCP 接口直接操作笔记库；其执行和模型配置由对应客户端负责。

## 使用

启动后在设置中选择本地文档库。每篇笔记由目录中的 `index.md` 表达，目录可以嵌套；标题、图标、概要和归档状态使用 YAML frontmatter。`assets/` 存放附件，`assets/icon.png` 是专用图标，`canvas/index.html` 与 `canvas/main.js` 定义 HTML 画布；模版调用方使用互斥的 `canvas/index.json`；`assets` 和 `canvas` 为保留目录，不作为子笔记。画布目录可包含脚本、样式、图片及嵌套资源。

通过侧栏或命令面板选择笔记，使用标题栏复制引用；“编辑笔记”和“打开笔记所在目录”收纳在笔记菜单中，侧栏右键菜单与命令面板也提供这些操作。归档笔记可在设置中恢复。文件由外部工具修改后，应用重新读取变化；Git 状态用于提示正文、画布和附件的变化。

Markdown 正文由 GPUI Kit TextView 原生渲染，支持文本选择、全选和复制。图库、附件、图片与链接通过扩展接入；代码块完整显示，不提供独立复制按钮或展开收起。

普通笔记页面在正文头部展示概要和自定义 YAML 数据。YAML 字段使用无边框、无背景的 DescriptionList，标签列按实际显示字体自动计算宽度并对齐，key 保持单行，属性行采用紧凑间距，保留原有容器与文字样式；对象逐层展开为子属性，数组按原顺序逐项展示（序号从 1 开始），空对象、空数组和 null 分别保留为 `{}`、`[]`、`null`，不把嵌套内容转成 JSON 文本。画布笔记使用同样的字段展示。

在笔记 frontmatter 中配置 `shortcuts`，标题栏右侧和命令面板会提供对应的快捷入口：

```yaml
title: 示例项目
shortcuts:
  website: https://example.com
  github: https://github.com/example/project
  directory: ~/Projects/example
```

网站和 GitHub 使用默认浏览器打开，只接受 HTTP(S) 地址。目录复用设置中的笔记编辑器：选择 VS Code 时在 VS Code 中打开，选择系统默认时由系统打开目录。支持绝对路径、`~/` 和相对路径；相对路径始终基于定义该配置的笔记目录，`~/` 指运行应用的本机用户目录。目录无需位于笔记库内，但必须是存在且可访问的目录；配置不是 Shell 命令。

子笔记未定义 `shortcuts` 时，沿目录层级使用最近一个定义了它的祖先笔记，跳过没有 `index.md` 的组织目录。当前笔记一旦定义 `shortcuts`，就整组替换祖先配置，不逐字段合并；`shortcuts: null` 阻断当前笔记与未自行配置的后代继承，`shortcuts: {}` 定义空组。组内字段省略或设为 `null` 时不显示对应入口。其他 YAML 属性不继承。

继承入口的提示会显示来源笔记；继承值不会写入子笔记。父笔记保存后，文件观察刷新会更新后代入口。无效配置保持当前继承边界，不回退到祖先；有效字段仍可用，标题栏显示配置错误提示。移动笔记后，继承来源和相对路径按新位置重新计算，移动前应检查这些变化。

笔记存在 `canvas/index.html` 或 `canvas/index.json` 时，标题栏和头部元数据以下显示画布。Locus 内置 Tailwind，通过原生 WebView 直接运行 HTML 和 JavaScript，无须 Node.js、npm 或开发服务器。保存后在画布内按 Command/Ctrl+R 或 F5 刷新，离开笔记会释放网页及临时状态。画布笔记的 `index.md` 只保留 frontmatter；没有画布时显示正文。文件格式与资源边界见[画布编写规范](resources/docs/canvas.md)。程序不自动转换或删除原有组件包。

画布标题栏提供“下载画布”按钮，在“设置 → 通用 → 导出目录”选择保存位置，默认使用系统下载目录。导出没有外部文件时生成单个 HTML；需要本地资源时生成包含 `canvas.html` 与引用资源的文件夹。同名自动编号，不覆盖已有导出。导出读取已保存内容，动态资源需先触发加载；在线数据仍需联网，本地模块或 Worker 可能需要本地 HTTP 服务。

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

在项目目录运行：

```sh
cargo run
```

## 构建应用

应用版本采用 `0.<年份>.<当年发布序号>`，序号从 1 开始，同年递增，跨年重置。版本值统一维护在根目录 `Cargo.toml` 的 `workspace.package.version`，打包产物读取该版本；修改时同步 `Cargo.lock` 中继承该版本的工作区包。

安装构建工具后，在项目目录执行：

```sh
cargo install --path crates/oneo-tool-packer --locked
oneo-tool-packer
```

构建配置位于 [packer.toml](packer.toml)。默认构建当前平台的 Release 版本，产物保存到 `.build/<target>/`：macOS 为 `.app`，Windows 为包含可执行文件与运行依赖的目录。

编译目录遵循 Cargo 的全局或项目 `build.target-dir` 配置及 `CARGO_TARGET_DIR` 环境变量。打包工具自动从 Cargo 构建消息读取实际可执行文件路径，支持多个项目共用编译目录，无须在 `packer.toml` 中配置路径；最终应用包仍保存到项目 `.build/<target>/`。配置解析与优先级遵循 [Cargo 配置规则](https://doc.rust-lang.org/cargo/reference/config.html#buildtarget-dir)。

macOS 默认使用 ad-hoc 签名。构建 Windows 版本需要在 Windows 环境中运行。
