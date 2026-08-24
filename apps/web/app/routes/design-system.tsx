import { ThemeProvider } from '~/components/theme-provider';
import { DesignSystemBrowser } from '~/features/design-system/browser';

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
