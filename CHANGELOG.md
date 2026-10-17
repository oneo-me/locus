# Changelog

This file records completed, pending user-visible changes and confirmed release notes for Locus desktop users. Platform-specific changes state their scope. See [Running and building](docs/build.md#release-notes) for version and release rules.

## [Unreleased]

- Browse a library of plain Markdown notes as a tree or through note and action search, with back/forward history and remembered navigation and sidebar state.
- Read Markdown bodies, summaries, local images and galleries, and custom YAML properties, following external file changes with Git status.
- Show every reachable local image by scaling oversized pictures to the reading budget instead of marking them unavailable, and preview a gallery image in a full-window dialog with a close control in the top-right corner.
- Select across a Markdown body as one text range, copy code from blocks that name their language, and copy the selection as text, HTML, or Markdown.
- Open dialogs that animate in and out, and see command results return to the top when the query changes.
- Explore notes as a relation graph that combines the document hierarchy with links between notes, opened from the sidebar or command palette.
- Show the relation graph full-window over a dimmed backdrop like the image preview, without a title bar: the display-options menu and close control float in the top corner, and included attachments appear as yellow nodes.
- Show a note's own HTML, JavaScript, or JSON template canvas in the workspace, with manual refresh and export.
- Organize notes with Emoji or PNG icons, copy a reference, open the note in your editor, or reveal its folder.
- Choose light or dark themes that follow the desktop's text size and higher-contrast preferences, and an English, Simplified Chinese, Traditional Chinese, Japanese, or Korean interface.
- Let external Agents read your notes through read-only MCP on this computer; local-network access is no longer available.
- Open a built-in terminal with your shell or Agent CLI, and manage bundled authoring skills in Settings.
- Check for updates from a packaged app and install a newer GitHub release.
- Check for updates in Settings > About, which Check for Updates… in the menu and command palette opens, with release notes, download progress, and Restart to update; a background check that finds a new version prompts you with a toast and highlights the Settings button, and automatic checks and silent installation can be turned on or off there.
- Store settings and state in `~/.locus`, moving an existing `~/.oneo/locus` there on first launch.
- Download macOS disk images, the Windows app, and a Linux archive for Apple Silicon, Intel, Windows x64 and ARM64, and Linux x64 and ARM64.
