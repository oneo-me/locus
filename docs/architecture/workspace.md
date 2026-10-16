# Desktop workspace

`cmd/locus` assembles the native application; `internal/workspace` owns asynchronous file projections and selection. Onevra supplies the sidebar Shell, virtual tree, command palette, and settings; MyGo supplies the Markdown view and the native WebView element. Product code does not duplicate their rendering or native bridges.

## Composition and navigation

The initial and minimum window size is 980 × 820 logical pixels. The sidebar starts at 252 logical pixels and can be resized between 252 and 520 logical pixels. Shell applies the same limits when restoring a saved width. Shell owns collapse, resize, native materials, and titlebar controls. The content surface extends to the window top beneath Shell's floating header.

Markdown's scroll viewport starts at the window top so its generated background also covers the titlebar region. The article's inner top padding reserves Shell's dynamic header height, and its scrollbar track starts 8 logical pixels below that header. Canvas and empty views instead inset their outer content below the header. Note and canvas pages fill the remaining content width and height without outer side or bottom margins.

Markdown uses a centered reading column capped at 832 logical pixels, with 16-pixel horizontal padding and 22-pixel spacing between metadata and body. Its top starts 56 pixels from the article origin, accounting for the floating titlebar's reserved inset. Bottom padding is 56 pixels. These reading insets do not inset the canvas viewport.

Canvas summaries have 16-pixel horizontal and 12-pixel bottom padding, with no extra top padding beyond the floating header's reserved space, followed by a full-width, 1-pixel theme-colored divider above the canvas. Notes without a summary omit both the summary spacing and divider. With a library open but no note selected, the content area stays blank. Errors and confirmations appear as toasts.

The main header presents the selected note title and its actions, with a terminal button immediately before the trailing More actions menu and available even without a selected note. Its 600 × 500 logical-pixel Popover contains only the built-in terminal view; see [terminal sessions](mcp.md#built-in-terminal). Once a Markdown article scrolls below its top, a theme-tinted MyGo glass background and a 1-pixel theme-colored bottom border appear behind the header. Both disappear on returning to the top. Switching notes or libraries resets the article scroll position.

A rectangular, non-interactive layer spans the full content width from its top edge through the header height, preserving the original control positions and spacing. The material blurs the article content behind it and follows light and dark themes. The header remains unpainted for canvases and when no note is selected.

The sidebar presents a search-field-style button below its header that opens the command palette. Back and forward navigation is right-aligned in the sidebar header on the same row as Shell's collapse toggle, using Onevra's sidebar-header action slot. These actions disappear with the collapsed sidebar.

The search trigger follows Onevra's translucent text-field styling while remaining a button that opens the command palette. Its bottom toolbar places Settings and a compact relation-graph action on the left and an icon-only MCP status button immediately before the rightmost system, light, or dark theme menu. The graph action reuses the icon-button treatment with a smaller glyph, because Onevra fixes the icon size and the network glyph reads heavier than the settings and theme glyphs. MCP retains its status color, tooltip, and settings action. Refresh and the terminal remain available through commands and menus. Library opening remains available through settings, commands, and the native menu rather than a sidebar button or path label.

The virtual tree uses library-relative paths as stable identities. A keyed projection avoids searching the entire hierarchy for every rendered row. Compact rows and the selected page title share note icons: frontmatter Emoji or a decoded reserved PNG, with document, organizing-folder, or canvas icons as fallback. Rows also show optional small, fixed-size Git modification dots.

PNGs load asynchronously through the library resource boundary after each successful scan, including external asset changes. Generation checks isolate late results after library switches. Invalid or missing images retain the default icon. Decoding accepts PNGs up to 4 MiB and one million pixels, caches aspect-preserving 32-pixel thumbnails, and limits each projection to the first 512 image declarations. No image files are read during frame construction.

The tree's scroll viewport reaches the sidebar edges. Content padding keeps rows visually inset without insetting the scrollbar. Selection loads a complete body in the background. Organizing directories have no body. Back and forward history stays window-local and skips invalidated paths. Library switches clear history and component selection while retaining each library’s tree expansion; see [presentation state](settings.md#presentation-state).

Mouse back and forward side buttons delivered to the MyGo workspace surface use the same history, including when the sidebar is collapsed. They do not navigate behind Settings, command, image-preview, or library-picker dialogs. Side-button delivery over the native canvas browser remains dependent on the MyGo backend and requires separate acceptance testing.

Menus, buttons, and shortcuts route to shared actions. Command/Ctrl+O opens a native directory picker, Command/Ctrl+F opens note/action search, Command/Ctrl+R or F5 refreshes, Command/Ctrl+, opens settings, Command/Ctrl+backtick toggles the built-in terminal, and the application menu checks for updates. Native editing menu roles allow the focused browser/input to handle editing. Full browser/toolkit keyboard traversal remains subject to the MyGo WebView backend.

## Async state and reading

Each background task opens an independent rooted handle. Scan and read tickets reject old results after library switches, selection changes, and clears. Refresh retains existing content while rebuilding, verifies selection against the new tree, and reloads the selected body. A refresh requested during scanning schedules a further reconciliation.

Background tasks and event callbacks report outcomes through queued notices because only frame builders hold a UI context. The active page drains the queue into MyGo toasts. Shell renders them with Onevra's look at the top center, clearing the floating titlebar. MyGo manages timing, pausing, keyboard dismissal, and announcements.

Multi-line errors use their first line as the title and the rest as the description. A model error reports once until it clears, so a failing scan or watcher does not repeat every frame. An old failed scan does not overwrite a newer library. Canvas load failures keep their in-place message in the canvas pane and also report a toast.

MyGo's Markdown view parses the body incrementally and keeps unchanged blocks across frames. The app's own projections of gallery and attachment fences, together with decoded images, are recomputed only when the selected note or body changes. Summaries and custom YAML share one tinted, clipped panel above the body. YAML mappings and arrays expand into recursive label/value rows rather than serialized YAML or an editable property form.

Article backgrounds derive a stable cool palette, one of seven backdrop variants, and four to six geometric accents from the note title. Layout tables, shape families, deterministic jitter, and per-shape tones determine the composition. Light and dark themes change colors without moving geometry. The wash fades across 268 logical pixels, while texture stays at full strength through the first 70% before fading. Shapes use a centered 360 × 100 design space capped at 208 pixels high, with separate vertical and horizontal fades.

Decorations scroll with the article, do not intercept text interaction, and require no animation loop. Canvas pages retain their own presentation. Local Markdown images are decoded outside frame builders through the library resource boundary, with count, file-size, and pixel limits. Remote images are not fetched automatically. MyGo's Markdown view owns selectable text, tables, and code presentation.

### Markdown galleries

The exact, case-sensitive `gallery` fence renders Markdown images as wrapping, 180-pixel-high thumbnails. Pointer or button keyboard activation opens a full-window, contain-fit preview with no panel surface, alternative text, and a close control in the window's top-right corner; the control, Escape, or backdrop dismissal closes it. Image syntax, reference definitions, source order, and repeated images are supported. Empty blocks, missing alternative text, non-image content, and non-local destinations display a localized validation error. Missing or unsupported images retain an unavailable placeholder.

Gallery parsing is cached with the selected Markdown body and uses MyGo's custom code-block builders. The shared asynchronous image loader discovers gallery destinations as well as ordinary images, deduplicates decoded targets, limits a note to 64 pictures and 48 million retained pixels, and scales larger pictures down to share that budget instead of marking them unavailable; sources beyond 64 million pixels are not decoded. Builders do no file I/O. Note changes and image reloads dismiss previews; generation tickets reject stale image results. Remote gallery resources are rejected.

Gallery destinations belong to the current note's `assets/` directory; legacy bare filenames resolve there. Existing files stored beside `index.md` must be moved into `assets/` explicitly before they can load. Absolute paths, traversal, cross-note addresses, queries, and fragments are rejected. The [bundled attachment reference](../../source/resources/skills/locus/references/attachments.md) owns authoring examples.

### Markdown attachments

Legacy `attachments` fences render ordered, keyboard-accessible file buttons through MyGo's custom code-block interface. The cached parser accepts only Markdown links to direct `assets/<filename>` children with matching filename labels, excluding the reserved icon. Invalid blocks show a localized explanation. Activation checks the owning note and regular file through rooted library access in a background task, then reveals it in the system file manager; missing or unsafe files report an error. Rendering performs no filesystem I/O.

Visible `locus://` note links and relative links to existing projected notes navigate internally. HTTP, HTTPS, and mail links go to the operating system. Other destinations are not executed. Header buttons copy a reference, open the note in the selected editor, and open its directory.

All notes have a trailing More actions menu with Upload icon, which opens a native PNG picker. Canvas notes also offer export, disabled until a canvas session is available and while an export is running. Upload captures the target note and library, performs image processing and writes in the background, and refreshes the projection on success. Cancellation leaves files unchanged. Skill installation is an explicit user action.

### Relation graph

A sidebar action beside Settings and the command palette open the relation graph on Shell's replacement page, the same independent page the library picker uses. It covers the whole window, hides the sidebar and default content, and keeps the floating header as a drag region. A title bar below that header names the page, carries a display-options menu, and places a close action at its trailing edge; note-link navigation and double-clicking a node also leave the page and select that note. The graph action is disabled without a library.

Nodes are notes with stable library-relative paths, grouped by their top-level directory. Edges combine the parent-to-child hierarchy, drawn faintly, with Markdown links between existing notes. The display-options menu can show a note's attachments: regular, non-hidden files directly inside its `assets/` directory except the reserved `icon.png`. Attachments appear as small nodes joined to their owning note, use the reserved `assets` group name for their own color, and are not openable. The choice survives restarts in presentation state. Link destinations resolve `locus://` addresses and library-relative paths that name a note directory or its `index.md`; images, autolinks, external URLs, fragments, traversal, and unsafe segments are ignored.

Opening the graph resets the camera; dragging pans, the wheel or a pinch zooms around the pointer. A library with no notes renders an empty state rather than an error. Selecting a note records history like any other navigation.

Link extraction reads note bodies in one background task per library scan, governed by the same ticket and generation checks as other asynchronous work. The same task lists attachment files through rooted, symlink-rejecting access. The hierarchy and nodes render immediately from the current projection, and link edges and attachments appear when the read finishes. Frame construction performs no file I/O, and the projected nodes and edges are cached until the model, attachment visibility, or link results change.

## Canvas and settings

Canvas selection prepares resources in the background and creates the native browser on the UI thread outside rendering. Leaving closes it; the browser stays visible beneath the Settings dialog. Ordinary file watching updates note facts and canvas entry-point existence but does not rerun a live page's scripts. Refresh or reopening reruns the saved canvas. See [Canvas](canvas.md) for lifecycle and export details.

Without a selected library, a standalone Shell replacement page prompts the user to open a library and keeps settings accessible. Selecting a library restores the workspace without changing its sidebar presentation. Settings opens in a modal dialog over either page, with a close button, Escape, and backdrop dismissal. Closing it retains the selected note and browser document. The command palette is a native overlay.

Application startup uses a single-instance lock. A second invocation activates the retained window. On macOS, closing the main window saves presentation state. Packaged builds hide the retained window and stay active in the Dock. Development runs started through `go run` or `go tool mygo dev` exit so they leave no Dock activity.

Dock activation or a second invocation restores a retained window with its workspace, canvas, and any running terminal session intact. File watching and MCP continue while hidden. Explicit Quit, including Command+Q, closes the window, releases services and sessions, and flushes settings. Other platforms exit when the last window closes. See [Settings and persistence](settings.md).

## Acceptance boundaries

Headless tests exercise out-of-order task completion and removal or error handling. A headless test pins the development-versus-packaged close decision. A foreground-only macOS native lifecycle test (`LOCUS_NATIVE_FOREGROUND=1`) exercises close-to-hide, presentation saving, retained-window restoration, and cleanup when closing for quit in packaged-equivalent settings. Actual Dock clicks and menu or keyboard quit remain target-platform acceptance checks.

Opt-in native tests open temporary Markdown and template-canvas fixtures, verify real browser initialization and local fetch, execute DOM interaction, retain state across settings, and release the browser on departure. Atmosphere parity fixtures independently evaluate the reference formulas and tables for all six layouts, ten shape families, seven backdrop variants, and both themes, including CJK and astral titles. Native captures exercise the reference titles at two window widths in both themes.

These checks establish numeric source parity and exercise MyGo rendering, not pixel equivalence with GPUI. Arc paths use cubic Béziers, and colors enter MyGo's color representation at paint time. Texture cells are submitted as small paths rather than viewport-wide rotated masks, avoiding eviction of text glyphs from the shared rendering atlas. Captures of the MyGo surface alone do not include OS-composited materials or prove browser or toolkit overlay geometry.

Gallery tests cover fenced-block dispatch, syntax validation, local-path restrictions, missing-image placeholders, and preview opening and dismissal. The macOS reading smoke test loads gallery images, captures thumbnails and a preview, and verifies dismissal on note departure. A full native-suite run encountered a MyGo accessibility list-state nil dereference; the isolated reading test passed. This does not establish complete native keyboard or accessibility acceptance.

The native reading fixture includes legacy attachment rows in light and dark captures. File-manager activation and full keyboard acceptance remain manual checks. Native icon checks also process a large PNG, update a temporary note, and verify watcher-driven icon refresh.

Graph tests cover hierarchy, link and attachment projection, cached rebuilds, the title bar options menu, the sidebar and command entries, the disabled state without a library, the empty state, and note opening. Native pan, zoom, node dragging, and double-click acceptance remain manual checks.

Real acceptance still covers native picker interaction, long-list scrolling, tree keyboard paths, text selection, themes and languages, resizing, canvas input and overlays, accessibility, and close or reopen behavior. macOS checks do not establish Windows or Linux behavior. Sidebar ordering, font controls, and full window-bounds restoration are not implemented.
