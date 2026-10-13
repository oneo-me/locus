// Finishes the static build after `react-router build`.
//
// React Router prerenders routes under the basename directory when a subpath is
// configured, and writes the SPA fallback to index.html when that subpath owns
// the `/` route. A host that already serves the artifact at the subpath (a
// GitHub Pages project site) expects the pages at the artifact root, so this
// moves them up one level and keeps the fallback as 404.html.
import { cpSync, existsSync, readdirSync, renameSync, rmSync } from "node:fs";
import { join } from "node:path";

import { siteBase } from "../base-path.ts";

const output = "build/client";
const spaFallback = join(output, "__spa-fallback.html");
const spaIndex = join(output, "index.html");
const notFound = join(output, "404.html");

if (siteBase === "/") {
  if (existsSync(spaFallback)) {
    cpSync(spaFallback, notFound);
  }
} else {
  const nested = join(output, siteBase.replace(/\/+$/, ""));

  // The root index.html is React Router's SPA fallback here; reserve it as
  // 404.html before the real prerendered home page takes its place.
  if (existsSync(spaIndex)) {
    renameSync(spaIndex, notFound);
  }

  for (const entry of readdirSync(nested)) {
    renameSync(join(nested, entry), join(output, entry));
  }
  rmSync(nested, { recursive: true, force: true });
}
