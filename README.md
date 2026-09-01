# Locus

Locus 是本地优先的私有知识与 Agent 工作台。它让用户和 Agent 围绕同一套本地 Markdown 文件工作，使知识阅读、任务发起、执行观察和结果审阅形成连续流程。

Markdown 文档树始终是知识内容的唯一事实来源。Locus 不自行实现 Agent loop、模型调用或通用工具系统；提示词、上下文、工具、权限、认证与执行由各 Agent runtime 负责。

## 实现

Locus 使用 Rust 和 GPUI 构建原生桌面应用。当前基础应用会打开一个 1180 × 760 的原生窗口。

直接运行应用：

```bash
cargo run
```

构建并签名 macOS 应用包：

```bash
./scripts/bundle.sh debug
./scripts/bundle.sh release
```

应用包分别输出到 `target/debug/Locus Debug.app` 与 `target/release/Locus.app`。默认使用 ad-hoc 签名；需要 Developer ID 签名时设置 `LOCUS_CODESIGN_IDENTITY`。

构建前需要安装最新版稳定 Rust、Xcode 和 Xcode Command Line Tools。

## 项目文档

产品定义、设计语义、架构和模块边界以 `locus://projects/locus` 下的项目笔记为准。
