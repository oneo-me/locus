![Locus icon](source/resources/icon.png)

# Locus

**English** | [简体中文](README.zh-CN.md)

Locus is a local-first Markdown workspace with a native desktop interface built in Go with Onevra and MyGo. Notes stay ordinary files: read and organize them in Locus, edit them with your preferred editor, and let external Agents discover the same files through read-only MCP.

## Quick start

Requires Go 1.27.1 or later, a native desktop session, and **Onevra's WebView branch with its customized MyGo**. `source/go.mod` explicitly replaces both modules with local checkouts. Adjust those paths as described in [dependency setup](docs/build.md#dependencies).

```sh
cd source
GOWORK=off go run ./cmd/locus
# Optional startup library:
GOWORK=off go run ./cmd/locus -library /path/to/library
```

When no library is selected, a standalone welcome screen prompts you to choose **Open library**. You can also select a library in the Settings dialog. Each note is a directory containing `index.md`; directory nesting supplies the hierarchy. YAML frontmatter provides titles, summaries, and custom properties. Attachments live in `assets/`; canvases live in `canvas/`. Hidden directories and reserved resource directories are not notes.

## Reading and canvases

Browse the native tree or search notes through the command palette. Read Markdown, local images, summaries, and custom YAML. The toolbar copies references, opens notes in an editor, and opens directories. External file changes refresh projections; Git modification hints appear when the library is a repository root.

A note can own an HTML fragment and `canvas/main.js`, or a JSON template invocation. The Onevra WebView displays it inside the workspace. Settings hides and retains the page; leaving releases it. Refresh or reopen after editing canvas source. See the bundled [canvas reference](source/resources/skills/locus/references/canvas.md).

Canvas export creates a ZIP containing the page and resources loaded in the current session. Unpack it and use a local HTTP server when required for modules, fetch, or Workers. Unloaded dependencies are not discovered automatically, and online services still require network access.

## Agents and settings

Settings offers appearance/language choices, editor and export preferences, MCP, shell/Agent selection, and skill installation. Preferences retain existing YAML fields and detect external changes before saving. Sidebar and current-note state are saved on close.

Enable MCP in Settings to expose read-only discovery at `http://127.0.0.1:2301/mcp`. Clients need filesystem access to the returned paths. Local-network mode has no authentication and exposes searchable note content and paths; enable it only on a trusted network. See [MCP access](docs/mcp.md).

Install the bundled Locus skill explicitly into the library or home directory. Installation preserves existing targets and creates an independent copy. Updating that copy requires explicit reinstallation. The titlebar terminal button, left of More actions, opens a 600 × 500 popover showing only the built-in terminal. Choose a shell and Agent in Settings; None opens just the shell. Changes apply the next time the terminal opens. Hiding the popover retains the session, while switching libraries or quitting closes it. When the Agent exits normally or with interrupt status, the terminal and popover close automatically; Ctrl+C only triggers this if it exits the Agent. Failed launches and other errors keep their output visible. Shells and Agent CLIs must be installed separately and launch only on request.

| Action | Shortcut |
| --- | --- |
| Open library | Command/Ctrl+O |
| Search notes/actions | Command/Ctrl+E |
| Refresh current content | Command/Ctrl+R or F5 |
| Settings | Command/Ctrl+, |
| Toggle terminal | Command/Ctrl+backtick |
| Close window / quit | Command/Ctrl+W / Command/Ctrl+Q |

## Development and platform scope

```sh
cd source
GOWORK=off go test ./...
GOWORK=off go vet ./...
GOWORK=off go tool packer run
GOWORK=off go tool packer build
```

Outputs go to `dist/`; the configured packaging target is Apple Silicon macOS. No Rust toolchain is needed. See [Running and building](docs/build.md) for race checks and native smoke tests, and [Architecture](ARCHITECTURE.md) for boundaries and verification.

macOS native Markdown and WebView workflows have automated smoke tests. Windows and Linux need separate acceptance; the experimental WebView backend supports macOS and 64-bit Windows, not Linux. Automatic updates, sidebar reordering, richer attachment/gallery extensions, font controls, and full window-bounds restoration are not implemented. Published downloads at [GitHub Releases](https://github.com/oneo-me/locus/releases/latest) may differ from this development branch; local packaging does not publish them.

Licensed under the [MIT License](LICENSE).
