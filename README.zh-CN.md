# Locus

[English](README.md) | **简体中文**

Locus 是本地优先的 Markdown 知识工作台，使用 Rust 和 GPUI 构建原生桌面界面，服务于长期维护个人知识的开发者、创作者和一人团队。用户可以阅读与组织笔记、查看文件变化、打开笔记画布和外部终端，并通过 MCP 让 Agent 发现和维护同一套本地文件。

Markdown 文档树始终是知识内容的唯一事实来源，用户可以直接访问、编辑和版本管理。外部 Agent 通过 MCP 接口定位文件，按 Locus skill 使用自身文件工具编辑；模型调用、账户、权限、远程数据策略与计费由对应客户端负责。

## 快速开始

启动应用后，在设置中选择本地 Markdown 文档库。每篇笔记由目录中的 `index.md` 表达；应用会读取外部编辑器与 Agent 对真实文件的修改。

从源码运行需要 Rust 和平台构建工具，在项目根目录执行：

```sh
cd source
cargo run
```

macOS 需要 Xcode 与 Command Line Tools，最低系统版本为 macOS 13；Windows 需要 Visual Studio C++ 工具链与 Windows SDK。

## 笔记与画布

文档库是用户选择的本地根目录。目录层级组织笔记，每篇笔记的标题、图标、概要和归档状态来自 `index.md` 的 YAML frontmatter。`assets/` 存放附件，`assets/icon.png` 为笔记图标；`assets` 和 `canvas` 是保留目录，不作为子笔记。

通过侧栏或命令面板选择笔记，标题栏工具栏依次提供复制引用、编辑笔记、打开所在目录和更多菜单；更多菜单提供图标上传和归档，侧栏右键菜单保留这四项操作并绑定对应笔记。归档项在设置中恢复。外部文件变更会更新应用的派生视图，Git 状态提示正文、画布和附件的变化。

Markdown 支持连续文本选择、全选和复制。概要和自定义 YAML 在正文头部展示，对象与数组递归展开并保留顺序、空集合与 null。

有 `canvas/index.html` 与 `canvas/main.js` 的笔记运行 HTML/JavaScript 画布，模版调用方使用互斥的 `canvas/index.json`。原生画布当前支持 macOS，标题栏和头部以下显示网页，内置 Tailwind，通过系统 WebView 读取文件。画布笔记的 `index.md` 只保留 frontmatter；普通笔记显示 Markdown 正文。

保存画布后手动刷新或重新进入笔记生效，离开时释放网页临时状态。“下载画布”导出已保存的入口、数据和引用资源；默认保存到系统下载目录，可在通用设置中修改。结果为单个 HTML 或带 `canvas.html` 和资源的目录，同名自动编号；动态资源需先加载，在线数据仍需网络，模块或 Worker 可能需要本地 HTTP 服务。编写规则见随软件发布的 skill [画布参考](source/resources/skills/locus/references/canvas.md)与[附件参考](source/resources/skills/locus/references/attachments.md)。

## 外部 Agent

在终端设置中选择系统终端或 Ghostty，并选择 Codex、pi、OpenCode 或 Claude Code，Unix 平台还可通过 Shell 选项选择默认、zsh、bash 或 fish（3.0 或以上），需要先安装所选 Shell；默认系统终端、Codex 与原有平台 Shell。需要先安装所选软件。Locus 在当前文档库根目录运行 `codex`、`pi`、`opencode` 或 `claude`；切换文档库、修改设置或退出 Locus 不结束已有外部会话。

macOS 系统终端使用 Terminal，每次创建新窗口；Ghostty 需要 1.3.0 或以上，优先在最近使用的窗口中新建标签页，没有窗口时创建窗口。默认使用 zsh；选择 bash 或 fish 后加载对应 Shell 的登录与交互配置，Agent 退出或找不到命令时保留所选 Shell。Windows 使用系统控制台。

| 操作 | macOS 快捷键 |
| --- | --- |
| 命令面板 | Command+E |
| 设置 | Command+, |
| 打开 Agent | Command 与反引号 |
| 刷新当前笔记或聚焦的画布 | Command+R 或 F5 |
| 关闭窗口 | Command+W |

其他平台使用对应的 Control 组合键。MCP 在设置中启用，默认关闭、端口为 `2301`；只提供 `/mcp` 只读发现。使用方法放在随应用发布的 skills 中，可在 MCP 设置中安装到笔记库或全局 `.agents/skills`。局域网模式没有身份验证，客户端可以搜索笔记内容并获取本机路径。

源码位于 `source/`，最终应用产物位于 `dist/`。

采用 [MIT 开源协议](LICENSE)。
