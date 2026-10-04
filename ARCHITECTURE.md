# Locus architecture

This document is the overview of the current system. Read it in full before changing code, then select the relevant details from [Subjects](#subjects). See [README.md](README.md) for the product introduction and usage. The desktop workspace subject owns visible interface conventions.

Locus is a local-first Markdown knowledge workspace with a native desktop interface built with Rust and GPUI Kit. Users read, organize, and review local notes while external editors and Agents operate on the same files. The app provides read-only MCP file discovery; bundled skills, installed through Settings, define its usage.

## Core constraints

1. **The document tree owns content facts.** Each note is represented by `index.md` in its directory and owns `assets/`, `canvas/`, and child notes. Directories without a body can organize descendants.
2. **Derived state is rebuildable.** Navigation, search, relationships, Git status, and image caches derive from real files. Configuration and caches cannot become content authorities.
3. **Ownership is explicit.** The workspace orchestrates selection and presentation, the settings entity owns runtime preferences, the shared shell owns windows and generic layout, the library owns the file model and safety boundaries, and the MCP controller owns service lifecycle.
4. **MCP discovers real files.** `/mcp` returns original paths through read-only tools. External clients own file operations, model calls, execution permissions, and sessions. The Locus skill defines usage rules.
5. **The current note owns its canvas.** HTML/JavaScript or JSON template calls run in the macOS system WebView. The workspace manages creation and release; the canvas package manages pages, resources, and export.
6. **Generic capabilities remain product-neutral.** `oneo-gpui*` accepts neutral configuration, labels, data, and events. Locus product models, preferences, and localization stay in product packages.

## Processes and data flow

```text
Locus native desktop process
├─ GPUI / OS event loop
│  └─ Main window: document navigation + current note / inline settings
│     └─ macOS WebKit: current note canvas
└─ Optional MCP thread: Tokio + Axum + rmcp
   └─ /mcp: read-only file discovery

Local document tree ← desktop app / MCP / external editors and Agents
External terminal ← workspace launches the selected Agent on demand
```

Startup loads preferences and interface state, resolves appearance and language, and creates the workspace. Library scanning builds navigation and Git projections and watches files recursively. Selecting a note reads its complete body. Notes with a canvas display it below the header; other notes display Markdown.

File events are coalesced before rebuilding projections in the background. Changes relevant to the current note reload its body. Read results are isolated by library path and generation so late tasks cannot overwrite a newer selection. Adding or removing a canvas entry point updates page mode; already loaded pages read new source after manual refresh or reopening.

MCP discovers the same library on each call. Clients maintain content with their own filesystem tools. The workspace maps settings to explicit service configuration and stops the old service before starting a replacement. Agents launch at the library root captured when triggered. The workspace captures only the terminal, Agent, shell, and directory and hands off launching in the background. External terminals own sessions, tabs, PTYs, and process lifecycle. See [Library terminal](docs/architecture/terminal.md) for launch behavior and failure boundaries.

## Module boundaries

Source is a Rust 2024 Cargo workspace with `locus` as its default package. The direct GPUI Kit dependency baseline comes from `source/Cargo.toml`; resolved versions are fixed by `source/Cargo.lock`. Applications and shared components in one process use the same GPUI type instance.

| Package | Responsibility |
| --- | --- |
| `locus` | Product initialization, app startup, menus, and workspace assembly |
| `locus-workspace` | Navigation, reading, canvas orchestration, terminal handoff, interface state, and product actions |
| `locus-settings` | Preferences entity, settings pages, archive restoration, and settings events |
| `locus-preferences` | Product fields, defaults, validation, storage names, and option labels |
| `locus-library` | File model, scanning, reading and writing, watching, versions, links, and Git projections |
| `locus-mcp` | Read-only discovery tools, transport sessions, listening, and shutdown control |
| `locus-canvas` | WebView host, resource protocol, page lifecycle, themes, and export |
| `locus-ui` | Markdown extensions, icons and image caches, and platform file and terminal launching |
| `oneo-gpui` | App shell, windows, layout, appearance controls, and TOML configuration service |
| `oneo-gpui-webview` | Native macOS page composition, clipping, visibility, input, and focus |
| `oneo-gpui-graph` / `oneo-gpui-terminal` | Independent generic components and examples, currently unused by Locus |
| `oneo-tool-packer` | Independent build and app assembly CLI |

The main product dependency chain is `locus → locus-workspace → locus-settings`. The workspace also consumes public interfaces from canvas, UI, preferences, library, and MCP. Settings consumes UI, preferences, library, and MCP. Canvas consumes the library, shared language, and macOS WebView. UI consumes the library and preferences. The library is MCP's only product-domain dependency and does not depend on GUI code. Feature packages do not depend back on the app binary.

## Storage

| Location | Owner and contents |
| --- | --- |
| User-selected library | Markdown, frontmatter, attachments, canvases, and directory hierarchy |
| `~/.oneo/locus/settings.toml` | User preferences, including library, editor, export directory, appearance, terminal, Agent shell, and MCP |
| `~/.oneo/locus/state.toml` | Window restoration, current selection, and sidebar state |
| `~/.oneo/locus/sidebar-order.toml` | Sibling order by library and parent path |
| `~/.oneo/locus/thumbnails/`, `link-icons/` | Rebuildable image caches |

The configuration service reads TOML and saves atomically; runtime entities own configuration changes. Note operations use safe paths and content-version checks. External editors participate in optimistic conflict checking. Operations spanning multiple files do not promise transactional rollback after a crash.

## Subjects

Each subject owns current mechanisms, boundaries, and verification methods. The overview is limited to 16 KiB and each architecture subject to 24 KiB. See [AGENTS.md](AGENTS.md#verification) for documentation verification requirements.

| Document | Read before changing |
| --- | --- |
| [Library](docs/architecture/library.md) | File structure, addresses, scanning, watching, Git, content operations, and safety boundaries |
| [Desktop workspace](docs/architecture/workspace.md) | Tree navigation, reading and YAML, history, menus, commands, and focus |
| [Canvas](docs/architecture/canvas.md) | HTML/JSON, templates, resources, page lifecycle, composition, and export |
| [Agent integration](docs/architecture/mcp.md) | MCP discovery, skill installation, Host validation, and service lifecycle |
| [Settings and persistence](docs/architecture/settings.md) | Preference ownership, configuration I/O, appearance, language, and save scheduling |
| [Shared app shell](docs/architecture/shell.md) | Initialization, windows, layout, debug FPS, and platform behavior |
| [Library terminal](docs/architecture/terminal.md) | Terminal selection, commands, path quoting, handoff, and failure handling |
| [WebView integration](docs/architecture/webview.md) | Generic native component construction, visibility, input, and composition limits |
| [App packaging](docs/architecture/packaging.md) | CLI, targets, icons, resources, signing, runtime libraries, and output replacement |

## Repository layout

```text
AGENTS.md               Development and documentation maintenance rules
ARCHITECTURE.md         System overview and subject index
README.md               Default English product introduction and quick start
README.zh-CN.md         Simplified Chinese translation with language switching
CHANGELOG.md            Pending user-visible changes and confirmed releases
LICENSE                 MIT license
docs/architecture/      Current system description, one document per subject
docs/build.md           Running, building, and app delivery
docs/mcp.md             Client connections and MCP usage
source/Cargo.toml       Workspace and direct dependency baseline
source/Cargo.lock       Resolved versions
source/crates/          Product and generic Rust packages
source/resources/skills/ Bundled skills installed through symlinks on demand
source/packer.toml      App identity and packaging resources
.vscode/                Development and build tasks
dist/                   App outputs, ignored by Git
```

The Cargo workspace manages `source/crates/`. There are currently no JavaScript/TypeScript subpackages or root Bun workspace manifest. When introducing subpackages, register their actual paths in the new workspace manifest. The app version remains in `source/Cargo.toml`.

## Verification and known limitations

See [AGENTS.md](AGENTS.md) for routine checks and [Running and building](docs/build.md) for platform build environments. Historical check records do not establish that the current commit has been revalidated.

Native canvases currently support macOS. Windows builds and native behavior require Windows tooling and hardware verification; macOS compilation is not a substitute. The workspace subject records known clipping issues with mixed Chinese text and table text. Shutdown saving is constrained by GPUI's wait limit; see the settings subject for details.
