# Changelog

This file records completed, pending user-visible changes and confirmed release notes for Locus desktop users. Platform-specific changes state their scope. See [Running and building](docs/build.md#release-notes) for version and release rules.

## [Unreleased]

- View application details and update status in Settings → About. On macOS, About Locus opens this page, and the new Check for Updates menu command opens it and starts a Sparkle check; source builds explain when no updater is available.

- Publish universal macOS apps from version tags to GitHub Releases with Sparkle background checking, signed update archives, downloading, and native installation/restart. Apple developer membership is not required: builds use ad-hoc signing by default, with optional Developer ID signing/notarization. First installation may require a Gatekeeper exception; users of builds without an updater must manually install an updater-enabled release once.

- Move Download canvas into the note titlebar’s more menu and use a pencil icon for editing notes.
- Manage bundled skills in a separate Skills settings group; selecting a location automatically installs them and removes the old Locus symlink while preserving real directories.
- Choose zsh, bash, or fish in Terminal settings on Unix to run Agents and keep the selected shell open after they exit; the default launch behavior is unchanged.
- License the project under the MIT License.

- Navigate notes through the document tree and command palette, with back and forward history, sibling ordering, icon upload, archiving, and restoration.
- Read Markdown bodies, summaries, and custom YAML properties, with continuous text selection, select all, and copy.
- Copy note references, edit notes, and open their directories from the titlebar toolbar, with the edit and directory buttons placed after copy.
- Follow external file changes automatically and show Git status for note bodies, canvases, and attachments.
- Run note-owned HTML/JavaScript or JSON template canvases on macOS, with manual refresh and export.
- Search and discover local files through read-only MCP, install bundled skills on demand, and launch selected external terminals and Agents from the library directory.
- Use English or Chinese interfaces, light or dark themes, font settings, and window and sidebar state restoration.
