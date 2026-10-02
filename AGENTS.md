# Locus

开始修改前完整阅读 [ARCHITECTURE.md](ARCHITECTURE.md)，再阅读其专题表中与任务有关的文档。可见界面的约定由[桌面工作区](docs/architecture/workspace.md#界面与状态约定)维护。

Locus 是 Rust 与 GPUI Kit 构建的本地 Markdown 工作台。文档树拥有内容事实；工作区拥有呈现与编排；设置实体拥有运行时偏好；共享壳拥有窗口与通用布局；外部终端拥有 Agent 会话。

## 开发约定

- Rust Cargo workspace、代码、资源、图标和打包配置位于 `source/`。Bun 工作区入口为根目录 `package.json`，开发脚本位于根目录 `scripts/`，最终应用产物位于根目录 `dist/`。
- 按能力组织模块，通过公开事件、方法与数据类型协作。功能包不依赖应用二进制，不访问兄弟包内部实现；`oneo-gpui*` 保持产品中立。
- 使用锁定版本的 GPUI Kit 公开 API，先核对真实签名再实现。复用框架组件与共享壳行为，状态由明确的长期所有者持有。
- 自有配置使用 TOML，共享配置服务负责文件 IO。画布 JSON 属于内容数据。用户内容、配置和可重建缓存保持各自归属。
- 持有界面文案的包维护自己的中英文资源，变更语言同步组件 locale、业务 view 和原生菜单。
- 改动机制、文件结构或可见行为时，在同一变更中更新拥有该主题的文档。

## 验证

Cargo 命令在 `source/` 执行。按改动影响运行格式、相关包测试与 Clippy：

```sh
cargo fmt --all --check
cargo test -p <package> --locked
cargo clippy -p <package> --all-targets --all-features --locked -- -D warnings
```

跨包变更扩大到受影响包；全局依赖或共享接口变更按需运行 workspace 检查。默认 `cargo test` 只选择默认应用包，不能代表能力包已测试。MCP HTTP 回归需要本机临时端口，运行要求见对应专题。

文档变更在仓库根执行 `bun run check:docs` 和 `git diff --check`。开发脚本使用 Bun 与 TypeScript，放在根目录 `scripts/`，命令由 `package.json` 统一提供；新增 JavaScript/TypeScript 子包时按实际路径登记 `workspaces`。Node 画布测试也从仓库根运行，命令见画布专题。

报告实际执行的检查、覆盖平台与未验证事项。涉及窗口、阅读、终端或设置的界面变化需要目标平台实机确认；编译、模拟工具和无界面测试不能作为视觉、系统授权或原生交互验收。不要自动启动外部终端或 Agent 来替代用户验收。

## 文档约定

描述当前系统的对象、机制、所有权和失败处理，使用现在时。根目录 README 介绍产品与运行方式，ARCHITECTURE 是架构总览并索引 `docs/architecture/` 中每个主题。构建与客户端操作文档直接放在 `docs/`，每个主题只维护一份权威说明，其他入口用链接引用。需求编号留在需求索引，详细行为归专题；计划、评审和执行报告不写入架构文档。

总览每次完整阅读，目标上限为 16 KiB；架构专题每篇上限为 24 KiB。超出时按主题拆分并更新总览索引，保留事实。检查器还校验相对链接与标题锚点，以及专题表和实际文档的一致性。

`source/resources/docs/` 是随软件发布、由 MCP 编译嵌入的英文协议原文。项目文档链接同一源文件，避免复制出第二份协议。
