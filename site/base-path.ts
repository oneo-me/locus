// Hosts that serve the build from a subpath set SITE_BASE, for example
// "/locus" for a GitHub Pages project site. Vite's `base` and React Router's
// `basename` must both carry the trailing slash, because React Router requires
// the basename to start with the Vite base and Vite requires the trailing slash.
const raw = process.env.SITE_BASE ?? "/";

export const siteBase =
  raw === "/" ? "/" : `/${raw.replace(/^\/+|\/+$/g, "")}/`;
