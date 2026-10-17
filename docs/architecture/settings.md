# Settings and persistence

The application owns runtime preferences and composes Onevra Settings. `internal/preferences` owns storage and the ordered writer. Generic Onevra components do not know Locus fields or paths.

## Runtime preferences

Settings is a modal dialog in the same window, over the workspace and the library picker. It covers the library, editor, export directory, appearance, language, About and updates, skill installation, and MCP. The library button is labeled “Select library” and opens the native directory picker.

Closing with the header button, Escape, or the backdrop returns to the page underneath, including a retained canvas. Shell and Agent choices live in the Agents category and are persisted. Agent offers None for a shell-only session. A change applies the next time the terminal opens and replaces the previous session. Settings does not launch a terminal or an Agent. The legacy terminal preference stays readable and unused.

The app maps settings changes onto library, browser, and service operations. Those services do not read configuration files themselves. Appearance uses MyGo's theme source and Onevra's semantic theme. The sidebar theme menu stores the same system, light, and dark values. Its icon reflects the selected mode.

The trailing MCP button opens the MCP category. It uses the accent color while the service is running and the danger color after an error. Canvas appearance updates through the retained Page API without rerunning main.

The interface is English, Simplified Chinese, Traditional Chinese, Japanese, or Korean. Stored values are `system`, `en`, `zh-Hans`, `zh-Hant`, `ja`, and `ko`. Files that still say `english` or `chinese` load as English and Simplified Chinese. Startup normalizes a recognized value in memory and writes it on the next user save. System language comes from MyGo's application locale. Traditional Chinese stays distinct from Simplified Chinese.

Product-owned labels are translated. User content is not. Native OS labels and errors may stay platform-provided. Component-owned accessibility strings still need review.

### About and updates

About ends the Application section. It shows the running version, or “Development build”, with a link to the GitHub release history, then the update flow and its two preferences. The app drives `mygo.Updater` itself; it does not use MyGo's updater plugin or a separate update window.

Check for Updates… on the application menu and in the command palette opens Settings on About and starts a check. The software update row moves through checking, up to date, new version with its Markdown release notes, download progress, and Restart to update. A failed check or download is reported there; a failed download leaves the running app unchanged and can be retried. Development builds and installs the app cannot write, such as the `.deb` in `/opt`, say that the build does not update itself and offer no check.

“Check for updates automatically” (`update_checks`, on by default) checks 10 seconds after launch when the last successful check is a day old, and asks again every hour while the app runs. A background check that finds a new version prompts once per version per run: a toast that stays until closed, whose Details action opens About, and an accent-colored Settings button while the update is not installed. With “Download and install updates automatically” (`update_downloads`, off by default), a background find downloads and installs instead, then a toast offers Restart; otherwise the new version runs at the next launch. Manual checks never install without the user's choice.

## Configuration ownership and safety

Preferences live in `~/.oneo/locus/settings.yaml`. Recognized fields load without rewriting the file on startup. Unknown fields are preserved on save. A missing file produces defaults.

The preferences package uses only `settings.yaml` and `state.yaml`.

Legacy TOML and JSON files are not read, migrated, or deleted. Reapply preferences in Settings, or convert the old file to YAML by hand. Renaming a TOML file is not enough.

A parse or read failure keeps the file, disables its writer, and reports a toast. Runtime changes are not described as saved. Saving compares the current bytes with the last loaded or saved snapshot, merges recognized fields into the preserved ones, writes a temporary file in the same directory, and renames it into place. Non-regular targets are refused.

An external modification blocks the overwrite and asks for a restart. This is optimistic coordination, not a cross-process compare-and-swap. A change during the final check cannot be reconciled transactionally.

One writer goroutine serializes snapshots. Pending work keeps only the newest values. The active write finishes before later values are written. Shutdown closes and drains the writer. Error callbacks return to the UI thread and show a toast, so a flush does not wait on that thread.

## Presentation state

`state.yaml` stores the current library and selection, per-library tree expansion, sidebar width and collapse, the relation graph's attachment visibility, and the time of the last successful update check. It is separate from preferences and from note content. Expansion is keyed by the canonical library root and library-relative node paths. Switching libraries keeps each tree in memory. Close and quit persist every visited tree.

Restoration waits for a successful scan, ignores removed branches, and keeps expanded descendants under collapsed parents. Restoring a selection does not reopen saved collapsed ancestors. Explicit navigation still reveals its destination. Older state files without tree expansion keep the selection-reveal behavior. Selection is restored only when its library matches and the note is still visible.

A damaged state file is not overwritten during that run. The app saves presentation state on window close and quit, not continuously, and does not restore full window bounds. Sibling ordering is not implemented.

The single-instance lock stops ordinary duplicate launches from writing product state together. External tools can still edit configuration, and those conflicts stay visible. Logs report a failed state save at shutdown. Preference and state failures during a run surface as error toasts.

## Verification

Tests cover YAML defaults and round trips, unknown fields, external changes, malformed files, symlink targets, and writer ordering. Tree tests cover expansion round trips, library isolation, refresh pruning, keyboard expansion, and selection under collapsed ancestors. Workspace tests cover stale reads and removed selections. Native tests cover settings retention and tree expansion across window recreation and library switches. Update tests use a fake release source to cover About without updates, manual checks that open About, install progress and the pending relaunch, background prompts once per version, automatic downloads, and check failures.

Theme and language switching, native menus, keyboard paths, long error labels, permissions, slow-disk shutdown, and restoration after a real relaunch need target-platform acceptance. So do a real update check, download, installation, and relaunch from a published release; no test downloads or installs an update. Compilation and headless tests do not replace that.
