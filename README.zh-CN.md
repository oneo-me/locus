<p align="center">
  <img src="source/assets/icon.png" alt="Locus" width="128">
</p>

<h1 align="center">Locus</h1>

<p align="center">
  <strong>简体中文</strong> |
  <a href="README.md">English</a>
</p>

<p align="center">
  <a href="https://github.com/oneo-me"><img alt="GitHub" src="https://img.shields.io/badge/GitHub-oneo--me-181717?logo=github&logoColor=white"></a>
  <a href="https://oneo.me"><img alt="博客" src="https://img.shields.io/badge/%E5%8D%9A%E5%AE%A2-oneo.me-0969da"></a>
  <a href="https://x.com/oneo666"><img alt="X" src="https://img.shields.io/badge/X-%40oneo666-000000?logo=x&logoColor=white"></a>
  <a href="https://github.com/sponsors/oneo-me"><img alt="GitHub 赞助" src="https://img.shields.io/badge/GitHub%20%E8%B5%9E%E5%8A%A9-ea4aaa?logo=githubsponsors&logoColor=white"></a>
</p>

<p align="center">
  <a href="https://awesomemygo.dev/apps/locus"><img alt="Listed on Awesome MyGo" src="https://awesomemygo.dev/badge.svg"></a>
</p>

<p align="center">
  Locus 是面向 macOS、Windows 和 Linux 的本地优先 Markdown 笔记本。笔记始终是你所选文件夹中的普通文件，因此你可以在 Locus 中阅读和整理，同时用任何喜欢的应用编辑它们。
</p>

## 功能

- 每篇笔记是包含 `index.md` 的文件夹；文件夹嵌套形成笔记树，附件与笔记放在一起。
- 在一个窗口中阅读 Markdown、本地图片、概要与自定义 YAML 属性。
- 在全窗口关系图谱中呈现工作区的笔记层级与链接。
- 让笔记拥有自己的 HTML 与 JavaScript 页面，用于仪表盘、图表等画布。
- 通过可选的只读 MCP 让外部 Agent 找到笔记，并在内置终端中打开 Shell 或 Agent CLI。
- 使用亮色与暗色主题、五种语言、前进后退历史，并记住侧栏状态。

## 安装

从 [GitHub Releases](https://github.com/oneo-me/locus/releases/latest) 下载最新版本。

- **macOS**：打开磁盘映像，将 `Locus.app` 移到“应用程序”。需要 macOS 13 或更高版本，支持 Apple Silicon 和 Intel。
- **Windows**：运行与架构对应的 `Locus Setup` 安装程序。需要 Windows 10 或更高版本以及 WebView2 运行时，支持 x64 和 ARM64。
- **Linux**：安装 `.deb`，或解压 tar.gz 后运行 `locus`。需要 GTK 3 和 WebKitGTK，支持 x64 和 ARM64。

安装包每天检查一次更新，并在“设置 > 关于”中显示，可在此安装并重新启动；开发构建和 `.deb` 安装包不会自行更新。

发布版本尚未签名或公证。首次启动时，macOS 上请在“系统设置 > 隐私与安全性”中点击“仍要打开”，Windows SmartScreen 中请选择“更多信息 > 仍要运行”。

## 快速开始

1. 启动 Locus，选择**打开文档库**来指定存放笔记的文件夹。请将它初始化为 Git 仓库，让每次修改都有历史记录。
2. 用 Command/Ctrl+, 打开**设置**，选择你喜欢的 Shell 与 Agent，并安装让 Agent 操作 Locus 的技能。
3. 在设置中启用 MCP，并为你的 Agent 配置 `http://127.0.0.1:2301/mcp`；也可以把这个地址发给 Agent，让它自己配置。参见 [MCP 接入](docs/mcp.md)。
4. 开始使用吧：让 Agent 或任意编辑器创建包含 `index.md` 的文件夹，Locus 会自动跟随文件变化。

Command/Ctrl+F 搜索笔记与命令，Command/Ctrl+O 切换文档库，侧栏的图谱按钮可以查看笔记之间的关联。

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

## 许可证

采用 [MIT](LICENSE) 协议 © 2026 ONEO。
