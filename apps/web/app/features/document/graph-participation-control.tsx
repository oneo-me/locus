import { EllipsisIcon } from 'lucide-react';
import { lazy, Suspense, useState } from 'react';
import { buttonVariants } from '@locus/ui/components/button';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@locus/ui/components/popover';
import { cn } from '@locus/ui/utils';
import { useDocumentApi } from '~/lib/document-api-context';

const Switch = lazy(() =>
  import('@locus/ui/components/switch').then((module) => ({
    default: module.Switch,
  })),
);

export function DocumentGraphParticipationControl({
  documentPath,
  inherited,
  initialValue,
  onSaved,
}: {
  documentPath: string;
  inherited: boolean;
  initialValue: boolean;
  onSaved?: () => void | Promise<void>;
}) {
  const { setDocumentGraphParticipation } = useDocumentApi();
  const [participates, setParticipates] = useState(initialValue);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const updateParticipation = async (checked: boolean) => {
    const previous = participates;
    setParticipates(checked);
    setPending(true);
    setError(null);
    try {
      const result = await setDocumentGraphParticipation({
        path: documentPath,
        participates: checked,
      });
      setParticipates(result.participates);
      await onSaved?.();
    } catch (cause) {
      setParticipates(previous);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setPending(false);
    }
  };

  return (
    <Popover>
      <PopoverTrigger
        aria-label="笔记设置"
        className={cn(
          buttonVariants({ size: 'icon-sm', variant: 'secondary' }),
        )}
        title="笔记设置"
      >
        <EllipsisIcon aria-hidden="true" />
      </PopoverTrigger>
      <PopoverContent align="end" className="w-72" side="bottom">
        <div className="flex items-start justify-between gap-3 text-sm">
          <div className="min-w-0">
            <p>参与知识图谱</p>
            <p className="mt-0.5 text-xs text-muted-foreground">
              {inherited
                ? '父笔记已关闭此项，因此当前笔记不会显示在知识图谱中。'
                : '关闭后，此笔记及其关系不会显示在知识图谱中。'}
            </p>
          </div>
          <Suspense
            fallback={
              <span
                aria-hidden="true"
                className="h-5 w-8 shrink-0 rounded-full bg-muted"
              />
            }
          >
            <Switch
              aria-label="参与知识图谱"
              checked={inherited ? false : participates}
              disabled={pending || inherited}
              onCheckedChange={(checked) => {
                void updateParticipation(checked);
              }}
            />
          </Suspense>
        </div>
        {error ? (
          <p className="text-destructive text-xs" role="alert">
            {error}
          </p>
        ) : null}
      </PopoverContent>
    </Popover>
  );
}
