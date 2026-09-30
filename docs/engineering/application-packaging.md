# 应用打包

本篇“项目根”指 `packer.toml` 所在的工程目录；Locus 中为 `source/`，仓库根目录通过 `--project source` 指定它。

验证记录只覆盖已注明的测试范围与反馈对象；未注明适用提交的既有记录，其代码版本未知，不代表当前版本重新通过验证。

## 技术与职责

`oneo-tool-packer` 位于 Locus 的 `source/crates/oneo-tool-packer`，是 Rust 2024 CLI crate，不依赖 GPUI；版本、edition 和依赖继承 workspace，使用 `source/Cargo.lock`。使用 Serde、TOML、JSON 与 plist 处理标准元数据，anyhow 保留错误上下文，tempfile 管理暂存目录。Cargo.lock 固定依赖；不自行实现 TOML、JSON 或 plist 解析器。

`cli` 负责参数和目标解析；`config` 负责项目配置校验及应用身份；`cargo` 获取元数据、调用编译并消费 Cargo JSON 产物消息；`bundle` 负责目录、资源、macOS 图标签名和输出替换；`windows` 负责 PE 架构和 DLL 依赖收集；`windows_env` 负责发现并加载 Visual Studio C++ 环境。项目路径通过 dunce 规范化，在可表达为普通 Windows 路径时避免 verbatim 前缀影响依赖构建脚本的路径拼接。主入口只编排流程。

## 输入与构建

VSCode `build` 任务以 `shell` 在 `${workspaceFolder}/source` 执行 `cargo run --locked -p oneo-tool-packer --`，直接使用仓库内的工具；`dev` 任务也在该源码目录执行。项目任务不维护独立 PATH 或工具绝对路径，shell 环境必须能找到 Cargo 和 rustc。Windows 可直接从普通 PowerShell、CMD 或 VSCode 任务运行，打包工具自动加载所需 C++ 环境。完整任务配置以 `.vscode/tasks.json` 为准。

需要独立安装时，在 Locus 根目录通过 `cargo install --path source/crates/oneo-tool-packer --locked` 安装，Cargo bin 目录须位于 PATH；安装后在仓库根执行 `oneo-tool-packer --project source`。项目根 `packer.toml` 必填 `package`、`bin`、`name`、`identifier`；图标固定为项目根 `icon.icon/`，Info.plist 动态生成。可选 `output-directory` 指定最终产物目录，默认为项目目录内的 `dist`；相对路径基于 `--project` 指定的目录解析。Locus 设置为 `../dist`，直接输出到源码仓库根目录的 `dist/`。可选顶层 `resources` 列表声明所有平台共用的外置资源文件或目录。可选 `[macos].resources` 与 `[windows].resources` 列表声明仅对应平台使用的额外资源；`[windows]` 使用 `runtime-directories` 声明 DLL 搜索目录、`runtime-files` 声明运行时动态加载的 DLL。Locus 的配置位于 `source/packer.toml`，项目目录为 `source/`，图标和资源也位于该目录，支持字段的完整定义见 `source/crates/oneo-tool-packer/src/config.rs`；配置未知字段直接报错。

`--project` 默认当前目录；工具只构建 Release，不提供 profile 切换参数；`--target` 默认项目目录下 `rustc -vV` 的 host，覆盖 Cargo 默认目标。支持 `aarch64-apple-darwin`、`x86_64-apple-darwin`、`x86_64-pc-windows-msvc` 和 `aarch64-pc-windows-msvc`；`macos`、`windows` 别名保留 host 架构。其他目标明确拒绝。

应用包必须是 workspace 成员，且包含所选二进制。版本通过 `cargo metadata --no-deps --format-version 1` 获取，主程序真实路径通过 `cargo rustc` 的 JSON 产物消息识别，不根据目录名或文本扫描猜测版本。编译目录由 Cargo 解析全局或项目 `build.target-dir` 及 `CARGO_TARGET_DIR` 环境变量，遵循 Cargo 的相对路径与优先级规则；支持多个项目共用编译目录，无须在 `packer.toml` 中重复配置。构建固定使用 Cargo `--release --locked`，显式选择 package、bin 和 target。项目的 Cargo profile 继续定义 strip、LTO、优化等级和 panic 策略。

macOS Release 只向最终主程序编译传入 `-C link-arg=-Wl,-no_exported_symbols`，参数不进入依赖或 Windows。普通 `cargo build --release` 不自动执行该打包策略。`--dry-run` 仅读取配置与元数据，展示命令和目的地，不要求目标系统与 host 一致，也不构建或签名。

Windows MSVC 构建通过 Cargo 目标级 `--config` 设置 `target.<triple>.rustflags=["-C","target-feature=+crt-static"]`，将默认静态 CRT 策略应用于完整目标依赖树，而非只传给最终二进制。显式设置的 `RUSTFLAGS`、`CARGO_ENCODED_RUSTFLAGS` 或目标级 rustflags 环境变量遵循 Cargo 优先级，可能覆盖该默认值。仍按实际 EXE/DLL 依赖收集动态运行库，不假定所有原生库都能静态链接。

## 平台与产物

产物保存到配置的输出目录下的 `<应用名>[.app]`，不按平台或架构划分子目录，使用配置中的正式名称与 Bundle Identifier。Locus 项目 Git 忽略 `/dist/`；Cargo 编译缓存仍使用原 target 目录，配置 target-dir 不改变最终产物位置。显式 target 编译与普通 Cargo 默认 host 编译使用不同缓存，首次构建会编译该 target 的依赖。资源路径相对于项目根或为绝对路径；字符串条目以原文件名复制；`{ source = "tools/macos", destination = "tools" }` 条目可指定输出名称，名称须为单个合法文件名，拒绝目录层级与路径穿越。目录递归复制，重名、非普通文件与软链接报错。顶层 `resources` 在 macOS 复制到 `.app/Contents/Resources`，在 Windows 复制到输出目录的 `resources`；各平台专属资源随后复制到该平台的同一资源目录，其他平台的资源不会复制。macOS 资源复制在签名前完成，缺失或冲突阻止发布并保留旧产物。

macOS 要求 Mac、Xcode 与 Command Line Tools，最低系统版本统一为 13.0。项目根必须提供 `icon.icon/`，`actool --app-icon icon` 编译图标，工具动态生成 Info.plist 的应用名称、标识、可执行文件名、版本、最低系统版本和原生应用字段，合并图标元数据。项目不维护手写 Info.plist；macOS 配置段只声明平台专属资源。默认 ad-hoc 签名；`--codesign-identity` 优先于 `ONEO_PACKER_CODESIGN_IDENTITY`，Developer ID 使用 hardened runtime 与时间戳。签名后严格验证，不执行公证。

Windows 要求 Windows、Visual Studio C++ 工具链和 Windows SDK，包括 linker、`rc` 和 `dumpbin`；其他架构还需要相应 Rust 标准库和本地依赖工具链。当前环境缺少可用的 `dumpbin` 或 `rc` 时，工具通过 `vswhere` 查找安装了 C++ 工具的 Visual Studio，按目标架构调用 `VsDevCmd.bat`，解析其 UTF-16 环境并以该环境重新启动打包子进程。环境仅作用于打包进程及其子进程，不修改系统环境变量；加载后仍缺工具则报错，不重复重启。未安装工具链时提示通过 Visual Studio Installer 安装“使用 C++ 的桌面开发”和 Windows SDK。通过 `dumpbin /dependents` 递归获取静态与延迟导入，搜索产物目录、`runtime-directories`、`VCToolsRedistDir/<arch>/*.CRT`。VC 运行库从可分发目录收集，不从开发机 System32 取副本；API set 和系统目录中的系统库不复制。每个 EXE/DLL 检查 PE 架构，缺失依赖中止。通过 LoadLibrary 动态加载的非系统 DLL 必须声明到 `runtime-files`，也纳入依赖检查。项目根必须提供 `icon.ico`，工具使用 Windows SDK `rc` 编译临时图标资源并只传给所选主程序链接；图标缺失或编译失败时在 Cargo 构建前中止。工具不签名 Windows EXE。

工具不执行跨操作系统构建、不安装工具链、不生成安装程序；不同架构的构建能力还取决于应用自身依赖。相关接口依据：[Cargo rustc](https://doc.rust-lang.org/cargo/commands/cargo-rustc.html)、[DUMPBIN /DEPENDENTS](https://learn.microsoft.com/en-us/cpp/build/reference/dependents?view=msvc-170)、[Visual C++ 运行库](https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files?view=msvc-170)。

## 失败与验证

应用在同一输出文件系统的临时目录完整组装、检查和签名。发布目录用文件锁互斥，成功后整体替换；替换失败恢复旧目录，恢复也失败时保留暂存目录并报告恢复位置。构建、签名、资源或 DLL 检查失败不会删除旧产物。

检查使用 `cargo fmt --all --check`、`cargo clippy -p oneo-tool-packer --all-targets --locked -- -D warnings` 和 `cargo test -p oneo-tool-packer --all-targets --locked`。测试覆盖目标解析、专属链接参数、产物选择、DLL 递归与架构、资源碰撞、发布恢复，以及 Unix 上模拟编译工具的 CLI 契约。共享编译目录在 macOS 上通过真实 Cargo Release 构建验证全局相对路径配置、项目相对路径配置及环境变量三种来源，断言实际可执行文件路径、项目内不生成默认 target 目录，且打包目的地仍为项目 `dist/`。既有源码目录布局的验证记录包含 `cargo fmt --all --check`、打包工具严格 Clippy、19 项单元测试与 4 项 CLI 测试，以及 `cargo check --workspace --all-targets --all-features --locked --offline`。根目录构建命令的 `--dry-run` 确认读取 `source/packer.toml` 并输出至 `dist/Locus.app`；输出路径测试覆盖四种支持的目标，不包含平台或架构子目录。此项不替代完整应用打包或 Windows 实机构建验收。真实 macOS 包需检查 plist、导出符号与签名；Windows 原生构建、运行库完整性和启动需在 Windows 环境验证，模拟测试不作为 Windows 实机验收。

Windows 实机验证（用户确认）：提交 `c93c3ba` 的 Windows 版本验证通过，覆盖对应打包与启动修复。该反馈不替代不同架构、缺失工具链或显式 rustflags 覆盖场景的专项测试。

平台资源自动化验证（覆盖当时的平台资源实现）：打包工具 15 项单元测试与 4 项 CLI 测试通过；测试验证共用模板存在，macOS 与 Windows 工具统一映射到 tools 且互不混入，目的名称拒绝越界和碰撞。Clippy 检查通过。
