# Canvas

`internal/canvas` owns page preparation, resource handling, bootstrap, and export. `cmd/locus` owns the current browser/session and native actions. Onevra's WebView plugin and customized MyGo own native attachment, layout, clipping, focus, and input routing. The app uses their public APIs rather than copying a bridge.

## Entry points and lifecycle

A note owns exactly one `canvas/index.html` or `canvas/index.json`. HTML is a fragment with a required `canvas/main.js` module exporting `async main(data)`. Direct opening passes `null`. JSON contains exactly `template` and `data`; the template must resolve directly to an HTML canvas in the same library. Extra fields, dual entry points, template chains, absent main modules, and empty fragments fail visibly.

The host supplies Tailwind, base styling, system appearance, and a bootstrap that imports main once after page resources load. Initialization failures display a readable error. Timers and later handlers remain page-owned. Explicit theme settings update the retained page without reloading. There is no Node runtime, package installation, TypeScript transpilation, or web development server.

Preparation happens off the UI thread. A generation check discards results from older selections before native browser creation. Each page uses a random custom-scheme origin and its own rooted library handle. Leaving the note unregisters the origin and closes the browser. Settings hides the slot while retaining the document. Host shutdown releases all sessions, including preparation racing with shutdown.

A live page is not automatically reloaded after source writes. Explicit refresh/reopening rereads the fragment and template input and reruns main. The app can update entry-point mode after a file scan without silently discarding page interaction state.

## Resource and security boundaries

MyGo registers `locus-canvas` before browser creation and serves its requests through the app's router. It does not listen on TCP. Unknown/released origins and unsafe targets fail. The protocol accepts GET and HEAD only. Every resource request reads current files through the [library boundary](library.md#addresses-and-rooted-access), with no directory browsing or general filesystem endpoint.

Relative resources belong to the rendering canvas; module imports belong to the importing module. For JSON callers, these are template-owned resources. The bootstrap maps `fetch('locus://…')` and provides `locus.resourceUrl` for explicit library resource addresses. Image/source attributes and stylesheet-link attributes are adapted; arbitrary CSS text is not rewritten. Use relative CSS URLs or URLs returned by the helper in generated styles.

Complete `index.md` files and `?part=yaml` / `?part=body` return source data. Note navigation is separate from file access. Native navigation handlers keep the browser on its host page; logical note links go to the workspace. Normal external links use the OS browser. Browser permissions are denied and no Go services are bound to page JavaScript.

CSP permits local scripts/styles/fonts, HTTPS images and data fetch, and local/blob Workers. It blocks remote scripts/styles/fonts, frames, plugins, form submission, and arbitrary file URLs. Inline local scripts remain allowed for authored canvases and the embedded bootstrap/Tailwind runtime. HTTPS features can send requests to external services and still require CORS permission. CSP does not make user-authored code trustworthy or turn a library into an authorization boundary between its notes.

## Export

Export creates a new randomly named ZIP in the chosen directory, defaulting to Downloads. It contains `canvas.html`, Tailwind's license, and bounded snapshots of resources requested by the current session. The generated page uses archive-relative resources and preserves template data. Body/YAML part responses receive separate snapshot filenames. Existing outputs are never overwritten; failed writes remove the partial ZIP.

Export rejects pages whose main module has not loaded and sessions exceeding the 64 MiB snapshot budget. It does not crawl unused files, discover unloaded dynamic dependencies, save transient DOM state, or promise a standalone single-file HTML. Remote services still require a network. After unpacking, serve the directory over local HTTP when the browser restricts module imports, fetch, or Workers under file origins. See the bundled [authoring reference](../../source/resources/skills/locus/references/canvas.md).

## Platform and verification

The selected Onevra branch supports experimental native composition on macOS and 64-bit Windows; other platforms report unsupported. Rectangular clipping and overlays are supported by the dependency. Rounded native masks, transformed/opacity-animated browser surfaces, and unified DOM/toolkit Tab traversal are not guaranteed. Browser dialogs and fullscreen belong to the dependency's hidden donor window.

Unit tests cover templates, JSON escaping, CSP, resource traversal/symlink rejection, origin release, export contents, and conflicting entry points. macOS native tests verify real template initialization, resource fetch, DOM interaction, settings retention, and browser release. Full visual composition, pointer routing, input methods, accessibility, and Windows execution require target-platform acceptance. No external terminal or Agent is launched for these tests.
