import type { Config } from "@react-router/dev/config";

// Static output for Cloudflare: no server runtime, every route prerendered.
export default {
  ssr: false,
  prerender: true,
} satisfies Config;
