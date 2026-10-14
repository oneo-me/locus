# Agent integration

The app coordinates read-only MCP discovery, explicit skill installation, and built-in terminal sessions. These are separate capabilities: enabling MCP does not launch an Agent. The desktop hosts the terminal; external Agent CLIs own authentication, model calls, permissions, and their own session semantics.

## Discovery and transport

`internal/discovery` uses the official Go MCP SDK and Streamable HTTP at `/mcp`. Each service captures one library root. It is disabled by default and always binds loopback at `127.0.0.1:<port>`, defaulting to port 2301. Settings exposes only the port, so other machines cannot reach the endpoint.

The service exposes only:

- `list_articles`: direct child notes and organizing directories; default limit 50, maximum 200; a numeric opaque-to-clients cursor returns the next page.
- `resolve_article`: visible note lookup by literal relative path or `locus://` address, optionally ending in `/index.md`.
- `search_articles`: case-insensitive substring matching over visible note paths, titles, summaries, and bodies; default limit 20, maximum 100, bounded body excerpts.

Responses identify navigation paths and real absolute file/directory paths. Organizing directories have no body file. Each call scans current content; MCP has no content-write tools, content-transfer service, or documentation Resources capability. Clients must have access to the returned local files and use their own tools to edit them. The bundled skill owns authoring guidance.

HTTP requests are bounded and protected against cross-origin browser requests. Host validation allows localhost and loopback addresses, rejecting arbitrary hostnames and unrelated IPs. SDK localhost protection remains enabled. There is no wildcard CORS policy. Stateless transport does not retain client sessions across calls. Disabling, changing the library, or applying connection settings closes the old listener before creating a new server. Failures appear in settings; listening is not described as running before bind succeeds.

## Skill installation

The desktop app resolves MyGo's resource directory and passes it to `internal/skills`, which reads the skill and references from `skills/locus/`. The source files live in `source/resources/skills/`. MyGo copies that directory into the app; the skill is not embedded in the executable. Missing resources fail installation rather than creating an empty skill.

Installation targets the selected library or the home directory's `.agents/skills/locus`. It creates a copy, not a symlink back to the source tree. Existing targets are refused, including earlier copies and symlinks; the user must move them before reinstalling. Redirected `.agents` or `skills` parents are rejected.

The installer stages resource files, reserves a previously absent destination, then moves entries into it. It does not claim a multi-file transaction: publication failures may leave an incomplete new directory and report its path for inspection. It never merges into an existing installation or cleans up other skill locations automatically. Moving the app does not invalidate installed copies; updates require explicit reinstallation.

## Built-in terminal

The titlebar terminal button, immediately left of More actions when that menu is present, opens a 600 × 500 logical-pixel Onevra Popover containing only the terminal view. It uses the same MyGo terminal component as Onevra’s terminal gallery, with a real PTY and the libghostty-vt emulator. Locus does not implement terminal rendering or a native bridge. Opening the popover initially starts the configured Agent through the selected shell in the library directory, only after explicit user activation; selecting None for Agent starts only the shell.

While a session is being created or its process is alive, the header button pulses with a repeating keyframe sequence: the icon warms toward the accent color and a rounded accent outline fades in and out. The pulse stops when creation fails, the process exits (including a failure that retains its output), or the session closes; Reduce Motion keeps the accent icon and outline without movement. The indicator therefore keeps a terminal that was left running behind a dismissed popover visible from any page.

The Agents settings category offers Default, zsh, bash, fish, sh, and PowerShell on Unix. Windows offers Default, PowerShell variants, and cmd. Agent choices include Codex, Claude Code, OpenCode, pi, Gemini CLI, and Aider. These programs must already be installed.

Default shell-only sessions use MyGo’s platform shell discovery. Agent launches use the configured shell or the environment’s supported default shell. Unsupported defaults produce a visible error. Explicit shell launches resolve the executable before PTY creation, preferring PATH and then a matching absolute `SHELL` path.

On macOS, lookup also checks standard Homebrew, MacPorts, and system binary directories, so a Finder launch can find an installed shell with a minimal PATH. Explicit paths remain authoritative. A missing shell is an error, not a switch to another shell. Unix shells load an interactive login configuration, invoke the Agent through the shell so aliases and functions apply, and exit with its status. PowerShell and cmd run the Agent without keeping a shell after it exits. Library paths are a working directory, never text inside a shell command.

Changing a selection in Settings saves a preference without executing it. The next terminal activation replaces the current session with the new selection. Exit status 0, or interrupt status 130, releases the terminal and closes the popover. Reopening starts a new session. Any other exit status keeps the terminal and its output, including a missing command or a PTY startup failure.

Hiding and reopening a failed terminal retries. Ctrl+C is delivered to the program. It closes the popover only when the program exits, not when the Agent only cancels a task. Reopening after a creation error retries. Hiding the popover with its button, Escape, an outside click, or Settings keeps the process and output. Switching libraries and quitting close the session. There is one session per window, and it is not restored after restart. The MyGo terminal handles exit output, input, selection, copying, pasting, and resizing.

Native loading occurs off the UI thread. A generation guard disposes of late results after replacement, library changes, or shutdown. Exit notifications return to the UI thread and check session identity, so a previous process cannot dismiss a replacement. Each session records its own exit, so the running indicator stops even when a nonzero status retains the terminal.

The popover stays empty while loading. A creation error appears in its place, and reopening retries. MyGo bundles the native emulator from Onevra’s plugin manifest. A direct `go run` may download that emulator into MyGo’s cache on first use.

## Verification

Tests cover discovery paths, notes that still carry legacy archive metadata, pagination, search, real MCP client and HTTP calls, the tool inventory, and Host/Origin rejection. Installer tests verify copied content, refusal of an existing target, and rejection of a redirected parent.

Shell-function fixtures check login-shell invocation and success, failure, and interrupt statuses without calling an installed Agent. Terminal tests check launch arguments, unknown commands, popover size and dismissal, settings choices, header order, the running indicator, and stale completions and exit notifications. They do not launch an Agent.

An opt-in native test renders the terminal from a connection fixture in light and dark themes. It keeps the session across dismissal, closes it on a library change, and dismisses the popover when the connection exits, without launching a shell or Agent. Further native tests use a controlled shell fixture: success and interrupt dismiss the terminal, and a nonzero failure keeps the output. They never launch an Agent.

Permissions, actual shell and Agent interaction, selected-shell startup files, Windows PTYs, and client skill reloading require user acceptance. See [MCP access](../mcp.md) for client setup and [Development checks](../build.md#development-checks) for automated commands.
