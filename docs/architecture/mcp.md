# Agent integration

This document owns current behavior, implementation boundaries, and acceptance requirements.

## Purpose and entry point

Locus discovers notes in the current local library through a single Streamable HTTP `/mcp` endpoint, returning metadata and real file paths. External clients own file reads and writes, the Agent loop, model calls, execution permissions, and sessions. Locus does not provide a complete operation interface, content-transfer tools, or software-documentation resource queries. Clients need access to the same filesystem; a remote MCP connection alone does not grant file-editing access.

The bundled [Locus skill](../../source/resources/skills/locus/SKILL.md) defines usage. Initialization instructions state only read-only discovery capabilities and point to the installed skill. The three tool descriptions retain minimal capability descriptions, and parameter schemas explain accepted path forms. There is no special built-in documentation query address or declared MCP Resources capability.

## Discovery tools

`/mcp` exposes three read-only tools:

- `list_articles`: `parent` accepts a library-relative parent path or `locus://` address and lists direct child notes with pagination. An empty value or `locus://` browses the library root. The default limit is 50 and the maximum is 200; continue with `nextCursor`.
- `search_articles`: performs case-insensitive substring searches across library paths, titles, summaries, attachment names, and bodies, returning bounded body excerpts. The default limit is 20 and the maximum is 100.
- `resolve_article`: `path` accepts a known library-relative path or `locus://` address; both forms accept a trailing `/index.md`. The library root cannot resolve to a note. Empty segments, `.`, `..`, and out-of-bounds paths are rejected.

Results include metadata, hierarchy navigation `path`, absolute file path `file`, and directory path `directory`. Clients maintain bodies and attachments with their own filesystem tools. Discovery reuses the ordinary navigation projection, excluding archived notes and descendants. Calls without a selected library return a real error.

`locus://` addresses reuse the shared library parser. Percent encoding is decoded once, rejecting encoded out-of-bounds segments, separators, null bytes, and invalid UTF-8. Queries and fragments are not accepted; hashes within names must be encoded. Plain relative paths remain literal and are not percent-decoded. Tools retain the parameter names `path` and `parent`, not `uri`.

## Skill distribution and installation

`source/resources/skills/locus/` contains `SKILL.md`, `references/attachments.md`, and `references/canvas.md`. The entry point defines file discovery, the note model, and filesystem workflows. Read attachment or canvas references as needed; their rules are not duplicated at other entry points. Each library organizes shared canvas templates itself; the app does not prescribe a template directory or manifest.

`source/packer.toml` copies `resources/skills` to `Contents/Resources/skills` on macOS and `resources/skills` in the Windows portable directory. The settings installer resolves app resources relative to the running executable. Development runs outside a bundle use source resources. Installation creates a directory symlink rather than embedding or copying a second version. MCP neither loads nor distributes skill bodies.

Settings → Skills provides an installation location selector, defaulting to Library, in its own navigation group. Destinations are `<library>/.agents/skills/locus` and `~/.agents/skills/locus`. The preference persists as `skills_location`; after saving the selection, settings automatically installs at that location and removes the Locus symlink from the other location. Without a library, Library installation shows guidance to select one. Selecting the same location again retries installation and cleanup.

The settings entity captures both destinations at selection time and creates an absolute directory symlink to resources in the background. It disables the location selector during installation and reports progress, the actual destination, or errors. New installation completes before old-link cleanup; destination conflicts leave the old link untouched. Cleanup preserves real files, directories, other skills, and the resource source, rejects redirected parents, and skips identical destinations. Cleanup failures report that the new link was installed and the old link could not be removed; selecting again retries. These filesystem steps are not a cross-location transaction. Existing symlinks are replaced after staging a new link; failure attempts to restore the old link. If restoration fails, the backup is retained and its path reported. Real directories and files are preserved, with guidance to move them before retrying. Other skills are retained, and installation parent directories reject symlinks and non-regular types. The resource directory must contain `SKILL.md`; if absent, existing links remain untouched. Reinstallation can repair dangling links.

Updating app resources in place makes links read the new version. Moving or deleting the app bundle invalidates old links and requires selecting the location again from the new app location. macOS uses Unix symlinks; Windows uses directory symlinks. If the system forbids link creation, report the real error and require the corresponding system permission before retrying, without falling back to copying. Changing destinations removes the other location's Locus symlink after successful installation. Installation does not launch a terminal or Agent automatically.

## Listening and lifecycle

- The service is disabled by default and uses port `2301`. Local mode binds `127.0.0.1`; local-network mode binds `0.0.0.0`.
- Local-network mode has no authentication. Settings explains that network clients can search note content and obtain local paths. MCP tools themselves do not write or delete content.
- Each connection restricts Host: localhost, loopback addresses, and the local IP receiving the request are allowed; other hostnames and unrelated IPs are rejected. The app does not enumerate interfaces or select or display a LAN address.
- Missing libraries, invalid addresses, boundary violations, missing notes, and read failures return real errors instead of fabricated empty results.
- Configuration changes stop the old service before starting a replacement. Quitting releases the service. Old sessions cannot survive service restarts or library switches.
- Settings exposes service enablement, local-network mode, port, real runtime status, and startup errors. Clients configure their own port and fixed path; see [MCP access](../mcp.md).
- The sidebar's bottom-left area displays a non-interactive MCP icon: success green when running, danger red on failure, and muted_foreground gray when stopped. Tooltips and accessible names identify the actual state without taking keyboard focus.

## Service implementation

`locus-mcp/src/lib.rs` mounts a discovery service in each connection's Axum Router. Service clones share a session manager. TCP `local_addr` supplies the IP receiving the request for rmcp Host validation. The MCP package forbids `unsafe`, does not directly depend on `libc`, and composes connection services with `tower::service_fn` rather than implementing Host parsing or validation itself. `service.rs` owns the three-tool contract and directly consumes library scanning, reading, and address parsing, independently of workspace selection.

The workspace maps settings explicitly to `McpConfiguration`, passing only the library path, enablement, listening scope, and port. The controller does not read preference storage; settings notifies the workspace through `SettingsEvent`. Skill installation belongs to settings and does not change service configuration or lifecycle.

## Verification methods

Run `cargo test -p locus-mcp -p locus-settings -p locus-preferences -p oneo-tool-packer --locked -- --test-threads=1` in `source/`. Check the three tools and response structures, absence of Resources capability and the removed full endpoint, symlink installation and updates, preservation of real directories and path rejection, location preference round trips, and resource copying on both platforms. HTTP tests require permission to listen on temporary local ports. Serial execution reduces port-rebinding interference after shutdown.

Keyboard operation of settings installation, both languages, global and library permissions, and failure recovery require target-platform confirmation. Protocol and resource-copy tests do not replace native settings or final app-bundle acceptance testing.
