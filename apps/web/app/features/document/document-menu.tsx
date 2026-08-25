import { ImageUpIcon } from 'lucide-react';
import { lazy, Suspense, useRef, useState } from 'react';
import { useNavigate } from 'react-router';
import type { documentGraphParticipation } from '@locus/document-library/contracts/library-state';
import { Button, buttonVariants } from '@locus/ui/components/button';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@locus/ui/components/popover';
import { Separator } from '@locus/ui/components/separator';
import { cn } from '@locus/ui/utils';
import { useDocumentApi } from '~/lib/document-api-context';
import { DocumentIcon } from './icons';

const Switch = lazy(() =>
  import('@locus/ui/components/switch').then((module) => ({
    default: module.Switch,
  })),
);

function SwitchFallback() {
  return (
    <span
      aria-hidden="true"
      className="h-5 w-8 shrink-0 rounded-full bg-muted"
    />
  );
}

export function DocumentMenu({
  archived,
  documentPath,
  graphParticipation,
  icon,
  iconImageSrc,
  iconPending,
  onGraphParticipationSaved,
  onImageUpload,
}: {
  archived: boolean;
  documentPath: string;
  graphParticipation: ReturnType<typeof documentGraphParticipation>;
  icon: string | null;
  iconImageSrc?: string | null;
  iconPending: boolean;
  onGraphParticipationSaved?: () => void | Promise<void>;
  onImageUpload: (file: File) => void;
}) {
  const { setDocumentArchived, setDocumentGraphParticipation } =
    useDocumentApi();
  const navigate = useNavigate();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [participates, setParticipates] = useState(
    graphParticipation.configured,
  );
  const [graphPending, setGraphPending] = useState(false);
  const [graphError, setGraphError] = useState<string | null>(null);
  const [archivePending, setArchivePending] = useState(false);
  const [archiveError, setArchiveError] = useState<string | null>(null);
  const inherited = graphParticipation.inheritedFrom !== null;

  const updateParticipation = async (checked: boolean) => {
    const previous = participates;
    setParticipates(checked);
    setGraphPending(true);
    setGraphError(null);
    try {
      const result = await setDocumentGraphParticipation({
        path: documentPath,
        participates: checked,
      });
      setParticipates(result.participates);
      await onGraphParticipationSaved?.();
    } catch (cause) {
      setParticipates(previous);
      setGraphError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setGraphPending(false);
    }
  };

  const archiveDocument = async () => {
    setArchivePending(true);
    setArchiveError(null);
    try {
      await setDocumentArchived({ path: documentPath, archived: true });
      await navigate('/notes');
    } catch (cause) {
      setArchiveError(cause instanceof Error ? cause.message : String(cause));
      setArchivePending(false);
    }
  };

  return (
    <>
      <input
        accept="image/*"
        disabled={iconPending}
        hidden
        onChange={(event) => {
          const file = event.currentTarget.files?.[0];
          event.currentTarget.value = '';
          if (file) onImageUpload(file);
        }}
        ref={fileInputRef}
        type="file"
      />
      <Popover>
        <PopoverTrigger
          aria-label="笔记设置"
          className={cn(
            buttonVariants({ size: 'icon-sm', variant: 'ghost' }),
            'size-8 shrink-0 shadow-none leading-none',
          )}
          title="笔记设置"
        >
          <DocumentIcon
            className="size-6 text-xl"
            icon={icon}
            imageSrc={iconImageSrc}
          />
        </PopoverTrigger>
        <PopoverContent align="start" className="w-72" side="bottom">
          <Button
            className="w-full justify-start"
            disabled={iconPending}
            onClick={() => fileInputRef.current?.click()}
            size="sm"
            type="button"
            variant="ghost"
          >
            <ImageUpIcon aria-hidden="true" data-icon="inline-start" />
            {iconPending
              ? '正在处理图标…'
              : iconImageSrc
                ? '更换图标图片'
                : '上传图标图片'}
          </Button>
          <Separator />
          <div className="flex items-start justify-between gap-3 text-sm">
            <div className="min-w-0">
              <p>参与知识图谱</p>
              <p className="mt-0.5 text-xs text-nowrap text-muted-foreground">
                {inherited
                  ? '父笔记已关闭，当前笔记不会显示。'
                  : '关闭后，此笔记及其关系不再显示。'}
              </p>
            </div>
            <Suspense fallback={<SwitchFallback />}>
              <Switch
                aria-label="参与知识图谱"
                checked={inherited ? false : participates}
                disabled={graphPending || inherited}
                onCheckedChange={(checked) => {
                  void updateParticipation(checked);
                }}
              />
            </Suspense>
          </div>
          {graphError ? (
            <p className="text-xs text-destructive" role="alert">
              {graphError}
            </p>
          ) : null}
          <Separator />
          <div className="flex items-start justify-between gap-3 text-sm">
            <div className="min-w-0">
              <p>归档笔记</p>
              <p className="mt-0.5 text-xs text-nowrap text-muted-foreground">
                归档后，此笔记及子笔记不再显示。
              </p>
            </div>
            <Suspense fallback={<SwitchFallback />}>
              <Switch
                aria-label="归档笔记"
                checked={archived || archivePending}
                disabled={archived || archivePending}
                onCheckedChange={(checked) => {
                  if (checked) void archiveDocument();
                }}
              />
            </Suspense>
          </div>
          {archiveError ? (
            <p className="text-xs text-destructive" role="alert">
              {archiveError}
            </p>
          ) : null}
        </PopoverContent>
      </Popover>
    </>
  );
}
