# Locus

修改前完整阅读 [ARCHITECTURE.md](ARCHITECTURE.md)，再读 Subjects 中相关专题。Locus 是 Rust 与 GPUI Kit 构建的本地 Markdown 工作台；文档树拥有内容事实，工作区负责呈现与编排，外部终端拥有 Agent 会话。

## 文档维护

描述当前机制，每个主题一份权威说明。机制变更同步更新所属专题及受影响的总览、索引和使用说明；用户可见变更同步维护 [CHANGELOG.md](CHANGELOG.md) 的 Unreleased，纯重构、测试和常规文档调整无需发布条目。

文档与代码保持同一分支，合并时协调当前定义与验证依据。总览上限 16 KiB，专题上限 24 KiB；超出时按主题拆分，保留事实并更新 Subjects。

## 验证

完成前按[开发检查](docs/build.md#开发检查)验证受影响的包；界面变化需目标平台实机确认，不自动启动外部终端或 Agent 代替验收。文档变更在仓库根运行技能自带检查器和 `git diff --check`：

```sh
bun run "$HOME/.agents/skills/project-docs/scripts/check-docs.ts" --root .
git diff --check
```
