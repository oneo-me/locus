# Locus

Locus 是本地优先的私有知识与 Agent 工作台。它让用户和 Agent 围绕同一套本地 Markdown 文件工作，使知识阅读、任务发起、执行观察和结果审阅形成连续流程。

Markdown 文档树始终是知识内容的唯一事实来源。Locus 不自行实现 Agent loop、模型调用或通用工具系统；提示词、上下文、工具、权限、认证与执行由各 Agent runtime 负责。

## 实现

Locus 使用 Rust 和 GPUI 构建原生桌面应用。应用初始窗口尺寸为 980 × 820，启动时恢复已保存的可用窗口位置与尺寸。

直接运行应用：

```bash
cargo run
```

应用入口位于 `crates/locus`，产品能力按 crate 开发，边界见 [工程定义](locus://projects/locus/engineering)。检查和测试整个 workspace：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## 打包

先安装 workspace 中的公共工具：

```sh
cargo install --path crates/oneo-tool-packer --locked
```

确保 Cargo 的 `bin` 目录位于 PATH 中。VSCode 的 `build` 任务与以下命令使用同一个打包入口：

```sh
oneo-tool-packer
oneo-tool-packer --target aarch64-apple-darwin
oneo-tool-packer --target x86_64-pc-windows-msvc
```

工具只构建 Release；省略 `--target` 时使用当前 Rust host。macOS 在 Mac 上生成 `.app` 目录，Windows 在 Visual Studio C++ 开发环境中生成包含 `.exe` 和运行库的目录，不生成安装程序。产物固定保存到项目根 `.build/<target>/`，`.build/` 已由 Git 忽略；平台与架构必须具备对应工具链。

项目配置见 `packer.toml`，macOS 图标固定为根目录 `icon.icon/`，Info.plist 由工具生成；Windows 固定使用根目录 `icon.ico` 并嵌入 EXE。macOS 需要 Xcode 与 Command Line Tools，默认 ad-hoc 签名；Developer ID 签名使用 `--codesign-identity` 或 `ONEO_PACKER_CODESIGN_IDENTITY`。macOS Release 主程序的禁用符号导出由工具统一处理；普通 `cargo build --release` 只应用项目 Cargo profile，不自动禁用导出。工具的配置和 Windows 运行库收集规则见 [Oneo Tool Packer](locus://projects/locus/areas/application-packaging)。

## 项目文档

产品定义、设计语义、架构和模块边界以 `locus://projects/locus` 下的项目笔记为准。
