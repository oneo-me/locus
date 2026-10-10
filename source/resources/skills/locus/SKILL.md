---
name: locus
description: Create, edit, and organize notes, attachments, and HTML canvases in a Locus Markdown library. Use when working with Locus notes or its MCP discovery tools; not for unrelated Markdown or standalone websites.
---

# Locus

Locus reads a local directory of Markdown notes. Use the client's filesystem tools to edit content; MCP only discovers notes. Work in the library the user selected, and preserve existing content and custom metadata.

## Discover the target

- With a known library-relative path or `locus://` note address, call `resolve_article` with `path`; do not search first.
- Use `search_articles` with `query` for titles or keywords (default limit 20, maximum 100). Results include excerpts, not full note bodies.
- Use `list_articles` with `parent` to browse direct children (default limit 50, maximum 200); an empty parent or `locus://` browses the root. Continue using `nextCursor` as `cursor` until it is null.
- Results include `path` for navigation and absolute `file` / `directory` for filesystem access. Read the returned `index.md` before editing. If MCP is unavailable, use the explicitly selected library directory; do not infer a different library from a global skill's location.

The default Streamable HTTP URL is `http://127.0.0.1:2301/mcp`; the user can change the port. Clients must be able to access the returned filesystem paths. A remote MCP connection alone cannot edit the library. There are no MCP content-write tools or software-documentation resources.

## File model and editing

Each note is a directory containing `index.md`, with optional `assets/`, `canvas/`, and child-note directories. Directories without `index.md` may organize descendants. `assets` and `canvas` are reserved names; hidden directories, symbolic links and installed `node_modules` are excluded from note discovery. The root itself is not a note.

YAML frontmatter owns `title`, `summary`, and `icon`; other fields are retained as authored and shown as custom properties. Keep unrelated fields, prose, and newline style when editing. The reserved icon is `assets/icon.png`, referenced by `icon` in frontmatter, and is not an ordinary attachment.

Create a note by creating its directory and `index.md`. Use existing parent directories or create explicit organization directories as needed. Avoid overwriting existing files. Read complete current files before replacement; if they have changed since reading, reconcile those changes. Moves carry the entire note directory, including descendants, assets and canvas. Review inbound Markdown links, relative outbound links, canvas/script references and template addresses after moving or renaming. Permanent deletion removes real files: follow the user's requested scope and authorization.

`locus://projects/example` identifies a note in the current library. Percent-encode spaces, `%` and `#` in addresses; decoding occurs once. Do not use traversal, encoded separators, queries or fragments in note addresses. Ordinary library-relative filesystem paths remain literal. Use relative Markdown links for note references and canonical `locus://` note addresses for canvas navigation.

## Attachments and canvases

Before adding images, galleries or file references, read [references/attachments.md](references/attachments.md). Attachments belong to their note's `assets/` directory.

Before creating or changing a canvas, read [references/canvas.md](references/canvas.md). Locus hosts an HTML fragment plus `canvas/main.js` exporting `async main(data)`, or a mutually exclusive `canvas/index.json` template invocation. It does not run a standalone web project or compile React/TSX. Follow the host's resource ownership, theme and network rules.

File watching refreshes note projections. Saving canvas files does not reload an already open canvas: refresh with Cmd/Ctrl+R or F5 while it has focus, or reopen the note. Native canvas rendering requires a supported Onevra WebView backend; macOS has native smoke tests, while Windows still needs target-platform acceptance. Filesystem edits alone do not verify appearance.
