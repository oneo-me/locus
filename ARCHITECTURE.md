# Locus architecture

Locus is a local-first Markdown workspace implemented in Go with Onevra and MyGo. This overview maps the current implementation; read it before making changes, then read the relevant [Subjects](#subjects). [README](README.md) introduces usage and [Running and building](docs/build.md) owns development commands.

## Core constraints

1. **Files own content facts.** Each note is a directory containing `index.md`, optional `assets/` and `canvas/`, and child-note directories. Organizing directories can have no body. Derived projections are rebuildable.
2. **The library owns file boundaries.** Rooted access, path validation, and bounded reads stay independent of UI code. External editors operate on the same files; caches never overwrite their content.
3. **The workspace owns asynchronous selection.** File scans and reads run in background tasks, with generation checks before results enter the UI thread. Onevra owns generic controls, not product state.
4. **Agents remain external programs.** MCP only discovers real files. Agent CLIs own reads, edits, model calls, and permissions. The workspace hosts explicit terminal sessions through Onevra’s MyGo terminal; no process launches on startup.
5. **A note owns its canvas.** The native browser receives a read-only resource protocol scoped to the selected library. No filesystem-write, shell, or Agent service is bound to page JavaScript.

## Dependencies and process flow

`source/go.mod` and `source/go.sum` own dependencies. Locus uses published Onevra `v0.2026.8` and MyGo `v0.4.8`, pinned with module checksums and no local replacements. MyGo supplies the built-in WebView. See [dependency setup](docs/build.md#dependencies).

```text
Go desktop process
├─ MyGo native event loop
│  └─ Onevra sidebar Shell
│     ├─ Virtual document tree, Markdown, commands, settings
│     └─ MyGo WebView → system browser
├─ Background scans, reads, Git projections and recursive file watcher
├─ Optional MCP HTTP server → read-only discovery
├─ Built-in terminal popover → MyGo PTY and external shell/Agent CLI
└─ Ordered settings writer

Local library ← external editors and Agents
Shell/Agent CLI ← explicit user launch in the built-in terminal
```

The browser uses a registered custom URL scheme, not a local web server. A random session origin routes each canvas to its own rooted file handle. Leaving a canvas closes the browser and releases its origin; the Settings dialog draws over the live browser through MyGo hybrid composition.

## Ownership

| Package | Responsibility |
| --- | --- |
| `cmd/locus` | Native lifecycle, menus, UI composition, settings coordination, history, image loading, toast notifications, and canvas ownership |
| `internal/library` | Hierarchy, metadata, resources, safe addresses, file watching, and Git projections |
| `internal/workspace` | Asynchronous scans, reads, selected body, errors, and stale-result isolation |
| `internal/canvas` | HTML/JSON entry resolution, read-only resource protocol, page bootstrap, CSP, and ZIP export |
| `internal/discovery` | Three read-only MCP tools, transport, request boundary checks, listening, and shutdown |
| `internal/preferences` | YAML preferences/state, atomic replacement, external-change checks, and ordered save worker |
| `internal/terminal` | Shell/Agent choices and validated launch arguments |
| `internal/skills` | Resource-based authoring skill and explicit, non-overwriting installation |
| Onevra | Shell, controls, Markdown, and command palette |
| MyGo | OS event loop, rendering, input, menus, dialogs, browser API and native WebView composition, terminal emulator/PTYs, and desktop integration |

Internal domain packages do not import UI code. The app assembles them through public APIs. Canvas preparation, image decoding, scans, and note reads happen outside frame construction; native browser creation occurs after preparation returns to the UI thread. The app does not copy a native browser bridge into its source.

## Storage

| Location | Contents |
| --- | --- |
| Selected library | Notes, frontmatter, attachments, canvases, and child directories |
| `~/.oneo/locus/settings.yaml` | Library path, appearance, language, editor, shell/Agent, export directory, and MCP configuration |
| `~/.oneo/locus/state.yaml` | Selected note/library, per-library tree expansion, and sidebar width/collapse state |
| Chosen export directory | Non-overwriting canvas ZIP snapshots |
| Library or home `.agents/skills/locus` | Explicitly installed copy of the bundled authoring skill |

Existing recognized YAML fields are read directly; unrelated preference fields survive saves. Malformed settings are not overwritten. Presentation state is not a content authority. Configuration writes are atomic replacements, not a transaction with content or another process.

## Subjects

The overview is limited to 16 KiB and each subject to 24 KiB. Keep each mechanism in its owning subject, with current limitations distinguished from implemented behavior.

| Document | Read before changing |
| --- | --- |
| [Library](docs/architecture/library.md) | Note structure, path/resource boundaries, watching, and Git projections |
| [Desktop workspace](docs/architecture/workspace.md) | UI composition, async state, navigation, reading, native actions, and acceptance |
| [Canvas](docs/architecture/canvas.md) | Native browser lifecycle, resources, HTML/JSON, security policy, and export |
| [Agent integration](docs/architecture/mcp.md) | MCP tools, Host/Origin protection, lifecycle, skills, and built-in terminal sessions |
| [Settings and persistence](docs/architecture/settings.md) | Preference ownership, configuration safety, language, appearance, and shutdown saving |
| [App packaging](docs/architecture/packaging.md) | Go tools, app identity, bundled resources, platform coverage, and release boundary |

## Repository layout

```text
source/go.mod, go.sum    Go module and dependency hashes
source/cmd/locus/        Desktop application and native smoke tests
source/internal/         Product packages and headless tests
source/resources/skills/ Bundled authoring skills
source/packer.yaml       Onevra Go packager configuration
source/resources/icon.* Platform icon assets
.vscode/tasks.json       Go development and build tasks
docs/                    Architecture and usage instructions
dist/                    Ignored app outputs
```

There is one Go module. No Cargo workspace, project-owned native bridge, or alternate implementation is required. Do not install or invoke Rust tooling.

## Verification and current limitations

[Development checks](docs/build.md#development-checks) cover tests, race detection, vet, build, packaging, and opt-in native tests. Native tests exercise Markdown and real WebView initialization, template data, local fetch, DOM interaction, page state across Settings, and release on note departure. They do not establish full pointer/keyboard, accessibility, installation, or cross-platform acceptance. Do not launch external terminals or Agents as a substitute for user acceptance.

Automatic update delivery, sidebar reordering, font controls, and full window-bounds restoration are not implemented. Canvas ZIP export contains the resources loaded in the current session rather than a complete dependency crawler. Windows and Linux require their own verification; native canvases depend on MyGo's platform browser backend.
