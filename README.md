![Locus icon](source/resources/icon.png)

# Locus

**English** | [简体中文](README.zh-CN.md)

Locus is a local-first Markdown notebook for macOS. Notes stay ordinary files in a folder you choose, so you can read and organize them in Locus while editing them in any app you like.

## Features

- **Plain files, no lock-in**: each note is a folder containing `index.md`; nested folders form the note tree, and attachments live beside the note.
- **Built-in reading**: browse the tree or search notes and actions, then read Markdown, local images, summaries, and custom YAML properties.
- **Canvases**: a note can show its own HTML and JavaScript page, such as a dashboard or chart, inside the workspace.
- **Agent friendly**: optional read-only MCP lets external Agents find the same notes, and a built-in terminal opens your shell or Agent CLI on request.
- **Comfortable by default**: light and dark themes, an English or Chinese interface, back/forward history, and remembered navigation and sidebar state.

## Install

Download the latest release from [GitHub Releases](https://github.com/oneo-me/locus/releases/latest), then move Locus to Applications. macOS 13 or later on Apple Silicon is required.

Published downloads can lag behind the development branch; Windows and Linux builds are not available yet.

## Getting started

1. Launch Locus and choose **Open library** to select the folder that holds your notes. Command/Ctrl+O opens the picker again.
2. Add notes outside Locus: create a folder containing an `index.md` file in Finder, an editor, or an Agent. Locus follows file changes automatically.
3. Search notes and commands with Command/Ctrl+F. The note toolbar copies a reference, opens the note in your editor, or opens its folder.
4. Open **Settings** with Command/Ctrl+, for theme, language, editor, export folder, shell and Agent choices, and skills.
5. To let Agents read your notes, enable MCP in Settings and connect a client to `http://127.0.0.1:2301/mcp`. See [MCP access](docs/mcp.md).

## Development

Build and test from `source/` with Go 1.27.1 or later:

```sh
cd source
GOWORK=off go test ./...
GOWORK=off go vet ./...
GOWORK=off go tool packer run
GOWORK=off go tool packer build
```

See [Running and building](docs/build.md) for dependencies, checks and packaging, [Architecture](ARCHITECTURE.md) for design boundaries, and [AGENTS.md](AGENTS.md) for contributor rules. User-visible changes are listed in [CHANGELOG.md](CHANGELOG.md).

## Current limitations

- Windows and Linux are unverified; native macOS Markdown and canvas workflows have automated smoke tests.
- Automatic updates are not implemented; install new releases manually.
- Sidebar reordering, dedicated attachment lists, font controls, and full window-bounds restoration are not implemented.

Licensed under the [MIT License](LICENSE).
