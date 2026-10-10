# Desktop workspace

`cmd/locus` assembles the native application; `internal/workspace` owns asynchronous file projections and selection. Onevra supplies the sidebar Shell, virtual tree, Markdown view, command palette, settings, and native WebView slot. Product code does not duplicate their rendering or native bridges.

## Composition and navigation

The initial and minimum window size is 980 × 820 logical pixels. The sidebar starts at 252 pixels and cannot be reduced below that configured width. Shell owns collapse, resize, native materials, and titlebar controls. The content surface extends to the window top beneath Shell's floating header. Its top padding matches Shell's dynamic header height, keeping body content and controls below the titlebar without an additional fixed gap. Note and canvas pages fill the remaining content width and height without outer side or bottom margins. Markdown's scroll region provides theme-scaled reading padding around the summary, metadata, and body; this inner padding is separate from the titlebar safe inset and does not inset the canvas viewport. With a library open but no note selected, the content area stays blank except for actionable errors or feedback.

The virtual tree uses library-relative paths as stable identities. A keyed projection avoids searching the entire hierarchy for every rendered row. The app exposes open-library and command-search actions and displays optional Git modification markers. Selection loads a complete body in the background; organizing directories have no body. Back/forward history stays window-local and skips invalidated paths. Library switches clear history and component selection.

Menus, buttons, and shortcuts route to shared actions. Command/Ctrl+O opens a native directory picker, Command/Ctrl+E opens note/action search, Command/Ctrl+R or F5 refreshes, Command/Ctrl+, opens settings, and Command/Ctrl+backtick requests an external Agent. Native editing menu roles allow the focused browser/input to handle editing. Full browser/toolkit keyboard traversal remains subject to the experimental WebView backend.

## Async state and reading

Each background task opens an independent rooted handle. Scan and read tickets reject old results after library switches, selection changes, and clears. Refresh retains existing content while rebuilding, verifies selection against the new tree, and reloads the selected body. A refresh requested during scanning schedules a further reconciliation. Errors remain visible; an old failed scan does not overwrite a newer library.

The app caches parsed Markdown independently of frames. Summaries and custom YAML are displayed above the body; custom values currently use a readable YAML presentation rather than a property editor. Local Markdown images are decoded outside frame builders through the library resource boundary, with count, file-size, and pixel limits. Remote images are not fetched automatically. Onevra owns selectable text, tables, and code presentation.

Visible `locus://` note links and relative links to existing projected notes navigate internally. HTTP, HTTPS, and mail links go to the operating system. Other destinations are not executed. Toolbar actions copy a reference, open the note in the selected editor, open its directory, confirm archive, and export a canvas where present. Skills and content mutations are explicit user actions.

## Canvas and settings

Canvas selection prepares resources in the background and creates the native browser on the UI thread outside rendering. Leaving closes it; settings hides and retains it. Ordinary file watching updates note facts and canvas entry-point existence but does not rerun a live page's scripts. Refresh or reopening reruns the saved canvas. See [Canvas](canvas.md) for lifecycle and export details.

Without a selected library, a standalone Shell replacement page prompts the user to open a library and keeps settings accessible. Selecting a library restores the workspace without changing its sidebar presentation. Settings opens in a modal dialog over either page, with a close button, Escape, and backdrop dismissal; closing it retains the selected note and browser document. The command palette is a native overlay. Application startup uses a single-instance lock; a second invocation activates the retained window. Window closing saves presentation state and quitting flushes settings. See [Settings and persistence](settings.md).

## Acceptance boundaries

Headless tests exercise out-of-order task completion and removal/error handling. Opt-in native tests open temporary Markdown and template-canvas fixtures, verify real browser initialization and local fetch, execute DOM interaction, retain state across settings, and release the browser on departure. Captures of the MyGo surface alone do not include OS-composited materials or prove browser/toolkit overlay geometry.

Real acceptance still covers native picker interaction, long-list scrolling, tree keyboard paths, text selection, themes and languages, resizing, canvas input and overlays, accessibility, and close/reopen behavior. macOS checks do not establish Windows or Linux behavior. Sidebar ordering, icon upload, font controls, richer attachment/gallery extensions, and full window-bounds restoration are not implemented.
