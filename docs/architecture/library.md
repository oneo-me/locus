# Library

This document owns current behavior, implementation boundaries, and acceptance requirements.

## Purpose

The library lets users and Agents work on the same local Markdown files. It owns the content model, file boundaries, and rebuildable projections, not window layout, user preferences, Agent runtimes, or terminal processes.

## Objects and rules

- A library is a local root directory selected by the user.
- `index.md` in a non-hidden directory defines a note. Directory nesting defines note hierarchy; directories without `index.md` may only organize descendants.
- Titles, icons, summaries, and archive state come from YAML frontmatter. Missing titles may fall back to directory names. Other frontmatter fields are preserved and presented as custom metadata.
- `assets/` holds attachments; the dedicated icon `assets/icon.png` does not count as an attachment. `canvas/` holds the canvas entry point and resources. The reserved directory names `assets` and `canvas` cannot be notes or organizing directories, and scanning does not identify their contents as child notes. Note paths exclude symlinks, hidden directories, and `node_modules`.
- `locus://<library-relative path>` is the canonical library address. Spaces, percent signs, and hashes may be encoded. Navigation, MCP, and canvas components decode once and reject encoded separators and parent segments. Standard Markdown links reference body content; target resolution must stay within library boundaries.
- Explicitly archived notes and their descendants are excluded from ordinary navigation. Settings can still discover explicitly archived entries and restore them individually.

Markdown, canvas, and attachment files are content facts. Titles, navigation, search, relationships, attachment counts, Git status, and thumbnails derive from files and must not become authoritative data.

Custom YAML properties display the current note's original values. They do not inherit parent properties or turn custom fields into website or project-directory operations.

## Navigation and reading

Scanning builds navigation and metadata projections. Complete bodies are read only after selection. The workspace persists sibling order separately without changing content directories. See [Desktop workspace](workspace.md#reading-and-metadata) for reading and metadata presentation.

## Canvases

A note identifies its single owned canvas with `canvas/index.html` or the mutually exclusive `canvas/index.json`. Scanning checks entry-point existence without executing source. `canvas` and `assets` are reserved directories. Canvas files do not count as attachments, and installed `node_modules` are excluded from note resources and library change notifications. A canvas replaces the reading area below the title and metadata; otherwise the body is displayed. See [Canvas](canvas.md) for runtime, reuse, and resource contracts and [Desktop workspace](workspace.md) for native layout and commands.

## File changes and conflicts

Startup and library switches build an in-memory projection without retaining every complete body. Recursive watching uses the same boundaries. Relevant events are briefly coalesced before a full background navigation-tree rebuild; individual events do not directly mutate derived state. If paths cannot be reliably identified, perform a full reconciliation. Note and attachment creation, deletion, and archiving refresh projections through the same batch.

Changes relevant to the current note reload its body in the background while retaining existing content and reading position. Deletion, archiving, or invalidation clears selection and body. Late tasks from old libraries or selections must not overwrite current state. Watch failures preserve the last successful projection and expose an error instead of using polling to imply freshness.

When files conflict with caches, external writes, or user operations, expose the real state and do not silently overwrite newer files. Archiving, restoration, and icon updates must maintain real files atomically, preserving unrelated frontmatter, body, BOM, and newline style. Reject writes that cannot be made safely.

## Git and attachments

Git status uses the library root as the repository boundary. Changes to `index.md` and files at any depth under `canvas/` and `assets/` are attributed to their owning notes and aggregated through ancestors. If the library is not a Git repository root or Git is unavailable, change indicators degrade without blocking reading.

Local raster images can produce aspect-preserving thumbnails and note icons. Thumbnails and remote link icons are rebuildable caches outside the library. Uploading a note icon writes `assets/icon.png` in the current note and synchronizes frontmatter, without overwriting symlinks or non-regular files.

## Interfaces and dependencies

- [Desktop workspace](workspace.md) consumes navigation, current bodies, and explicit archive, restore, and icon-update operations.
- [Agent integration](mcp.md) reuses the content model. Read-only discovery locates files, clients read and write real content with their own filesystem tools, and skills define workflows.
- [Library terminal](terminal.md) receives only the current library root and does not read or modify domain state.
- `source/crates/locus-library/src/` owns scanning, reading, mutations, Git projections, and watching, independently of GPUI.
- GPUI Kit TextView owns generic Markdown parsing, layout, and selection. `source/crates/locus-ui/src/markdown_extensions/` adds galleries, attachments, images, link icons, and platform behavior through extension interfaces.

Domain changes must at least verify path normalization, boundary traversal, frontmatter fidelity, archiving, icon writes, create/delete refreshes, Git aggregation, event batches, current-body reloads, and late-task isolation.

## Content operation implementation

`locus-library/src/access.rs` defines logical addresses and content targets, file reads and writes, SHA-256 versions, and safety boundaries. `access/organize.rs` handles subtree manifests, moves, deletion, and Markdown links. It reuses scanner metadata interpretation and existing icon/frontmatter mutation capabilities for consistency with interface and canvas consumers. `reference.rs` decodes logical addresses once for navigation, link icons, MCP, and canvas components so encoded addresses identify the same object through every entry point. Content interfaces recognize the note structure of `index.md`, `assets/`, `canvas/`, and child-note directories; user configuration remains TOML.

`content_version_from_reader` computes SHA-256 in chunks and returns the byte count and lowercase hexadecimal digest. These domain interfaces serve code consuming the library directly. MCP exposes only discovery tools; external clients maintain files and handle edit conflicts themselves.
