import type { ReactNode } from 'react';

export function SettingsPage({
  children,
  title,
}: {
  children: ReactNode;
  title: string;
}) {
  return (
    <main className="flex min-h-0 min-w-0 flex-col bg-card">
      <header className="flex h-11 shrink-0 items-center border-b px-5">
        <h1 className="truncate text-sm font-medium">{title}</h1>
      </header>
      <div className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto overscroll-contain">
        <div className="px-5 pt-5 pb-8">{children}</div>
      </div>
    </main>
  );
}
