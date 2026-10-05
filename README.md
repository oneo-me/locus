![Locus icon](icon.png)

# Locus

**English** | [简体中文](README.zh-CN.md)

Locus is a local-first Markdown knowledge workspace with a native desktop interface built with Rust and GPUI. It serves developers, creators, and one-person teams maintaining personal knowledge over time. Users can read and organize notes, inspect file changes, open note canvases and external terminals, and use MCP to let Agents discover and maintain the same local files.

The Markdown document tree is the sole source of truth for knowledge content. Users can access, edit, and version the files directly. External Agents locate files through MCP and edit them with their own filesystem tools following the Locus skill. Their clients own model calls, accounts, permissions, remote-data policies, and billing.

![Locus workspace preview](preview.jpeg)

## Quick start

For macOS, download the universal Apple Silicon / Intel ZIP from [GitHub Releases](https://github.com/oneo-me/locus/releases/latest), extract it, and move `Locus.app` into Applications. Release builds check and download updates automatically through Sparkle, which handles installation and restart. Users of older builds without an updater must install the first updater-enabled release manually once. Source builds do not include automatic updating. Default release builds are ad-hoc signed, not Apple-notarized; on first installation, macOS may block opening the app. Only if you trust the release, allow it in System Settings → Privacy & Security after attempting to open it. See [release setup](docs/build.md#automated-macos-releases) for maintainer configuration.

Settings → About shows the app version, license and project links, and update status. On macOS, About Locus in the application menu opens this page; Check for Updates opens it and starts a Sparkle check. The page also provides the same check button and a manual download link.

After launching the app, select a local Markdown library in Settings. Each note is represented by `index.md` in its directory. The app reads changes made to those files by external editors and Agents.

Running from source requires Rust and the platform's build tools. From the repository root:

```sh
cd source
cargo run --locked -p oneo-tool-packer -- run
```

This builds and launches `dist/Locus Dev.app` on macOS (or `dist/Locus Dev/` on Windows), including bundled resources. The development app shares existing Locus settings and data. See [Running and building](docs/build.md) for details.

On macOS, install Xcode and Command Line Tools and use macOS 13 or later. On Windows, install Visual Studio C++ tools and the Windows SDK.

## Notes and canvases

A library is a local root directory selected by the user. Directory nesting organizes notes. Each note's title, icon, summary, and archive state come from the YAML frontmatter in `index.md`. Attachments live in `assets/`, and `assets/icon.png` is the note icon. The reserved directories `assets` and `canvas` are not child notes.

Select notes through the sidebar or command palette. The titlebar toolbar provides, in order, copy reference, edit note, open directory, and a more menu. The more menu offers icon upload and archiving. The sidebar context menu retains all four operations and binds them to the corresponding note. Restore archived notes in Settings. External file changes update the app's derived views, and Git indicators reflect changes to the body, canvas, and attachments.

Markdown supports continuous text selection, select all, and copy. Summaries and custom YAML properties appear above the body. Objects and arrays expand recursively, preserving order, empty collections, and null.

Notes with `canvas/index.html` and `canvas/main.js` run an HTML/JavaScript canvas. Template callers instead use the mutually exclusive `canvas/index.json`. Native canvases currently support macOS. Web content appears below the titlebar and metadata, includes bundled Tailwind, and reads files through the system WebView. A canvas note's `index.md` contains only frontmatter; ordinary notes display the Markdown body.

After saving a canvas, refresh it manually or reopen the note to load the changes. Leaving the note releases transient page state. Download canvas in the titlebar’s more menu exports the saved entry point, data, and referenced resources. It saves to the system Downloads directory by default; change this in General settings. The result is a single HTML file or a directory containing `canvas.html` and resources, with automatic numbering for duplicate names. Dynamic resources must be loaded first, online data still requires a network, and modules or Workers may require a local HTTP server. Authoring rules are in the bundled skill's [canvas reference](source/resources/skills/locus/references/canvas.md) and [attachment reference](source/resources/skills/locus/references/attachments.md).

## External Agents

In Terminal settings, select the system terminal or Ghostty and choose Codex, pi, OpenCode, or Claude Code. On Unix, the Shell selector offers Default, zsh, bash, and fish (3.0 or later). Install the selected shell first. The defaults are the system terminal, Codex, and the existing platform shell. Install the selected software first. Locus runs `codex`, `pi`, `opencode`, or `claude` at the current library root. Switching libraries, changing settings, or quitting Locus does not terminate existing external sessions.

On macOS, the system terminal is Terminal, which opens a new window for each launch. Ghostty requires version 1.3.0 or later and creates a new tab in the most recently used window, or a window when none exists. Default uses zsh; selecting bash or fish loads that shell's login and interactive configuration instead. Launching keeps the selected shell open if the Agent exits or its command cannot be found. Windows uses the system console.

| Action | macOS shortcut |
| --- | --- |
| Command palette | Command+E |
| Settings | Command+, |
| Open Agent | Command+backtick |
| Refresh the current note or focused canvas | Command+R or F5 |
| Close window | Command+W |

Other platforms use the corresponding Control combinations. Enable MCP in Settings. It is disabled by default and uses port `2301`, exposing only read-only discovery at `/mcp`. Usage instructions live in the bundled skills, which Skills settings automatically installs into the selected library or global `.agents/skills` directory, removing the old Locus link when switching locations. Local-network mode has no authentication; clients can search note content and obtain local paths.

Source lives in `source/`; final app outputs live in `dist/`.

Licensed under the [MIT License](LICENSE).
