# Library

`internal/library` owns content interpretation, file access, archive updates, and rebuildable projections. It has no UI dependencies. The workspace owns presentation and selected-body orchestration.

## File model and navigation

A note is a non-hidden directory containing `index.md`; the selected root itself is not a note. Organizing directories appear only when they contain visible descendants. `assets`, `canvas`, and `node_modules` are not child notes. Scanning excludes symlink directories. A boolean `archived: true` hides the entire subtree; a quoted string does not count as the flag. Settings separately lists explicitly archived notes for restoration.

Titles fall back to directory names; summaries come from YAML. Selected documents retain the parsed YAML tree and complete Markdown body. Source files are never regenerated simply to display metadata. Scans read metadata or the first ordinary body line, capped at 1 MiB; selected bodies and individual resources are capped at 32 MiB. Hierarchies deeper than 256 levels are rejected. Malformed YAML, unsafe bodies, excessive metadata, and I/O failures fail a scan visibly rather than fabricate a complete partial projection.

## Addresses and rooted access

`locus://` addresses encode individual path segments. Decoding happens once and rejects hidden/reserved note segments, traversal, empty segments, encoded separators, invalid UTF-8, null bytes, queries, fragments, absolute paths, and backslashes. Plain paths supplied to MCP stay literal until encoded for the common address validator.

The user-selected root is canonicalized and opened with Go's rooted filesystem API. Body/resource components reject symlinks and non-regular files. Rooted operations additionally prevent a concurrent path replacement from escaping the root, but component checks do not create a transactional snapshot against another process mutating directories inside it.

Resource access accepts only an owning note's `index.md` or files under its `assets/` and `canvas/`. Hidden resource files are allowed inside those trees; `node_modules`, traversal, symlinks, directory browsing, and arbitrary project files are not. Note-body part extraction preserves original YAML/body bytes rather than reserializing values. Consumers decide whether a projected note is visible; raw resource access is not an archive authorization boundary.

## Archive updates

Archiving is an explicit, confirmed UI action. The app captures a SHA-256 content version before confirmation. The library checks that version, prepares the changed body in a same-directory temporary file, synchronizes it, checks the version again, and replaces the target. Restoration uses the same mechanism.

Only the archive field is changed; unrelated frontmatter, prose, BOM, and newline style remain intact. Complex archive values and flow-style frontmatter require external editing rather than potentially lossy rewriting. Source files without frontmatter receive a new block. Multi-process checks are optimistic: there is no atomic compare-and-swap with external editors, and the final check-to-rename interval is not a cross-process transaction.

## Watching and Git

The app owns one cancellable recursive fsnotify watcher per library. It adds real directories, excludes hidden directories and `node_modules`, and coalesces events for 150 ms before requesting reconciliation. New directories trigger subscription reconciliation. Changes during an in-flight scan request another scan rather than silently dropping freshness. Watch failures are shown and do not fall back to polling. Switching libraries cancels the old watcher and invalidates its queued callbacks.

Git status is optional and runs with a timeout only when the selected library is the repository root. Porcelain NUL-delimited paths, including rename pairs, attribute body, attachment, and canvas changes to their notes, then aggregate through ancestors. Git failure removes indicators without preventing reading. Git indexes are not content authorities.

## Verification

Tests cover hierarchy and archive filtering, Unicode addresses, unsafe paths and symlinks, BOM/CRLF preservation, malformed and oversized inputs, version conflicts, recursive watch notifications and cancellation, and canvas/MCP resource boundaries. Target-platform checks remain necessary for external-editor races, permission failures, unusual filesystem layouts, and large libraries. See [Development checks](../build.md#development-checks).
