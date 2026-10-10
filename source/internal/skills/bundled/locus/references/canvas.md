# Canvas authoring

Each note can own one canvas under `canvas/`. Choose exactly one entry:

- `index.html`: an HTML fragment, with a required `main.js` module exporting `async main(data)`.
- `index.json`: a template invocation containing exactly `template` and `data`.

Resolve the note through MCP and use the client's filesystem tools to edit its files. The canvas replaces the Markdown reading area. Prefer keeping only frontmatter in a canvas note's `index.md`, but never discard existing prose without the user's request. Locus does not compile React/TSX, install dependencies, or run a web development server.

## HTML and initialization

Write a fragment without document-level `<html>`, `<head>`, or `<body>` tags. The host supplies the title, base styling, Tailwind, and theme. Keep substantial CSS and JavaScript in local files. Scope styles to the canvas container rather than overriding the host document.

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

The host waits for page resources, imports main once, and awaits it. Direct opening passes `null`; template callers pass their exact JSON `data`. Even static canvases export a main function. Initialization failures show an error; fix the saved files and refresh. Later event handlers and asynchronous work own their own error handling.

Prepare useful initial content inside main, including required asynchronous data. Do not substitute an empty loading screen for a ready-to-use page. Use sample data for a directly opened reusable template; reject invalid input rather than silently changing its meaning.

## Presentation and lifecycle

Onevra hosts the system browser inside Locus on supported platforms. Settings hides and retains the page. Leaving the note releases its browser and transient state. Full native pointer, keyboard, accessibility, and overlay behavior must be checked on the target platform; filesystem edits alone do not verify appearance.

Saved source changes do not automatically rerun an open canvas. Refresh with Command/Ctrl+R or F5, or reopen the note. Refresh discards page interaction state. Switching appearance updates the retained page; use Tailwind `dark:` classes or a `.dark` ancestor. Combine related internal views with tabs or sections in one canvas while preserving their state.

Use the available width and reflow instead of locking the page to a fixed desktop preview width. For scrolling pages, a `min-h-full` root lets the background continue below the viewport. Reserve `h-full` for fixed frames with independently scrolling inner regions. Keep identifiers unique and selectors scoped.

## Resources and network policy

Relative HTML resources and fetches belong to the rendering canvas. Module imports belong to the importing module; CSS URLs belong to the stylesheet. A template's relative resources belong to the template, not its caller. Nested files and local scripts, styles, fonts, and data are supported. Use relative CSS URLs; arbitrary CSS text containing library schemes is not rewritten.

Resource reads are limited to existing note bodies and files under `assets/` or `canvas/`. Traversal, symlinks, `node_modules`, directory browsing, and arbitrary filesystem access are rejected. Hidden files may be resources inside the resource trees, not note paths. Individual reads are bounded to 32 MiB.

The page may fetch HTTPS data and load HTTPS images, subject to remote CORS policy. Local and blob Workers are allowed. Remote scripts, styles, fonts, frames, plugins, form submissions, and arbitrary file URLs are blocked. Network features can send information to their service; follow the user's data-handling requirements. No Go file-write, shell, or Agent service is exposed to page JavaScript, and browser permission requests are denied.

## Templates

A caller's `canvas/index.json`:

```json
{
  "template": "locus://templates/message",
  "data": { "message": "Template input" }
}
```

Both fields are required and extra fields are rejected. The template identifies an existing note with HTML and main.js; template chains and dual HTML/JSON entries are invalid. `data` can be any JSON value. The host neither merges defaults nor invents a schema.

A template could implement:

```js
export async function main(data) {
  const input = data === null ? { message: 'Example message' } : data;
  if (!input || typeof input !== 'object' || typeof input.message !== 'string') {
    throw new TypeError('message must be a string');
  }
  document.querySelector('.result').textContent = input.message;
}
```

Reserve `null` for sample data when following this convention; an explicit caller `null` receives the same example. Put caller-owned file addresses in data as explicit `locus://` resources.

## Library resource addresses

| Address | Response |
| --- | --- |
| `locus://projects/example/assets/photo.png` | Attachment bytes |
| `locus://templates/example/canvas/style.css` | Canvas resource bytes |
| `locus://projects/example/index.md` | Complete note source |
| `locus://projects/example/index.md?part=yaml` | Original YAML without delimiters |
| `locus://projects/example/index.md?part=body` | Markdown body without frontmatter |

Use `fetch('locus://…')` and normal Response methods. The host maps the address and validates it, reading current saved bytes. Resource image/source attributes and stylesheet-link attributes are adapted as well. For Workers or generated resource URLs, use `locus.resourceUrl(uri)`; it maps addresses but does not grant additional permissions.

Encode each filename segment, especially spaces, `%`, and `#`. Decoding happens once; encoded separators, parent segments, and unsafe names are rejected. Use queries only for the two supported note-body parts.

A note navigation link is different from a file resource: `<a href="locus://projects/example">Open note</a>` selects a visible note in the current workspace. Do not append `/index.md`, queries, or fragments to canvas navigation addresses. HTTP/HTTPS links open through the OS browser. Exported pages do not provide native note navigation.

## Export

Use the toolbar's Export canvas action. The destination is configured in General settings, defaulting to Downloads. Locus writes a new ZIP containing `canvas.html`, Tailwind's license, and resource snapshots loaded in this session. Existing outputs are not replaced. The snapshot budget is 64 MiB; wait for main to load before exporting.

Load required dynamic resources before export. Unused dependencies are not discovered, unrelated notes are not bundled, and transient DOM state is not saved. Template input and resource ownership are preserved. Online services still require network access. After unpacking, use a local HTTP server when browser restrictions prevent modules, fetch, or Workers from working under file origins. Export is not a promise of a single standalone HTML file.
