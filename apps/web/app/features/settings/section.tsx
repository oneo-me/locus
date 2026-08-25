import { Children, Fragment, isValidElement, type ReactNode } from 'react';
import { Separator } from '@locus/ui/components/separator';

export function SettingsSection({
  children,
  title,
}: {
  children: ReactNode;
  title: string;
}) {
  const items = Children.toArray(children);

  return (
    <section className="flex flex-col gap-2">
      <h2 className="px-2 text-sm font-medium">{title}</h2>
      <div className="flex flex-col overflow-hidden rounded-xl bg-foreground/2 p-3">
        {items.map((item, index) => (
          <Fragment
            key={isValidElement(item) && item.key !== null ? item.key : index}
          >
            {index > 0 ? <Separator /> : null}
            {item}
          </Fragment>
        ))}
      </div>
    </section>
  );
}

export function SettingsRow({
  children,
  description,
  label,
}: {
  children: ReactNode;
  description?: string;
  label: ReactNode;
}) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 py-3 first:pt-0 last:pb-0">
      <div className="min-w-0">
        <p className="text-sm">{label}</p>
        {description ? (
          <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>
        ) : null}
      </div>
      <div className="flex min-w-0 items-center justify-end gap-2">
        {children}
      </div>
    </div>
  );
}
