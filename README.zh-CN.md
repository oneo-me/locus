![Locus 图标](source/resources/icon.png)

# Locus

[English](README.md) | **简体中文**

Locus 是本地优先的 Markdown 工作台，使用 Go、Onevra 和 MyGo 构建原生桌面界面。笔记始终是普通文件：在 Locus 中阅读和组织，用喜欢的编辑器修改，通过只读 MCP 让外部 Agent 发现同一套文件。

## 快速开始

需要 Go 1.27.1 或以上及原生桌面会话。`source/go.mod` 已固定 Onevra 和 MyGo 的线上发布版本，无需本地依赖源码。参见[依赖设置](docs/build.md#dependencies)。

```sh
cd source
GOWORK=off go run ./cmd/locus
# 也可以在启动时指定文档库：
GOWORK=off go run ./cmd/locus -library /path/to/library
```

未选择文档库时，独立欢迎界面会提示你**打开文档库**；也可在设置对话框中选择文档库。每篇笔记是包含 `index.md` 的目录，目录嵌套表达层级。YAML frontmatter 提供标题、概要和自定义属性。附件放在 `assets/`，画布放在 `canvas/`；隐藏目录和保留的资源目录不作为笔记。

本地图片组支持旧版 `gallery` 围栏：使用笔记 `assets/` 目录中的 Markdown 图片，显示自动换行的缩略图，并可打开大图预览。参见[附件语法](source/resources/skills/locus/references/attachments.md#galleries)。

## 阅读与画布

通过原生目录树浏览，或在命令面板中搜索笔记。阅读 Markdown、本地图片、概要及自定义 YAML。工具栏提供复制引用、在编辑器中打开笔记及打开目录。外部文件变化会刷新派生视图；文档库为 Git 仓库根目录时，会显示文件修改提示。

笔记可以拥有 HTML 片段与 `canvas/main.js`，或 JSON 模板调用。MyGo 自带的 WebView 在工作区内显示页面；进入设置会隐藏并保留页面，离开笔记会释放它。修改画布源文件后，需刷新或重新打开。详见内置的[画布参考](source/resources/skills/locus/references/canvas.md)。

画布导出会生成 ZIP，包含页面及本次会话中已加载的资源。解压后，如模块、fetch 或 Worker 需要，请通过本地 HTTP 服务打开。未加载的依赖不会自动收集，在线服务仍然需要网络。

## Agent 与设置

设置提供外观与语言、编辑器与导出目录、MCP、shell／Agent 选择与技能安装。偏好设置保留已有 YAML 字段，并在保存前检测外部修改。侧栏和当前笔记状态在关闭时保存。

在设置中启用 MCP 后，本机只读发现地址默认为 `http://127.0.0.1:2301/mcp`。客户端需要能够访问返回的文件路径。详见 [MCP 接入](docs/mcp.md)。

可主动将内置 Locus 技能安装到文档库或用户主目录。安装会保留已有目标，并创建独立副本；更新副本需要明确重新安装。标题栏“更多操作”左侧的终端按钮打开 600 × 500 的 Popover，仅显示内置终端。在设置中选择 shell 与 Agent；选择“无”则只打开 shell，更改在下次打开终端时生效。收起弹层保留会话，切换文档库或退出应用会关闭会话。Agent 正常退出或以中断状态退出后会自动释放终端并关闭弹层；Ctrl+C 只有在使 Agent 退出时才会触发关闭。启动失败及其他异常会保留错误输出。会话启动或运行期间，标题栏终端按钮会持续脉动，收起弹层后仍能看到终端正在运行。Shell 和 Agent CLI 需自行安装，仅在用户请求时启动。

| 操作 | 快捷键 |
| --- | --- |
| 打开文档库 | Command/Ctrl+O |
| 搜索笔记与操作 | Command/Ctrl+F |
| 刷新当前内容 | Command/Ctrl+R 或 F5 |
| 设置 | Command/Ctrl+, |
| 展开或收起终端 | Command/Ctrl 与反引号 |
| 关闭窗口 / 退出 | Command/Ctrl+W / Command/Ctrl+Q |

## 开发与平台范围

```sh
cd source
GOWORK=off go test ./...
GOWORK=off go vet ./...
GOWORK=off go tool packer run
GOWORK=off go tool packer build
```

产物输出至 `dist/`，构建成功后自动打开当前版本的 release 目录。当前打包目标为 Apple Silicon macOS。不需要 Rust 工具链。竞态检查与原生冒烟测试详见[运行与构建](docs/build.md)；职责边界和验证范围见[架构说明](ARCHITECTURE.md)。

macOS 的原生 Markdown 与 WebView 流程已有自动冒烟测试。Windows 和 Linux 需要独立验收；MyGo 提供 macOS、Windows 和 Linux 原生 WebView 后端，但本分支仅在 macOS 上进行了验证。自动更新、侧栏排序、专用附件列表、字体控件及完整窗口位置尺寸恢复尚未实现。[GitHub Releases](https://github.com/oneo-me/locus/releases/latest) 中的发布下载可能与本开发分支不同；本地打包不会自动发布。

采用 [MIT 开源协议](LICENSE)。
