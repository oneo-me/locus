# Locus

Locus 是本地优先的 Markdown 知识工作台，使用 Rust 和 GPUI 构建原生桌面界面。用户可以阅读与组织笔记、查看文件变化、打开笔记画布和文档库终端，并通过 MCP 让外部 Agent 发现同一套本地文件。

Markdown 文档树始终是知识内容的唯一事实来源。外部 Agent 通过自己的文件工具编辑内容，其执行、权限、认证和模型配置由对应客户端负责。

## 使用

启动后在设置中选择本地文档库。每篇笔记由目录中的 `index.md` 表达，目录可以嵌套；标题、图标、概要和归档状态使用 YAML frontmatter。同目录普通文件作为附件，`icon.png` 和 `canvas.html` 分别用于图标与画布。

通过侧栏或命令面板选择笔记，使用标题栏操作复制引用、打开外部编辑器或显示所属目录。归档笔记可在设置中恢复。文件由外部工具修改后，应用重新读取变化；Git 状态用于提示正文、画布和附件的变化。

笔记存在 `canvas.html` 时，可以在右栏打开画布。画布内容使用 HTML 片段，基本页面所需的 Tailwind 运行时随应用提供。离开笔记会关闭所属画布；保存文件后可刷新重新加载。自动打开画布的设置默认关闭。

终端按需在当前文档库根目录创建，多个会话各自占用右栏标签。以下为 macOS 快捷键：

| 操作 | 快捷键 |
| --- | --- |
| 命令面板 | Command+E |
| 设置 | Command+, |
| 新建终端 | Command 与反引号组合键 |
| 刷新当前笔记或获得焦点的画布 | Command+R 或 F5 |
| 关闭当前右栏标签 | 右栏获得焦点时 Command+W |

在设置中启用 MCP 后，可复制连接地址或客户端配置。服务默认关闭，默认端口为 `2301`，提供 `list_articles`、`search_articles` 和 `resolve_article` 三个发现工具，返回笔记元数据和原始文件路径。局域网模式没有身份验证，启用前应确认同一网络客户端可检索的内容与路径范围。

## 实现

Locus 使用 Rust 和 GPUI 构建原生桌面应用。应用初始窗口尺寸为 980 × 820，启动时恢复已保存的可用窗口位置与尺寸。

直接运行应用：

```bash
cargo run
```

应用入口位于 [crates/locus](crates/locus)。各包按职责组织：

| 位置 | 职责 |
| --- | --- |
| [locus-workspace](crates/locus-workspace) | 导航、笔记、命令和右栏标签编排 |
| [locus-library](crates/locus-library) | 文件模型、读取、修改、观察与 Git 投影 |
| [locus-settings](crates/locus-settings)、[locus-preferences](crates/locus-preferences) | 设置界面、偏好及配置 |
| [locus-canvas](crates/locus-canvas) | WebView 画布、资源协议与生命周期 |
| [locus-mcp](crates/locus-mcp) | MCP 工具与监听服务 |
| [locus-ui](crates/locus-ui) | 笔记图标、Markdown 扩展与附件呈现 |
| [oneo-gpui](crates/oneo-gpui) | 共享应用壳、窗口、面板与配置服务 |
| [oneo-gpui-markdown](crates/oneo-gpui-markdown)、[oneo-gpui-terminal](crates/oneo-gpui-terminal) | Markdown 渲染与 PTY 终端 |
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
NODE_PATH=/tmp/locus-canvas-tests/node_modules node --test crates/locus-canvas/tests/templates.cjs
```

自动化测试不代替各平台的窗口、WebView、终端输入和实际视觉体验验证。

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

工具只构建 Release；省略 `--target` 时使用当前 Rust host。macOS 在 Mac 上生成 `.app` 目录，Windows 在 Visual Studio C++ 开发环境中生成包含 `.exe` 和运行库的目录，不生成安装程序。产物固定保存到项目根 `.build/<target>/`，`.build/` 已由 Git 忽略；平台与架构必须具备对应工具链。

项目配置见 [packer.toml](packer.toml)，必填 `package`、`bin`、`name` 和 `identifier`。可选 `[windows]` 下的 `resources` 声明附加资源，`runtime-directories` 指定 DLL 搜索目录，`runtime-files` 指定运行时动态加载的非系统 DLL；配置未知字段会报错。完整字段见 [config.rs](crates/oneo-tool-packer/src/config.rs)。

工具支持 `aarch64-apple-darwin`、`x86_64-apple-darwin`、`x86_64-pc-windows-msvc` 和 `aarch64-pc-windows-msvc`；应用依赖还需支持相应目标。`--project` 指定项目目录，`--dry-run` 展示构建命令与目的地，不执行编译或签名。

macOS 图标固定为根目录 `icon.icon/`，Info.plist 由工具生成，最低系统版本为 13.0。macOS 构建需要 Xcode 与 Command Line Tools，默认 ad-hoc 签名；Developer ID 签名使用 `--codesign-identity` 或 `ONEO_PACKER_CODESIGN_IDENTITY`，工具不执行公证。Release 主程序的禁用符号导出由工具统一处理；普通 `cargo build --release` 只应用项目 Cargo profile，不自动禁用导出。

Windows 图标固定为根目录 `icon.ico` 并嵌入 EXE。构建环境需要 Visual Studio C++ 工具链、Windows SDK 的 `rc` 和 `dumpbin`。工具递归检查 EXE/DLL 的依赖和架构，从产物目录、配置目录及 VC 可分发目录收集运行库；系统库不复制，缺失依赖会中止打包。运行时动态加载的 DLL 需要显式声明。实现见 [windows.rs](crates/oneo-tool-packer/src/windows.rs)。

工具在临时目录完整组装并检查产物后替换输出目录，构建、签名或资源检查失败时保留旧产物。真实 macOS 签名与 Windows 运行库完整性、启动行为仍需在目标平台验证。
