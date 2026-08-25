import { getDocumentIconUrl } from '@locus/document-library/contracts/asset';
import {
  type LibrarySelectionView,
  type LibraryWorkspaceView,
} from '@locus/document-library/contracts/library-api';
import { libraryReferenceScheme } from '@locus/document-library/contracts/library-reference';
import { documentGraphParticipation } from '@locus/document-library/contracts/library-state';
import type { DocumentContent } from '@locus/document-library/contracts/library-types';
import { CircleAlertIcon, CopyIcon } from 'lucide-react';
import { type CSSProperties, type ReactNode, useState } from 'react';
import { Link, useRevalidator } from 'react-router';
import { Alert, AlertDescription } from '@locus/ui/components/alert';
import { Button } from '@locus/ui/components/button';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from '@locus/ui/components/empty';
import { useDocumentApi } from '~/lib/document-api-context';
import { deriveYamlPanelHue } from './atmosphere';
import { MarkdownContent } from './markdown';
import { DocumentHeaderAtmosphere } from './header-atmosphere';
import { processDocumentIconFile } from './icon-image';
import { DocumentIconUpload } from './icon-upload';
import { DocumentGraphParticipationControl } from './graph-participation-control';

export function DocumentBrowser({
  data,
  selectionPending = false,
}: {
  data: LibraryWorkspaceView & LibrarySelectionView;
  selectionPending?: boolean;
}) {
  return (
    <>
      <main
        aria-busy={selectionPending || undefined}
        className="h-full min-h-0 min-w-0 overflow-x-hidden overflow-y-auto overscroll-contain bg-card text-sm [container-type:inline-size]"
        data-selection-pending={selectionPending ? '' : undefined}
        id="main-content"
        inert={selectionPending || undefined}
      >
        {data.selectionError ? (
          <MessageState title="无法打开文档">
            <p>{data.selectionError}</p>
            <Link to="/notes">返回笔记导航</Link>
          </MessageState>
        ) : data.selected ? (
          <DocumentArticle
            key={data.selected.path}
            document={data.selected}
            documents={data.documents}
            graphParticipation={documentGraphParticipation(
              data.libraryState,
              data.selected.path,
            )}
          />
        ) : data.selectedPath ? null : data.documents.length === 0 ? (
          <MessageState title="文档库还是空的">
            <p>
              在 <code>{data.status.rootPath}</code>{' '}
              下新建文章目录，并在其中加入 <code>index.md</code>
              ，页面会自动建立索引。
            </p>
          </MessageState>
        ) : null}
      </main>
      {selectionPending ? (
        <span className="sr-only" role="status">
          正在加载新文章…
        </span>
      ) : null}
    </>
  );
}

function DocumentArticle({
  document,
  documents,
  graphParticipation,
}: {
  document: NonNullable<LibrarySelectionView['selected']>;
  documents: LibraryWorkspaceView['documents'];
  graphParticipation: ReturnType<typeof documentGraphParticipation>;
}) {
  const { updateDocumentIconImage } = useDocumentApi();
  const { revalidate } = useRevalidator();
  const [iconPending, setIconPending] = useState(false);
  const [iconError, setIconError] = useState<string | null>(null);

  const saveIconImage = async (file: File) => {
    setIconPending(true);
    setIconError(null);
    try {
      const dataBase64 = await processDocumentIconFile(file);
      await updateDocumentIconImage({
        path: document.path,
        dataBase64,
      });
      await revalidate();
    } catch (error) {
      setIconError(error instanceof Error ? error.message : String(error));
    } finally {
      setIconPending(false);
    }
  };

  return (
    <div className="relative min-h-full">
      <DocumentHeaderAtmosphere title={document.title} />
      <article className="relative mx-auto flex w-[calc(100%-4rem)] max-w-[52rem] flex-col gap-6 pt-6 pb-14">
        <header className="flex flex-col gap-3">
          <DocumentYamlMetadata
            actions={
              <DocumentActions
                documentPath={document.path}
                graphParticipation={graphParticipation}
                onGraphParticipationSaved={revalidate}
                path={document.directory}
              />
            }
            document={document}
            error={iconError}
            onImageUpload={saveIconImage}
            pending={iconPending}
          />
          {document.issue ? (
            <Alert variant="destructive">
              <CircleAlertIcon aria-hidden="true" />
              <AlertDescription>{document.issue}</AlertDescription>
            </Alert>
          ) : null}
        </header>
        <MarkdownContent
          assets={document.assets}
          content={document.content}
          documentPath={document.path}
          documents={documents}
        />
      </article>
    </div>
  );
}

export async function copyDocumentText(
  value: string,
  clipboard: Pick<Clipboard, 'writeText'> | undefined = globalThis.navigator
    ?.clipboard,
): Promise<boolean> {
  if (!clipboard) return false;

  try {
    await clipboard.writeText(value);
    return true;
  } catch {
    return false;
  }
}

export function formatDocumentReference(path: string): string {
  return `${libraryReferenceScheme}${path}`;
}

export function DocumentActions({
  documentPath,
  graphParticipation,
  onGraphParticipationSaved,
  path,
}: {
  documentPath: string;
  graphParticipation: ReturnType<typeof documentGraphParticipation>;
  onGraphParticipationSaved?: () => void | Promise<void>;
  path: string;
}) {
  const [referenceStatus, setReferenceStatus] = useState<string | null>(null);

  const copyValue = async (value: string) => {
    setReferenceStatus(null);
    const copied = await copyDocumentText(value);
    setReferenceStatus(copied ? '已复制文章引用' : '文章引用复制失败');
  };

  return (
    <div className="flex flex-nowrap items-center justify-end gap-1.5">
      <Button
        aria-label="复制文章引用"
        onClick={() => {
          void copyValue(formatDocumentReference(path));
        }}
        size="icon-sm"
        title={referenceStatus ?? '复制文章引用'}
        variant="secondary"
      >
        <CopyIcon aria-hidden="true" data-icon="inline-start" />
      </Button>
      <DocumentGraphParticipationControl
        documentPath={documentPath}
        inherited={graphParticipation.inheritedFrom !== null}
        initialValue={graphParticipation.configured}
        key={documentPath}
        onSaved={onGraphParticipationSaved}
      />
      <p aria-live="polite" className="sr-only" role="status">
        {referenceStatus}
      </p>
    </div>
  );
}

export function DocumentYamlMetadata({
  actions,
  document,
  error,
  onImageUpload,
  pending,
}: {
  actions?: ReactNode;
  document: Omit<DocumentContent, 'content'>;
  error: string | null;
  onImageUpload: (file: File) => void;
  pending: boolean;
}) {
  const summary = document.summary?.trim() || null;

  return (
    <section className="flex flex-col gap-6" aria-label="文档元数据">
      <div className="grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2.5">
        <DocumentIconUpload
          className="size-8"
          disabled={pending}
          icon={document.icon}
          iconClassName="size-6 text-xl"
          imageSrc={getDocumentIconUrl(
            document.path,
            document.iconImageModifiedAt,
          )}
          onImageUpload={onImageUpload}
        />
        <h1 className="col-start-2 min-w-0 truncate text-xl leading-7 font-medium">
          {document.title}
        </h1>
        {actions ? (
          <div className="col-start-3 min-w-0 justify-self-end">{actions}</div>
        ) : null}
      </div>
      {summary || error ? (
        <div className="flex flex-col gap-2">
          {summary ? (
            <p
              aria-label="摘要"
              className="doc-yaml-panel w-full rounded-lg px-3.5 py-2.5 text-sm text-pretty leading-relaxed text-foreground backdrop-blur-[3px]"
              style={
                {
                  '--doc-yaml-hue': deriveYamlPanelHue(document.title),
                } as CSSProperties
              }
            >
              {summary}
            </p>
          ) : null}
          {error ? (
            <p className="text-destructive text-xs" role="alert">
              {error}
            </p>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

function MessageState({
  title,
  children,
  media,
}: {
  title: string;
  children: ReactNode;
  media?: ReactNode;
}) {
  return (
    <Empty
      className="mx-auto mt-[15vh] w-[calc(100%-3rem)] max-w-2xl flex-none"
      role="status"
    >
      {media}
      <EmptyHeader>
        <EmptyTitle>{title}</EmptyTitle>
        <EmptyDescription>{children}</EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
