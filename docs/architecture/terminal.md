# Library terminal

This document owns current behavior, implementation boundaries, and acceptance requirements.

## Purpose

Users launch installed Agents from the current knowledge workspace to work at the same library root. Locus selects only the terminal, Agent, and launch directory. External software owns terminal appearance, sessions, authentication, models, permissions, and process lifecycle.

## Product behavior

- Terminal settings offers the system terminal and Ghostty; Windows offers only the system terminal. The system terminal is the default.
- Agent choices are Codex, pi, OpenCode, and Claude Code, with Codex as the default. Commands are `codex`, `pi`, `opencode`, and `claude`, without unspecified initialization commands, arguments, or prompts.
- The shortcut remains Command+backtick, or the corresponding Control combination on other platforms. The command palette offers the same Open Agent… action. Each press triggers once; holding the key does not repeatedly create windows. Duplicate submissions are rejected during launch handoff. The shortcut also works with canvas focus.
- The launch directory is always the current library root captured at activation, not the current note directory. The command is disabled without a library; an invalid directory produces an error. Opening a terminal does not switch the Locus page or expand a sidebar.
- On macOS, the system terminal is Terminal. Ghostty uses the AppleScript interface in version 1.3.0 or later. Each operation requests one Agent session and reuses a running app. Terminal creates a new window; Ghostty creates a tab in the most recently used existing window or creates a window if none exists. It does not type commands into existing sessions. zsh login and interactive configuration resolves installed Agents. The shell remains available if an Agent exits or its command cannot be found.
- Users install the terminal and Agent first. Terminal launch failures appear in app notifications. Agent command, authentication, and runtime errors appear in the external terminal. Locus does not describe a terminal handoff as successful Agent execution.
- Locus does not save, close, or restart opened external sessions. Library switches, selection changes, and quitting do not affect them. Subsequent launches use the new selection.
- Preferences store only external terminal and Agent selections. Old embedded-terminal fields do not participate in current configuration. Reading old TOML preserves other valid preferences; normal saves remove fields outside the type.

## Implementation boundaries and acceptance

`locus-preferences` defines terminal and Agent choices; the settings entity owns the sole runtime selection. `locus-workspace/src/workspace/terminal.rs` owns actions, background handoff, duplicate-submit protection, and failure notifications. `locus-ui/src/platform/terminal.rs` owns platform commands and path quoting. Locus uses external terminals; `oneo-gpui-terminal` remains an independent generic component.

Both macOS terminals first receive AppleScript `launch`, then create a session, then `activate`, avoiding a default window from normal startup. Terminal uses `do script`. Ghostty uses `new surface configuration` with `initial working directory` and `command`, then `new tab in front window with configuration` when a window exists or `new window with configuration` otherwise. Directories and commands pass through separate arguments and shell quoting. See [Apple's launch definition](https://developer.apple.com/library/archive/documentation/AppleScript/Conceptual/AppleScriptLangGuide/reference/ASLR_cmds.html) and [Ghostty AppleScript documentation](https://ghostty.org/docs/features/applescript). Ghostty's normal session restoration follows user configuration; Locus does not modify terminal settings or close existing windows. Windows runs Agents in new system consoles. Users verify target-platform launching, system authorization, shell configuration, and Agent installation. Do not automatically launch terminals or Agents for acceptance testing.

Acceptance covers terminal and Agent preference persistence; removal of old fields while retaining valid preferences; missing or invalid libraries; paths containing Chinese, spaces, quotes, newlines, and shell special characters; actual launch directories and commands; exactly one new Agent session when terminals are stopped, running, or running without windows; Ghostty tabs without additional windows, use of the most recently used window among several, and exactly one new window when none exists; preservation of existing sessions; one activation per held key; canvas-focused shortcuts; missing terminals, missing Agents, and exit behavior; and settings or library switches affecting only subsequent sessions.

Run `cargo test -p locus-ui --locked platform::terminal::tests` for directory validity, command arguments, and path quoting. AppleScript compilation does not execute terminals or prove session reuse, authorization, or successful Agent startup.

## Independent terminal component

`oneo-gpui-terminal` is an independent generic GPUI component. Locus uses the external-launch workflow above. The component reuses `alacritty_terminal` 0.26's PTY and VT state machine with default features disabled. Its GPUI view maps the grid, input, selection, focus, and refresh. It accepts neutral configuration without reading Locus preferences or knowledge content.

Wide characters occupy two cells; the following `WIDE_CHAR_SPACER` generates no extra text. GPUI native interfaces handle IME. Composition appears only at the cursor, committed text enters the PTY, and control combinations follow terminal encoding. Changes to this component require its own package tests; external Agent-launch checks are not a substitute.
