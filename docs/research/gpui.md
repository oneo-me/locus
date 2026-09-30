# GPUI 原生重写

## 问题与结论

Locus 选择 Rust 与 GPUI 建立原生桌面实现。GPUI 可以承接应用壳、Markdown 排版、附件、导航、原生交互和自绘表面，并让文档库、终端与 MCP 保持在同一 Rust 工程边界中。

## 依赖与组件基线

原生应用通过 crates.io 的 `gpui-kit` 统一使用 GPUI、操作系统后端、组件与资源。Kit 负责匹配框架发布包，`source/Cargo.lock` 锁定版本，避免同一进程出现多个不兼容的 GPUI 类型实例。具体依赖约束见[工程](../engineering.md)。

应用在事件循环入口初始化 GPUI Kit 组件层、注册组件资源，并以组件根容器包裹产品实体。主题、通用图标、滚动条与控件优先使用组件库；窗口和面板行为属于共享应用壳，不因控件实现变化而转移所有权。

## 窗口材质与布局

工作区使用 GPUI 横向布局，左栏受约束，主内容接收剩余空间。macOS 通过 GPUI 创建的整窗 `NSVisualEffectView` 使用系统 Sidebar 材质；组件根保持透明，Locus 在其上提供中性染层，左栏透明，主内容使用不透明表面保证阅读稳定。

GPUI 当前绘制管线没有可直接用于局部内容背板的稳定 backdrop blur。AppKit 原生效果视图也不能插入同一 GPUI Metal 表面的两个绘制层之间。因此菜单、浮层、主内容不依赖局部模糊；只有 GPUI 上游提供稳定原语，或项目明确接受维护渲染器分支时才重新评估。

面板折叠只改变 GPUI 可见宽度并裁切完整内容，内容布局由共享壳层协调。系统启用减少动态效果时不创建过渡。平台专属材质、窗口与输入处理限制在共享平台边界中。

## 画布边界

允许 HTML、CSS 和 JavaScript 在隔离环境中运行的文档画布不等同于 GPUI 低层 canvas。实现这类内容模型需要独立 WebView；纯 GPUI 实现则需要明确的原生场景模型，不能通过兼容层同时维护两套路径。

Locus 在笔记页面下方采用独立 WebView 承载 HTML 画布，Markdown 正文继续由 GPUI 渲染。固定分区使网页不参与正文滚动与自动高度计算；主线使用独立原生桥接组件保持网页运行，并协调 GPUI 浮层覆盖、输入命中与焦点。画布实现由 `locus-canvas::CanvasView` 持有 WebView，工作区管理所属笔记、分区与生命周期；详细运行和资源边界见[工程](../engineering.md)与[文档库](../topics/document-library.md)，交互与实机验收要求见[桌面工作区](../topics/workspace.md)。

## 参考

- [GPUI README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md)
- [GPUI Kit Getting Started](https://gpui-kit.com/zh-CN/docs/getting-started)
- [GPUI Kit Design Guides](https://gpui-kit.com/zh-CN/docs/design-guides)
- [GPUI canvas API](https://docs.rs/gpui/latest/gpui/fn.canvas.html)
- [Zed WebView 跟踪](https://github.com/zed-industries/zed/issues/21208)
- Waku 的 `src/app/render.rs`：已有源码调研依据；未记录可公开核验的仓库地址与版本。

已采用的系统结构与项目约束见[工程](../engineering.md)。
