# Settings and persistence

The application owns runtime preferences and composes Onevra Settings; `internal/preferences` owns storage and the ordered writer. Generic Onevra components do not know Locus fields or paths.

## Runtime preferences

Settings is a modal dialog in the same window, available over both the workspace and the standalone library picker. It provides library/editor/export controls, system/light/dark appearance, system/English/Chinese language choice, terminal/Agent/shell choices, explicit skill installation, archive restoration, MCP controls, and About. Closing via the header button, Escape, or backdrop restores access to the underlying page and a retained canvas. The app maps settings changes to explicit library, browser, service, and terminal operations; those services do not read configuration files themselves.

Appearance uses MyGo's theme source and Onevra's semantic theme. Canvas appearance updates through the retained Page API without rerunning main. English and Chinese labels are product-owned; user content is never translated. System language comes from MyGo's application locale. Native OS labels and errors can remain platform-provided. Component-specific accessibility/localization and complete bilingual acceptance still need review.

About identifies the app and license and links to GitHub releases. Automatic updating is not configured; the UI says so instead of reporting an unverified up-to-date status.

## Configuration ownership and safety

Preferences live in `~/.oneo/locus/settings.toml`. Recognized existing fields are loaded without rewriting on startup. Unknown fields are preserved when saving, allowing unrelated preferences to survive. Missing files produce defaults. Parse/read failures preserve the file and disable its writer while exposing the error; runtime changes are not falsely described as persisted.

Saving compares the current file bytes with the last loaded/saved snapshot, merges recognized fields into preserved unrelated fields, serializes TOML, writes and synchronizes a same-directory temporary file, and renames it into place. Non-regular targets are refused. A detected external modification prevents overwrite and asks the user to restart. This is optimistic coordination, not cross-process atomic compare-and-swap; files changed during the final check/write interval cannot be transactionally reconciled.

A single writer goroutine serializes preference snapshots. Pending work is coalesced to the most recent values; the active write finishes before later values are written. Shutdown closes and drains the writer. Error callbacks return asynchronously to the UI thread, so flushing does not wait on a callback that needs that thread.

## Presentation state

`state.toml` stores current library/selection and sidebar width/collapse, separately from preferences and content. Selection is restored only when its library matches and the note is still visible. A damaged state file is not overwritten during that run. The current implementation saves presentation state on window close/quit rather than continuously persisting window bounds. Full size/position restoration and sibling ordering are not implemented.

The single-instance lock prevents ordinary duplicate app launches from concurrently writing product state. External tools may still edit configuration, and their conflicts must not be hidden. Logs report shutdown state-save failures; runtime feedback surfaces preference failures.

## Verification

Tests cover TOML defaults and round trips, preservation of unknown fields, detected external changes, malformed-file refusal, symlink targets, and writer flush ordering. Workspace tests cover restoration-adjacent stale reads and removed selections; native tests cover settings/browser retention.

Actual theme/language switching, native menus, keyboard paths, long error labels, permissions, slow-disk shutdown, and state restoration after real app relaunch require target-platform acceptance. Compilation and headless tests are not substitutes.
