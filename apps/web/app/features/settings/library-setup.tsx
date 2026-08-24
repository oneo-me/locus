import { lazy, Suspense } from 'react';

const SettingsDialog = lazy(() =>
  import('./browser').then((module) => ({
    default: module.SettingsDialog,
  })),
);

export function LibrarySetup({ error }: { error?: string }) {
  return (
    <main className="flex min-h-dvh items-center justify-center bg-background px-6 text-center">
      <p className="text-sm text-muted-foreground">
        {error ?? '请先设置文档库路径。'}
      </p>
      <Suspense fallback={null}>
        <SettingsDialog
          initialSection="library"
          initiallyOpen
          libraryError={error}
          requiredLibraryPath
        />
      </Suspense>
    </main>
  );
}
