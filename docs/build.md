# 运行与构建

需要 Rust 工具链及对应平台的构建环境：

- macOS：Xcode 与 Command Line Tools，最低系统版本为 macOS 13。
- Windows：Visual Studio C++ 工具链与 Windows SDK。

代码、Cargo 工作区、资源、图标和打包配置统一位于 `source/`；最终应用产物统一位于根目录 `dist/`，由 Git 忽略，不按平台或架构划分子目录。开发约定见[AGENTS.md](../AGENTS.md)，模块与目录职责见[架构总览](../ARCHITECTURE.md)。

进入源码目录运行：

```sh
cd source
cargo run
```

## 构建应用

应用版本采用 `0.<年份>.<当年发布序号>`，序号从 1 开始，同年递增，跨年重置。版本值统一维护在 `source/Cargo.toml` 的 `workspace.package.version`，打包产物读取该版本；修改时同步 `source/Cargo.lock` 中继承该版本的工作区包。

在项目根目录执行：

```sh
cargo run --manifest-path source/Cargo.toml --locked -p oneo-tool-packer -- --project source
```

构建配置位于 [source/packer.toml](../source/packer.toml)。默认构建当前平台的 Release 版本，产物保存到 `dist/`：macOS 为 `dist/Locus.app`，Windows 为包含可执行文件与运行依赖的 `dist/Locus/`。`output-directory = "../dist"` 相对于打包配置所在的源码目录解析；相同名称的产物由后续成功打包替换。

VS Code 的 `dev` 和 `build` 任务均在 `source/` 执行；`build` 通过 Cargo 运行仓库内的打包工具。需要单独安装工具时，执行 `cargo install --path source/crates/oneo-tool-packer --locked`，随后在根目录运行 `oneo-tool-packer --project source`。

编译目录遵循 Cargo 的全局或项目 `build.target-dir` 配置及 `CARGO_TARGET_DIR` 环境变量。打包工具自动从 Cargo 构建消息读取实际可执行文件路径，支持多个项目共用编译目录，无须在 `packer.toml` 中配置路径；最终应用包仍保存到项目 `dist/`。配置解析与优先级遵循 [Cargo 配置规则](https://doc.rust-lang.org/cargo/reference/config.html#buildtarget-dir)。

macOS 默认使用 ad-hoc 签名。构建 Windows 版本需要在 Windows 环境中运行。

## 发布记录

根目录 [CHANGELOG.md](../CHANGELOG.md) 面向 Locus 桌面用户，记录已完成但尚未发布的变化及已确认发布的版本。当前打包工具从 Cargo 元数据读取版本并组装应用，不读取 Changelog，也不自动生成发布说明；发布记录由维护者同步整理。

日常用户可见变更直接更新 `Unreleased`，不把计划或未完成能力写成发布承诺。发布前核对实际分支及各平台产物，将本次交付的条目移入确认的版本章节，保留顶部 `Unreleased` 和未交付条目。已有发布版本不从当前 Cargo 版本或提交历史推测补写。

确认版本后，按上文规则同步 `source/Cargo.toml` 与 `source/Cargo.lock`，构建并完成目标平台检查。纯文档维护不触发版本递增、打标签或发布。


## 开发检查

Cargo 命令在 `source/` 执行，按改动影响运行格式、相关包测试与 Clippy：

```sh
cargo fmt --all --check
cargo test -p <package> --locked
cargo clippy -p <package> --all-targets --all-features --locked -- -D warnings
```

跨包变更扩大到受影响包，全局依赖或共享接口变更按需运行 workspace 检查。默认 `cargo test` 只选择默认应用包，不能代表能力包已测试。MCP HTTP 回归需要本机临时端口，运行要求见 [Agent 接入](architecture/mcp.md#验证方法)。Node 画布测试也从仓库根运行，命令见[画布](architecture/canvas.md#验证边界)。报告实际检查、覆盖平台与未验证事项；编译和无界面测试不能代替窗口、阅读、终端、设置、系统授权及原生交互的实机验收。

工程使用锁定版本的 GPUI Kit 公开 API，先核对真实签名再实现。模块按能力组织，通过公开事件、方法与数据类型协作；功能包不访问兄弟包内部实现，`oneo-gpui*` 保持产品中立。运行时配置与语言资源归属见[设置与持久化](architecture/settings.md)，界面约定见[桌面工作区](architecture/workspace.md#界面与状态约定)。

新增开发脚本使用 Bun 与 TypeScript，放在根目录 `scripts/`，命令由 `package.json` 提供；新增 JavaScript/TypeScript 子包时登记实际 `workspaces` 路径。文档检查使用已安装的 project-docs 技能脚本，命令见 [AGENTS.md](../AGENTS.md#验证)，项目不维护检查器副本。

检查器自动扫描根目录与 `docs/` 下的 Markdown，检查预算、Subjects、相对链接、标题锚点和 Unreleased 结构。项目文档指向源码或随应用发布的 skill 原文时，检查其目标文件和锚点，但不递归扫描这些资源目录。机制覆盖、文档与实现一致性及发布条目准确性由维护者核对。运行检查前需安装 Bun 与 project-docs 技能。
