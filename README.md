# Locus

Locus 是本地优先的 Markdown 知识工作台，使用 Rust 和 GPUI 构建原生桌面界面。用户可以阅读与组织笔记、查看文件变化、打开笔记画布和文档库终端，并通过 MCP 让外部 Agent 发现和维护同一套本地文件。

Markdown 文档树始终是知识内容的唯一事实来源。外部 Agent 可通过标准 MCP 接口定位文件后自行编辑，也可通过完整 MCP 接口直接操作笔记库；其执行和模型配置由对应客户端负责。

## 使用

启动后在设置中选择本地文档库。每篇笔记由目录中的 `index.md` 表达，目录可以嵌套；标题、图标、概要和归档状态使用 YAML frontmatter。`assets/` 存放附件，`assets/icon.png` 是专用图标，`canvas/index.html` 与 `canvas/main.js` 定义普通画布，`main.js` 导出 `async main(data)`；模板调用方改用互斥的 `canvas/index.json`，其中只有 `template` 和 `data` 两个字段；`assets` 和 `canvas` 为保留目录，不作为子笔记。画布目录可包含脚本、样式、图片及嵌套资源。

通过侧栏或命令面板选择笔记，使用标题栏操作复制引用、打开外部编辑器或显示所属目录。归档笔记可在设置中恢复。文件由外部工具修改后，应用重新读取变化；Git 状态用于提示正文、画布和附件的变化。

普通笔记页面在正文头部展示概要和自定义 YAML 数据。

笔记存在 `canvas/index.html` 或 `canvas/index.json` 时，标题栏和头部概要、YAML 数据以下只显示画布，不再混合显示 Markdown 正文，也不生成笔记装饰背景；标题栏和信息区使用白底风格，信息区按内容确定高度，过长时独立滚动，画布占满剩余空间。没有画布时显示 Markdown 正文。画布笔记的 `index.md` 只保留 frontmatter，不写正文；已有混合内容暂不自动整理。画布使用 HTML 片段，基本页面所需的 Tailwind 运行时随应用提供。离开笔记会释放所属画布；保存文件后，在画布内使用 Command/Ctrl+R 或 F5 重新加载。

画布笔记的标题栏操作区第一位提供独立的“下载画布”图标按钮，位于复制按钮之前。可在“设置 → 通用 → 导出目录”选择保存位置，默认使用系统下载目录；点击下载后，没有引用附件时直接生成“笔记标题.html”；有引用附件时生成以笔记标题命名的文件夹，包含完整的 `canvas.html` 和引用附件。文件和文件夹同名时自动编号，打开 HTML 即可查看。模版、画布数据与 Tailwind 内置在页面中，附件保留原文件名，不复制无关附件；在线地图和数据仍需联网。导出读取已保存源码，不保存网页临时交互状态；动态拼接的依赖需先触发加载。本地模块脚本或 Worker 可能需要通过本地 HTTP 服务打开，以满足浏览器的来源限制。

在“设置 → 终端”选择系统终端或 Ghostty，并选择 Codex、pi、OpenCode 或 Claude Code。默认为系统终端与 Codex。打开 Agent 时，应用调用外部终端，在当前笔记库根目录运行对应的 `codex`、`pi`、`opencode` 或 `claude` 命令；需要先安装所选终端和 Agent。外部会话由终端软件管理，切换笔记库或退出 Locus 不结束会话，设置变化只影响后续启动。

macOS 系统终端使用 Terminal，Ghostty 使用其 [AppleScript 接口](https://ghostty.org/docs/features/applescript)（需要 1.3.0 或以上）；两者均先后台启动，再创建 Agent 会话并激活，复用已运行的应用。Terminal 创建新窗口；Ghostty 优先在最近使用的已有窗口中新建标签页，没有窗口时才创建新窗口。启动时加载 zsh 登录与交互配置，Agent 退出或找不到命令时保留 Shell，便于查看错误。Windows 使用系统控制台，不提供 Ghostty 选项。共享 shell 只提供左侧栏与主内容区。旧内嵌终端与自动打开画布设置不再使用，下一次保存设置时移除旧字段，其他偏好保留。以下为 macOS 快捷键：

| 操作 | 快捷键 |
| --- | --- |
| 命令面板 | Command+E |
| 设置 | Command+, |
| 打开 Agent | Command 与反引号组合键 |
| 刷新当前笔记或获得焦点的画布 | Command+R 或 F5 |
| 关闭窗口 | Command+W |

设置中可启停 MCP、配置局域网访问和端口，并查看运行状态；侧边栏左下角也保留只读状态。应用不提供连接引导、地址展示或复制功能。标准接口 `/mcp` 与完整接口 `/mcp/full` 同时提供，共用服务、端口与局域网设置；服务默认关闭，默认端口为 `2301`，客户端连接方式见下方使用说明。标准接口保留三个发现工具及原始文件路径；完整接口通过逻辑地址读取、编辑、上传、下载、移动和删除笔记、附件、图标及画布，不返回服务器文件路径。局域网模式没有身份验证，网络客户端可以读写和永久删除库内内容。工具、传输和冲突规则见 [MCP 使用说明](crates/locus-mcp/README.md)。

## 实现

Locus 使用 Rust 和 GPUI 构建原生桌面应用。应用初始窗口尺寸为 980 × 820，启动时恢复已保存的可用窗口位置与尺寸。

直接运行应用：

```bash
cargo run
```

应用入口位于 [crates/locus](crates/locus)。各包按职责组织：

| 位置 | 职责 |
| --- | --- |
| [locus-workspace](crates/locus-workspace) | 导航、笔记画布、命令和外部 Agent 启动 |
| [locus-library](crates/locus-library) | 文件模型、读取、修改、观察与 Git 投影 |
| [locus-settings](crates/locus-settings)、[locus-preferences](crates/locus-preferences) | 设置界面、偏好及配置 |
| [locus-canvas](crates/locus-canvas) | WebView 画布、资源协议与生命周期 |
| [locus-mcp](crates/locus-mcp) | MCP 工具与监听服务 |
| [locus-ui](crates/locus-ui) | 笔记图标、Markdown 扩展与附件呈现 |
| [oneo-gpui](crates/oneo-gpui) | 共享应用壳、窗口、面板与配置服务 |
| [oneo-gpui-markdown](crates/oneo-gpui-markdown) | Markdown 渲染 |
| [oneo-gpui-terminal](crates/oneo-gpui-terminal) | 独立 PTY 组件，未接入 Locus |
| [oneo-tool-packer](crates/oneo-tool-packer) | 构建与应用目录打包 |

检查和测试整个 workspace：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

默认 `cargo test` 只覆盖应用入口包；可使用 `cargo test -p locus-library -p locus-mcp` 单独验证文档库与 MCP。画布模板的 DOM 契约测试位于 [templates.cjs](crates/locus-canvas/tests/templates.cjs)，使用 Node.js 与 jsdom：

```sh
npm install --prefix /tmp/locus-canvas-tests --no-save jsdom@30.0.1
NODE_PATH=/tmp/locus-canvas-tests/node_modules node --test crates/locus-canvas/tests/templates.cjs crates/locus-canvas/tests/export.cjs
```

自动化测试不代替各平台的窗口、WebView、外部终端启动和实际视觉体验验证。

## 打包

先安装 workspace 中的公共工具：

```sh
cargo install --path crates/oneo-tool-packer --locked
```

确保 Cargo 的 `bin` 目录位于 PATH 中。VSCode 的 `build` 任务与以下命令使用同一个打包入口：

```sh
oneo-tool-packer
oneo-tool-packer --target aarch64-apple-darwin
oneo-tool-packer --target x86_64-pc-windows-msvc
```

工具只构建 Release；省略 `--target` 时使用当前 Rust host。macOS 在 Mac 上生成 `.app` 目录，Windows 自动加载 Visual Studio C++ 环境并默认静态链接 CRT，生成包含 `.exe` 和仍需动态加载的运行库的目录，不生成安装程序。静态 CRT 通过目标级 Cargo 配置应用于完整依赖树；显式设置的 `RUSTFLAGS`、`CARGO_ENCODED_RUSTFLAGS` 或目标级 rustflags 环境变量遵循 Cargo 的优先级规则，可能覆盖该默认值。工具仍扫描并收集实际 DLL 依赖，不假定所有原生库都能静态链接。产物固定保存到项目根 `.build/<target>/`，`.build/` 已由 Git 忽略；平台与架构必须具备对应工具链。

项目配置见 [packer.toml](packer.toml)，必填 `package`、`bin`、`name` 和 `identifier`。顶层 `resources` 声明共用资源，`[macos].resources` 和 `[windows].resources` 仅复制对应平台的附加资源；`[windows]` 下的 `runtime-directories` 指定 DLL 搜索目录，`runtime-files` 指定运行时动态加载的非系统 DLL；资源条目可以是源路径字符串，或 `{ source = "tools/macos", destination = "tools" }`，后者指定输出名称，名称不能包含目录层级；配置未知字段会报错。完整字段见 [config.rs](crates/oneo-tool-packer/src/config.rs)。

工具支持 `aarch64-apple-darwin`、`x86_64-apple-darwin`、`x86_64-pc-windows-msvc` 和 `aarch64-pc-windows-msvc`；应用依赖还需支持相应目标。`--project` 指定项目目录，`--dry-run` 展示构建命令与目的地，不执行编译或签名。

macOS 图标固定为根目录 `icon.icon/`，Info.plist 由工具生成，最低系统版本为 13.0。macOS 构建需要 Xcode 与 Command Line Tools，默认 ad-hoc 签名；Developer ID 签名使用 `--codesign-identity` 或 `ONEO_PACKER_CODESIGN_IDENTITY`，工具不执行公证。Release 主程序的禁用符号导出由工具统一处理；普通 `cargo build --release` 只应用项目 Cargo profile，不自动禁用导出。

Windows 图标固定为根目录 `icon.ico` 并嵌入 EXE。构建环境需要 Visual Studio C++ 工具链、Windows SDK 的 `rc` 和 `dumpbin`。可以直接从普通 PowerShell、CMD 或 VS Code 任务运行；当当前环境缺少 `dumpbin` 或 `rc` 时，工具会通过 `vswhere` 查找安装了 C++ 工具的 Visual Studio，按目标架构加载 `VsDevCmd.bat`，并在该环境中继续打包。环境设置仅作用于打包进程及其子进程，不修改系统环境变量。未安装工具链时，请通过 Visual Studio Installer 安装“使用 C++ 的桌面开发”和 Windows SDK。工具递归检查 EXE/DLL 的依赖和架构，从产物目录、配置目录及 VC 可分发目录收集运行库；系统库不复制，缺失依赖会中止打包。运行时动态加载的 DLL 需要显式声明。实现见 [windows.rs](crates/oneo-tool-packer/src/windows.rs)。

工具在临时目录完整组装并检查产物后替换输出目录，构建、签名或资源检查失败时保留旧产物。真实 macOS 签名与 Windows 运行库完整性、启动行为仍需在目标平台验证。
