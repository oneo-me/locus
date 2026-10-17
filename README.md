![Locus icon](source/assets/icon.png)

# Locus

**English** | [简体中文](README.zh-CN.md)

Locus is a local-first Markdown notebook for macOS, Windows, and Linux. Notes stay ordinary files in a folder you choose, so you can read and organize them in Locus while editing them in any app you like.

## Features

- **Plain files, no lock-in**: each note is a folder containing `index.md`; nested folders form the note tree, and attachments live beside the note.
- **Built-in reading**: browse the tree or search notes and actions, then read Markdown, local images, summaries, and custom YAML properties.
- **Relation graph**: open a full-window graph over the workspace that maps the note hierarchy together with the links between notes, with an option to include each note's attachment files as yellow nodes.
- **Canvases**: a note can show its own HTML and JavaScript page, such as a dashboard or chart, inside the workspace.
- **Agent friendly**: optional read-only MCP lets external Agents find the same notes, and a built-in terminal opens your shell or Agent CLI on request.
- **Comfortable by default**: light and dark themes; English, Simplified and Traditional Chinese, Japanese, and Korean; back/forward history and remembered sidebar state.

## Install

Download the latest release from [GitHub Releases](https://github.com/oneo-me/locus/releases/latest).

- **macOS**: open the disk image and move `Locus.app` to Applications. macOS 13 or later, Apple Silicon or Intel.
- **Windows**: run the `Locus Setup` installer for your architecture. Windows 10 or later, with the WebView2 runtime, on x64 or ARM64.
- **Linux**: install the `.deb`, or unpack the tar.gz and run `locus`. GTK 3 and WebKitGTK are required, on x64 or ARM64.

A packaged app checks for updates once a day and tells you when a newer release is available. Settings > About shows the version, release notes, download progress, and Restart to update, and turns automatic checks and silent installation on or off; Check for Updates… in the application menu and command palette opens it. Development builds and the `.deb` package do not update themselves.

Releases are not yet signed or notarized. On first launch, allow Locus under System Settings > Privacy & Security > Open Anyway on macOS, or choose More info > Run anyway in Windows SmartScreen.

## Getting started

1. Launch Locus and choose **Open library** to select the folder that holds your notes. Command/Ctrl+O opens the picker again.
2. Add notes outside Locus: create a folder containing an `index.md` file in Finder, an editor, or an Agent. Locus follows file changes automatically.
3. Search notes and commands with Command/Ctrl+F. The note toolbar copies a reference, opens the note in your editor, or opens its folder. The sidebar's graph button shows how notes are connected.
4. Open **Settings** with Command/Ctrl+, for theme, language, editor, export folder, shell and Agent choices, and skills.
5. To let Agents read your notes, enable MCP in Settings and connect a client to `http://127.0.0.1:2301/mcp`. See [MCP access](docs/mcp.md).

## Development

Build and test from `source/` with Go 1.27.1 or later:

```sh
cd source
GOWORK=off go test ./...
GOWORK=off go vet ./...
GOWORK=off go tool mygo dev
GOWORK=off go tool mygo build -platform darwin/arm64,darwin/amd64,windows/amd64,windows/arm64,linux/amd64,linux/arm64
```

See [Running and building](docs/build.md) for dependencies, checks and packaging, [Architecture](ARCHITECTURE.md) for design boundaries, and [AGENTS.md](AGENTS.md) for contributor rules. User-visible changes are listed in [CHANGELOG.md](CHANGELOG.md).

## Current limitations

- Windows and Linux are unverified; native macOS Markdown and canvas workflows have automated smoke tests, while Windows and Linux archives are cross-built and need target-platform acceptance.
- Sidebar reordering, dedicated attachment lists, font controls, and full window-bounds restoration are not implemented.

Licensed under the [MIT License](LICENSE).
