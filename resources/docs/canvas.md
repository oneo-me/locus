# Canvas authoring

Each note can own one canvas under `canvas/`. Choose exactly one entry:

- `index.html`: an HTML fragment with a required `main.js` ES module exporting `async function main(data)`.
- `index.json`: a template invocation containing exactly `template` and `data`. Do not also create `index.html` or a caller bootstrap script.

With the standard MCP endpoint, edit files using local filesystem tools. On the full endpoint, the `canvas` content target addresses `canvas/index.html`; use `canvas_file` with `name: "index.json"`, `name: "main.js"`, or another relative path for the other canvas files. Respect content versions; upload tools accept binary resources.
A canvas replaces the Markdown reading area. Keep only YAML frontmatter in `index.md` when authoring a canvas note; put independently maintained prose in another note. Do not automatically discard existing Markdown when updating an existing mixed note. Do not use a Markdown `canvas` fence or an attachment reference to display a canvas.

## HTML format

Write an HTML fragment, not a complete page: omit `<!doctype html>`, `<html>`, `<head>`, and `<body>`. The host supplies the note title, theme and Tailwind. Keep markup in `index.html`, initialization and interaction in `main.js`, and substantial styles in a CSS file. Small styles and non-executable JSON/text data blocks can remain in the fragment. Local third-party scripts may use ordinary `<script src>` tags.

`canvas/index.html`:

```html
<link rel="stylesheet" href="style.css">
<main class="min-h-full p-6">
  <button id="counter">Clicks: 0</button>
</main>
```

`canvas/main.js`:

```js
export async function main(data) {
  let count = 0;
  document.getElementById('counter').addEventListener('click', function () {
    this.textContent = `Clicks: ${++count}`;
  });
}
```

The host waits for the page's resources, imports `main.js` once and awaits `main(data)`. A directly opened HTML canvas receives `null`; a template receives the exact JSON `data` value. Even static canvases must export a main function, which may do nothing. The host reports entry/configuration errors, module import failures, missing main and rejected main calls; fix the cause and refresh. Initialization belongs in main, not in an automatically executed bootstrap. Timers and later event handlers own their subsequent errors.

## Presentation and lifecycle

The native canvas is supported on macOS. WebKit renders below the GPUI layer, keeping application overlays visible and interactive while the page runs. Settings hides and retains the page.

The canvas fills the content area below the title and metadata. Summary and custom YAML fields remain visible above it; long metadata can scroll independently. The canvas is excluded from attachment lists and counts. It has no separate tab, automatic-opening switch, refresh button, or context menu.

Use Cmd/Ctrl+A to select all, Cmd/Ctrl+C to copy, Cmd/Ctrl+X to cut, and Cmd/Ctrl+V to paste. These commands act on the focused input or the page selection using native WebView behavior. Cut and paste require an editable target. On macOS, the same commands are available in the Edit menu.

Saving a file does not reload an open canvas. With canvas focus, press Cmd/Ctrl+R or F5 to reload its owning note's canvas and rerun scripts. Leaving and reopening the note also reloads it. In-memory interaction state is lost on refresh or departure. An empty or unreadable file produces a recoverable message; fix it and refresh.

Combine multiple views in one canvas using tabs, sections, or disclosure panels. Preserve other sections' state during internal navigation. Use unique DOM IDs, scoped selectors, and separate script scopes.

The canvas uses the available content width. Use `dark:` classes or `.dark` selectors to follow the application theme; theme changes do not reload the page.

For a page that scrolls, use `min-h-full` (`min-height:100%`) on the root so the background continues below the first viewport. The host fixes the heights of `html` and `body`; `h-full` locks the root to one viewport. Use `h-full` only for a fixed frame with independently scrolling inner regions. Avoid a fixed pixel height for the whole page.

## Resources and network policy

Relative HTML resource URLs and fetches resolve from the rendering canvas’s directory; module imports resolve from their importing module, and CSS URLs from their stylesheet. Thus a template’s relative resources belong to the template, not the calling note; nested directories, JavaScript modules, CSS, images, fonts and data files are supported, without an extension allowlist. For example, `<img src="photo.jpg" alt="Photo description">` loads a local image. URL-encode filenames containing spaces or non-ASCII characters. Data and blob images are supported.

Local resources are read on request without caching. Canvas subdirectories and hidden resource files are allowed. Paths must remain in the resource tree; symbolic links, installed `node_modules` and directory browsing are unavailable. Use explicit library addresses to read other note resources. Read `locus-docs://attachments` through MCP for Markdown attachment syntax.

HTTPS images and HTTPS data requests are allowed, including map tiles and map data. Cross-origin requests still require server CORS permission. Canvas-local and blob Web Workers are allowed. Remote scripts, remote CSS and fonts, iframes, form submission, and arbitrary local file access are blocked. Basic canvases and local images work offline. Network features send requests to the corresponding service; handle failures and retries within the page. HTTP/HTTPS links open in the system browser.

## Reusable templates

Any note with `canvas/index.html` and `canvas/main.js` can serve as a template. A caller writes only `canvas/index.json`:

```json
{
  "template": "locus://templates/message",
  "data": { "message": "Template input" }
}
```

The two fields are required, and extra fields are rejected. `template` identifies an existing note in the current library, without a resource suffix. It must resolve directly to an HTML canvas, not another JSON invocation; chained or cyclic template configurations are rejected. HTML and JSON entries are mutually exclusive. `data` can be any JSON value; the template's main defines, validates and handles its input schema. The host does not merge inputs or infer defaults.

For this example, the template's HTML contains `<p class="result"></p>`, and its main module contains:

```js
export async function main(data) {
  if (data === null) {
    document.querySelector('.result').textContent = 'Call this template with message data.';
    return;
  }
  if (typeof data.message !== 'string') throw new TypeError('message must be a string');
  document.querySelector('.result').textContent = data.message;
}
```

The template supplies the entire canvas content and uses the same theme and security policy as an ordinary canvas. It may import helper modules and load local CSS or other resources. Pass caller-owned resources as explicit `locus://` addresses in `data`; relative addresses belong to the template. Template edits take effect when the caller refreshes or reopens. There is no imperative template loader or per-script context.

Template addresses reject parent traversal, hidden paths, symbolic links, non-note directories and paths outside the library. Encoded addresses are decoded once. Reading raw `canvas/index.json` through a library resource URL returns its JSON contents, not rendered HTML.

## Library resources and note data

Use these addresses from a canvas:

| Address | Result |
| --- | --- |
| `locus://projects/example/assets/photo.png` | An attachment or reserved icon |
| `locus://templates/example/canvas/index.html` | Raw canvas entry fragment |
| `locus://templates/example/canvas/styles/theme.css` | Any file in a canvas directory |
| `locus://projects/example/index.md` | Complete note text, including frontmatter |
| `locus://projects/example/index.md?part=yaml` | YAML source without fence delimiters; empty when absent |
| `locus://projects/example/index.md?part=body` | Markdown body without frontmatter |

`fetch('locus://...')` returns a standard Response. Use `.text()` for YAML, Markdown and HTML, `.json()` for JSON files, or `.blob()`/`.arrayBuffer()` for binary data. Reads return current saved content and never write notes. Missing, invalid or unsafe targets return 404. YAML remains source text rather than a lossy metadata projection.

Static HTML/CSS resource addresses and image/link attributes accept these addresses. For dynamic resource constructors, modules or Workers, use `locus.resourceUrl(uri)` to obtain the host URL, for example `new Worker(locus.resourceUrl('locus://example/canvas/worker.js'))`. The helper only maps an address; the host validates access. Encoded note names and resource filenames are decoded once. Parent traversal and symlinks are rejected; note paths cannot pass through reserved resource directories. Library paths never grant arbitrary filesystem access.

To open a note inside Locus, use a canonical note address in an anchor, for example `<a href="locus://projects/example">Open note</a>`. This works for static and dynamically created links, including linked cards. Do not append `/index.md`, a query, or a fragment, and do not pass navigation addresses to `locus.resourceUrl`; file addresses are for resource reads. Note links use the current library and the workspace's normal selection and back/forward history. Missing or archived targets leave the current selection unchanged. A new-window target still opens the note in the current workspace. HTTP/HTTPS links continue to open in the system browser. Exported HTML preserves note addresses but does not provide Locus navigation or bundle linked notes.

```js
export async function main(data) {
  const response = await fetch('locus://projects/example/index.md?part=yaml');
  if (!response.ok) throw new Error('Unable to read note data');
  const yamlSource = await response.text();
  console.log(yamlSource);
}
```

Use the canvas titlebar download button to export saved content. Settings → General → Export directory selects the destination; the default is the system Downloads directory. A self-contained canvas produces a single HTML file; referenced files produce a folder with `canvas.html`. Existing output names are numbered instead of overwritten. In-memory interaction state is not exported.

Downloads include referenced nested canvas files and library resources, and snapshot referenced YAML/body responses for offline fetch. Dynamic dependencies must have been loaded before downloading. Online data still requires a network connection.

Downloads resolve the template and preserve its data and resource ownership. Self-contained main modules are embedded as data URLs and imported with the same lifecycle as live canvases. Main modules with imports or `import.meta`, and modules imported by other modules, remain files to preserve module resolution and identity. The export permits its embedded data module in the script policy. Native module imports and Workers may require a local HTTP server when a browser blocks file-origin requests.
