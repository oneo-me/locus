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

## Usage

1. Launch Locus and choose **Open library** to select the folder for your notes. Initialize it as a Git repository so every change has history.
2. Open **Settings** with Command/Ctrl+, to choose your shell and Agent, and install the skills that teach Agents to work with Locus.
3. Enable MCP in Settings and add `http://127.0.0.1:2301/mcp` to your Agent; you can send it the address and ask it to configure itself. See [MCP access](docs/mcp.md).
4. Start writing: ask your Agent, or use any editor, to create a folder with an `index.md`. Locus follows file changes automatically.

Command/Ctrl+F searches notes and commands, Command/Ctrl+O switches libraries, and the sidebar's graph button shows how notes connect.

## Tips

- Have your Agent write a note with your library's conventions, then link it from the library's `AGENTS.md` so Agents follow it.
- A canvas's HTML can serve as a template that other notes call with their own data. With the skills installed, your Agent knows how to use it.
- In Markdown, an `attachments` code block lists a note's files, and a `gallery` code block shows its images as thumbnails.

## Roadmap

- Configure Agent skills and MCP automatically.
- Ship built-in canvas templates.
- Refine parts of the interface.
- Reorder notes in the sidebar.

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

Licensed under the [MIT License](LICENSE).
