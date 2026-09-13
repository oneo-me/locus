# Canvas authoring

Each note can own one canvas: `canvas/index.html` under the note directory containing `index.md`. Create or edit that file directly when using the standard MCP endpoint and local filesystem tools. On the full MCP endpoint, read or write the note's `canvas` content target through its content tools, respecting content versions. Use `canvas_file` with a relative `name` for other files under `canvas/`, including nested paths; upload tools accept binary resources.

A canvas replaces the Markdown reading area. Keep only YAML frontmatter in `index.md` when authoring a canvas note; put independently maintained prose in another note. Do not automatically discard existing Markdown when updating an existing mixed note. Do not use a Markdown `canvas` fence or an attachment reference to display a canvas.

## HTML format

Write an HTML fragment, not a complete page: omit `<!doctype html>`, `<html>`, `<head>`, and `<body>`. The host uses the note title, so do not set a separate page title. Tailwind CSS is already available. Fragments can contain Tailwind classes, `<style>`, `<style type="text/tailwindcss">`, and `<script>`.

```html
<main class="min-h-full bg-white p-6 text-slate-900 dark:bg-slate-900 dark:text-slate-100">
  <h1 class="mb-4 text-xl font-semibold">Counter</h1>
  <button id="counter" class="rounded-lg bg-blue-600 px-4 py-2 text-white">Clicks: 0</button>
  <script>
    (() => {
      let count = 0;
      document.getElementById('counter').addEventListener('click', function () {
        this.textContent = `Clicks: ${++count}`;
      });
    })();
  </script>
</main>
```

## Presentation and lifecycle

The canvas fills the content area below the title and metadata. Summary and custom YAML fields remain visible above it; long metadata can scroll independently. The canvas is excluded from attachment lists and counts. It has no separate tab, automatic-opening switch, refresh button, or context menu.

Saving a file does not reload an open canvas. With canvas focus, press Cmd/Ctrl+R or F5 to reload its owning note's canvas and rerun scripts. Leaving and reopening the note also reloads it. In-memory interaction state is lost on refresh or departure. An empty or unreadable file produces a recoverable message; fix it and refresh.

Combine multiple views in one file using tabs, sections, or disclosure panels. Preserve other sections' state during internal navigation. Use unique DOM IDs, scoped selectors, and separate script scopes.

The canvas uses the available content width. Use `dark:` classes or `.dark` selectors to follow the application theme; theme changes do not reload the page.

For a page that scrolls, use `min-h-full` (`min-height:100%`) on the root so the background continues below the first viewport. The host fixes the heights of `html` and `body`; `h-full` locks the root to one viewport. Use `h-full` only for a fixed frame with independently scrolling inner regions. Avoid a fixed pixel height for the whole page.

## Resources and network policy

Styles and scripts can be inline or reference files under the note’s `canvas/` directory. Relative URLs resolve from `canvas/index.html`; nested directories, JavaScript modules, CSS, images, fonts and data files are supported, without an extension allowlist. For example, `<img src="photo.jpg" alt="Photo description">` loads a local image. URL-encode filenames containing spaces or non-ASCII characters. Data and blob images are supported.

Local resources are read on request without caching. Canvas subdirectories and hidden resource files are allowed. Paths must remain in the resource tree; symbolic links and directory browsing are unavailable. Use explicit library addresses to read other note resources. Read `locus-docs://attachments` through MCP for Markdown attachment syntax.

HTTPS images and HTTPS data requests are allowed, including map tiles and map data. Cross-origin requests still require server CORS permission. Canvas-local and blob Web Workers are allowed. Remote scripts, remote CSS and fonts, iframes, form submission, and arbitrary local file access are blocked. Basic canvases and local images work offline. Network features send requests to the corresponding service; handle failures and retries within the page. HTTP/HTTPS links open in the system browser.

## Reusable templates

Any note's canvas can serve as a template. Call:

```html
<script>
locus.loadCanvas('locus://templates/message', { message: 'Template input' })
  .catch(error => {
    document.body.textContent = 'Unable to load the template. Refresh to retry.';
    console.error(error);
  });
</script>
```

`await locus.loadCanvas(uri, data)` reads the template fragment, appends a container to the current page, and runs its scripts with the supplied input. It resolves to `undefined`; there is no returned renderer or exported object and no container argument. The address identifies an existing note in the current library, without a resource suffix. Percent-encode special characters in note addresses; the empty library-root address is not a template.

Templates interpret their input and own rendering and interaction. At the start of an inline classic script, capture `document.currentScript.canvas`: `data` is the caller's input and `root` is the inserted container. Save this context before asynchronous work; `document.currentScript` is not available later in a callback.

```html
<p class="result"></p>
<script>
(() => {
  const canvas = document.currentScript.canvas;
  if (!canvas) return;
  canvas.root.querySelector('.result').textContent = canvas.data.message;
})();
</script>
```

Template scripts execute in file order. Imported fragments support inline classic JavaScript, local `script src`, module scripts and JSON data blocks. External and module scripts wait for their load event before later scripts run; use `document.currentScript.canvas` in classic external scripts as well. Modules do not expose `document.currentScript`. Load failures, invalid fragments, and synchronous script errors reject the Promise and remove the inserted container. Global side effects already performed by scripts are not rolled back. Resolution waits for script execution and external/module script loading, not scripts’ subsequent network requests or asynchronous work; templates handle those states and errors themselves.

Templates run in the caller's page with the same Tailwind, theme, and content security policy, without an iframe or separate sandbox. Relative resources use the calling canvas directory. Use explicit `locus://` resource addresses for files belonging to the template note. Scope scripts and styles carefully: repeated loads do not isolate globals, CSS, or duplicate IDs. Nested template calls are allowed; authors must prevent cycles and manage timers, subscriptions, and other resources. When a template is opened directly, `document.currentScript.canvas` is absent; provide a preview or explanatory message if needed.

Template addresses identify existing notes in the current library. The host reads their fixed `canvas/index.html`; parent traversal, hidden paths, symbolic links, non-note directories, and paths outside the library are rejected. Every call rereads the file. Template edits affect an existing caller only after refresh or reopening. Templates cannot be navigated to as separate WebView pages. Arbitrary file access and remote scripts remain blocked.

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

```html
<img src="locus://projects/example/assets/photo.png" alt="Example">
<script>
(async () => {
  const response = await fetch('locus://projects/example/index.md?part=yaml');
  if (!response.ok) throw new Error('Unable to read note data');
  const yamlSource = await response.text();
  console.log(yamlSource);
})();
</script>
```

Downloads include referenced nested canvas files and library resources, and snapshot referenced YAML/body responses for offline fetch. Dynamic dependencies must have been loaded before downloading. Online data still requires a network connection.
