# Locus

Locus is a local-first Markdown notebook written in Go with Onevra and MyGo. Read [ARCHITECTURE.md](ARCHITECTURE.md) in full before coding, then the relevant Subjects. Go commands run in `source/`.

## Principles

- The note files own content facts; caches and projections never overwrite them. The workspace owns presentation and orchestration.
- Onevra is the shared component library. Prefer its components and shells; add reusable UI capabilities there, not as wrappers or workarounds in Locus.
- Use MyGo primitives directly, including its WebView and Markdown view; no parallel rendering, input, theme or native bridge. Do not install or invoke Rust tooling.
- Frame builders never perform I/O or start processes. Scans, reads, image decoding and canvas preparation run off the UI thread.
- Agent CLIs own Agent behavior. MCP stays read-only, and the built-in terminal starts a shell or Agent only when the user asks.
- Tests use temporary libraries and storage, never the user's library or the real `~/.locus/` directory.
- Write code comments and all documentation in English. Only `README.zh-CN.md` is Chinese; locale catalogs and test fixtures are product content.

## Working with the user

- Never create sub-Agents or Git worktrees; work directly in the current checkout.
- Onevra and MyGo are consumed as the published versions in `source/go.mod`; never copy their code or add a `replace` directive without the user's agreement. Discuss any change they need with the user first.
- Commit only when authorized. Never push, tag or publish without explicit authorization.
- An explicit request to publish a named version authorizes the [Release notes](docs/build.md#release-notes) steps: version bump, changelog, release commit, tag and push. Preparing a release or editing documents does not.
- UI changes need native review on the target platform. Report what you could not verify, and never launch external terminals or Agents as a substitute.

## Verification

- While iterating, run `gofmt -l cmd internal` and `GOWORK=off go test` only for the affected packages, narrowed with `-run` where it helps.
- Before handing off, run [Development checks](docs/build.md#development-checks) once and `git diff --check` from the repository root. Do not rerun them when nothing changed since they passed.
- Leave the opt-in native tests to the user unless asked.
- Add tests for new behavior and fixed bugs, extending an existing test where one fits; not for styling, copy, layout tweaks or behavior-preserving refactors.

## Documentation

- Update a document only when a fact its reader needs changes. README is onboarding, ARCHITECTURE.md is boundaries and rationale, each subject owns one mechanism, CHANGELOG.md is user-facing release notes.
- Avoid secondary labels and descriptions that restate a heading or value; let the layout explain.
- `source/resources/skills/locus/` guides Agents writing notes. Update it when the note format or MCP tools change.
- Keep external development skills and private tooling paths out of project documentation.
- Keep budgets: AGENTS.md 4 KiB, each README 8 KiB, ARCHITECTURE.md 12 KiB, each document under `docs/` 16 KiB. Split an oversized subject and update Subjects.
- CHANGELOG.md records what a user notices or must do: one short clause per outcome, in the product's words with references such as `(#12 by @alice)`, under `## [Unreleased]`.
- Order entries as `Breaking:`, Add, other, Fix; merge related entries instead of appending task histories. Omit refactors, tests, styling, copy tweaks and internal work.
- When you edit a document, trim stale, duplicated or overly detailed text.
- Keep `README.md` and `README.zh-CN.md` equivalent, with reciprocal language links; update both in the same change.
- Keep paragraphs to four short sentences and list items to two lines; use lists and tables for steps and mappings.
- `CLAUDE.md` is a relative symlink to `AGENTS.md`; edit `AGENTS.md`.
- Keep documentation on the code branch and reconcile it during merges.
