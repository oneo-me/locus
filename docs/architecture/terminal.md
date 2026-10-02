# 文档库终端

本篇维护当前行为、实现边界与验收要求。需求编号见[需求索引](../requirements.md)。

## 目标

用户从当前知识工作台快捷启动已安装的 Agent，在同一笔记库根目录工作。Locus 只选择终端、Agent 与启动目录；终端外观、会话、认证、模型、权限和进程生命周期归外部软件管理。

## 产品行为

- 设置中的终端选择提供系统终端和 Ghostty；Windows 只提供系统终端。默认使用系统终端。
- Agent 选择提供 Codex、pi、OpenCode、Claude Code，默认 Codex；启动命令分别为 `codex`、`pi`、`opencode`、`claude`，不添加未指定的初始化命令、参数或提示词。
- 沿用 Command 与反引号组合键；在其他平台使用对应 Control 组合键。命令面板提供同一“打开 Agent…”操作。每次按键触发一次，长按不重复创建窗口，启动交接期间拒绝重复提交；画布焦点下也可触发。
- 启动目录始终是触发时当前笔记库根目录，不是当前笔记目录。未选择笔记库时禁用命令；目录失效时报告错误。打开外部终端不切换 Locus 页面或展开任何侧栏。
- macOS 系统终端使用 Terminal；Ghostty 使用 1.3.0 或以上版本的 AppleScript 接口。每次操作只请求一个 Agent 会话，复用已运行的应用；Terminal 创建新窗口，Ghostty 优先在最近使用的已有窗口中新建标签页，没有窗口时才创建新窗口。不向已有会话输入命令；zsh 登录与交互配置用于解析已安装的 Agent。Agent 退出或找不到命令时保留 Shell，便于查看错误。
- 用户先安装终端和 Agent。终端启动失败通过应用通知显示；Agent 命令、认证及运行错误由外部终端呈现，Locus 不将已交给终端的请求表述为 Agent 已运行成功。
- 已打开的外部会话不由 Locus 保存、关闭或重启。切换笔记库、修改选择、退出 Locus 均不影响已存在的外部会话；后续启动使用新选择。
- 偏好只保存外部终端与 Agent 选择；旧内嵌终端字段不参与当前配置。读取旧 TOML 时保留其他有效偏好，正常保存时移除类型未包含的字段。

## 实现边界与验收

`locus-preferences` 定义终端和 Agent 选项，设置实体拥有唯一运行时选择。`locus-workspace/src/workspace/terminal.rs` 负责动作、后台启动交接、重复提交保护和失败通知；`locus-ui/src/platform/terminal.rs` 负责平台命令及路径转义。Locus 使用外部终端；`oneo-gpui-terminal` 作为通用组件独立保留。

macOS 两种终端先发送 AppleScript `launch`，再创建会话，最后 `activate`，避免常规启动先打开默认窗口。Terminal 使用 `do script`；Ghostty 使用 `new surface configuration` 设置 `initial working directory` 和 `command`，有窗口时调用 `new tab in front window with configuration`，无窗口时调用 `new window with configuration`。目录与命令经独立参数传递和 Shell 引号保护。依据 [Apple launch 定义](https://developer.apple.com/library/archive/documentation/AppleScript/Conceptual/AppleScriptLangGuide/reference/ASLR_cmds.html)和 [Ghostty AppleScript 文档](https://ghostty.org/docs/features/applescript)。Ghostty 的正常会话恢复仍由用户自己的配置控制；Locus 不修改终端配置或关闭已有窗口。Windows 使用新系统控制台运行 Agent。实际目标平台启动、系统授权、Shell 配置和 Agent 安装情况由用户验证，不自动打开终端或 Agent 进行验收。

验收覆盖：终端与 Agent 选择持久化；旧设置清理且其他偏好保留；无库与失效库；中文、空格、引号、换行与 Shell 特殊字符路径；实际启动目录和命令；终端未运行、已运行及已运行但没有窗口时均只新增一个 Agent 会话；Ghostty 已有窗口时新增标签页且窗口数量不增加，多个窗口时使用最近使用的窗口，无窗口时只新建一个 Agent 窗口，原有会话不受影响；长按只触发一次；画布焦点快捷键；终端缺失、Agent 缺失和退出；设置及笔记库切换只影响后续会话。

自动化入口为 `cargo test -p locus-ui --locked platform::terminal::tests`，覆盖目录有效性、命令参数及路径转义。AppleScript 编译检查不执行终端，也不能证明会话复用、系统授权或 Agent 启动成功。

## 独立终端组件

`oneo-gpui-terminal` 是独立 GPUI 通用组件，当前 Locus 产品使用上文的外部启动流程。它复用 `alacritty_terminal` 0.26 的 PTY 与 VT 状态机，关闭默认 features，由 GPUI 视图映射网格、输入、选择、焦点和刷新；输入接受中性配置，不读取 Locus 偏好或知识内容。

宽字符占据两个单元格，后继 `WIDE_CHAR_SPACER` 不生成额外文本。输入法通过 GPUI 原生接口处理，组合态只显示在光标处，提交态才写入 PTY，控制组合键按终端协议编码。相关组件改动独立运行该包测试，不能用外部 Agent 启动检查代替。
