import { EllipsisIcon } from 'lucide-react';
import { lazy, Suspense, useRef, useState } from 'react';
import type { documentGraphParticipation } from '@locus/document-library/contracts/library-state';
import { Button } from '@locus/ui/components/button';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@locus/ui/components/popover';
import { Separator } from '@locus/ui/components/separator';
import { DocumentIcon } from './icons';
import { useDocumentLibraryUi } from './runtime';

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
}: {
  archived: boolean;
  documentPath: string;
  graphParticipation: ReturnType<typeof documentGraphParticipation>;
}) {
  const { setDocumentArchived, setDocumentGraphParticipation } =
    useDocumentLibraryUi();
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
      const participates = await setDocumentGraphParticipation(
        documentPath,
        checked,
      );
      setParticipates(participates);
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
      await setDocumentArchived(documentPath);
    } catch (cause) {
      setArchiveError(cause instanceof Error ? cause.message : String(cause));
      setArchivePending(false);
    }
  };

  return (
    <Popover>
      <PopoverTrigger
        render={
          <Button
            aria-label="笔记设置"
            className="document-theme-button"
            size="icon-sm"
            title="笔记设置"
            variant="secondary"
          />
        }
      >
        <EllipsisIcon aria-hidden="true" data-icon="inline-start" />
      </PopoverTrigger>
      <PopoverContent align="end" className="w-72" side="bottom">
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
  );
}

export function DocumentIconUpload({
  icon,
  iconImageSrc,
  pending,
  onUpload,
}: {
  icon: string | null;
  iconImageSrc?: string | null;
  pending: boolean;
  onUpload: (file: File) => void;
}) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const label = pending
    ? '正在处理图标…'
    : iconImageSrc
      ? '更换图标图片'
      : '上传图标图片';

  return (
    <>
      <input
        accept="image/*"
        disabled={pending}
        hidden
        onChange={(event) => {
          const file = event.currentTarget.files?.[0];
          event.currentTarget.value = '';
          if (file) onUpload(file);
        }}
        ref={fileInputRef}
        type="file"
      />
      <Button
        aria-label={label}
        className="shadow-none leading-none"
        disabled={pending}
        onClick={() => fileInputRef.current?.click()}
        size="icon-sm"
        title={label}
        type="button"
        variant="ghost"
      >
        <DocumentIcon
          className="size-6 text-xl"
          icon={icon}
          imageSrc={iconImageSrc}
        />
      </Button>
    </>
  );
}
