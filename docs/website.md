# Official website

`website/` contains the Locus official website, a static React Router app styled with Tailwind CSS and shadcn/ui components. It is independent of the Go module: the desktop app does not load it, and Go checks do not build it. Read the [architecture overview](../ARCHITECTURE.md) before changing what the site claims about the product.

## Content and routes

| Route | Language |
| --- | --- |
| `/` | English |
| `/zh-CN` | Simplified Chinese |
| `/zh-TW` | Traditional Chinese |
| `/ja` | Japanese |
| `/ko` | Korean |

`website/app/content.ts` owns all page text, the repository URL, and download links. Keep every language aligned with [README](../README.md). `website/app/components/landing.tsx` renders the shared layout; its header offers a language dropdown, a light, dark, or system theme setting, and an icon link to the repository. The download button opens `https://github.com/oneo-me/locus/releases/latest`, which GitHub redirects to the newest published release, or to the release list when none exists. The website does not host packages. The platform note lists the same targets as [packaging](architecture/packaging.md).

`website/app/lib/theme.ts` owns the theme choice. It persists the selection in `localStorage` under `locus-theme`, and `root.tsx` injects an inline script that applies the `.dark` class before paint, so the stored appearance is used without a flash. `app.css` defines the `.dark` variable set and the class-based `dark` variant. shadcn/ui uses the `base-nova` style on Base UI; add components with `bunx --bun shadcn@latest add <component>` from `website/`.

## Development

Use [Bun](https://bun.sh). From `website/`:

```sh
bun install
bun run dev        # development server with hot reload
bun run typecheck
bun run build      # static output in build/client
```

`react-router.config.ts` sets `ssr: false` and `prerender: true`, so every route becomes an HTML file. `scripts/postbuild.ts` runs after the framework build and writes `404.html`, a copy of React Router's SPA fallback, which renders the root error boundary for unknown paths.

## Base path

The build serves from the domain root by default. Set `SITE_BASE` to a subpath when the host serves the site from one, such as a GitHub Pages project site at `https://<owner>.github.io/locus/`:

```sh
SITE_BASE=/locus/ bun run build
```

[base-path.ts](../website/base-path.ts) normalizes the value to one leading and trailing slash and uses it for both Vite's `base` and React Router's `basename`, because React Router requires the basename to start with the Vite base. React Router rewrites `Link` targets, and code that references files outside the router (currently the icon) prefixes `import.meta.env.BASE_URL`.

With a subpath, React Router prerenders the pages inside a directory named after the basename. [postbuild.ts](../website/scripts/postbuild.ts) moves them to the artifact root, for hosts that already serve the artifact at the basename, and turns React Router's SPA fallback into `404.html`. Without a subpath it only writes `404.html`.

## Deploying to Cloudflare

`website/wrangler.jsonc` deploys `build/client` as Workers static assets with no Worker script, using `404.html` for unmatched paths:

```sh
bun run deploy     # build, then wrangler deploy
bun run preview    # build, then serve locally with wrangler dev
```

`wrangler deploy` requires a Cloudflare login (`bunx wrangler login`) or `CLOUDFLARE_API_TOKEN`. For Cloudflare Pages Git integration instead, set the root directory to `website`, the build command to `bun run build`, and the output directory to `build/client`. Cloudflare serves the build from the domain root, so no `SITE_BASE` is required.

## Deploying to GitHub Pages

`.github/workflows/deploy-pages.yml` builds with `SITE_BASE=/<repository>/` and publishes `website/build/client` on pushes to `main` that touch `website/`, or when started manually with `workflow_dispatch`. Enable Pages with "GitHub Actions" as the source in the repository settings. Set the repository variable `SITE_BASE` to `/` when a user/org site or a custom domain serves the build from the domain root.

This workflow deploys the website only. App releases are [published from version tags](architecture/packaging.md#release). The Cloudflare commands above remain manual.
