![Locus 图标](source/resources/icon.png)

# Locus

[English](README.md) | **简体中文**

Locus 是面向 macOS、Windows 和 Linux 的本地优先 Markdown 笔记本。笔记始终是普通文件，因此你可以在 Locus 中阅读和整理，同时用任何喜欢的应用编辑它们。

## 功能

- **纯文件、不锁定**：每篇笔记是包含 `index.md` 的文件夹，文件夹嵌套形成笔记树，附件与笔记放在一起。
- **内置阅读**：浏览目录树，或搜索笔记与操作，阅读 Markdown、本地图片、概要及自定义 YAML 属性。
- **画布**：笔记可以在工作区内显示自有的 HTML 与 JavaScript 页面，例如仪表盘或图表。
- **对 Agent 友好**：可选的只读 MCP 让外部 Agent 找到同一批笔记，内置终端在你需要时打开 Shell 或 Agent CLI。
- **开箱舒适**：亮色与暗色主题、英文或中文界面、前进后退历史，并记住导航位置与侧栏状态。

## 安装

从 [GitHub Releases](https://github.com/oneo-me/locus/releases/latest) 下载最新版本。

- **macOS**：将 `Locus.app` 移到“应用程序”。需要 macOS 13 或更高版本，支持 Apple Silicon 和 Intel。
- **Windows**：解压 zip 并运行 `Locus.exe`。需要 Windows 10 或更高版本以及 WebView2 运行时。
- **Linux**：解压 tar.gz 并运行 `locus`。需要 GTK 3 和 WebKitGTK。

已发布的下载可能落后于开发分支。Windows 和 Linux 归档由同一源码交叉构建，仍需在目标平台上完成验收测试。

## 快速上手

1. 启动 Locus，选择**打开文档库**来指定存放笔记的文件夹。Command/Ctrl+O 可以再次打开选择器。
2. 在 Locus 之外添加笔记：在 Finder、编辑器或 Agent 中创建包含 `index.md` 的文件夹。Locus 会自动跟随文件变化。
3. 用 Command/Ctrl+F 搜索笔记与命令。笔记工具栏可以复制引用、在编辑器中打开笔记或打开所在文件夹。
4. 用 Command/Ctrl+, 打开**设置**，选择主题、语言、编辑器、导出目录、shell 与 Agent，以及技能。
5. 若要让 Agent 读取笔记，在设置中启用 MCP，将客户端连接到 `http://127.0.0.1:2301/mcp`。参见 [MCP 接入](docs/mcp.md)。

## 开发

在 `source/` 中使用 Go 1.27.1 或更高版本构建与测试：

```sh
cd source
GOWORK=off go test ./...
GOWORK=off go vet ./...
GOWORK=off go tool packer run
GOWORK=off go tool packer build
```

依赖、检查与打包参见[运行与构建](docs/build.md)，职责边界参见[架构说明](ARCHITECTURE.md)，协作规则参见 [AGENTS.md](AGENTS.md)。用户可见变更记录在 [CHANGELOG.md](CHANGELOG.md)。

## 当前限制

- Windows 和 Linux 尚未验证；macOS 原生的 Markdown 与画布流程已有自动冒烟测试，Windows 和 Linux 归档为交叉构建，仍需在目标平台上验收。
- 尚未实现自动更新，请手动安装新版本。
- 侧栏排序、专用附件列表、字体控件及完整窗口位置尺寸恢复尚未实现。

采用 [MIT 开源协议](LICENSE)。
