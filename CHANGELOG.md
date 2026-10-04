# Changelog

This file records completed, pending user-visible changes and confirmed release notes for Locus desktop users. Platform-specific changes state their scope. See [Running and building](docs/build.md#release-notes) for version and release rules.

## [Unreleased]

- Manage bundled skills in a separate Skills settings group; selecting a location automatically installs them and removes the old Locus symlink while preserving real directories.

- Choose zsh, bash, or fish in Terminal settings on Unix to run Agents and keep the selected shell open after they exit; the default launch behavior is unchanged.
- License the project under the MIT License.

## [0.2026.1]

The first Locus release provides a workspace for reading and organizing local Markdown notes and collaborating with external Agents.

- Navigate notes through the document tree and command palette, with back and forward history, sibling ordering, icon upload, archiving, and restoration.
- Read Markdown bodies, summaries, and custom YAML properties, with continuous text selection, select all, and copy.
- Copy note references, edit notes, and open their directories from the titlebar toolbar, with the edit and directory buttons placed after copy.
- Follow external file changes automatically and show Git status for note bodies, canvases, and attachments.
- Run note-owned HTML/JavaScript or JSON template canvases on macOS, with manual refresh and export.
- Search and discover local files through read-only MCP, install bundled skills on demand, and launch selected external terminals and Agents from the library directory.
- Use English or Chinese interfaces, light or dark themes, font settings, and window and sidebar state restoration.
