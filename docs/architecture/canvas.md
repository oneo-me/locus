# Canvas

## Purpose and runtime boundaries

Users and Agents maintain canvases as local web content for complex interactive pages and design previews. Each note owns one canvas. The native workspace supplies title, metadata, navigation, and commands; the page occupies the area below them. Locus reads files directly through a WebView custom protocol, includes Tailwind, and does not start Vite, install npm dependencies, or require system Node.js. Native canvases support macOS.

Scanning recognizes only `canvas/index.html` or `canvas/index.json` as entry points. Other files alone do not enable canvases. Ordinary canvases use an HTML fragment in `canvas/index.html` and a required `canvas/main.js`, whose entry module exports `async function main(data)`. The host waits for page resources, imports the module once, and waits for initialization. Direct opening passes null. Initialization errors produce recoverable guidance; canvases handle errors in later events and asynchronous tasks themselves. Scripts use native browser JavaScript and ES modules. TSX, npm package specifiers, React hosts, and source transformation are outside the runtime protocol.

## Templates and resource ownership

Callers write only the mutually exclusive `canvas/index.json`, containing exactly template and data. template is an explicit `locus://` note address in the current library and must directly identify an HTML/main.js template; template chains are forbidden. data is any JSON value passed unchanged, with its input defined and validated by the template. The host does not merge defaults or transmit callbacks, children, or Context through JSON.

Relative HTML and fetch URLs belong to the rendered canvas; template-relative resources belong to the template. Module imports resolve relative to the importing module, and CSS URLs relative to the stylesheet. Callers pass resources through data using explicit library addresses. `locus.resourceUrl` maps addresses. fetch can read canvas files, attachments, complete notes, raw YAML, and bodies within the library. Resources are read per request without caching. Reads reject boundary traversal, symlinks, node_modules, and arbitrary local files. The bundled [skill canvas reference](../../source/resources/skills/locus/references/canvas.md) owns detailed format, runtime, and safety contracts.

## Lifecycle and themes

Saving files does not automatically update open pages. With canvas focus, Command/Ctrl+R or F5 refreshes the owning note and reruns its entry point. Leaving and reentering also recreates the page. Switching notes or libraries releases old pages and transient interaction state. Theme changes synchronize root dark/light and color-scheme without reloading. Internal view changes should retain state in other sections.

Basic pages and local resources work offline. HTTPS images and data requests, canvas-local Workers, and blob Workers are allowed. Network requests still require server CORS support. Remote scripts, styles, fonts, iframes, form submissions, arbitrary local files, and shell interfaces are forbidden. HTTP/HTTPS links open in the system browser. Static or dynamic canvas links, including whole cards, may use `locus://` note-directory addresses in the current library to open notes in the current workspace, reusing sidebar selection and back/forward history. New-window links also open in the current workspace. Addresses do not append `/index.md`, queries, or fragments; resource file addresses remain for reading. Invalid, missing, archived, or non-note targets leave selection unchanged and are not sent to the system browser.

## Canvas export

Canvas titlebars provide Download canvas in the more menu. Background preparation shows a busy menu trigger and disables the download item; completion reports the saved location, and failures report reasons. Operations bind the note, title, and export directory captured at click time. Settings → General → Export directory selects the destination, defaulting to system Downloads and persisting in TOML.

Exports read saved content, including templates and caller data, Tailwind, referenced local resources, and loaded dynamic resources. They exclude transient page state and unrelated attachments. Note navigation links retain their original addresses without bundling target notes; exported pages do not provide in-app note navigation. Referenced note YAML and bodies become separate resource snapshots, with the same ownership as live reads. Single-file results save as `<note title>.html`; multifile results save `canvas.html` and resources in a same-named directory. Duplicate names receive automatic numbering without overwriting. Failures leave no partial output. Pure entry modules may be embedded; modules with imports or import.meta remain files. Module and Worker restrictions under file origins may require a local HTTP server. Online data still requires a network.

## Native WebView composition

Native canvases currently support only macOS and use the independent `oneo-gpui-webview` package. Pages must remain performant while GPUI components, menus, command palettes, dialogs, and sheets appear above them and receive input. The implementation does not modify GPUI dependencies or display pages through per-frame screenshots or CPU pixel readback. It composes system WebKit and transparent GPUI layers natively, without exposing web texture handles or bundling Chromium. Frame rates depend on page, system, and device; there is no uniform numeric guarantee.

The WebView entity owns the native host, page, focus handle, and subscriptions. lb-wry manages WebKit configuration, navigation, IPC, and scripts. An AppKit clipping container sits below the GPUI native view, with a transparent input-routing view above it. Routing chooses targets from current-frame GPUI hitboxes, actual pointer position, visibility, and pointer capture. Original events pass to WebKit, preserving native scrolling, selection, editing, and IME. Standard GPUI occlusion prevents the page from receiving input in overlay regions. Focus subscriptions synchronize the first responder between page and GPUI controls.

A window-local event monitor supplies GPUI mouse movement when other native responders own focus, retaining original events for AppKit/WebKit tracking without moving keyboard focus on hover. Cursor management is acquired only in active windows. On entry to the page, the host first calls `invalidateCursorRectsForView:` for the GPUI view while window cursor management is enabled, matching GPUI's native cursor-change path. It then pauses automatic window cursor-rectangle management and calls `discardCursorRects` on the GPUI view. This handoff also occurs when GPUI has kept the default arrow since activation. Each page entry resets a residual GPUI cursor to the arrow once; WebKit then manages link and text cursors without continuous overrides. Leaving, hiding, or releasing the page, or window deactivation, restores only management acquired by this host.

Mouse presses originating in the page also dispatch to GPUI so overlay outside-click listeners dismiss menus. Matching drags and releases complete even when the pointer leaves or the page hides, avoiding stuck press state. Original events still reach WebKit, preserving page clicks. Presses starting in GPUI follow AppKit dispatch without duplication. The native bridge tracks press ownership until release, preventing page hit testing and cursor takeover. Sidebar drags retain ownership even without GPUI pointer capture when crossing into the page. Window deactivation or movement with no pressed buttons clears interrupted GPUI press state.

Windows use transparent or blurred backgrounds, and GPUI ancestors covering the page do not paint backgrounds. Titlebars, metadata, sidebars, loading, and error regions paint their own semantic surfaces. Composition supports rectangular layout and clipping with non-overlapping WebViews. GPUI opacity, rotation, and rounded masks do not automatically affect underlying native pages. Custom interactive GPUI overlays must occlude. Pages use opaque backgrounds; this composition is not arbitrary GPUI texture rendering.

Pages remain visible and running under overlays. Settings hides and retains them when replacing the workspace. Hidden, released, or fully clipped pages do not receive input; hiding a page holding native focus returns focus to GPUI. Note switches release old pages, and the host removes native views and input callbacks. The independent package provides no shell or local-file capabilities. The canvas package owns Locus links, downloads, IPC, and shortcut policy.

## Implementation boundaries

`locus-canvas::CanvasView` owns WebView, theme, focus, refresh, and page lifecycle. `CanvasEvent` emits note navigation, app command, window close, and external-link intents. The workspace owns current-note identity, creation, release, and layout. The settings entity owns the export directory. Markdown rendering does not participate in canvas loading.

`view` manages page lifecycle, `resources` owns safe resource mapping and MIME, and `export` collects dependencies, creates snapshots, and saves without overwriting. The custom protocol reads files in the background per request. The Tailwind browser script is embedded in the app, and entry points execute once per page-load lifecycle. During export, the workspace captures the note and directory, then prepares and saves in the background.

The host shortcut script retains a private message identifier in a closure and allows only trusted, non-repeated browser key events to send launch intents. Rust IPC validates a per-WebView identifier, generated with UUID v4.

See [WebView integration](webview.md) for generic construction, visibility, and composition boundaries. GPUI uses public APIs from the published version. cc compiles the Objective-C bridge, with macOS deployment target 13.0 by default, overridable through MACOSX_DEPLOYMENT_TARGET.

## Verification boundaries

Rust regressions cover mutually exclusive HTML/JSON entry points, main data, template-chain rejection, resource ownership, safe paths, refresh, export references and module identity, snapshots, collision avoidance, and failure cleanup. Run relevant library, canvas, workspace, settings, preferences, and MCP package tests.

From the repository root, run `node --experimental-vm-modules --test source/crates/locus-canvas/tests/*.cjs` for entry lifecycle, initialization errors, resource mapping, offline export fetch, and native editing-shortcut bridges. The development environment must resolve jsdom. Node and jsdom are test-only dependencies.

Native bridge regressions cover input ownership, overlay exclusion, focus, hiding, clipping, release, and cursor handoff without constructing a full AppKit/WebKit window. Acceptance requires overlay coverage without click-through, page clicks after menu dismissal, GPUI/WebKit cursor handoff, Chinese IME, selection and editing, themes, settings transitions, window resizing and display changes, VoiceOver, and CPU/GPU usage for dynamic and static pages. Headless tests do not establish native display or performance acceptance.
