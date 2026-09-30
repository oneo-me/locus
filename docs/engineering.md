# 工程

## 文档职责

本篇定义技术结构与开发约束；当前需求清单见[需求](requirements.md)，具体产品行为见[专题](topics/README.md)。应用打包工具作为工程设施维护，其 CLI、平台构建与发布契约见[应用打包](engineering/application-packaging.md)。

## 文档与源码布局

- `docs/README.md`：产品定义与文档导航。
- `docs/requirements.md`：需求及实现状态。
- `docs/engineering.md` 与 `docs/engineering/`：工程规则、应用打包与 WebView 接入。
- `docs/topics/`、`docs/research/`：功能专题与技术依据。
- `docs/usage/`：用户操作、MCP 与构建说明。
- `source/resources/docs/`：供 MCP 编译嵌入并随应用打包的协议原文，属于应用运行资源；项目文档通过相对链接引用，不复制或迁入 `docs/`。
- `source/`：Cargo 工作区、源码与应用资源；文中的 crate 内路径相对于对应 `source/crates/<crate>/`。
- `dist/`：最终应用产物；`AGENTS.md` 为忽略提交的本地规则入口。

文档随当前代码分支维护。既有验证记录如未注明适用提交，其适用代码版本未知，不能视为当前工作区已重新验证；用户实机反馈仅覆盖记载的平台与行为。

[WebView 接入](engineering/webview.md)说明共享原生组件的使用方式。

## 技术基线

应用版本规则见[构建说明](usage/build.md)，版本值以`source/Cargo.toml` 的 `workspace.package.version` 为准，继承该版本的工作区包与 `Cargo.lock` 同步更新。

源码仓库根目录保留 `.vscode/`、`dist/`、`docs/`、`source/`、`.gitignore`、`AGENTS.md` 与 `README.md`，Git 自身目录 `.git/` 除外。代码、Cargo 清单与锁文件、资源、图标和打包配置统一放在 `source/`；最终应用产物直接放在 `dist/`，不按平台或架构划分子目录，Git 忽略整个 `dist/`。VS Code 通过 `source/Cargo.toml` 识别 Rust 工作区。

Locus 使用 Rust 2024 edition、Cargo、`gpui-kit` 0.6.6 构建原生桌面应用。`source/` 是虚拟 Cargo workspace，所有包均位于 `source/crates/`；`source/crates/locus` 是默认运行包，应用启动与能力实现分离：

各项目分别维护自己的 `Cargo.lock`，分别锁定 GPUI Kit 及其传递依赖的发布版本；单个项目内的应用、工作台与共享壳必须解析到同一个 GPUI 类型实例。共享壳变更需要分别验证使用它的项目。

- `oneo-tool-packer`：独立 CLI，读取项目 packer.toml，统一构建与组装 macOS 应用目录和 Windows 便携目录；不依赖 GUI 或 Locus 产品类型。
- `locus`：通过共享 shell 启动应用与窗口，提供 Locus 偏好、语言、业务初始化、菜单和关于面板；组装工作区。
- `locus-workspace`：窗口内业务编排、导航、笔记画布、外部终端启动、界面状态、侧栏排序、产品动作与菜单。
- `locus-settings`：偏好实体、设置页面、归档恢复与设置事件；通过公开事件和方法与工作区协作。
- `locus-preferences`：产品偏好字段、默认值与校验、项目存储名称和产品选项标签；组合共享外观配置，配置 IO 消费共享服务。
- `locus-library`：笔记、画布文件与附件模型、扫描、读取、修改、Git 投影与文件观察；不依赖 GUI。
- `locus-canvas`：独立画布视图、自定义资源协议、HTML 宿主、导出、主题与焦点同步、网页事件和生命周期；通过事件与工作区协作，笔记导航事件由工作区复用现有选择与历史。
- `locus-mcp`：MCP 工具契约、监听和线程生命周期；只消费文档库及显式服务配置，不依赖 GUI 或用户偏好类型。
- `locus-ui`：Locus 专属笔记图标、Markdown 扩展、附件呈现、可重建图像缓存与平台文件及终端打开操作。
- `oneo-gpui`：核心 shell：应用启动与初始化、窗口创建与恢复、系统重新激活、平台行为、TOML 配置服务、左栏与主内容工作区、内嵌设置、共享外观配置与控件、菜单与面板状态。
- `oneo-gpui-graph`：与产品无关的关系图数据校验、力导向布局、节点与连线绘制、视口、拖拽缩放、邻域强调与节点激活事件。
- `oneo-gpui-terminal`：不依赖产品偏好的 Alacritty PTY 状态、输入映射与 GPUI 终端视图。
- `oneo-gpui-webview`：macOS WebKit 原生分层合成、裁剪、可见性、输入命中与焦点生命周期；不依赖 Locus 产品类型。

应用与共享 crate 直接依赖 crates.io 的 `gpui-kit = "0.6.6"`，通过 `gpui_kit`、`gpui_kit::component`、`gpui_kit::assets` 和 `gpui_kit::platform` 使用框架；必须解析到同一个 GPUI 类型实例，发布版本以 `source/Cargo.lock` 为准。图标 HTTP 请求使用匹配的 `gpui-pre-http-client` 与 `gpui-pre-reqwest-client` 发布包。项目采用 GPUI 的依据与渲染边界见[GPUI 原生重写](research/gpui.md)。

持有界面文案的包各自维护 `locales/ui.yml`，在自己的 crate root 调用 `rust_i18n::i18n!("locales", fallback = "en")`，并从本包使用 `t!` 读取：`locus-workspace` 持有工作区、菜单和外部终端启动的产品文案，`locus-settings` 持有产品设置页面文案，`oneo-gpui` 持有共享设置与外观控件文案，`locus-ui` 持有笔记扩展文案，`locus-canvas` 持有画布加载与错误文案，`locus-preferences` 持有偏好选项标签。各包只保留自己使用的资源，提供英文和简体中文，不通过偏好包代查其他包的 key，也不依赖跨包资源合并。没有自有文案、只消费调用方标签的共享组件不建立空语言包。

`oneo_gpui::Language` 提供语言偏好、locale 解析、选项名称和统一语言切换；调用方通过 `t!(..., locale = language.locale())` 显式选择语言并使用宏的命名参数替换。切换语言同步组件全局 locale、已渲染 view 与原生菜单；全局 locale 本身不触发 GPUI 重绘。`extend!` 仅在确需覆盖组件文案时按组件命名空间注册一次，当前不注册空扩展。用法遵循[GPUI Kit 国际化规范](https://gpui-kit.com/zh-CN/docs/i18n/)。

每个语言资源包通过自身 `build.rs` 声明 `cargo:rerun-if-changed=locales`，保证只增删改 YAML 资源也会重新生成编译期翻译表。各包测试核对两种语言的每条资源实际解析值，笔记 UI 覆盖文件名中的字面占位符。标准输入、选择、按钮、开关、标签和菜单直接复用 `gpui_kit::component`，应用只持有偏好与产品 workflow，不建立平行控件状态机。

每个窗口只创建一个 `gpui_kit::component::Root`；产品根统一渲染 sheet、dialog 与 notification layer。界面基础字号写入 `gpui_kit::component::Theme` 并同步 Base projection，由 `Root` 设置窗口 `rem`；明暗主题切换和系统外观变化必须保留当前用户字号，普通界面布局使用随该基准缩放的 rem 尺寸。

产品 crate 依赖方向为：`locus → locus-workspace → locus-settings`；工作区还消费 `locus-canvas`；画布包消费 `locus-library`、`gpui-kit`、macOS 专属 `oneo-gpui-webview` 和共享语言类型，不依赖工作区、设置或 Markdown 渲染包。工作区与设置消费 `locus-ui`、`locus-preferences`、`locus-library` 和 `locus-mcp` 的公开接口。`locus-ui` 消费文档库模型与用户偏好，`locus-mcp` 只依赖 `locus-library`。功能 crate 不依赖应用二进制，也不跨包访问内部模块；`oneo-gpui*` 不依赖任何 Locus 产品 crate。导航、笔记画布和外部终端启动编排保留在工作区内部；`oneo-gpui-graph` 作为独立通用组件保留，Locus 产品 crate 不依赖或实例化它，不按每个屏幕或辅助函数继续拆包。

工作区将偏好显式映射为 `McpConfiguration`，只传入文档库路径、启用状态、监听范围和端口；控制器不读取偏好存储。设置继续持有运行时偏好并通过 `SettingsEvent` 通知工作区，不建立另一份长期配置状态。

## 文档导航实现

`locus-workspace/src/workspace/sidebar.rs` 使用 `gpui_kit::base::Tree` 与持久持有的 `TreeState` 提供虚拟列表、键盘导航及树形可访问语义；应用通过行渲染接口保留侧栏选中样式、笔记图标、附件数量、Git 提示、右键菜单与同级拖拽排序。文档路径作为节点身份；工作区在启动、选择、排序、库切换与扫描更新时同步 Tree 投影，展开事件回传工作区，Tree 发起的选择携带该节点的实际展开状态，不能因节点有子项而重新展开；后台扫描保留仍有效的展开状态。滚动由 Tree 的列表句柄持有，列表使用 0.5 rem 横向内边距，外层滚动区域占满侧栏宽度，滚动条贴齐区域右边缘。同级排序动画由工作区持有统一起始时间，持续 150 毫秒；条目重建只采样原动画进度，结束后清理，库切换或投影替换取消动画，减少动态效果时直接呈现最终位置。

验证：`cargo test -p locus-workspace --locked` 通过 35 项测试。覆盖扫描结果到 Tree 的层级、同名节点身份、排序后选择和空列表；Tree 原生点击到应用选择投影的收起再展开回归，以及条目及后代重新出现时共享原动画截止时间的回归。工作区编译检查通过。Tree 的实际选中外观、键盘导航、拖拽动画与长列表滚动待用户验证。

## 核心 shell 与 Rust 模块组织

`oneo-gpui` 是后续 Oneo 应用复用的核心 shell，不依赖 Locus 产品类型。`src/lib.rs` 声明模块并集中导出公共接口；`app.rs`、`window.rs` 等文件各自定义一个职责模块，同名目录只放该模块内部实现。Rust 的目录不自动成为模块，父模块通过 `mod` 声明子模块，通过 `pub use` 有选择地公开接口。现有工作区、设置与按钮组件按自身职责保留，避免为了目录整齐而建立只转发的层级。

- `src/app.rs` 与 `src/app/menu.rs`：`AppShell` 管理 GPUI 平台应用、默认图标资源、组件初始化、减少动态效果、通用快捷键与退出/关闭/全屏动作、主窗口重新激活；菜单接受产品标签和菜单项。
- `src/window.rs` 与 `src/window/`：`ShellWindow` 创建窗口和唯一的 `Root`，处理窗口恢复、标题栏几何、macOS Sidebar 材质与平台关闭行为。
- `src/config.rs`：`SettingsStore` 负责项目路径解析、泛型 TOML 读取、原子替换与仅创建，不拥有运行时偏好实体。
- `src/workspace.rs`、`src/settings.rs`：左栏及布局状态、设置模式、侧栏设置按钮与设置页面的通用呈现。
- `src/appearance.rs` 与 `src/appearance/`：共享外观配置、主题与系统语言解析、界面字号、受控选择器，以及原生窗口外观应用。
- `src/button.rs`：工具栏按钮。

`AppShell::new().run(...)` 在产品初始化回调之前初始化组件、平台减少动态效果与 shell 动作；回调加载产品配置、注册业务能力并返回主窗口句柄。启动错误包含原因并输出到标准错误后退出。系统重新打开应用时激活该主窗口；关闭与全屏命令作用于活动窗口。macOS Release 关闭时隐藏并保留窗口，Debug 关闭时退出；其他平台 Release 关闭时移除窗口。应用可以通过 `with_assets` 替换完整资源来源，左栏和设置快捷键始终提供。

`WorkspaceSidebar` 仅在 Debug 构建中于底部最左侧挂载帧率控件，位于调用方底部内容与设置按钮之前。控件沿用紧凑描边容器：高度随数字与内边距确定，容器垂直居中；按钮四周内边距 0.25 rem，使用主题 radius_lg 圆角及 10% 不透明度的 sidebar_foreground 描边。数字为 0.625 rem，颜色固定为 muted_foreground；悬停只改变内部背景，文字与描边不变；Release 不创建统计实体或占用布局。Locus、Theme Smith、Opus Index 与共享壳示例统一使用侧栏入口，内容标题栏不显示帧率，不再提供标题栏帧率接口。`oneo-gpui/src/fps.rs` 使用 GPUI 的 `FrameTimingCollector`，只计入当前窗口最近一秒的 Present 事件，每 250 毫秒采样，仅数值变化时通知刷新；当前 FPS 初始为 0，不请求动画帧。MAX 保留当前窗口最近 120 个 Draw 事件的绘制耗时，以帧数除以耗时总和估算帧率并四舍五入；不是历史峰值，估算值按窗口当前所在显示器的刷新率封顶，最终结果四舍五入。macOS 使用 CoreGraphics 的当前显示模式，Windows 使用 GDI 的当前显示设置，每次采样重新读取以跟随换屏与刷新率变化；系统未报告有效固定刷新率时保持耗时估算，不猜测屏幕上限。平台查询直接使用锁文件已有的 `core-graphics` 与 `windows` 依赖。无样本或耗时总和为零时显示 0，无新绘制时保留最后估计。显示单个数字，组件创建时默认显示最大帧率；外层容器持有稳定描边，内部复用标准 Ghost Button 的背景反馈，数字子元素显式设置字号和颜色。点击、Enter 或 Space 切换当前与最大模式，采样不重置选择；模式不持久化，组件重建恢复最大帧率。中英文 tooltip 与可访问名称说明当前模式及切换结果。按钮命中区域隔离标题栏拖动，保留标准键盘焦点。显示刷新自身产生的真实提交同样计入帧数。窗口内的 keyed state 持有统计实体，侧栏统计元素离开渲染树后释放实体及定时任务，重新显示重新采样。Debug 应用启动时开启进程级帧追踪；共享壳不再依赖 `gpui-fps`，不显示浮动 HUD、图表或资源用量。

FPS 与侧栏滚动验证（部分验证）：工作区 35 项测试、共享壳 27 项测试通过，覆盖统计窗口、显示器上限、侧栏紧凑尺寸与垂直对齐，以及模拟点击和 Enter/Space 按键切换默认最大值与当前值、鼠标事件隔离。相关包全目标全功能严格 Clippy、格式检查、中英文文案校验和两个消费应用编译通过。侧栏数字容器的实际布局、字号、明暗主题、点击切换与悬停反馈、60Hz 上限、换屏后的读数及 Tree 滚动条贴边，待用户验证。当前 host 仅有 aarch64-apple-darwin，Windows 查询未进行目标平台编译与运行验证。

`ShellWindow::new(title)` 默认初始尺寸为 980 × 820、最小尺寸为 640 × 480 个逻辑像素，应用通过 `with_size`、`with_min_size` 和 `with_saved_bounds` 提供自己的几何约束；Locus 的尺寸由 `locus-preferences::config` 提供。窗口恢复要求标题栏在至少一个显示器可用区域内露出至少 120 × 24 个逻辑像素，否则按初始尺寸居中。`ShellWindow::min_size()` 只读返回当前最小尺寸，调用方可读取已配置的窗口约束。窗口只包裹一次 `Root`，产品内容与焦点仍由调用方实体拥有；窗口状态的保存仍属于产品界面状态。

`WorkspaceShellConfig` 与 `AppShellState` 提供左栏与主内容配置：左栏初始宽度及最小宽度均为 252 个逻辑像素，最大宽度 420；中间内容最小宽度 360。`WorkspaceShell` 默认创建始终可见的左栏折叠按钮、平台标题栏位置与分隔线，调用方处理共享面板动作；共享工具按钮隔离自身命中区域，单击、按住拖拽与双击不触发背后标题栏的窗口移动或缩放，禁用时同样隔离；工具栏下拉菜单使用 `toolbar_dropdown_menu`，由外层统一隔离命中区域，保留内部菜单触发；左栏及设置入口不可关闭。`WorkspaceSidebar` 默认提供可拖拽标题栏及底部右侧的设置按钮，开放标题栏右侧、内容与底部左侧插槽；底部内边距四周均为 0.5 rem。侧栏正文顶部不叠加额外留白，利用 macOS 标题栏已有的底部间距。`WorkspacePanel` 提供可扩展内容及可选标题栏、底栏，标题栏高度为 48 个逻辑像素，默认使用半粗体；面板默认界面文字使用 `text_sm`（基准字号 16 时为 14），随全局字号按比例缩放，业务正文可覆盖。

Locus 在 `locus-preferences::config` 中提供产品窗口及左栏尺寸；左栏使用同一配置完成初始化与分隔线复位，具体尺寸与行为见[桌面工作区](topics/workspace.md)。共享壳管理左栏折叠与调宽，不联动改变原生窗口宽度。

`SettingsShell::appearance` 默认提供完整的主题、语言与字号设置分组及导航，调用方仅追加业务设置；`AppearanceControls` 保留控件状态，产品仍拥有配置保存及变更通知。`SettingsShell` 在 macOS 和 Windows 上保持根容器与侧栏透明，统一透出应用根容器使用的共享 `workspace_background` 与原生窗口材质；设置内容区仍使用不透明语义背景。Locus 与 Theme Smith 直接复用该处理，不在产品设置页覆盖侧栏背景。`SettingsShell` 固定使用 252 个逻辑像素宽的侧栏，默认在侧栏顶部的平台标题栏下提供返回条目，复用设置导航条目的文字、图标、悬停与焦点样式，派发 `CloseSettings`；调用方负责恢复原焦点。内容区保留高度 48 的空白拖拽标题栏，不显示页面标题。`SettingsGroup` 提供分类标题与圆角容器，标题左侧内边距为 0.75 rem，圆角为 13/16 rem；`SettingsItem` 提供标题、可选描述与右侧自定义控件，标题与描述间距为 0.125 rem，标题行高为 1.125 rem，描述行高为 1 rem。产品仅提供分类和设置内容，不重复实现默认按钮、行布局或分组容器。

`SettingsStore::for_project(name)` 使用 `~/.oneo/<name>/settings.toml`，名称必须是单个非空目录名，不允许绝对路径、`.`、`..` 或路径分隔符。`SettingsStore::at_path(path)` 直接替换完整文件路径，要求绝对文件路径，不解释 `~` 等 shell 语法。两者是明确的可选构造方式；Locus 使用标准项目名称 `locus`。

配置文件不存在时返回应用类型的默认值且不自动创建文件；字段缺省规则由应用的 Serde 类型定义。读取或 TOML 格式错误返回带路径的错误，不自动覆盖文件。Locus 在加载失败时使用默认偏好并向既有设置错误界面传递原因。保存会先序列化，再建立父目录、写入同目录临时文件并同步，然后替换目标；失败不截断已有文件。服务保存完整文档，不保留注释和类型未包含的字段，不提供合并、监听或迁移。调用方负责串行化配置变更；多个写入方同时保存时最后一次替换生效。运行时偏好继续由设置实体单独持有。

`oneo-gpui` 默认启用 `shell` feature，包含应用、窗口和界面组件。配置服务不依赖 GPUI；纯领域与偏好包可以使用 `default-features = false`，只编译配置服务、外观数据类型及文件读写依赖。`read` 用 `Option` 区分文件不存在，`load` 在缺失时返回默认值，`create` 原子创建且拒绝覆盖已有文件；`save` 原子替换已有文件。Theme Smith 的目录元数据使用 `create`，避免并发创建时覆盖其他写入方。

`AppearancePreferences` 统一定义主题、语言与界面字号，应用的偏好类型使用 `#[serde(flatten)]` 组合，TOML 中使用顶层 `theme`、`language`、`ui_font_size` 字段。主题默认 `system`，可选 `light`、`dark`；语言默认 `system`，可选 `chinese`、`english`；界面字号默认 16，可选 12、14、15、16、17、18、20，不支持的字号读取后归一为默认值。已有显式语言选择不改写为系统偏好。

系统语言通过 `sys-locale` 在进程首次解析时读取：中文语言标识使用简体中文目录，其他语言或读取不到时使用英文目录；更改操作系统语言后重启应用生效。选择明确语言或重新选择跟随系统时立即刷新当前界面与组件 locale，产品方通过变更事件更新自己的菜单及业务文案。系统明暗变化在跟随系统主题时即时应用；主题切换同步原生窗口、GPUI Theme 与 Base projection，并保留用户字号。

`AppearanceControls` 只拥有选择器状态及订阅，外观配置与保存错误继续由应用的设置实体单独持有；父级处理 `AppearanceChange`、应用配置、同步选择器并保存完整产品偏好。共享层不创建第二份产品偏好、不写入固定产品路径。Locus 的外观页组合这三个共享控件及自身界面字体、代码字体与代码字号；终端、文档库和 MCP 设置仍属于产品层。

独立使用示例位于 `source/crates/oneo-gpui/examples/minimal.rs`，通过 `cargo run -p oneo-gpui --example minimal` 运行；示例演示项目名称、TOML 类型、应用启动、左栏与主内容工作区组合，不依赖 Locus 产品包。左栏默认展开，两个区域各有可拖动标题栏；侧栏支持按钮与共享快捷键折叠、拖拽分隔线调宽、双击分隔线恢复初始宽度。示例复用共享布局状态，不持久化侧栏状态。左侧栏底部保留设置按钮，Command+, 打开同一内嵌设置页；返回保留面板状态并恢复工作区焦点。外观修改即时应用并保存到 `~/.oneo/oneo-example/settings.toml`，保存失败在设置页显示错误。示例文案独立维护于 `examples/locales/ui.yml`。

## 系统与运行边界

当前产品以原生桌面进程为宿主，不包含独立业务后端；画布笔记在标题栏及头部元数据以下嵌入系统 WebView，网页运行由平台浏览器引擎承载：

```text
GPUI / OS event loop
└─ Locus window
   ├─ desktop workspace
   │  ├─ document navigation
   │  └─ selected note
   │     ├─ note titlebar
   │     └─ Markdown body or exclusive canvas
   └─ settings

optional MCP thread
└─ Tokio + Axum + rmcp Streamable HTTP
```

`locus-workspace/src/commands.rs` 组装原生“编辑”菜单，复用 GPUI Kit 的 Copy、Cut、Paste、SelectAll 动作及已初始化的快捷键，通过 `MenuItem::os_action` 关联系统编辑命令。macOS 将命令交给当前原生响应者，画布由 WKWebView 处理选区与剪贴板，GPUI 输入控件继续使用组件动作；不在画布 IPC 或 JavaScript 中复制编辑实现。

文档库投影直接读取用户选择的本地目录；外部终端在该目录启动所选 Agent；MCP 在独立线程和 Tokio runtime 中按调用读取同一文档库。

Locus 主程序在 Windows 非 Debug 构建中使用 Windows GUI 子系统，启动时不附带控制台；Debug 保留控制台。`locus-library/src/process_environment.rs` 为后台 Git 命令设置 `CREATE_NO_WINDOW`，`locus-ui/src/platform.rs` 对 VS Code 启动命令应用同一标志，避免后台调用闪现控制台并保留命令输出读取能力。用户显式启动外部 Agent 时仍由终端模块创建新控制台，行为见[文档库终端](topics/terminal.md)。

macOS 使用 GPUI 创建的整窗模糊视图承载系统 Sidebar 材质，GPUI 根视图保持透明，应用层负责中性染色和不透明内容表面。Windows 由共享 ShellWindow 在窗口创建回调中调用 GPUI 的 `window.set_background_appearance(WindowBackgroundAppearance::MicaBackdrop)`，由框架同步原生材质与渲染状态，不自行检测支持情况或实现 Blur 降级；平台模块继续同步材质明暗。共享工作区背景在 Windows 返回透明色，Locus 设置根容器、共享设置根容器与侧栏不绘制不透明背景，主内容仍使用不透明语义表面。复用已有 windows 0.58 与 raw-window-handle，仅启用 DWM 和窗口操作所需 feature，不增加依赖包。平台专属实现限制在平台模块与共享壳 crate 中，业务领域不直接依赖 AppKit 或 Win32。共享 content_titlebar 组合业务标题区与 Windows 窗口按钮，按钮通过 WindowControlArea 接入原生命中；设置页复用相同标题栏。完整平台行为见[桌面工作区](topics/workspace.md)。

`locus-canvas::CanvasView` 独立拥有 WebView、主题同步、刷新和网页生命周期，通过 `CanvasEvent` 提交窗口关闭、应用命令与外部链接意图。`locus-workspace/src/workspace/canvas.rs` 管理当前笔记唯一画布，拥有笔记身份、创建释放、焦点和标题栏及头部元数据以下的画布布局；笔记进入或画布存在性变化驱动生命周期。设置不再提供 `auto_open_canvas`。`note.rs` 按画布存在性选择页面布局，普通笔记保留头部概要及元数据；画布页不生成装饰背景和正文，标题栏固定使用语义背景与底部分隔线；头部复用 `note_metadata`，使用白底风格的语义背景。`canvas_page` 按纵向布局分配标题栏、可独立滚动且限高的元数据区域和剩余画布空间，避免 GPUI 信息被原生 WebView 覆盖。Markdown 渲染与 `locus-ui` 不参与画布加载；具体行为见[桌面工作区](topics/workspace.md)和[文档库](topics/document-library.md)。

画布初始化脚本以闭包保存私有启动消息，只允许浏览器真实且非重复的快捷键发送；Rust IPC 验证每个 WebView 的消息标识，拒绝网页直接发送启动命令。标识生成复用锁文件已有的 UUID v4，仅增加 `locus-canvas` 的直接依赖，不增加运行服务。

工作区持有当前笔记的 CanvasView。locus-library 解析 HTML/JSON 入口并提供安全资源读取；locus-canvas 通过 WebView 自定义协议在后台按请求读取页面及资源。CanvasView 持有网页、焦点、主题与已加载资源集合；切换笔记或文档库释放网页，不存在库级 Node/Vite 进程。Tailwind 浏览器脚本嵌入应用。HTML 宿主按页面加载生命周期执行 main(data)，文件变化不热更新网页。

locus-canvas 的 view 负责网页生命周期，resources 负责资源映射与 MIME，export 负责依赖收集、快照与不覆盖保存。工作区捕获导出对象与目录，在后台准备并保存，设置实体持有 export_directory；未配置时解析系统下载目录，其他偏好与原持久化契约保留。完整协议与验收集中在[画布](topics/canvas.md)。原生 WebView 由独立 `oneo-gpui-webview` 提供，仅支持 macOS；lb-wry 0.53.3 继续负责系统 WebKit 创建、导航、IPC 与脚本配置。GPUI 只使用已发布版本的公开接口，不引入 vendor、patch 或 gpui-pre 源码修改。Objective-C AppKit 桥接通过已有 cc 编译，默认 macOS 部署目标 13.0，与应用打包最低版本一致，可由 MACOSX_DEPLOYMENT_TARGET 显式覆盖。详细合成与输入契约见画布专题。

## 状态与数据流

1. 应用加载用户偏好和窗口状态，解析主题与语言，创建工作区根实体。
2. [文档库](topics/document-library.md)扫描当前目录，建立不长期保存全部正文的导航、归档、附件和 Git 状态投影，并启动递归文件观察。
3. [桌面工作区](topics/workspace.md)消费投影；选择笔记时保留并解析当前正文；打开画布后由独立画布视图保留网页状态，导航和阅读不维护另一份内容事实。
4. 文件事件合并后在后台触发导航投影重建；与当前笔记相关的变化重新读取当前正文；画布存在性同步更新，当前画布源码修改后通过手动刷新或重新打开生效。笔记读取的迟到任务按文档库路径和读取代次丢弃。
5. [文档库终端](topics/terminal.md)按需调用外部终端并执行所选 Agent 命令，工作目录只来自触发时当前文档库。
6. [Agent 接入](topics/agent-access.md)通过标准接口返回原始路径，或通过完整接口按逻辑对象直接读写真实文件；两个入口共享同一文档库与监听生命周期。

工作区按左栏、笔记页面建立 GPUI cached View 边界。FPS 采样和刷新由共享壳的侧栏统计实体负责，具体机制见“核心 shell 与 Rust 模块组织”。Locus、Theme Smith、Opus Index 与独立示例统一使用左栏和主内容接口；共享 shell 不再拥有右栏实体、开关、快捷键、尺寸、动画或窗口联动。

产品状态、可重建投影和外部进程必须通过明确接口协作。文档库文件是内容事实；工作区拥有窗口内选择与呈现状态；偏好实体拥有用户配置；外部终端软件拥有会话与进程状态；MCP 控制器拥有监听生命周期。

## 持久化

- `~/.oneo/locus/settings.toml`：文档库、编辑器、外观、语言、字体、终端和 MCP 用户偏好。
- `~/.oneo/locus/state.toml`：侧栏状态、当前选择与普通窗口恢复边界。
- `~/.oneo/locus/sidebar-order.toml`：按文档库和父路径保存的侧边栏同级顺序，不修改文档库内容。
- `~/.oneo/locus/thumbnails/` 与 `link-icons/`：可从来源重建的本地缓存。

Rust 项目的自有配置统一使用 TOML，不引入 JSON 配置。设置、界面状态与侧栏顺序均使用 TOML，并统一通过共享配置服务读写。旧终端字段及 `auto_open_canvas` 不再读取为有效偏好，正常保存时由完整序列化移除，其他有效偏好保留；不主动覆盖既有用户配置。设置、界面状态和缓存不能成为笔记内容、层级或关系的权威来源。

窗口移动或缩放只更新工作区内存中的边界；每次事件重置 100 毫秒延迟，停歇后才提交最新界面状态。`InterfaceStateWriter` 将窗口边界、左栏状态和当前选择的完整快照统一交给 GPUI 后台执行器，按提交顺序完成 TOML 序列化、原子写入和磁盘同步，避免 UI 线程写盘及旧快照覆盖新状态。退出时取消待触发的延迟，提交最新快照并等待写入队列；等待受 GPUI 的 200 毫秒退出上限约束，异常缓慢的磁盘仍可能来不及完成最终保存。保存失败继续通过现有标准错误报告。

## 项目专属约束

- Rust GPUI 开发先复用 `gpui_kit::component` 与项目锁定版本的现有能力。
- 产品领域类型、Locus 偏好、本地化、缓存目录和平台打开操作不得进入可复用 crate；共享 crate 只接受中性配置、标签、回调或扩展接口。
- `oneo-gpui` 拥有左栏与主内容布局、设置模式、菜单、侧栏尺寸与折叠和平台行为；左栏、主内容、标题栏及完整设置是必选能力；Locus 只提供业务内容、产品动作、本地化标签与可持久化初始状态，不在应用内复制壳层状态机。
- Markdown 标准能力直接使用 `gpui_kit::component::text::TextView`。当前笔记持有 `Entity<TextViewState>`，同一笔记重载时更新正文，切换笔记时释放旧状态；框架负责解析、原生排版、连续选择、全选和复制。`locus-ui/src/markdown_extensions/` 通过块与行内插件接入图库、附件、图片、链接图标及导航回调；插件解析器只产生数据，渲染时读取当前附件、语言与图标投影。扩展声明不进入纯文本复制结果。代码正文使用代码字体，不启用语法高亮，不提供独立复制按钮或展开收起。应用不再维护自研 Markdown 渲染 crate；扩展语法使用 TextView 已依赖的 `markdown` 解析库。
- `oneo-gpui-graph` 只依赖 `gpui-kit`，接受不透明稳定 ID、标签、显式聚类、节点半径、连线距离/强度以及明暗主题外观；不读取路径、文档类型、偏好或本地化资源。该组件不接入当前 Locus 工作区。通用组件公开接口及输入约束见 `source/crates/oneo-gpui-graph/src/lib.rs`，独立应用示例见 `source/crates/oneo-gpui-graph/examples/relationships.rs`。
- `oneo-gpui-terminal` 保留为独立通用组件，Locus 不再依赖或实例化。外部启动由 `locus-ui/src/platform/terminal.rs` 负责平台命令，工作区负责后台调用、重复提交保护和错误通知；设置选择只影响后续启动。
- 用户可见文案不得用于解释界面结构、实现进度或填充留白；状态通过真实内容、准确控件状态和可行动反馈表达。

## 运行与验证

应用打包统一使用 [应用打包](engineering/application-packaging.md)，由 `source/packer.toml` 提供应用包、二进制、名称和标识；macOS 图标固定为`source/icon.icon/`，Info.plist 由工具动态生成，平台专属资源分别由 `[macos].resources` 和 `[windows].resources` 声明；Windows 固定使用`source/icon.ico` 并嵌入主程序。VSCode `build` 任务在 `source/` 执行 `cargo run --locked -p oneo-tool-packer --`；默认目标为当前 Rust host，可通过 `--target` 指定受支持的平台与架构。macOS 在 Mac 上生成 `.app`，Windows 自动加载 Visual Studio C++ 环境并默认静态链接 CRT，生成包含主程序、仍需动态加载的运行库和资源的目录，不生成安装程序。工具只构建 Release，打包配置以 `output-directory = "../dist"` 将产物直接保存到项目根 `dist/`，该目录已由 Git 忽略。

工具在 macOS Release 构建时通过 `cargo rustc --bin` 仅向主程序传入 `-Wl,-no_exported_symbols`，不提供供动态插件链接的主程序导出符号接口；Windows 不传入该参数。项目不维护专用导出配置 `build.rs`；普通 `cargo build --release` 不自动禁用导出。`strip = true` 等构建优化继续由项目 Cargo profile 定义。

开发运行和下列 Cargo 检查均在 `source/` 执行，使用 `cargo run` 启动应用。工具通过 `cargo install --path source/crates/oneo-tool-packer --locked` 安装，Cargo bin 目录需要位于 PATH 中；安装后在项目根执行 `oneo-tool-packer --project source`，工具固定生成 Release 包。产品包版本与 edition 继承 `source/Cargo.toml` 的 `[workspace.package]`，打包工具通过 Cargo 元数据读取应用包的实际版本。macOS 需要 Xcode，默认使用 ad-hoc 签名，Developer ID 签名使用 `--codesign-identity` 或 `ONEO_PACKER_CODESIGN_IDENTITY`。

既有源码目录布局的验证记录包含工作区全目标、全功能离线编译检查，打包工具严格 Clippy、19 项单元测试与 4 项 CLI 测试、格式和差异检查。根目录构建命令的预演确认使用 `source/` 配置并输出至 `dist/Locus.app`；已有应用包保留到 `dist/`，该证据不包含完整应用包重建或 Windows 实机构建。

Cargo 命令默认在 `source/` 中执行；带 `source/` 路径的 Node 命令在仓库根目录执行。代码变更按影响执行：

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

默认 `cargo test` 只覆盖应用启动包；按能力验证使用 `cargo test -p <package>`，跨包变更使用上面的 workspace 命令。文档库与 MCP 可独立测试，无需构建 GUI。关系图包使用 `cargo test -p oneo-gpui-graph` 独立验证；`cargo run -p oneo-gpui-graph --example relationships` 运行不依赖 Locus 模型的示例。

涉及窗口、Markdown、终端或设置的界面变化还需由用户实际运行确认交互、主题、键盘路径和可访问性；不能用 Agent 截图代替用户确认。

TextView 工程验证（部分验证）：`cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings`、`cargo test --workspace --all-features --locked --offline`、`cargo build -p locus --locked --offline`、`cargo fmt --all --check` 及`git diff --check` 通过。Markdown 使用框架 TextView；产品不提供代码块复制及折叠入口。原生编译使用现有 Clang 缓存，本机 HTTP 测试在允许端口监听的环境执行；保留传递依赖 `block 0.1.6` 的未来兼容性警告。当前验证平台为 macOS，其他平台与实际界面体验待验证。

TextView 已知问题（未修复）：实际阅读中，含行内代码的中文正文及含链接按钮的表格单元格出现右侧裁切。本机 macOS 对照中，移除 Locus 插件及自定义字体设置后，原生行内代码仍可触发越界；纯文本与原生链接对照未发现越界。`gpui-base 0.6.6` 的 `text/inline_flow.rs` 将普通文字交给 `gpui-pre 0.3.6` 的 `LineWrapper` 逐字符测量，最终绘制却按整段字形排版。18px 系统字体下，`中文，中文` 的逐字符宽度之和为 80.46px，整段为 89.46px；300px 换行复现中估算 294.57px、实际 303.57px。这确认了混排宽度低估的机制，但尚未单独验证链接按钮的测量与绘制是否完全一致。当前不引入依赖补丁，不能把工程检查通过视为排版问题已解决。

外部终端验证（部分验证）：`cargo test -p locus-ui --locked platform::terminal::tests` 通过 3 项测试，覆盖无效目录、启动目录与 Agent 参数、Shell 特殊字符路径；`cargo fmt --all --check` 与 Ghostty AppleScript 的 `osacompile` 编译检查通过。脚本未实际执行；Ghostty 已有单窗口、多窗口、无窗口及冷启动时的窗口复用、标签页呈现与焦点，以及实际 Agent 启动，待用户验证。验收要求见[文档库终端](topics/terminal.md)。

画布 JavaScript 验证使用 `node --experimental-vm-modules --test source/crates/locus-canvas/tests/*.cjs`，测试环境需可解析 jsdom；Node 与 jsdom 仅用于开发测试，不是画布运行依赖。画布、文档库、工作区、设置、偏好和 MCP 分别运行对应包测试；MCP HTTP 回归需要允许本机临时端口监听。具体能力验证见画布专题。

画布工程验证：`cargo clippy -p locus-canvas -p locus-library -p locus-workspace -p locus-mcp -p locus-settings -p locus-preferences --all-targets --all-features --locked --offline -- -D warnings`、`cargo fmt --all --check`、`git diff --check` 和 `cargo build -p locus --locked --offline` 通过。Clippy 的原生 AppKit 编译需允许写入现有 Clang 模块缓存；MCP HTTP 回归需允许本机临时端口监听，没有更改工具链缓存或全局配置。构建提示传递依赖 block 0.1.6 存在未来 Rust 兼容警告，当前构建成功。当前主机为 macOS，其他平台未验证；能力测试、原生合成历史证据与待用户验证事项见画布及桌面工作区专题。

画布笔记导航工程验证：`cargo clippy -p locus-canvas -p locus-workspace --all-targets --all-features --locked --offline -- -D warnings`、`cargo build -p locus --locked --offline`、`cargo fmt --all --check` 与`git diff --check` 通过。Clippy 的 AppKit 编译在允许写入现有 Clang 模块缓存的环境完成，没有修改缓存配置；沿用上述 block 传递依赖兼容警告。实际原生导航与视觉体验待用户验证。

Rust 自有配置统一 TOML；画布模版的 index.json 是内容数据。

Windows 交叉编译验证受限：通过 Rustup 的 Windows GNU 标准库与明确指定的 Rustup rustc 尝试检查共享壳，构建在 GPUI 的 Windows manifest 资源阶段因缺少 `x86_64-w64-mingw32-windres` 失败，尚未完成 Windows 专属分支的整包编译。用户确认提交 `c93c3ba` 的 Windows 版本验证通过，包括对应启动与后台命令控制台修复；打包和窗口材质的实机证据分别见应用打包与桌面工作区。本机交叉编译仍受上述工具链限制，用户实机反馈与 Agent 编译检查分开记录。

## 完整 MCP 操作边界

`locus-library/src/access.rs` 定义逻辑地址与内容目标、文件读写、SHA-256 版本及安全边界；`access/organize.rs` 处理子树清单、移动删除和 Markdown 链接。复用扫描器元数据解释及现有图标、frontmatter 修改能力，保持与界面和画布消费者一致。`reference.rs` 统一将逻辑地址解码一次，导航、链接图标、标准及完整 MCP 与画布组件共同消费，避免编码地址在不同入口指向不同对象。笔记内容结构为 `index.md`、`assets/`、`canvas/` 和子笔记目录；当前库直接迁入该结构，不提供旧布局兼容，已有 TOML 配置保持原格式。

`locus-mcp/src/lib.rs` 在每条连接的 Axum Router 中挂载标准和完整服务；服务克隆共享各自原有会话管理器和文档库操作锁。通过 TCP 连接的 `local_addr` 取得接收请求的本机 IP，与 localhost 及回环地址一起交给 rmcp 进行 Host 校验，不枚举网卡或维护局域网地址缓存。MCP 包禁止 `unsafe`，不直接依赖 `libc`；直接引用锁文件已有的 `tower`，使用 `service_fn` 组装连接服务，不自行实现 Host 解析或校验。`service.rs` 保留标准契约；`full.rs` 适配完整工具并把阻塞文件工作放入 Tokio blocking 任务，所有完整会话共用一个文档库操作锁。`full/transfer.rs` 持有会话内暂存上传，服务关闭或会话释放回收文件；取消令牌防止停止后的排队请求执行。

完整接口只暴露逻辑对象和内容，不序列化内部 PathBuf 或带本机路径的 I/O 错误。标准接口继续保留原始路径。设置只提供服务开关、局域网开关、端口、运行状态及启动错误，不生成连接地址或客户端配置。侧边栏保留只读服务状态。详细工具、传输限制与失败契约统一归[Agent 接入](topics/agent-access.md)，公开使用说明为仓库 `docs/usage/mcp.md`。

直接使用已在锁文件中的 `base64`、`sha2`、`tempfile`、`pulldown-cmark` 与 `libc` 完成传输编码、版本摘要、暂存文件、Markdown 目的地址解析及不覆盖目标的原生目录重命名；不自行实现这些通用格式，也不引入运行服务或内容存储副本。新增成本是这些已有依赖在对应 crate 中的直接引用。

## 内置软件文档资源

`source/resources/docs/attachments.md` 与 `source/resources/docs/canvas.md` 是随软件维护的英文使用规范。`source/crates/locus-mcp/src/docs.rs` 持有固定资源清单，通过 `include_str!` 引入同一源文件，两个服务共用发现与读取实现，不依赖文档库路径、GUI 或客户端文件访问。规范随二进制版本更新，不增加运行期缓存、路径查找或回退逻辑。

`source/packer.toml` 将源码目录内的 `resources/docs` 映射到打包资源目录的 `docs`：macOS 为 `Contents/Resources/docs`，Windows 便携目录为 `resources/docs`。正文由同一源文件同时提供编译期内容与可读取的打包文件。MCP 地址与用户读取契约见[Agent 接入](topics/agent-access.md)。

验证（已验证）：在 macOS 对采用 `source/resources/docs/` 布局的工作区执行 `cargo test --manifest-path source/Cargo.toml -p locus-mcp -p oneo-tool-packer --locked --offline -- --test-threads=1`，16 项 MCP 测试、19 项打包单元测试与 4 项 CLI 测试通过。并行执行曾在服务停止后重新绑定临时端口时出现 `AddrInUse`，串行执行通过；该项存在端口复用的测试隔离风险。两个 MCP 入口在未选择文档库时均能读取完整协议；打包测试使用项目实际 `packer.toml` 和资源复制实现，核对 macOS 与 Windows 资源目录中的两份文件与源内容逐字节相同。该结果覆盖文档路径、编译嵌入与资源复制，不代表完整应用打包、启动或 Windows 实机验收。基础 Markdown 与相对链接检查、需求正文及实现状态的一致性检查、Cargo 格式检查及差异检查通过。

新笔记结构的工程验证：受影响五包的 `cargo clippy --all-targets --all-features --locked --offline -- -D warnings`、`cargo fmt --all --check` 和应用 `cargo build -p locus --locked --offline` 通过。MCP HTTP 回归需要本机临时端口，在允许本机监听的环境执行通过；没有改变服务部署或用户配置。能力覆盖和迁移后的库校验见文档库专题；当前主机为 macOS，Windows 运行体验待验证。

## 多层元数据与依赖基线

自定义 YAML 只作为当前笔记的数据保留，不解析父级继承或网站、GitHub、项目目录动作。标准 MCP 发现返回固定公开字段，完整接口按原文返回自定义元数据，不生成派生快捷入口。

验证（部分验证）：在 macOS 工作区执行 `cargo test --manifest-path source/Cargo.toml -p locus-library -p locus-mcp -p locus-workspace --locked --offline -- --test-threads=1`，44 项文档库、16 项 MCP 与 41 项工作区测试通过。回归覆盖自定义元数据原文保留、标准发现结果不提升旧字段、完整接口不生成继承结果，以及已有导航、菜单和 YAML 呈现行为。应用的 `assets::tests::application_assets_include_toolbar_and_component_icons` 通过，确认剩余下载与通用组件图标可读取。四包全目标、全功能严格 Clippy、格式与差异检查通过；实际标题栏与命令面板体验待用户验证。

头部元数据由 `locus-workspace/src/workspace/note_metadata.rs` 组合 GPUI Kit DescriptionList。组件负责属性布局，应用按 YAML 解析后的对象与数组递归生成嵌套行；每层根据实际显示的界面字体、字重和窗口 rem 测量 key，列宽向上取整并保持 key 单行，不设比例上限；属性单元格上下留白为 0.125 rem，沿用无边框、无标签背景的样式。

直接依赖基线为 GPUI Kit 0.6.6、GPUI HTTP 与 reqwest 客户端 0.3.6、base64 0.23 和 sha2 0.11；其他直接与传递依赖的具体版本以 `source/Cargo.lock` 为准。GPUI Base、Component、Assets 均解析为 0.6.6，GPUI Pre 为 0.3.6，rmcp 为 3.4.0，serde-saphyr 为 1.3.0。Windows API 绑定保留既有 0.58 约束。

sha2 0.11 不再提供旧的 std::io::Write 和摘要 LowerHex 接口。内容校验通过 `locus-library::content_version_from_reader` 分块读取并更新摘要，统一返回字节数与小写十六进制 SHA-256；文件版本与 MCP 上传复用该入口。MCP 包不再直接依赖 sha2。固定空内容、abc 哈希及跨缓冲区文件回归用于核对格式与流式一致性，不改变已有内容版本契约。

依赖升级验证（部分验证）：全工作区全功能测试 261 项通过；针对当前 ServerConfig API 与哈希测试，文档库 46 项与 MCP 16 项测试通过。全工作区全目标全功能严格 Clippy、格式和差异检查，以及 `cargo build -p locus --locked --offline` 通过。固定哈希向量与流式文件结果一致，MCP HTTP 内容传输通过，工作区覆盖嵌套 YAML 和混合数组。该记录不包含真实界面或 Windows 专属分支验证；长属性层级、图标、编辑器打开及视觉效果待用户验证。构建仍提示既有 block 0.1.6 的未来 Rust 兼容警告。
