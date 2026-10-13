# Official website

`site/` contains the Locus official website, a static React Router app styled with Tailwind CSS and shadcn/ui components. It is independent of the Go module: the desktop app does not load it, and Go checks do not build it. Read the [architecture overview](../ARCHITECTURE.md) before changing what the site claims about the product.

## Content and routes

| Route | Language |
| --- | --- |
| `/` | English |
| `/zh-CN` | Simplified Chinese |

`site/app/content.ts` owns all page text, the repository URL, and download links; keep both languages aligned with [README](../README.md). `site/app/components/landing.tsx` renders the shared layout. The download button opens `https://github.com/oneo-me/locus/releases/latest`, which GitHub redirects to the newest published release, or to the release list when none exists. The site does not host or mirror archives. Its platform note states the configured Apple Silicon macOS package target; update it when [packaging](architecture/packaging.md) adds verified targets.

The page follows the system light or dark appearance through `prefers-color-scheme`. shadcn/ui uses the `base-nova` style on Base UI; add components with `bunx --bun shadcn@latest add <component>` from `site/`.

## Development

Use [Bun](https://bun.sh). From `site/`:

```sh
bun install
bun run dev        # development server with hot reload
bun run typecheck
bun run build      # static output in build/client
```

`react-router.config.ts` sets `ssr: false` and `prerender: true`, so every route becomes an HTML file. The build also writes `404.html`, a copy of React Router's SPA fallback, which renders the root error boundary for unknown paths.

## Deploying to Cloudflare

`site/wrangler.jsonc` deploys `build/client` as Workers static assets with no Worker script, using `404.html` for unmatched paths:

```sh
bun run deploy     # build, then wrangler deploy
bun run preview    # build, then serve locally with wrangler dev
```

`wrangler deploy` requires a Cloudflare login (`bunx wrangler login`) or `CLOUDFLARE_API_TOKEN`. For Cloudflare Pages Git integration instead, set the root directory to `site`, the build command to `bun run build`, and the output directory to `build/client`. No deployment is automated from this repository.
