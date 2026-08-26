import { getDocumentIconUrl } from '@locus/document-library/contracts/asset';
import type {
  LibraryNavigationDocument,
  LibraryWorkspaceView,
} from '@locus/document-library/contracts/library-api';
import {
  type LibraryState,
  participatesInDocumentGraph,
} from '@locus/document-library/contracts/library-state';
import {
  documentPathToUrlPath,
  pathnameToUrlPath,
  urlPathToHref,
} from '@locus/document-library/contracts/library-route';
import {
  CopyIcon,
  FilePenLineIcon,
  FileTextIcon,
  FolderOpenIcon,
  SquareTerminalIcon,
} from 'lucide-react';
import type { ComponentProps } from 'react';
import { useEffect, useRef, useState } from 'react';
import { useLocation, useNavigate } from 'react-router';
import {
  Command,
  CommandDialog,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  commandFilter,
} from '@locus/ui/components/command';
import { Spinner } from '@locus/ui/components/spinner';
import { apiGet } from '~/lib/api-client';
import { useDocumentApi } from '~/lib/document-api-context';
import {
  copyDocumentText,
  formatDocumentReference,
} from '../document/document-actions';
import { DocumentIcon } from '../document/icons';
import { useTerminalPanel } from '../terminal/panel';

type DocumentAction = 'copy' | 'directory' | 'terminal' | 'vscode';

const SEARCH_DEBOUNCE_MS = 150;
const DOCUMENT_ACTIONS = [
  {
    action: 'copy',
    aliases: ['copy', 'reference', 'link', 'uri'],
    englishLabel: 'Copy Reference',
    icon: CopyIcon,
    label: '复制笔记引用',
  },
  {
    action: 'vscode',
    aliases: ['edit', 'vscode', 'visual studio code'],
    englishLabel: 'Edit in VS Code',
    icon: FilePenLineIcon,
    label: '用 VS Code 编辑',
  },
  {
    action: 'directory',
    aliases: ['open folder', 'folder', 'reveal'],
    englishLabel: 'Open Folder',
    icon: FolderOpenIcon,
    label: '打开所在目录',
  },
  {
    action: 'terminal',
    aliases: ['terminal', 'shell', 'panel'],
    englishLabel: 'Show Library Terminal',
    icon: SquareTerminalIcon,
    label: '显示文档库终端',
  },
] satisfies {
  action: DocumentAction;
  aliases: string[];
  englishLabel: string;
  icon: typeof FolderOpenIcon;
  label: string;
}[];

export function AppCommandPalette({
  defaultExpanded,
  onOpenChange,
  open,
}: {
  defaultExpanded: boolean;
  onOpenChange: (open: boolean) => void;
  open: boolean;
}) {
  const location = useLocation();
  const navigate = useNavigate();
  const { openDocument } = useDocumentApi();
  const { show: showTerminal } = useTerminalPanel();
  const listRef = useRef<HTMLDivElement>(null);
  const [documents, setDocuments] = useState<LibraryNavigationDocument[]>([]);
  const [libraryState, setLibraryState] = useState<LibraryState | null>(null);
  const [documentsError, setDocumentsError] = useState<string | null>(null);
  const [documentsPending, setDocumentsPending] = useState(true);
  const [pendingAction, setPendingAction] = useState<DocumentAction | null>(
    null,
  );
  const [actionError, setActionError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [selection, setSelection] = useState({ query: '', value: '' });
  const [expandedByKeyboard, setExpandedByKeyboard] = useState(false);
  const expanded =
    defaultExpanded || expandedByKeyboard || query.trim().length > 0;
  const currentPath = location.pathname.startsWith('/notes')
    ? pathnameToUrlPath(location.pathname)
    : undefined;
  const currentDocument = documents.find(
    (document) => documentPathToUrlPath(document.path) === currentPath,
  );
  const matchingDocumentActions = currentDocument
    ? DOCUMENT_ACTIONS.map((item) => ({
        item,
        score: scoreCommand(item.label, query, [
          item.englishLabel,
          ...item.aliases,
        ]),
      }))
        .filter(({ score }) => score > 0)
        .sort((left, right) => right.score - left.score)
        .map(({ item }) => item)
    : [];
  const matchingDocuments = documents
    .filter(
      (document) =>
        libraryState !== null &&
        participatesInDocumentGraph(libraryState, document.path),
    )
    .map((document) => ({
      document,
      score: scoreCommand(document.title, query, [
        document.directory,
        document.path,
      ]),
    }))
    .filter(({ score }) => score > 0)
    .sort((left, right) => right.score - left.score)
    .map(({ document }) => document);
  const firstSelectableValue =
    matchingDocumentActions[0]?.action ?? matchingDocuments[0]?.path ?? '';
  const selectableValues = [
    ...matchingDocumentActions.map((item) => item.action),
    ...matchingDocuments.map((document) => document.path),
  ];
  const selectedValue =
    expanded &&
    selection.query === query &&
    selectableValues.includes(selection.value)
      ? selection.value
      : expanded
        ? firstSelectableValue
        : '';

  useEffect(() => {
    let active = true;
    void apiGet<LibraryWorkspaceView>('/api/library')
      .then((library) => {
        if (active) {
          setDocuments(library.documents);
          setLibraryState(library.libraryState);
        }
      })
      .catch((error: unknown) => {
        if (active) {
          setDocumentsError(
            error instanceof Error ? error.message : String(error),
          );
        }
      })
      .finally(() => {
        if (active) setDocumentsPending(false);
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    const frame = requestAnimationFrame(() => {
      if (listRef.current) listRef.current.scrollTop = 0;
    });
    return () => cancelAnimationFrame(frame);
  }, [query]);

  const close = () => onOpenChange(false);
  const selectDocument = (document: LibraryNavigationDocument) => {
    close();
    void navigate(urlPathToHref(documentPathToUrlPath(document.path)));
  };
  const runDocumentAction = async (action: DocumentAction) => {
    if (!currentDocument) return;
    setPendingAction(action);
    setActionError(null);
    try {
      if (action === 'copy') {
        const copied = await copyDocumentText(
          formatDocumentReference(currentDocument.directory),
        );
        if (!copied) throw new Error('无法写入剪贴板。');
      } else if (action === 'terminal') {
        showTerminal();
      } else {
        await openDocument({ path: currentDocument.path, target: action });
      }
      close();
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setPendingAction(null);
    }
  };

  return (
    <CommandDialog
      description="搜索笔记或执行当前笔记操作"
      onOpenChange={(nextOpen) => {
        setQuery('');
        setSelection({ query: '', value: '' });
        setExpandedByKeyboard(false);
        if (nextOpen) setActionError(null);
        onOpenChange(nextOpen);
      }}
      open={open}
      title="命令面板"
    >
      <Command
        label="命令面板"
        onValueChange={(value) => setSelection({ query, value })}
        shouldFilter={false}
        value={selectedValue}
      >
        <DebouncedCommandInput
          autoFocus
          key={open ? 'open' : 'closed'}
          onDebouncedValueChange={setQuery}
          onKeyDown={(event) => {
            if (event.key === 'ArrowDown' && !expanded) {
              event.preventDefault();
              setExpandedByKeyboard(true);
            }
          }}
          placeholder="搜索笔记或输入命令…"
        />
        {expanded ? (
          <CommandList ref={listRef}>
            {documentsPending ? (
              <CommandGroup heading="笔记">
                <CommandItem disabled value="正在载入笔记">
                  <Spinner aria-hidden="true" />
                  正在载入笔记…
                </CommandItem>
              </CommandGroup>
            ) : null}
            {documentsError ? (
              <CommandGroup heading="笔记">
                <CommandItem disabled value={documentsError}>
                  无法读取笔记：{documentsError}
                </CommandItem>
              </CommandGroup>
            ) : null}
            {currentDocument && matchingDocumentActions.length > 0 ? (
              <CommandGroup heading={`当前笔记 · ${currentDocument.title}`}>
                {matchingDocumentActions.map((item) => (
                  <DocumentActionItem
                    key={item.action}
                    {...item}
                    onSelect={runDocumentAction}
                    pendingAction={pendingAction}
                  />
                ))}
                {actionError ? (
                  <p
                    className="px-2 py-1.5 text-xs text-destructive"
                    role="alert"
                  >
                    {actionError}
                  </p>
                ) : null}
              </CommandGroup>
            ) : null}
            {matchingDocumentActions.length > 0 &&
            matchingDocuments.length > 0 ? (
              <CommandSeparator />
            ) : null}
            {matchingDocuments.length > 0 ? (
              <CommandGroup heading="切换笔记">
                {matchingDocuments.map((document) => (
                  <CommandItem
                    key={document.path}
                    keywords={[
                      document.title,
                      document.directory,
                      document.path,
                    ]}
                    onSelect={() => selectDocument(document)}
                    value={document.path}
                  >
                    {document.icon || document.iconImageModifiedAt ? (
                      <DocumentIcon
                        className="size-4 text-sm"
                        icon={document.icon}
                        imageSrc={getDocumentIconUrl(
                          document.path,
                          document.iconImageModifiedAt,
                        )}
                      />
                    ) : (
                      <FileTextIcon aria-hidden="true" />
                    )}
                    <span className="min-w-0 flex-1 truncate">
                      {document.title}
                    </span>
                    <span className="max-w-56 truncate text-xs text-muted-foreground">
                      {document.directory || '根目录'}
                    </span>
                  </CommandItem>
                ))}
              </CommandGroup>
            ) : null}
            {!documentsPending &&
            !documentsError &&
            matchingDocumentActions.length === 0 &&
            matchingDocuments.length === 0 ? (
              <p className="py-8 text-center text-sm text-muted-foreground">
                没有匹配的笔记或命令
              </p>
            ) : null}
          </CommandList>
        ) : null}
      </Command>
    </CommandDialog>
  );
}

function scoreCommand(
  value: string,
  query: string,
  keywords: string[],
): number {
  const normalizedQuery = query.trim().normalize('NFKC').toLocaleLowerCase();
  if (!normalizedQuery) return 1;

  const normalizedValue = value.normalize('NFKC').toLocaleLowerCase();
  const normalizedKeywords = keywords.map((keyword) =>
    keyword.normalize('NFKC').toLocaleLowerCase(),
  );
  const candidates = [normalizedValue, ...normalizedKeywords];
  const exactIndex = candidates.findIndex(
    (candidate) => candidate === normalizedQuery,
  );
  if (exactIndex >= 0) return 4 - exactIndex / 100;

  const prefixIndex = candidates.findIndex((candidate) =>
    candidate.startsWith(normalizedQuery),
  );
  if (prefixIndex >= 0) return 3 - prefixIndex / 100;

  const includesIndex = candidates.findIndex((candidate) =>
    candidate.includes(normalizedQuery),
  );
  if (includesIndex >= 0) return 2 - includesIndex / 100;

  return commandFilter(normalizedValue, normalizedQuery, normalizedKeywords);
}

function DebouncedCommandInput({
  onDebouncedValueChange,
  ...props
}: Omit<ComponentProps<typeof CommandInput>, 'onValueChange' | 'value'> & {
  onDebouncedValueChange: (value: string) => void;
}) {
  const [value, setValue] = useState('');
  const onDebouncedValueChangeRef = useRef(onDebouncedValueChange);

  useEffect(() => {
    onDebouncedValueChangeRef.current = onDebouncedValueChange;
  }, [onDebouncedValueChange]);

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      onDebouncedValueChangeRef.current(value);
    }, SEARCH_DEBOUNCE_MS);
    return () => window.clearTimeout(timeout);
  }, [value]);

  return <CommandInput {...props} onValueChange={setValue} value={value} />;
}

function DocumentActionItem({
  action,
  aliases,
  englishLabel,
  icon: Icon,
  label,
  onSelect,
  pendingAction,
}: {
  action: DocumentAction;
  aliases: string[];
  englishLabel: string;
  icon: typeof FolderOpenIcon;
  label: string;
  onSelect: (action: DocumentAction) => Promise<void>;
  pendingAction: DocumentAction | null;
}) {
  return (
    <CommandItem
      disabled={pendingAction !== null}
      keywords={[englishLabel, ...aliases]}
      onSelect={() => void onSelect(action)}
      value={action}
    >
      {pendingAction === action ? (
        <Spinner aria-hidden="true" />
      ) : (
        <Icon aria-hidden="true" />
      )}
      <span className="min-w-0 flex-1 truncate">{label}</span>
      <span className="text-xs text-muted-foreground">{englishLabel}</span>
    </CommandItem>
  );
}
