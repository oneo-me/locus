import { getDocumentIconUrl } from '@locus/document-library/contracts/asset';
import {
  type LibrarySelectionView,
  type LibraryWorkspaceView,
} from '@locus/document-library/contracts/library-api';
import { libraryReferenceScheme } from '@locus/document-library/contracts/library-reference';
import {
  type DocumentGraphParticipation,
  documentGraphParticipation,
} from '@locus/document-library/contracts/library-state';
import type { DocumentContent } from '@locus/document-library/contracts/library-types';
import FluentIconErrorCircle from '@oneo/fluentui-icons-like-react/FluentIconErrorCircle';
import FluentIconImageCopy from '@oneo/fluentui-icons-like-react/FluentIconImageCopy';
import FluentIconTextQuote from '@oneo/fluentui-icons-like-react/FluentIconTextQuote';
import { DownloadIcon } from 'lucide-react';
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
import { Spinner } from '@locus/ui/components/spinner';
import { useDocumentApi } from '~/lib/document-api-context';
import { deriveYamlPanelHue } from './atmosphere';
import { MarkdownContent } from './markdown';
import {
  copyDocumentAsImage,
  documentExportRootSelector,
  documentImageFilename,
  downloadDocumentAsImage,
} from './document-export';
import { DocumentGraphParticipationControl } from './graph-participation-control';
import { DocumentHeaderAtmosphere } from './header-atmosphere';
import { processDocumentIconFile } from './icon-image';
import {
  DocumentIconPicker,
  type DocumentMetadataPatch,
} from './metadata-editor';

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
            <Link to="/">返回文档导航</Link>
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
        ) : data.selectedPath ? null : data.search?.total === 0 ? (
          <MessageState title="没有匹配的文档">
            <p>换一个关键词，或清除搜索查看全部文档。</p>
            <Link to="/">清除搜索</Link>
          </MessageState>
        ) : data.documents.length === 0 ? (
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
  graphParticipation: DocumentGraphParticipation;
}) {
  const { updateDocumentIconImage, updateDocumentMetadata } = useDocumentApi();
  const { revalidate } = useRevalidator();
  const [metadataPending, setMetadataPending] = useState(false);
  const [metadataError, setMetadataError] = useState<string | null>(null);

  const saveMetadata = async (patch: DocumentMetadataPatch) => {
    setMetadataPending(true);
    setMetadataError(null);
    try {
      if (document.iconImageModifiedAt) {
        await updateDocumentIconImage({
          path: document.path,
          dataBase64: null,
        });
      }
      await updateDocumentMetadata({
        path: document.path,
        ...patch,
      });
      await revalidate();
    } catch (error) {
      setMetadataError(error instanceof Error ? error.message : String(error));
    } finally {
      setMetadataPending(false);
    }
  };

  const saveIconImage = async (file: File) => {
    setMetadataPending(true);
    setMetadataError(null);
    try {
      const dataBase64 = await processDocumentIconFile(file);
      await updateDocumentIconImage({
        path: document.path,
        dataBase64,
      });
      await revalidate();
    } catch (error) {
      setMetadataError(error instanceof Error ? error.message : String(error));
    } finally {
      setMetadataPending(false);
    }
  };

  const removeIconImage = async () => {
    setMetadataPending(true);
    setMetadataError(null);
    try {
      await updateDocumentIconImage({
        path: document.path,
        dataBase64: null,
      });
      await revalidate();
    } catch (error) {
      setMetadataError(error instanceof Error ? error.message : String(error));
    } finally {
      setMetadataPending(false);
    }
  };

  return (
    <div className="relative min-h-full" data-document-export-root>
      <DocumentHeaderAtmosphere title={document.title} />
      <article
        className="relative mx-auto flex w-[calc(100%-2rem)] max-w-[52rem] flex-col gap-6 py-4 md:w-[calc(100%-4rem)] md:pt-6 md:pb-14"
        data-document-export-content
      >
        <header className="flex flex-col gap-3">
          <DocumentYamlMetadata
            actions={
              <DocumentActions
                documentPath={document.path}
                graphParticipation={graphParticipation}
                onGraphParticipationSaved={revalidate}
                path={document.directory}
                title={document.title}
              />
            }
            document={document}
            error={metadataError}
            onImageRemove={removeIconImage}
            onImageUpload={saveIconImage}
            onSave={saveMetadata}
            pending={metadataPending}
          />
          {document.issue ? (
            <Alert variant="destructive">
              <FluentIconErrorCircle aria-hidden="true" title={null} />
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
  title,
}: {
  documentPath: string;
  graphParticipation: DocumentGraphParticipation;
  onGraphParticipationSaved?: () => void | Promise<void>;
  path: string;
  title: string;
}) {
  const [imageStatus, setImageStatus] = useState<string | null>(null);
  const [referenceStatus, setReferenceStatus] = useState<string | null>(null);
  const [imagePending, setImagePending] = useState(false);

  const copyValue = async (value: string) => {
    setReferenceStatus(null);
    const copied = await copyDocumentText(value);
    setReferenceStatus(copied ? '已复制文章引用' : '文章引用复制失败');
  };

  const exportRoot = () =>
    globalThis.document?.querySelector<HTMLElement>(
      documentExportRootSelector,
    ) ?? null;

  const copyAsImage = async () => {
    setImageStatus('正在生成图片');
    setImagePending(true);
    try {
      const root = exportRoot();
      const copied = root ? await copyDocumentAsImage(root) : false;
      setImageStatus(copied ? '已复制为图片' : '复制为图片失败');
    } finally {
      setImagePending(false);
    }
  };

  const downloadAsImage = async () => {
    setImageStatus('正在生成图片');
    setImagePending(true);
    try {
      const root = exportRoot();
      const downloaded = root
        ? await downloadDocumentAsImage(root, documentImageFilename(title))
        : false;
      setImageStatus(downloaded ? '已下载图片' : '下载图片失败');
    } finally {
      setImagePending(false);
    }
  };

  return (
    <div
      className="flex flex-nowrap items-center justify-end gap-1.5"
      data-document-actions
      data-document-export-exclude
    >
      <Button
        aria-busy={imagePending || undefined}
        aria-label={imagePending ? '正在生成图片' : '复制为图片'}
        className="phone-hidden"
        disabled={imagePending}
        onClick={() => {
          void copyAsImage();
        }}
        size="icon-sm"
        title={imageStatus ?? '复制为图片'}
        variant="secondary"
      >
        {imagePending ? (
          <Spinner aria-label="正在生成图片" data-icon="inline-start" />
        ) : (
          <FluentIconImageCopy
            aria-hidden="true"
            data-icon="inline-start"
            title={null}
          />
        )}
      </Button>
      <Button
        aria-busy={imagePending || undefined}
        aria-label={imagePending ? '正在生成图片' : '下载图片'}
        className="phone-only"
        disabled={imagePending}
        onClick={() => {
          void downloadAsImage();
        }}
        size="icon-sm"
        title={imageStatus ?? '下载图片'}
        variant="secondary"
      >
        {imagePending ? (
          <Spinner aria-label="正在生成图片" data-icon="inline-start" />
        ) : (
          <DownloadIcon aria-hidden="true" data-icon="inline-start" />
        )}
      </Button>
      <Button
        aria-label="复制文章引用"
        className="phone-hidden"
        onClick={() => {
          void copyValue(formatDocumentReference(path));
        }}
        size="icon-sm"
        title={referenceStatus ?? '复制文章引用'}
        variant="secondary"
      >
        <FluentIconTextQuote
          aria-hidden="true"
          data-icon="inline-start"
          title={null}
        />
      </Button>
      <DocumentGraphParticipationControl
        documentPath={documentPath}
        inherited={graphParticipation.inheritedFrom !== null}
        initialValue={graphParticipation.configured}
        key={documentPath}
        onSaved={onGraphParticipationSaved}
      />
      <p aria-live="polite" className="sr-only" role="status">
        {imageStatus}
      </p>
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
  onImageRemove,
  onImageUpload,
  onSave,
  pending,
}: {
  actions?: ReactNode;
  document: Omit<DocumentContent, 'content'>;
  error: string | null;
  onImageRemove: () => void;
  onImageUpload: (file: File) => void;
  onSave: (patch: DocumentMetadataPatch) => void;
  pending: boolean;
}) {
  const summary = document.summary?.trim() || null;

  return (
    <section className="flex flex-col gap-6" aria-label="文档元数据">
      <div className="grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2.5">
        <DocumentIconPicker
          className="size-8 shrink-0 border-0 shadow-none leading-none"
          disabled={pending}
          icon={document.icon}
          iconClassName="size-6 text-xl"
          imageSrc={getDocumentIconUrl(
            document.path,
            document.iconImageModifiedAt,
          )}
          onChange={(icon) => onSave({ icon })}
          onImageRemove={onImageRemove}
          onImageUpload={onImageUpload}
        />
        <h1
          className="col-start-2 min-w-0 truncate text-xl leading-7 font-medium"
          data-document-export-title
        >
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
