# Desktop workspace

`locus-workspace` orchestrates navigation, current notes, focus, history, canvases, and product actions within a window. The [Library](library.md) owns file content, the [Settings entity](settings.md) owns runtime preferences, and the [Shared shell](shell.md) provides generic window and panel behavior.

## Layout and pages

The main window contains document navigation on the left and the current note page. Locus's initial and minimum window size is 980 × 820 logical pixels. The sidebar's default and minimum width is 252; sizes come from `locus-preferences::config`. The shared shell coordinates collapse, resizing, and divider reset without changing native window width when collapsing. Window-level capture listeners keep resizing across canvas or occluded regions. Releasing stops the drag and saves the width.

Ordinary notes display Markdown. Notes with a canvas entry point display their single canvas below the titlebar and header metadata. Header height follows content; long headers scroll independently while the canvas fills the remaining space. Canvas pages use all available width with side padding and clearance for window buttons. Their titlebars use semantic backgrounds and bottom separators; summaries and YAML reuse ordinary note content. A fixed 1 px inset on the canvas's left does not scale with interface font size.

Entering a canvas note creates a page automatically and preserves reading focus. Leaving, clearing selection, or switching libraries releases it. Settings replaces the workspace in the same window, hiding and retaining the page. Returning restores workspace and focus. Watching canvas addition or removal updates page mode. Saved changes to existing source load on manual refresh or reopening. See [Canvas](canvas.md) for loading, themes, resources, export, and failures.

## Document navigation

`workspace/sidebar.rs` uses GPUI Kit's `base::Tree` and a retained `TreeState` for virtual lists, keyboard navigation, and tree accessibility semantics. The app provides row presentation, note icons, attachment counts, Git hints, context menus, and sibling ordering. Node identities derive from document paths; same-named nodes cannot share identity.

Startup, selection, reordering, library switches, and scan updates synchronize the Tree projection while preserving valid expansion state. Expansion events return to the workspace. Tree selections carry the actual expansion state and must not re-expand a node merely because it has children. Up/down changes selection; left/right and Enter expand or collapse branches.

Lists use 0.5 rem horizontal padding, with scrollbars aligned to the scroll region's right edge. The workspace owns a common start time for sibling-order animations, lasting 150 ms. Rebuilt rows sample existing progress; completion clears it. Library switches or projection replacements cancel animation, and reduced-motion mode presents final positions directly. Order persists separately in `sidebar-order.toml`, without rewriting document directories.

Document and settings navigation use selection and hover fills at 5% opacity of theme foreground, with full foreground for selected text and icons. The bottom displays read-only MCP status; the shell supplies the generic settings button. See the shared shell for debug FPS statistics and lifecycle.

## Selection and history

- Selecting a note updates ancestor expansion, scrolling, and window-local back/forward history, then reads the body in the background.
- Ordinary selection clears forward history. Back, forward, and mouse side buttons use the same transition. Library switches clear history, and invalid paths are removed.
- Startup restores only selections valid in the current projection. Deleting or archiving the current note clears its page.
- Refresh retains content and reading position. Actual body changes or note switches clear invalid selections and transient interaction state. Reads are isolated by library path and generation; late results cannot overwrite new selections.

## Reading and metadata

Ordinary note titles and icons appear in the titlebar. Summaries, descriptions, custom YAML, and body appear in the reading area, with 3.5 rem top spacing. Custom properties present current-note values, retaining objects, arrays, empty collections, null, and multiline strings.

`workspace/note_metadata.rs` recursively composes GPUI Kit DescriptionList. Each level measures the longest key with the actual interface font, weight, and window rem, rounds column width upward, and keeps keys on one line without a proportional cap. Values wrap. Arrays retain order and are numbered from 1; nested objects and arrays expand recursively. Cells use 0.125 rem vertical spacing, with component borders, separators, and label backgrounds disabled.

Markdown uses `gpui_kit::component::text::TextView`. The current note retains an `Entity<TextViewState>`. Reloading the same note updates its body; switching releases old state. The framework owns generic parsing, layout, continuous selection, select all, and copy. Code blocks display fully, use the code font and theme foreground, and participate in whole-document selection and copy. They have no syntax highlighting or separate copy and expand/collapse buttons.

`locus-ui/src/markdown_extensions/` adds galleries, attachments, images, link icons, and navigation callbacks through block and inline plugins. Parsing produces data; rendering reads current attachments, language, and icon projections. Extension declarations are excluded from plain-text copy output. Extensions reuse the `markdown` parser already used by TextView. See the library subject for file boundaries.

## Actions and focus

The note titlebar toolbar provides copy reference, edit note, open directory, and a more menu in that order. The more menu offers icon upload and archiving. Sidebar context menus retain edit, open directory, icon upload, and archiving, binding directly to their target notes rather than another current selection. Canvas notes also offer Download canvas in the more menu, disabled during preparation while the menu trigger shows a busy state; see the canvas subject. Editing uses a pencil icon in the toolbar and sidebar context menus.

Open the command palette from the macOS View menu or Command/Ctrl+E; the same shortcut closes it. It includes app commands, current-note commands, and visible notes from the current projection. Search does not preload all bodies. Menus, buttons, palette, and shortcuts reuse the same Actions and enablement conditions. Confirmation closes it; Escape cancels and restores focus. The palette and backdrop transition together.

Command/Ctrl+R and F5 refresh the focused canvas or otherwise reload the current body. Command/Ctrl+W closes the window, Command/Ctrl+, opens settings, and Command/Ctrl+backtick launches an external Agent. They also work under canvas focus. See [Library terminal](terminal.md) for handoff and duplicate-submit protection.

Application-menu About opens Settings → About instead of the native AppKit About panel. Check for Updates opens the same page and defers a Sparkle check. The workspace's action handlers share `show_about`; app-wide fallback handlers activate the retained main window when no focused view can handle the command. The menu remains available for source runs, where About explains that updating is unavailable rather than claiming to be current. See [About and update checks](settings.md#about-and-update-checks).

Select all, copy, cut, and paste use GPUI Kit editing Actions and standard shortcuts. `commands.rs` associates native editing commands through `MenuItem::os_action`. On macOS, the current native responder handles them: WKWebView handles page selections and editing targets, while GPUI inputs retain component behavior. Ordinary page text is read-only; cut and paste affect only editable targets.

Without a selected note or when selecting a directory without a body, main-content titlebar text and empty space drag the window. Tool buttons isolate clicks, drags, double-clicks, and disabled hit areas from window dragging. See the shell subject for platform window controls, materials, and closing, and settings for layout and state saving.

## Interface and state conventions

The workspace centers current content and precise operations. Spacing, alignment, text weight, and restrained boundaries establish hierarchy. Navigation and status remain supporting elements. Accent color marks primary actions, selection, focus, or explicit semantic states. Cards serve independent objects or layers that need elevation.

Themes, colors, fonts, radii, shadows, and states use GPUI Kit Theme semantic tokens. Generic components, icons, scrollbars, and window roots reuse the framework. Icons share a family and size. Content presentation avoids decorative emoji, neon, gradient text, and motion without information. Small compositions address behavior the framework cannot express reliably while preserving native controls and desktop interaction semantics. See the shell for native materials and opaque content surfaces.

English and Chinese share information hierarchy and control structure. Language changes synchronize business copy, component locale, and native menus. Paths, long titles, code, tables, and localized text wrap or truncate safely. Window resizing and overlays must not cause page-level horizontal overflow.

Loading, empty, error, disconnected, disabled, selected, conflict, and completed states reflect reality, explain impact, and expose available actions. Users can identify the current library, note, target, and persistence outcome. Empty states suggest a next step, errors offer recovery, and disagreement between files and derived views exposes conflicts. Deletion, overwrite, rollback, external permissions, and operations that may send content to remote services explain their object, scope, and impact before execution.

Accessibility uses WCAG AA as its baseline, including complete keyboard paths, visible focus, logical Tab order, sufficient contrast, and screen-reader names and states. State colors pair with text, icons, or accessible names. Essential actions do not depend solely on hover, color, or animation. Motion conveys feedback or spatial change, preserves reading position, and respects reduced-motion settings. Validate desktop structure across window sizes, content lengths, overflow, and overlays.

## Known issues

Existing hardware reports describe right-side clipping in Chinese body text containing inline code and in table cells containing link buttons, with no recorded fix. Native inline code still overflows after removing Locus plugins and custom fonts. GPUI Base 0.6.6's `text/inline_flow.rs` estimates wrapping character by character but paints complete shaped spans, potentially underestimating mixed Chinese text width. In an existing 18 px system-font comparison, the sample `\u4E2D\u6587\uFF0C\u4E2D\u6587` (Unicode escapes) was estimated at 80.46 px but measured at 89.46 px. A 300 px wrapping comparison estimated 294.57 px but measured 303.57 px. Link-button measurement and painting have not been independently verified. These are existing issue records, not new measurements from documentation maintenance.

## Verification boundaries

Workspace tests cover Tree projections and stable identity, selection after ordering, expansion/collapse, shared animation deadlines, navigation history, late tasks, window-level dragging, target binding for menus, metadata projections, and interface-save order. `locus-ui` tests cover TextView and extensions.

Acceptance requires checking long-list scrolling, dragging, ordinary and canvas notes, mixed text and tables in narrow windows, cross-paragraph selection, settings transitions and focus, menus and keyboard actions, both themes and languages, fullscreen, and maximization. Headless regressions and compilation do not replace native-window interaction, text-layout, or accessibility acceptance.
