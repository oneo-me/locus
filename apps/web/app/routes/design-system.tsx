import { DesignSystemBrowser } from '@locus/ui/design-system';
import { ThemeProvider } from '@locus/ui/theme-provider';

export default function DesignSystemRoute() {
  return (
    <ThemeProvider>
      <DesignSystemBrowser />
    </ThemeProvider>
  );
}

export function meta() {
  return [{ title: '设计系统 — Asterweft' }];
}
