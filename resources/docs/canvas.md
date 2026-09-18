# Canvas authoring

Locus canvases are React component packages generated and maintained by Agents. Each note owns `canvas/package.json`, source files and assets. Locus starts one local Vite service for the active library using the computer's Node.js and npm. It supplies the React root, note routes, React 19.3, React DOM, Tailwind CSS 4.3, dependency installation and hot module replacement. Node.js and npm are not bundled: install Node.js 20.19+ or 22.12+ (or a newer supported major) with npm. Initial host/package installation needs access to the npm registry; installed, unchanged dependencies can run offline.

## Package and entry

Create `index.md` with accurate YAML frontmatter. A canvas fills the reading area below the note title and metadata. Keep independently maintained prose in another note; do not automatically discard existing Markdown. `canvas/` and `assets/` are reserved directories, never child notes.

`canvas/package.json`:

```json
{
  "name": "example-canvas",
  "private": true,
  "type": "module",
  "exports": { ".": "./src/index.tsx" },
  "peerDependencies": { "react": "^19.3.0", "react-dom": "^19.3.0" }
}
```

`canvas/src/index.tsx`:

```tsx
import { useState } from 'react';
import { useCanvas } from '@locus/canvas';

export default function Canvas() {
  const { note } = useCanvas();
  const [count, setCount] = useState(0);
  return <main className="min-h-screen bg-white p-6 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
    <p>{note}</p>
    <button onClick={() => setCount(count + 1)}>Count: {count}</button>
  </main>;
}
```

Export a default React component. Do not create another React root, HTML shell, Vite configuration or development server. Additional named exports and explicit package subpath exports are supported. Entries must be relative source files within the owning canvas directory. Do not add React or React DOM to dependencies/devDependencies; declare compatible peerDependencies. The host resolves them to one shared instance. `@locus/canvas` is a host module; do not install it from npm.

Declare ordinary browser packages in the note's `dependencies`. Locus runs system npm in that package, preserves its `package-lock.json`, and rechecks dependencies when the manifest changes. Installation is serialized within the service. Lifecycle scripts are disabled during automatic installation; dependencies requiring such scripts need an explicitly managed installation by the Agent. Keep `node_modules/` out of Git with a canvas `.gitignore`. Installed dependencies are excluded from note resources and library change notifications.

The JSON files above are standard npm manifests and lockfiles, not Locus application configuration. Locus maintains the common Vite, React plugin and Tailwind compiler configuration. A note cannot supply server-side plugins or a separate Vite configuration.

## Agent workflow

1. Read the note's requirements and existing component packages; inspect public exports before reusing them.
2. Create or edit the package manifest, TSX components, CSS and local assets. Preserve existing note contents unless changing them is authorized.
3. Open the note in Locus. Saving source triggers Vite/React hot updates. Editing a referenced note updates active consumers as well. New dependencies/configuration may cause a full reload; hot updates do not guarantee preservation of all local state.
4. Correct module/compiler errors shown by Vite and component failures shown by the host. Cmd/Ctrl+R or F5 retries loading; a stopped service is restarted on retry. Missing Node/npm or failed installation produces a recoverable host error.
5. Run appropriate source/behavior checks. Visual and animation acceptance remains the user's task.

The standard MCP endpoint returns original paths for filesystem edits. On the full endpoint, the `canvas` resource is `canvas/package.json`; `canvas_file` with `name: "src/index.tsx"` or another relative name addresses other files. Use content versions and the existing binary upload protocol. The host only recognizes a package manifest as a canvas entry. Existing HTML/main.js and JSON-template files are not converted, executed or deleted automatically.

## Cross-note composition

Use the public component entry of another note:

```tsx
import Requirements from 'locus:templates/requirements';
import { Button } from 'locus:design/basalt';
import { useState } from 'react';

export default function Canvas() {
  const [selected, setSelected] = useState(null);
  return <Requirements selected={selected} onSelect={setSelected}
    toolbar={<Button onClick={() => setSelected(null)}>Clear selection</Button>} />;
}
```

`locus:templates/requirements` resolves that note's `exports["."]`. An import such as `locus:design/basalt#ui` resolves `exports["./ui"]`. Paths may be URI encoded once. Dependencies resolve from the package that imports them. Vite watches the original source, so there is no copied component or separate server per note. Components can share props, callbacks, children, Context and state in the same React tree. Design reusable interfaces explicitly; an arbitrary whole-page component does not automatically expose replaceable sections.

`useCanvas().note` identifies the displayed/calling note even inside imported components. To read resources owned by a reusable package, use module-relative imports/`new URL('./file', import.meta.url)` or explicit library addresses. Do not interpret the caller's note address as the component package's directory.

## Styles and resources

The host supplies Tailwind's theme and base reset once. Utilities are compiled from each referenced canvas package's source, including cross-note components. Use `dark:` for the application theme; `useTheme()` from `@locus/canvas` returns `light` or `dark` and reacts without reloading. Use CSS Modules or scoped classes for ordinary components. Place design-system variables on an explicit theme container; avoid global `body`, `html` and `:root` overrides in reusable components. Configure popup/portal containers to inherit the intended theme. Independent packages do not isolate global CSS.

shadcn components are ordinary React source files. Add their dependencies and theme variables to the owning package; use relative imports or aliases explicitly supported by the host, rather than assuming a separate Vite project. Large styles belong in CSS files. Bundle fonts locally. Use module imports for images, CSS and Workers so Vite can resolve their dependencies.

The following SDK functions are supplied by `@locus/canvas`:

- `useCanvas()` returns `{ note }`, the displayed note's logical address.
- `useTheme()` observes the application theme.
- `resourceUrl(address)` maps a library resource to an HTTP URL for fetch, images and other browser consumers.
- `readText(address)` reads saved UTF-8 note/resource content and rejects non-success responses.
- `subscribeResource(address, listener)` subscribes to changes in a previously loaded resource and returns an unsubscribe function; use it inside a React effect and return that cleanup.

```tsx
import { readText, resourceUrl } from '@locus/canvas';

const yaml = await readText('locus://projects/example/index.md?part=yaml');
const body = await readText('locus://projects/example/index.md?part=body');
const data = await fetch(resourceUrl('locus://projects/example/canvas/data.json')).then(r => r.json());
```

Supported resources are `locus://<note>/index.md`, `index.md?part=yaml`, `index.md?part=body`, `assets/<file>` and `canvas/<file>`. YAML is returned as source text; missing frontmatter returns an empty string. Missing/unsafe resources return 404. Addresses are decoded once. Traversal, symbolic links, installed `node_modules` and arbitrary filesystem access are rejected. Library APIs are read-only.

The service listens only on loopback. It permits local scripts/styles/fonts, HTTPS images/data, local/blob Workers and standard browser DOM APIs. Remote executable scripts, remote styles/fonts, frames, form submission and arbitrary filesystem access are not available. Network requests still need service-side CORS support. Components have no native Shell or arbitrary filesystem bridge. HTTP/HTTPS external links open in the system browser.

## Lifecycle

In both Debug and Release, the shared canvas runtime is extracted to `~/.oneo/locus/canvas/<runtime-content-hash>/`. This directory contains the host source and installed host dependencies; Vite's per-library cache is under `node_modules/.vite/<library-path-hash>/`. Note dependencies remain in each note's `canvas/node_modules/`. Previous runtime cache directories are not automatically migrated or deleted.

React owns component state and effects. Release timers, subscriptions, observers and event handlers in effect cleanup. Leaving a note releases its WebView and in-memory state; the service stays available for other notes in the same library and stops when its owner is released. Theme changes do not reload. Native editing shortcuts, focus, application shortcuts and hiding during application overlays retain their normal behavior.
