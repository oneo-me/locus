# Locus

Read [ARCHITECTURE.md](ARCHITECTURE.md) in full before making changes, then read the relevant subjects listed in Subjects. Locus is a local Markdown workspace built with Rust and GPUI Kit. The document tree owns content facts, the workspace owns presentation and orchestration, and external terminals own Agent sessions.

## Documentation maintenance

Write project documentation and code comments in English, except for explicitly maintained translations such as `README.zh-CN.md`. Keep localized interface strings and multilingual test fixtures in their intended languages. Keep external development skills and private tooling paths out of project documentation; bundled Locus skills are product documentation. Maintain matching content in `README.md` and `README.zh-CN.md`, with reciprocal language links.

Describe current mechanisms, with one authoritative document per subject. Update the owning subject and any affected overview, indexes, and usage instructions alongside mechanism changes. Record user-visible changes under Unreleased in [CHANGELOG.md](CHANGELOG.md); pure refactors, tests, and routine documentation edits do not need release notes.

Keep documentation and code on the same branch. Reconcile current definitions and verification evidence during merges. The overview is limited to 16 KiB and each subject to 24 KiB. Split oversized documents by subject, preserve the facts, and update Subjects.

## Verification

Before finishing, validate affected packages according to [Development checks](docs/build.md#development-checks). Interface changes require confirmation on the target platform. Do not automatically launch external terminals or Agents as a substitute for acceptance testing. For documentation changes, verify size budgets, Subjects coverage, relative links, heading anchors, and changelog structure, then run `git diff --check` from the repository root:

```sh
git diff --check
```
