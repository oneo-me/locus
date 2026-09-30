# 运行与构建

需要 Rust 工具链及对应平台的构建环境：

- macOS：Xcode 与 Command Line Tools，最低系统版本为 macOS 13。
- Windows：Visual Studio C++ 工具链与 Windows SDK。

代码、Cargo 工作区、资源、图标和打包配置统一位于 `source/`；最终应用产物统一位于根目录 `dist/`，由 Git 忽略，不按平台或架构划分子目录。根目录保留 `.vscode/`、`dist/`、`docs/`、`source/`、`.gitignore`、`AGENTS.md` 与 `README.md`（Git 自身的 `.git/` 除外）。

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

构建配置位于 [source/packer.toml](../../source/packer.toml)。默认构建当前平台的 Release 版本，产物保存到 `dist/`：macOS 为 `dist/Locus.app`，Windows 为包含可执行文件与运行依赖的 `dist/Locus/`。`output-directory = "../dist"` 相对于打包配置所在的源码目录解析；相同名称的产物由后续成功打包替换。

VS Code 的 `dev` 和 `build` 任务均在 `source/` 执行；`build` 通过 Cargo 运行仓库内的打包工具。需要单独安装工具时，执行 `cargo install --path source/crates/oneo-tool-packer --locked`，随后在根目录运行 `oneo-tool-packer --project source`。

编译目录遵循 Cargo 的全局或项目 `build.target-dir` 配置及 `CARGO_TARGET_DIR` 环境变量。打包工具自动从 Cargo 构建消息读取实际可执行文件路径，支持多个项目共用编译目录，无须在 `packer.toml` 中配置路径；最终应用包仍保存到项目 `dist/`。配置解析与优先级遵循 [Cargo 配置规则](https://doc.rust-lang.org/cargo/reference/config.html#buildtarget-dir)。

macOS 默认使用 ad-hoc 签名。构建 Windows 版本需要在 Windows 环境中运行。
