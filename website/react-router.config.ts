import type { Config } from "@react-router/dev/config";

import { siteBase } from "./base-path.ts";

// Static output: no server runtime, every route prerendered. SITE_BASE lets the
// same build serve from a subpath, such as a GitHub Pages project site.
export default {
  ssr: false,
  prerender: true,
  basename: siteBase,
} satisfies Config;
