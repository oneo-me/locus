# Agent integration

The app coordinates read-only MCP discovery, explicit skill installation, and external terminal handoff. These are separate capabilities: enabling MCP does not launch an Agent, and a terminal session is not managed by the desktop app.

## Discovery and transport

`internal/discovery` uses the official Go MCP SDK and Streamable HTTP at `/mcp`. Each service captures one library root. It is disabled by default, binds `127.0.0.1:2301` by default, and can explicitly bind `0.0.0.0` for local-network use. Network mode has no authentication; the settings description warns that clients can search note content and obtain local paths.

The service exposes only:

- `list_articles`: direct child notes and organizing directories; default limit 50, maximum 200; a numeric opaque-to-clients cursor returns the next page.
- `resolve_article`: visible note lookup by literal relative path or `locus://` address, optionally ending in `/index.md`.
- `search_articles`: case-insensitive substring matching over visible note paths, titles, summaries, and bodies; default limit 20, maximum 100, bounded body excerpts.

Responses identify navigation paths and real absolute file/directory paths. Organizing directories have no body file. Each call scans current content; MCP has no content-write tools, content-transfer service, or documentation Resources capability. Clients must have access to the returned local files and use their own tools to edit them. The bundled skill owns authoring guidance.

HTTP requests are bounded and protected against cross-origin browser requests. Host validation allows localhost, loopback addresses, and the local IP receiving the request, rejecting arbitrary hostnames and unrelated IPs. SDK localhost protection remains enabled. There is no wildcard CORS policy. Stateless transport does not retain client sessions across calls. Disabling, changing the library, or applying connection settings closes the old listener before creating a new server. Failures appear in settings; listening is not described as running before bind succeeds.

## Skill installation

The `internal/skills` package embeds the skill and its references, so direct Go runs and bundles have the same content. Installation explicitly targets the selected library or home directory's `.agents/skills/locus`. It creates a copy, not a source-tree-dependent symlink. Existing targets are refused, including earlier copies and symlinks; the user must move them before reinstalling. Redirected `.agents` or `skills` parents are rejected.

The installer stages embedded files, reserves a previously absent destination, then moves entries into it. It does not claim a multi-file transaction: publication failures may leave an incomplete new directory and report its path for inspection. It never merges into an existing installation or cleans up other skill locations automatically. Moving the app does not invalidate installed copies; updates require explicit reinstallation.

## External terminals

`internal/terminal` validates the Agent and shell against fixed choices, captures the library root at activation, and hands off in the background. The UI prevents duplicate submissions while handoff is pending. Switching libraries or quitting does not terminate launched sessions.

macOS supports Terminal and Ghostty. Terminal opens a new window through `do script`; Ghostty uses its AppleScript surface configuration to create a tab in the front window, or a new window. Ghostty must support that automation interface. Selected zsh/bash/fish shells load login and interactive configuration; Default uses zsh on macOS. The same selected shell remains open after the Agent command exits or fails. Commands and directories are quoted as data. Codex, pi, OpenCode, and Claude Code must already be installed by the user.

Windows uses a new system console with an inherited working directory. Other platforms return an explicit unsupported error for Agent launching. The app does not implement PTYs, terminal tabs, authentication, model selection, billing, or Agent process ownership. Successful handoff is not proof that an Agent authenticated or ran successfully.

## Verification

Tests cover discovery paths, visibility of notes with legacy archive metadata, pagination, search, real MCP client/HTTP calls, tool inventory, and Host/Origin rejection. Installer tests verify copied content, existing-target preservation, and redirected-parent rejection. Terminal tests verify validation and shell quoting without launching a terminal or Agent.

Local-network deployment, permissions, actual Terminal/Ghostty behavior, selected-shell startup files, Windows consoles, and client skill reloading require user acceptance. See [MCP access](../mcp.md) for client setup and [Development checks](../build.md#development-checks) for automated commands.
