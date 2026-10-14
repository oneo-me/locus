export const themeStorageKey = "locus-theme";

export type Theme = "light" | "dark" | "system";

export function isTheme(value: string | null | undefined): value is Theme {
  return value === "light" || value === "dark" || value === "system";
}

export function getStoredTheme(): Theme {
  if (typeof window === "undefined") return "system";
  try {
    const stored = window.localStorage.getItem(themeStorageKey);
    return isTheme(stored) ? stored : "system";
  } catch {
    return "system";
  }
}

export function resolveTheme(theme: Theme): "light" | "dark" {
  if (theme === "light" || theme === "dark") return theme;
  if (typeof window === "undefined") return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export function applyTheme(theme: Theme): void {
  if (typeof document === "undefined") return;
  const resolved = resolveTheme(theme);
  document.documentElement.classList.toggle("dark", resolved === "dark");
  document.documentElement.style.colorScheme = resolved;
}

export function storeTheme(theme: Theme): void {
  try {
    window.localStorage.setItem(themeStorageKey, theme);
  } catch {
    // Storage can be unavailable; the theme still applies to this page.
  }
}

/**
 * Runs before paint so the stored theme is applied without a flash of the
 * wrong appearance. Keep the logic in sync with resolveTheme.
 */
export function themeInitScript(): string {
  return `(function(){try{var t=localStorage.getItem(${JSON.stringify(
    themeStorageKey,
  )});var d=t==="dark"||(t!=="light"&&window.matchMedia("(prefers-color-scheme: dark)").matches);document.documentElement.classList.toggle("dark",d);document.documentElement.style.colorScheme=d?"dark":"light";}catch(e){}})();`;
}
