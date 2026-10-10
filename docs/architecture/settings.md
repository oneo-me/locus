# Settings and persistence

The application owns runtime preferences and composes Onevra Settings; `internal/preferences` owns storage and the ordered writer. Generic Onevra components do not know Locus fields or paths.

## Runtime preferences

Settings is a modal dialog in the same window, available over both the workspace and the standalone library picker. It provides library/editor/export controls, system/light/dark appearance, system/English/Chinese language choice, explicit skill installation, and MCP controls. The library button is labeled “Select library” and opens the native directory picker; Settings has no About page. Closing via the header button, Escape, or backdrop restores access to the underlying page and a retained canvas. Shell and Agent choices live in the Agents settings category and are persisted as preferences. Agent offers None for a shell-only session. Changes apply when the terminal is next opened, replacing the previous session without launching anything from Settings. Settings has no terminal-application selector or Agent launch action; the legacy terminal preference remains readable but is unused. The app maps settings changes to explicit library, browser, and service operations; those services do not read configuration files themselves.

Appearance uses MyGo's theme source and Onevra's semantic theme. The sidebar theme menu offers the same persisted system/light/dark choice as Settings; its icon reflects the selected mode. The trailing MCP button opens the MCP settings category, with accent coloring while running and danger coloring on a service error. Canvas appearance updates through the retained Page API without rerunning main. English and Chinese labels are product-owned; user content is never translated. System language comes from MyGo's application locale. Native OS labels and errors can remain platform-provided. Component-specific accessibility/localization and complete bilingual acceptance still need review.

## Configuration ownership and safety

Preferences live in `~/.oneo/locus/settings.yaml`. Recognized existing fields are loaded without rewriting on startup. Unknown fields are preserved when saving, allowing unrelated preferences to survive. Missing files produce defaults. Only `settings.yaml` and `state.yaml` are used; legacy TOML/JSON files are neither read nor migrated or deleted. Reapply preferences in Settings or manually convert the old configuration to YAML (renaming alone is not sufficient for TOML). Parse/read failures preserve the file and disable its writer while exposing the error; runtime changes are not falsely described as persisted.

Saving compares the current file bytes with the last loaded/saved snapshot, merges recognized fields into preserved unrelated fields, serializes YAML, writes and synchronizes a same-directory temporary file, and renames it into place. Non-regular targets are refused. A detected external modification prevents overwrite and asks the user to restart. This is optimistic coordination, not cross-process atomic compare-and-swap; files changed during the final check/write interval cannot be transactionally reconciled.

A single writer goroutine serializes preference snapshots. Pending work is coalesced to the most recent values; the active write finishes before later values are written. Shutdown closes and drains the writer. Error callbacks return asynchronously to the UI thread, so flushing does not wait on a callback that needs that thread.

## Presentation state

`state.yaml` stores current library/selection, per-library tree expansion, and sidebar width/collapse, separately from preferences and content. Expansion is keyed by canonical library root and library-relative node paths. Switching libraries retains each tree in memory; close/quit persists all visited trees. Restoration waits for a successful scan, ignores removed branches, and preserves expanded descendants beneath collapsed parents. Restoring selection does not reopen saved collapsed ancestors; explicit navigation still reveals its destination. Older state files without tree expansion retain selection-reveal behavior. Selection is restored only when its library matches and the note is still visible. A damaged state file is not overwritten during that run. The current implementation saves presentation state on window close/quit rather than continuously persisting window bounds. Full size/position restoration and sibling ordering are not implemented.

The single-instance lock prevents ordinary duplicate app launches from concurrently writing product state. External tools may still edit configuration, and their conflicts must not be hidden. Logs report shutdown state-save failures; runtime feedback surfaces preference failures.

## Verification

Tests cover YAML defaults and round trips, preservation of unknown fields, detected external changes, malformed-file refusal, symlink targets, and writer flush ordering. Tree tests cover expansion round trips, library isolation, refresh pruning, keyboard expansion/collapse, and selection beneath collapsed ancestors. Workspace tests cover restoration-adjacent stale reads and removed selections; native tests cover settings/browser retention and tree expansion across window recreation and library switches.

Actual theme/language switching, native menus, keyboard paths, long error labels, permissions, slow-disk shutdown, and state restoration after real app relaunch require target-platform acceptance. Compilation and headless tests are not substitutes.
