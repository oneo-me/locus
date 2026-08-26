import { getDocumentIconUrl } from '@locus/document-library/contracts/asset';
import {
  type LibrarySelectionView,
  type LibraryWorkspaceView,
} from '@locus/document-library/contracts/library-api';
import { documentGraphParticipation } from '@locus/document-library/contracts/library-state';
import type { DocumentContent } from '@locus/document-library/contracts/library-types';
import {
  CircleAlertIcon,
  CopyIcon,
  FilePenLineIcon,
  FolderOpenIcon,
  SquareTerminalIcon,
} from 'lucide-react';
import {
  type CSSProperties,
  type ReactNode,
  useEffect,
  useRef,
  useState,
} from 'react';
import { Link, useRevalidator } from 'react-router';
import { Alert, AlertDescription } from '@locus/ui/components/alert';
import { Button } from '@locus/ui/components/button';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from '@locus/ui/components/empty';
import { cn } from '@locus/ui/utils';
import { useDocumentApi } from '~/lib/document-api-context';
import { deriveDocumentThemeHue } from './atmosphere';
import { MarkdownContent } from './markdown';
import { DocumentHeaderAtmosphere } from './header-atmosphere';
import { processDocumentIconFile } from './icon-image';
import { DocumentIconUpload, DocumentMenu } from './document-menu';
import { copyDocumentText, formatDocumentReference } from './document-actions';
import { useTerminalPanel } from '../terminal/panel';

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
    <div
      className="relative min-h-full"
      style={
        {
          '--document-theme-hue': deriveDocumentThemeHue(document.title),
        } as CSSProperties
      }
    >
      <DocumentHeaderAtmosphere title={document.title} />
      <DocumentTitleHeader
        actions={
          <DocumentActions
            documentPath={document.path}
            referencePath={document.directory}
          >
            <DocumentMenu
              archived={document.archived}
              documentPath={document.path}
              graphParticipation={graphParticipation}
              onGraphParticipationSaved={revalidate}
            />
          </DocumentActions>
        }
        icon={
          <DocumentIconUpload
            icon={document.icon}
            iconImageSrc={getDocumentIconUrl(
              document.path,
              document.iconImageModifiedAt,
            )}
            onUpload={saveIconImage}
            pending={iconPending}
          />
        }
        title={document.title}
      />
      <article className="relative mx-auto flex w-[calc(100%-4rem)] max-w-[52rem] flex-col gap-6 pt-6 pb-14">
        <DocumentYamlMetadata document={document} error={iconError} />
        {document.issue ? (
          <Alert variant="destructive">
            <CircleAlertIcon aria-hidden="true" />
            <AlertDescription>{document.issue}</AlertDescription>
          </Alert>
        ) : null}
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

export function DocumentActions({
  children,
  documentPath,
  referencePath,
}: {
  children?: ReactNode;
  documentPath: string;
  referencePath: string;
}) {
  const { openDocument } = useDocumentApi();
  const { open: terminalOpen, toggle: toggleTerminalPanel } =
    useTerminalPanel();
  const [pendingAction, setPendingAction] = useState<
    'directory' | 'terminal' | 'vscode' | null
  >(null);
  const [actionStatus, setActionStatus] = useState<string | null>(null);

  const copyValue = async (value: string) => {
    setActionStatus(null);
    const copied = await copyDocumentText(value);
    setActionStatus(copied ? '已复制文章引用' : '文章引用复制失败');
  };

  const open = async (target: 'directory' | 'vscode') => {
    setPendingAction(target);
    setActionStatus(null);
    try {
      await openDocument({ path: documentPath, target });
      setActionStatus(
        target === 'directory' ? '已打开目录' : '已在 VS Code 中打开',
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setActionStatus(
        `${target === 'directory' ? '打开目录' : '用 VS Code 编辑'}失败：${message}`,
      );
    } finally {
      setPendingAction(null);
    }
  };

  const toggleTerminal = async () => {
    setPendingAction('terminal');
    setActionStatus(null);
    try {
      await toggleTerminalPanel();
      if (terminalOpen) setActionStatus('终端会话已关闭');
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setActionStatus(`关闭终端会话失败：${message}`);
    } finally {
      setPendingAction(null);
    }
  };

  return (
    <div className="flex flex-nowrap items-center justify-end gap-1.5">
      <Button
        aria-label="复制文章引用"
        className="document-theme-button"
        onClick={() => {
          void copyValue(formatDocumentReference(referencePath));
        }}
        size="icon-sm"
        title="复制文章引用"
        variant="secondary"
      >
        <CopyIcon aria-hidden="true" data-icon="inline-start" />
      </Button>
      <Button
        aria-label="用 VS Code 编辑文件"
        className="document-theme-button"
        disabled={pendingAction !== null}
        onClick={() => {
          void open('vscode');
        }}
        size="icon-sm"
        title="用 VS Code 编辑文件"
        variant="secondary"
      >
        <FilePenLineIcon aria-hidden="true" data-icon="inline-start" />
      </Button>
      <Button
        aria-label="打开目录"
        className="document-theme-button"
        disabled={pendingAction !== null}
        onClick={() => {
          void open('directory');
        }}
        size="icon-sm"
        title="打开目录"
        variant="secondary"
      >
        <FolderOpenIcon aria-hidden="true" data-icon="inline-start" />
      </Button>
      <Button
        aria-label={terminalOpen ? '关闭文档库终端' : '显示文档库终端'}
        className="document-theme-button"
        disabled={pendingAction !== null}
        onClick={() => void toggleTerminal()}
        size="icon-sm"
        title={terminalOpen ? '关闭文档库终端' : '显示文档库终端'}
        variant={terminalOpen ? 'secondary-selected' : 'secondary'}
      >
        <SquareTerminalIcon aria-hidden="true" data-icon="inline-start" />
      </Button>
      {children}
      <p aria-live="polite" className="sr-only" role="status">
        {actionStatus}
      </p>
    </div>
  );
}

function DocumentTitleHeader({
  actions,
  icon,
  title,
}: {
  actions?: ReactNode;
  icon: ReactNode;
  title: string;
}) {
  const titleRef = useRef<HTMLElement>(null);
  const [stuck, setStuck] = useState(false);

  useEffect(() => {
    const header = titleRef.current;
    const scroller = header?.closest<HTMLElement>('#main-content');
    if (!header || !scroller) return;

    const update = () => {
      setStuck(
        scroller.scrollTop > 0 &&
          header.getBoundingClientRect().top <=
            scroller.getBoundingClientRect().top + 0.5,
      );
    };

    update();
    scroller.addEventListener('scroll', update, { passive: true });
    window.addEventListener('resize', update);
    return () => {
      scroller.removeEventListener('scroll', update);
      window.removeEventListener('resize', update);
    };
  }, []);

  return (
    <header
      className={cn(
        'document-theme-header sticky top-0 z-10 border-b border-transparent transition-[background-color,border-color] duration-200 motion-reduce:transition-none',
        stuck && 'backdrop-blur-md',
      )}
      data-stuck={stuck ? '' : undefined}
      ref={titleRef}
    >
      <div
        className={cn(
          'mx-auto grid w-[calc(100%-4rem)] max-w-[52rem] min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2.5 py-4 transition-[padding] duration-200 motion-reduce:transition-none',
          stuck && 'py-2.5',
        )}
      >
        {icon}
        <h1
          className={cn(
            'col-start-2 min-w-0 truncate font-medium transition-[font-size,line-height] duration-200 motion-reduce:transition-none',
            stuck ? 'text-sm leading-5' : 'text-xl leading-7',
          )}
        >
          {title}
        </h1>
        {actions ? (
          <div className="col-start-3 min-w-0 justify-self-end">{actions}</div>
        ) : null}
      </div>
    </header>
  );
}

export function DocumentYamlMetadata({
  document,
  error,
}: {
  document: Omit<DocumentContent, 'content'>;
  error: string | null;
}) {
  const summary = document.summary?.trim() || null;
  const additionalMetadata = Object.entries(document.additionalMetadata);
  if (!summary && additionalMetadata.length === 0 && !error) return null;

  return (
    <section aria-label="文档元数据" className="flex flex-col gap-2">
      {summary || additionalMetadata.length > 0 ? (
        <div className="doc-yaml-panel w-full overflow-hidden rounded-lg backdrop-blur-[3px]">
          {summary ? (
            <p
              aria-label="摘要"
              className="px-3.5 py-2.5 text-sm text-pretty leading-relaxed text-foreground"
            >
              {summary}
            </p>
          ) : null}
          {additionalMetadata.length > 0 ? (
            <dl className="grid grid-cols-[minmax(7rem,auto)_minmax(0,1fr)] py-1 text-xs">
              {additionalMetadata.map(([key, value]) => (
                <div className="col-span-2 grid grid-cols-subgrid" key={key}>
                  <dt className="min-w-0 px-3.5 py-1.5 font-medium break-words text-muted-foreground">
                    {key}
                  </dt>
                  <dd className="min-w-0 px-3.5 py-1.5 whitespace-pre-wrap break-words text-foreground">
                    {formatYamlMetadataValue(value)}
                  </dd>
                </div>
              ))}
            </dl>
          ) : null}
        </div>
      ) : null}
      {error ? (
        <p className="text-destructive text-xs" role="alert">
          {error}
        </p>
      ) : null}
    </section>
  );
}

function formatYamlMetadataValue(value: unknown): string {
  if (typeof value === 'string') return value;
  if (value === null) return 'null';
  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value);
  }
  return JSON.stringify(value, null, 2) ?? 'null';
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
