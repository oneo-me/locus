![Locus 图标](source/assets/icon.png)

# Locus

[English](README.md) | **简体中文**

Locus 是面向 macOS、Windows 和 Linux 的本地优先 Markdown 笔记本。笔记始终是普通文件，因此你可以在 Locus 中阅读和整理，同时用任何喜欢的应用编辑它们。

## 功能

- **纯文件、不锁定**：每篇笔记是包含 `index.md` 的文件夹，文件夹嵌套形成笔记树，附件与笔记放在一起。
- **内置阅读**：浏览目录树，或搜索笔记与操作，阅读 Markdown、本地图片、概要及自定义 YAML 属性。
- **关系图谱**：在工作区上方打开全窗口图谱，同时呈现笔记层级与笔记之间的链接，并可选择将每篇笔记的附件文件显示为黄色节点。
- **画布**：笔记可以在工作区内显示自有的 HTML 与 JavaScript 页面，例如仪表盘或图表。
- **对 Agent 友好**：可选的只读 MCP 让外部 Agent 找到同一批笔记，内置终端在你需要时打开 Shell 或 Agent CLI。
- **开箱舒适**：亮色与暗色主题；英文、简体中文、繁体中文、日文与韩文界面；前进后退历史，并记住导航与侧栏状态。

## 安装

从 [GitHub Releases](https://github.com/oneo-me/locus/releases/latest) 下载最新版本。

- **macOS**：打开磁盘映像，将 `Locus.app` 移到“应用程序”。需要 macOS 13 或更高版本，支持 Apple Silicon 和 Intel。
- **Windows**：运行与架构对应的 `Locus Setup` 安装程序。需要 Windows 10 或更高版本以及 WebView2 运行时，支持 x64 和 ARM64。
- **Linux**：安装 `.deb`，或解压 tar.gz 后运行 `locus`。需要 GTK 3 和 WebKitGTK，支持 x64 和 ARM64。

安装包每天检查一次更新，有新版本时会提示你。“设置 > 关于”显示当前版本、更新说明、下载进度和“重新启动以更新”，并可以开关自动检查和静默安装；应用菜单和命令面板中的“检查更新…”会打开这里。开发构建和 `.deb` 安装包不会自行更新。

发布版本尚未签名或公证。首次启动时，macOS 上请在“系统设置 > 隐私与安全性”中点击“仍要打开”，Windows SmartScreen 中请选择“更多信息 > 仍要运行”。

## 使用方法

1. 启动 Locus，选择**打开文档库**来指定存放笔记的文件夹。请将它初始化为 Git 仓库，让每次修改都有历史记录。
2. 用 Command/Ctrl+, 打开**设置**，选择你喜欢的 Shell 与 Agent，并安装让 Agent 操作 Locus 的技能。
3. 在设置中启用 MCP，并为你的 Agent 配置 `http://127.0.0.1:2301/mcp`；也可以把这个地址发给 Agent，让它自己配置。参见 [MCP 接入](docs/mcp.md)。
4. 开始使用吧：让 Agent 或任意编辑器创建包含 `index.md` 的文件夹，Locus 会自动跟随文件变化。

Command/Ctrl+F 搜索笔记与命令，Command/Ctrl+O 切换文档库，侧栏的图谱按钮可以查看笔记之间的关联。

## 小教程

- 让 Agent 创建一篇写明文档库规范的笔记，然后在文档库的 `AGENTS.md` 中链接它，Agent 就会遵循这些规范。
- 画布中的 HTML 可以作为模板，被其他笔记传入自己的数据调用。正确安装技能后，Agent 就知道如何使用。
- 在 Markdown 中，`attachments` 代码块列出笔记的附件，`gallery` 代码块把图片显示为画廊缩略图。

## 未来计划

- 自动配置 Agent 技能与 MCP。
- 内置一些画布模板。
- 优化部分界面用法。
- 支持调整笔记顺序。

## 开发

在 `source/` 中使用 Go 1.27.1 或更高版本构建与测试：

```sh
cd source
GOWORK=off go test ./...
GOWORK=off go vet ./...
GOWORK=off go tool mygo dev
GOWORK=off go tool mygo build -platform darwin/arm64,darwin/amd64,windows/amd64,windows/arm64,linux/amd64,linux/arm64
```

依赖、检查与打包参见[运行与构建](docs/build.md)，职责边界参见[架构说明](ARCHITECTURE.md)，协作规则参见 [AGENTS.md](AGENTS.md)。用户可见变更记录在 [CHANGELOG.md](CHANGELOG.md)。

采用 [MIT 开源协议](LICENSE)。
