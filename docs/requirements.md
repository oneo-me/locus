# 需求索引

编号用于追踪产品要求；完整行为和验收边界由链接的专题维护。状态沿用已有记录，本次重组未重新验收全部功能；“已实现”不代表当前提交已通过目标平台实机验证。

LOC-011 与 LOC-023 保留“部分实现”：已有记录未明确剩余范围，需对照专题的失败处理与持久化逐项核查。

## 文档库与知识阅读

| 编号 | 要求摘要 | 既有状态 | 权威文档 |
| --- | --- | --- | --- |
| LOC-001 | 本地文件与可重建投影 | 已实现 | [专题](architecture/library.md) |
| LOC-002 | 笔记层级、保留目录与扫描边界 | 已实现 | [专题](architecture/library.md) |
| LOC-003 | frontmatter、自定义 YAML 与头部呈现 | 已实现 | [专题](architecture/workspace.md) |
| LOC-004 | 附件、图标与资源归属 | 已实现 | [专题](architecture/library.md) |
| LOC-005 | Tree 导航与独立同级排序 | 已实现 | [专题](architecture/workspace.md) |
| LOC-006 | Markdown 阅读、连续选择与复制 | 已实现 | [专题](architecture/workspace.md) |
| LOC-007 | 笔记引用、链接与外部打开 | 已实现 | [专题](architecture/workspace.md) |
| LOC-008 | 归档、后代隐藏与恢复 | 已实现 | [专题](architecture/library.md) |
| LOC-009 | 安全图标上传与派生缓存 | 已实现 | [专题](architecture/library.md) |
| LOC-010 | Git 变化归并与阅读降级 | 已实现 | [专题](architecture/library.md) |
| LOC-011 | 文件观察、刷新、失败与迟到任务隔离 | 部分实现 | [专题](architecture/library.md) |

## 画布

| 编号 | 要求摘要 | 既有状态 | 权威文档 |
| --- | --- | --- | --- |
| LOC-012 | 画布独占内容区与生命周期 | 已实现 | [专题](architecture/canvas.md) |
| LOC-014 | HTML/main 入口及手动刷新 | 已实现 | [专题](architecture/canvas.md) |
| LOC-015 | JSON 模版调用与安全资源读取 | 已实现 | [专题](architecture/canvas.md) |
| LOC-016 | 离线、主题、网络与笔记导航 | 已实现 | [专题](architecture/canvas.md) |
| LOC-040 | 画布下载、快照与不覆盖保存 | 已实现 | [专题](architecture/canvas.md) |
| LOC-017 | 画布快捷键与原生 WebView 合成 | 已实现 | [专题](architecture/canvas.md) |
| LOC-044 | 画布原生选择与编辑命令 | 已实现 | [专题](architecture/canvas.md) |

## 桌面工作区与设置

| 编号 | 要求摘要 | 既有状态 | 权威文档 |
| --- | --- | --- | --- |
| LOC-018 | 共享左栏与主内容布局 | 已实现 | [专题](architecture/workspace.md) |
| LOC-041 | Debug 侧栏 FPS 统计与切换 | 已实现 | [专题](architecture/shell.md) |
| LOC-020 | 选择、导航历史与启动恢复 | 已实现 | [专题](architecture/workspace.md) |
| LOC-021 | 统一命令面板与焦点恢复 | 已实现 | [专题](architecture/workspace.md) |
| LOC-022 | 绑定目标笔记的菜单与标题栏 | 已实现 | [专题](architecture/workspace.md) |
| LOC-023 | 内嵌设置、状态持久化与失败反馈 | 部分实现 | [专题](architecture/settings.md) |
| LOC-024 | 共享主题、语言与界面字号 | 已实现 | [专题](architecture/settings.md) |
| LOC-025 | 字体、导出目录与产品偏好 | 已实现 | [专题](architecture/settings.md) |
| LOC-026 | 窗口恢复、关闭与重新激活 | 已实现 | [专题](architecture/shell.md) |
| LOC-042 | 平台标题栏、窗口按钮与材质 | 已实现 | [专题](architecture/shell.md) |
| LOC-027 | 本地化、可访问性与减少动态效果 | 已实现 | [桌面工作区](architecture/workspace.md#界面与状态约定) |

## 文档库终端

| 编号 | 要求摘要 | 既有状态 | 权威文档 |
| --- | --- | --- | --- |
| LOC-028 | 文档库根目录启动外部 Agent | 已实现 | [专题](architecture/terminal.md) |

## Agent 接入

| 编号 | 要求摘要 | 既有状态 | 权威文档 |
| --- | --- | --- | --- |
| LOC-032 | 标准 MCP 发现与路径解析 | 已实现 | [专题](architecture/mcp.md) |
| LOC-033 | 真实文件与外部客户端协作 | 已实现 | [专题](architecture/mcp.md) |
| LOC-034 | MCP 监听、Host 校验与启用提示 | 已实现 | [专题](architecture/mcp.md) |
| LOC-035 | 唯一服务生命周期与真实状态 | 已实现 | [专题](architecture/mcp.md) |
| LOC-037 | 完整 MCP 逻辑对象发现与读写 | 已移除，改由客户端文件工具与 skills 协作 | [专题](architecture/mcp.md) |
| LOC-038 | 内容传输与子树导出重建 | MCP 已移除，文件由客户端维护 | [专题](architecture/mcp.md) |
| LOC-039 | 版本冲突、移动删除与旧会话隔离 | MCP 仅保留会话隔离，内容操作由客户端负责 | [专题](architecture/mcp.md) |
| LOC-043 | 随软件发布的附件与画布规范 | 已改为可安装的 skills | [专题](architecture/mcp.md) |

## 构建与交付

| 编号 | 要求摘要 | 既有状态 | 权威文档 |
| --- | --- | --- | --- |
| LOC-036 | 锁定依赖、平台打包与失败保护 | 已实现 | [专题](architecture/packaging.md) |
