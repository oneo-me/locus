# 共享应用壳

`oneo-gpui` 是 Oneo 应用复用的核心 shell，不依赖 Locus 产品类型。`src/lib.rs` 声明模块并集中导出公共接口；`app.rs`、`window.rs` 等文件各自定义一个职责模块，同名目录只放该模块内部实现。Rust 的目录不自动成为模块，父模块通过 `mod` 声明子模块，通过 `pub use` 有选择地公开接口。现有工作区、设置与按钮组件按自身职责保留，避免为了目录整齐而建立只转发的层级。

- `src/app.rs` 与 `src/app/menu.rs`：`AppShell` 管理 GPUI 平台应用、默认图标资源、组件初始化、减少动态效果、通用快捷键与退出/关闭/全屏动作、主窗口重新激活；菜单接受产品标签和菜单项。
- `src/window.rs` 与 `src/window/`：`ShellWindow` 创建窗口和唯一的 `Root`，处理窗口恢复、标题栏几何、macOS Sidebar 材质与平台关闭行为。
- `src/config.rs`：`SettingsStore` 负责项目路径解析、泛型 TOML 读取、原子替换与仅创建，不拥有运行时偏好实体。
- `src/workspace.rs`、`src/settings.rs`：左栏及布局状态、设置模式、侧栏设置按钮与设置页面的通用呈现。
- `src/appearance.rs` 与 `src/appearance/`：共享外观配置、主题与系统语言解析、界面字号、受控选择器，以及原生窗口外观应用。
- `src/button.rs`：工具栏按钮。

`AppShell::new().run(...)` 在产品初始化回调之前初始化组件、平台减少动态效果与 shell 动作；回调加载产品配置、注册业务能力并返回主窗口句柄。启动错误包含原因并输出到标准错误后退出。系统重新打开应用时激活该主窗口；关闭与全屏命令作用于活动窗口。macOS Release 关闭时隐藏并保留窗口，Debug 关闭时退出；其他平台 Release 关闭时移除窗口。应用可以通过 `with_assets` 替换完整资源来源，左栏和设置快捷键始终提供。

每个窗口只创建一个 `gpui_kit::component::Root`，产品根统一渲染 sheet、dialog 与 notification layer。界面基础字号写入 Theme 并同步 Base projection，由 Root 设置窗口 rem；布局随该基准缩放，主题和系统外观变化保留用户字号。Locus 的左栏与笔记页面使用 GPUI cached View 边界。

## Debug 帧率

`WorkspaceSidebar` 仅在 Debug 构建中于底部最左侧挂载帧率控件，位于调用方底部内容与设置按钮之前。控件沿用紧凑描边容器：高度随数字与内边距确定，容器垂直居中；按钮四周内边距 0.25 rem，使用主题 radius_lg 圆角及 10% 不透明度的 sidebar_foreground 描边。数字为 0.625 rem，颜色固定为 muted_foreground；悬停只改变内部背景，文字与描边不变；Release 不创建统计实体或占用布局。Locus、Theme Smith、Opus Index 与共享壳示例统一使用侧栏入口，内容标题栏不显示帧率。

`oneo-gpui/src/fps.rs` 使用 GPUI 的 `FrameTimingCollector`，只计入当前窗口最近一秒的 Present 事件，每 250 毫秒采样，仅数值变化时通知刷新；当前 FPS 初始为 0，不请求动画帧。MAX 保留当前窗口最近 120 个 Draw 事件的绘制耗时，以帧数除以耗时总和估算帧率并四舍五入；不是历史峰值，估算值按窗口当前所在显示器的刷新率封顶，最终结果四舍五入。macOS 使用 CoreGraphics 的当前显示模式，Windows 使用 GDI 的当前显示设置，每次采样重新读取以跟随换屏与刷新率变化；系统未报告有效固定刷新率时保持耗时估算，不猜测屏幕上限。平台查询直接使用锁文件已有的 `core-graphics` 与 `windows` 依赖。无样本或耗时总和为零时显示 0，无新绘制时保留最后估计。

显示单个数字，组件创建时默认显示最大帧率；外层容器持有稳定描边，内部复用标准 Ghost Button 的背景反馈，数字子元素显式设置字号和颜色。点击、Enter 或 Space 切换当前与最大模式，采样不重置选择；模式不持久化，组件重建恢复最大帧率。中英文 tooltip 与可访问名称说明当前模式及切换结果。按钮命中区域隔离标题栏拖动，保留标准键盘焦点。显示刷新自身产生的真实提交同样计入帧数。窗口内的 keyed state 持有统计实体，侧栏统计元素离开渲染树后释放实体及定时任务，重新显示重新采样。Debug 应用启动时开启进程级帧追踪；共享壳使用框架帧追踪，不额外叠加性能浮层。

## 窗口与布局

`ShellWindow::new(title)` 默认初始尺寸为 980 × 820、最小尺寸为 640 × 480 个逻辑像素，应用通过 `with_size`、`with_min_size` 和 `with_saved_bounds` 提供自己的几何约束；Locus 的尺寸由 `locus-preferences::config` 提供。窗口恢复要求标题栏在至少一个显示器可用区域内露出至少 120 × 24 个逻辑像素，否则按初始尺寸居中。`ShellWindow::min_size()` 只读返回当前最小尺寸，调用方可读取已配置的窗口约束。窗口只包裹一次 `Root`，产品内容与焦点仍由调用方实体拥有；窗口状态的保存仍属于产品界面状态。

`WorkspaceShellConfig` 与 `AppShellState` 提供左栏与主内容配置：左栏初始宽度及最小宽度均为 252 个逻辑像素，最大宽度 420；中间内容最小宽度 360。`WorkspaceShell` 默认创建始终可见的左栏折叠按钮、平台标题栏位置与分隔线，调用方处理共享面板动作；共享工具按钮隔离自身命中区域，单击、按住拖拽与双击不触发背后标题栏的窗口移动或缩放，禁用时同样隔离；工具栏下拉菜单使用 `toolbar_dropdown_menu`，由外层统一隔离命中区域，保留内部菜单触发；左栏及设置入口不可关闭。`WorkspaceSidebar` 默认提供可拖拽标题栏及底部右侧的设置按钮，开放标题栏右侧、内容与底部左侧插槽；底部内边距四周均为 0.5 rem。侧栏正文顶部不叠加额外留白，利用 macOS 标题栏已有的底部间距。`WorkspacePanel` 提供可扩展内容及可选标题栏、底栏，标题栏高度为 48 个逻辑像素，默认使用半粗体；面板默认界面文字使用 `text_sm`（基准字号 16 时为 14），随全局字号按比例缩放，业务正文可覆盖。

Locus 在 `locus-preferences::config` 中提供产品窗口及左栏尺寸；左栏使用同一配置完成初始化与分隔线复位，具体尺寸与行为见[桌面工作区](workspace.md)。共享壳管理左栏折叠与调宽，不联动改变原生窗口宽度。

`SettingsShell::appearance` 默认提供完整的主题、语言与字号设置分组及导航，调用方仅追加业务设置；`AppearanceControls` 保留控件状态，产品仍拥有配置保存及变更通知。`SettingsShell` 在 macOS 和 Windows 上保持根容器与侧栏透明，统一透出应用根容器使用的共享 `workspace_background` 与原生窗口材质；设置内容区仍使用不透明语义背景。Locus 与 Theme Smith 直接复用该处理，不在产品设置页覆盖侧栏背景。`SettingsShell` 固定使用 252 个逻辑像素宽的侧栏，默认在侧栏顶部的平台标题栏下提供返回条目，复用设置导航条目的文字、图标、悬停与焦点样式，派发 `CloseSettings`；调用方负责恢复原焦点。内容区保留高度 48 的空白拖拽标题栏，不显示页面标题。`SettingsGroup` 提供分类标题与圆角容器，标题左侧内边距为 0.75 rem，圆角为 13/16 rem；`SettingsItem` 提供标题、可选描述与右侧自定义控件，标题与描述间距为 0.125 rem，标题行高为 1.125 rem，描述行高为 1 rem。产品仅提供分类和设置内容，不重复实现默认按钮、行布局或分组容器。

## 独立示例

独立使用示例位于 `source/crates/oneo-gpui/examples/minimal.rs`，通过 `cargo run -p oneo-gpui --example minimal` 运行；示例演示项目名称、TOML 类型、应用启动、左栏与主内容工作区组合，不依赖 Locus 产品包。左栏默认展开，两个区域各有可拖动标题栏；侧栏支持按钮与共享快捷键折叠、拖拽分隔线调宽、双击分隔线恢复初始宽度。示例复用共享布局状态，不持久化侧栏状态。左侧栏底部保留设置按钮，Command+, 打开同一内嵌设置页；返回保留面板状态并恢复工作区焦点。外观修改即时应用并保存到 `~/.oneo/oneo-example/settings.toml`，保存失败在设置页显示错误。示例文案独立维护于 `examples/locales/ui.yml`。

## 平台与原生行为

Locus 主程序在 Windows 非 Debug 构建中使用 Windows GUI 子系统，启动时不附带控制台；Debug 保留控制台。`locus-library/src/process_environment.rs` 为后台 Git 命令设置 `CREATE_NO_WINDOW`，`locus-ui/src/platform.rs` 对 VS Code 启动命令应用同一标志，避免后台调用闪现控制台并保留命令输出读取能力。用户显式启动外部 Agent 时仍由终端模块创建新控制台，行为见[文档库终端](terminal.md)。

macOS 使用 GPUI 创建的整窗模糊视图承载系统 Sidebar 材质，GPUI 根视图保持透明，应用层负责中性染色和不透明内容表面。Windows 由共享 ShellWindow 在窗口创建回调中调用 GPUI 的 `window.set_background_appearance(WindowBackgroundAppearance::MicaBackdrop)`，由框架同步原生材质与渲染状态，不自行检测支持情况或实现 Blur 降级；平台模块继续同步材质明暗。共享工作区背景在 Windows 返回透明色，Locus 设置根容器、共享设置根容器与侧栏不绘制不透明背景，主内容仍使用不透明语义表面。复用已有 windows 0.58 与 raw-window-handle，仅启用 DWM 和窗口操作所需 feature，不增加依赖包。平台专属实现限制在平台模块与共享壳 crate 中，业务领域不直接依赖 AppKit 或 Win32。共享 content_titlebar 组合业务标题区与 Windows 窗口按钮，按钮通过 WindowControlArea 接入原生命中；设置页复用相同标题栏。产品页面组合见[桌面工作区](workspace.md)。

macOS 侧栏的共享中性染层在明暗主题下使用 alpha 0.85；主内容、设置内容及浮层使用不透明语义表面，画布区域的 GPUI 祖先保持透明。

Windows 工作区与设置页标题栏右侧组合最小化、最大化或还原、关闭按钮。按钮使用紧凑正方形尺寸与主题圆角，在标题栏内垂直居中，上下及右侧留白一致；最大化状态显示还原图标，键盘激活使用相同动作。关闭按钮悬停为红底白图标，按下使用更深红底。按钮通过 WindowControlArea 提供系统命中区域，并与业务标题和工具栏并排布局。

设置返回条目在 macOS 避让交通灯标题区；Windows 顶部使用与左右一致的 0.75 rem 边距，右侧内容保留可拖动标题栏与窗口按钮。窗口拖动区域与工具按钮命中明确分离，所有平台均需保留键盘路径和焦点反馈。

原生 Mica 的可用性与实际呈现由 GPUI、系统版本、DWM 和系统透明效果策略决定。应用直接调用框架 API 并同步明暗主题，接受不支持时的框架表现；不建立产品侧能力检测与降级状态机。

原生材质作为窗口环境层，主内容和浮层使用稳定语义表面。AppKit 效果视图位于 GPUI 渲染表面之外，不能插入同一 Metal 表面的绘制层之间；菜单和内容背板因此不依赖局部背景模糊。面板折叠裁切完整内容，系统启用减少动态效果时直接更新布局。

## 验证边界

共享壳变更需运行自身测试，并检查受影响消费应用的编译与接入。侧栏拖动使用生产窗口级监听，回归覆盖跨入遮挡区域、释放及释放后停止改变宽度；工具按钮需检查鼠标与键盘动作以及标题栏命中隔离。

平台材质、窗口按钮、系统重新激活、FPS 换屏与显示器刷新率限制需要对应平台实机确认。macOS 检查或模拟 Windows 布局不代表 Windows 原生分支已编译或运行。配置服务与外观资源的契约见[设置与持久化](settings.md)。
