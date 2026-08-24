import { getDocumentIconUrl } from '@locus/document-library/contracts/asset';
import type { LibraryNavigationDocument } from '@locus/document-library/contracts/library-api';
import {
  documentPathToUrlPath,
  urlPathToDocumentPath,
  urlPathToHref,
} from '@locus/document-library/contracts/library-route';
import {
  closestCenter,
  DndContext,
  type DragEndEvent,
  type DraggableSyntheticListeners,
  DragOverlay,
  type DragStartEvent,
  KeyboardCode,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
} from '@dnd-kit/core';
import { restrictToVerticalAxis } from '@dnd-kit/modifiers';
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import FluentIconChevronRight from '@oneo/fluentui-icons-like-react/FluentIconChevronRight';
import FluentIconSearch from '@oneo/fluentui-icons-like-react/FluentIconSearch';
import { PaperclipIcon } from 'lucide-react';
import {
  type FormEvent,
  type MouseEvent,
  type ReactNode,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
} from 'react';
import { createPortal } from 'react-dom';
import {
  createSearchParams,
  Link,
  useLocation,
  useNavigate,
  useRevalidator,
} from 'react-router';
import { buttonVariants } from '@locus/ui/components/button';
import { Badge } from '@locus/ui/components/badge';
import {
  Collapsible,
  CollapsibleContent,
} from '@locus/ui/components/collapsible';
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from '@locus/ui/components/input-group';
import { DocumentIcon } from '../document/icons';
import { cn } from '@locus/ui/utils';
import type { LibraryWorkspaceView } from '@locus/document-library/contracts/library-api';
import { useDocumentApi } from '~/lib/document-api-context';
import {
  buildDocumentTree,
  type DocumentTreeNode,
  getOpenNodePaths,
  includeSearchResultAncestors,
} from '../library/tree';
import {
  type DocumentChangeStatus,
  type DocumentTreeChangeState,
  getDocumentChangeStatuses,
  getDocumentTreeChangeLabel,
  getDocumentTreeChangeStates,
} from './document-change-status';
import { type WorkspaceEventStatus, WorkspaceHeader } from './header';
import {
  workspaceMobileSecondaryState,
  workspaceRequestsMobileNavigation,
} from './mobile-navigation';
import { buildTreeReorderItems, reorderTreeNodes } from './tree-reorder';

const workspaceDesktopMediaQuery = '(min-width: 768px)';

function subscribeToWorkspaceDesktopViewport(onChange: () => void) {
  const mediaQuery = window.matchMedia(workspaceDesktopMediaQuery);
  mediaQuery.addEventListener('change', onChange);
  return () => mediaQuery.removeEventListener('change', onChange);
}

function getWorkspaceDesktopViewport() {
  return window.matchMedia(workspaceDesktopMediaQuery).matches;
}

function getWorkspaceServerDesktopViewport() {
  return false;
}

export function WorkspaceSidebar({
  currentPath,
  data,
  eventStatus,
  hiddenOnMobile,
  query,
}: {
  currentPath?: string;
  data: LibraryWorkspaceView;
  eventStatus: WorkspaceEventStatus;
  hiddenOnMobile: boolean;
  query?: string;
}) {
  const visibleDocuments = useMemo(
    () =>
      data.search
        ? includeSearchResultAncestors(data.search.results, data.documents)
        : data.documents,
    [data.documents, data.search],
  );
  const visibleDirectories = data.search ? [] : data.directories;
  const location = useLocation();
  const sidebar = useRef<HTMLElement>(null);
  const restoredFocusLocation = useRef<string | null>(null);

  useEffect(() => {
    if (
      hiddenOnMobile ||
      restoredFocusLocation.current === location.key ||
      !workspaceRequestsMobileNavigation(location.state) ||
      (currentPath !== undefined && visibleDocuments.length === 0) ||
      !window.matchMedia('(max-width: 767px)').matches
    ) {
      return;
    }

    const target = sidebar.current?.querySelector<HTMLElement>(
      currentPath === undefined
        ? 'a[aria-label="Locus 首页"]'
        : '[aria-current="page"]',
    );
    if (!target) return;

    target.focus({ preventScroll: true });
    restoredFocusLocation.current = location.key;
  }, [
    currentPath,
    hiddenOnMobile,
    location.key,
    location.state,
    visibleDocuments,
  ]);

  return (
    <aside
      className={cn(
        'min-h-0 flex-col overflow-hidden bg-sidebar text-sm font-normal text-sidebar-foreground max-md:p-2 md:flex',
        hiddenOnMobile ? 'hidden' : 'flex',
      )}
      aria-label="文档导航"
      data-workspace-mobile-pane="primary"
      ref={sidebar}
    >
      <WorkspaceHeader
        eventStatus={eventStatus}
        query={query}
        refreshing={data.status.refreshing}
      />
      <LibraryNavigation
        currentPath={currentPath}
        directories={visibleDirectories}
        documents={visibleDocuments}
        query={query}
        repositoryChanges={data.repositoryChanges}
        repositoryError={data.repositoryError}
        sortable={!data.search}
        treeState={data.libraryState}
      />
    </aside>
  );
}

function LibraryNavigation({
  currentPath,
  directories,
  documents,
  query,
  repositoryChanges,
  repositoryError,
  sortable,
  treeState,
}: {
  currentPath?: string;
  directories: string[];
  documents: LibraryNavigationDocument[];
  query?: string;
  repositoryChanges: LibraryWorkspaceView['repositoryChanges'];
  repositoryError: LibraryWorkspaceView['repositoryError'];
  sortable: boolean;
  treeState: LibraryWorkspaceView['libraryState'];
}) {
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const navRef = useRef<HTMLElement>(null);
  const restoredTreeScrollRef = useRef(false);
  const desktopViewport = useSyncExternalStore(
    subscribeToWorkspaceDesktopViewport,
    getWorkspaceDesktopViewport,
    getWorkspaceServerDesktopViewport,
  );
  const mobile = !desktopViewport;
  const tree = useMemo(
    () => buildDocumentTree(documents, directories, treeState.order),
    [directories, documents, treeState.order],
  );
  const changeStatuses = useMemo(
    () => getDocumentChangeStatuses(documents, repositoryChanges),
    [documents, repositoryChanges],
  );
  const treeChangeStates = useMemo(
    () => getDocumentTreeChangeStates(tree, changeStatuses),
    [changeStatuses, tree],
  );
  const searchOpenPaths = useMemo(
    () =>
      query
        ? documents.flatMap((document) =>
            getOpenNodePaths(document.path, directories),
          )
        : [],
    [directories, documents, query],
  );

  const [openNodes, setOpenNodes] = useState(() => {
    const initial = new Set<string>();
    if (currentPath !== undefined) {
      for (const path of getOpenNodePaths(
        urlPathToDocumentPath(currentPath),
        directories,
      )) {
        initial.add(path);
      }
    }
    for (const path of searchOpenPaths) initial.add(path);
    return initial;
  });
  const previousCurrentPathRef = useRef(currentPath);

  const toggleOpenNode = (path: string) => {
    setOpenNodes((current) => {
      if (current.has(path)) {
        const next = new Set(current);
        next.delete(path);
        return next;
      }
      return focusSelectedNode(
        getOpenNodePaths(`${path}/index.md`, directories),
        searchOpenPaths,
      );
    });
  };

  useEffect(() => {
    if (restoredTreeScrollRef.current) return;
    const nav = navRef.current;
    if (!nav) return;
    const target = nav.querySelector<HTMLElement>('[aria-current="page"]');
    if (!target) return;
    restoredTreeScrollRef.current = true;
    centerTreeScrollTarget(nav, target);
  }, [currentPath, documents]);

  useEffect(() => {
    setOpenNodes((current) => {
      let changed = false;
      const next = new Set(current);
      for (const path of searchOpenPaths) {
        if (!next.has(path)) {
          next.add(path);
          changed = true;
        }
      }
      return changed ? next : current;
    });
  }, [searchOpenPaths]);

  useEffect(() => {
    if (
      currentPath === undefined ||
      previousCurrentPathRef.current === currentPath
    ) {
      previousCurrentPathRef.current = currentPath;
      return;
    }
    previousCurrentPathRef.current = currentPath;
    const selectedDocumentPath = urlPathToDocumentPath(currentPath);
    const selectedParentPaths = getOpenNodePaths(
      selectedDocumentPath,
      directories,
    );
    setOpenNodes((current) => {
      const next = focusSelectedNode(selectedParentPaths, searchOpenPaths);
      if (
        next.size === current.size &&
        [...next].every((path) => current.has(path))
      ) {
        return current;
      }
      return next;
    });
  }, [currentPath, directories, searchOpenPaths]);

  const handleSearch = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const value = new FormData(event.currentTarget).get('q');
    const nextQuery = typeof value === 'string' ? value.trim() : '';
    void navigate(
      {
        pathname,
        search: nextQuery ? `?${createSearchParams({ q: nextQuery })}` : '',
      },
      { preventScrollReset: true },
    );
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <form
        className="shrink-0 px-3 pt-0.5 pb-2.5"
        method="get"
        onSubmit={handleSearch}
      >
        <InputGroup className="h-8 bg-card">
          <InputGroupInput
            key={query ?? ''}
            name="q"
            aria-label="搜索文档"
            defaultValue={query}
            placeholder="标题、标签或正文"
            autoComplete="off"
          />
          <InputGroupAddon>
            <FluentIconSearch aria-hidden="true" title={null} />
          </InputGroupAddon>
        </InputGroup>
      </form>

      {repositoryError ? (
        <p
          className="mx-3.5 mb-3 rounded-md border border-destructive/30 bg-destructive/10 px-2.5 py-2 text-destructive"
          role="alert"
          title={repositoryError}
        >
          Git 状态不可用：{repositoryError}
        </p>
      ) : null}

      {documents.length > 0 || directories.length > 0 ? (
        <nav
          className="min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto overscroll-contain px-2 pb-3"
          aria-label="文档树"
          ref={navRef}
        >
          <DocumentTreeItems
            currentPath={currentPath}
            changeStates={treeChangeStates}
            mobile={mobile}
            nodes={tree}
            onToggleOpen={toggleOpenNode}
            openNodes={openNodes}
            query={query}
            sortable={sortable && desktopViewport}
          />
        </nav>
      ) : (
        <p className="m-3 text-muted-foreground">没有匹配项。</p>
      )}
    </div>
  );
}

function nodeKey(node: DocumentTreeNode) {
  return node.document?.path ?? `directory:${node.path}`;
}

export function treeItemTapBehavior(
  mobile: boolean,
  hasChildren: boolean,
  open: boolean,
): 'expand' | 'view' {
  if (mobile && hasChildren && !open) return 'expand';
  return 'view';
}

export function focusSelectedNode(
  selectedParentPaths: readonly string[],
  searchOpenPaths: readonly string[],
): Set<string> {
  const next = new Set(selectedParentPaths);
  for (const path of searchOpenPaths) next.add(path);
  return next;
}

export function centerTreeScrollTarget(
  nav: HTMLElement,
  target: HTMLElement,
): void {
  if (
    nav.getClientRects().length === 0 ||
    target.getClientRects().length === 0
  ) {
    return;
  }
  const navRect = nav.getBoundingClientRect();
  const targetRect = target.getBoundingClientRect();
  const delta =
    targetRect.top - navRect.top + targetRect.height / 2 - navRect.height / 2;
  nav.scrollTop += delta;
}

export function reconcileTreeItems(
  items: DocumentTreeNode[],
  nodes: DocumentTreeNode[],
): DocumentTreeNode[] {
  const latestNodes = new Map(nodes.map((node) => [nodeKey(node), node]));
  const reconciled = items.flatMap((item) => {
    const latest = latestNodes.get(nodeKey(item));
    if (!latest) return [];
    latestNodes.delete(nodeKey(item));
    return [latest];
  });
  return [...reconciled, ...latestNodes.values()];
}

function DocumentTreeItems({
  changeStates,
  currentPath,
  mobile,
  nodes,
  onToggleOpen,
  openNodes,
  query,
  sortable,
}: {
  changeStates: Map<string, DocumentTreeChangeState>;
  currentPath?: string;
  mobile: boolean;
  nodes: DocumentTreeNode[];
  onToggleOpen: (path: string) => void;
  openNodes: Set<string>;
  query?: string;
  sortable: boolean;
}) {
  const { reorderDocuments } = useDocumentApi();
  const { revalidate } = useRevalidator();
  const [items, setItems] = useState(nodes);
  const [activeKey, setActiveKey] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const itemsRef = useRef(items);
  const originItemsRef = useRef(nodes);
  const nodesRef = useRef(nodes);
  const sortableId = useId();
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 4 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
      keyboardCodes: {
        start: [KeyboardCode.Space],
        cancel: [KeyboardCode.Esc],
        end: [KeyboardCode.Space],
      },
    }),
  );

  useEffect(() => {
    itemsRef.current = items;
  }, [items]);

  useEffect(() => {
    nodesRef.current = nodes;
    if (activeKey || pending) {
      setItems((current) => reconcileTreeItems(current, nodes));
      return;
    }
    setItems(nodes);
    originItemsRef.current = nodes;
  }, [activeKey, nodes, pending]);

  const persistOrder = async (nextItems: DocumentTreeNode[]) => {
    const payload = buildTreeReorderItems(nextItems);
    const originPayload = buildTreeReorderItems(originItemsRef.current);
    const unchanged =
      payload.length === originPayload.length &&
      payload.every(
        (item, index) =>
          item.path === originPayload[index]?.path &&
          item.order === originPayload[index]?.order,
      );
    if (unchanged || payload.length === 0) {
      setItems(nodesRef.current);
      return;
    }
    setPending(true);
    try {
      await reorderDocuments({ items: payload });
      originItemsRef.current = nextItems;
      await revalidate();
    } catch {
      setItems(nodesRef.current);
    } finally {
      setPending(false);
    }
  };
  const handleDragStart = ({ active }: DragStartEvent) => {
    setActiveKey(String(active.id));
  };

  const handleDragCancel = () => {
    setActiveKey(null);
    setItems(nodesRef.current);
  };

  const handleDragEnd = ({ active, over }: DragEndEvent) => {
    setActiveKey(null);
    if (!over || active.id === over.id) {
      setItems(nodesRef.current);
      return;
    }

    const currentItems = reconcileTreeItems(itemsRef.current, nodesRef.current);
    const fromIndex = currentItems.findIndex(
      (node) => nodeKey(node) === active.id,
    );
    const toIndex = currentItems.findIndex((node) => nodeKey(node) === over.id);
    const nextItems = reorderTreeNodes(currentItems, fromIndex, toIndex);
    itemsRef.current = nextItems;
    setItems(nextItems);
    void persistOrder(nextItems);
  };

  const renderedItems =
    activeKey || pending ? reconcileTreeItems(items, nodes) : nodes;
  const activeNode = activeKey
    ? renderedItems.find((node) => nodeKey(node) === activeKey)
    : undefined;

  return (
    <DndContext
      accessibility={{
        screenReaderInstructions: {
          draggable:
            '按空格开始排序，使用方向键移动，按空格放下，按 Escape 取消。',
        },
      }}
      collisionDetection={closestCenter}
      id={sortableId}
      modifiers={[restrictToVerticalAxis]}
      onDragCancel={handleDragCancel}
      onDragEnd={handleDragEnd}
      onDragStart={handleDragStart}
      sensors={sensors}
    >
      <SortableContext
        items={renderedItems.map(nodeKey)}
        strategy={verticalListSortingStrategy}
      >
        <ul className="flex w-full min-w-0 max-w-full flex-col gap-px overflow-x-hidden">
          {renderedItems.map((node) => (
            <SortableDocumentTreeItem
              changeState={changeStates.get(node.path)}
              changeStates={changeStates}
              currentPath={currentPath}
              key={nodeKey(node)}
              mobile={mobile}
              node={node}
              onToggleOpen={onToggleOpen}
              openNodes={openNodes}
              pending={pending}
              query={query}
              sortable={sortable}
            />
          ))}
        </ul>
      </SortableContext>
      {typeof document === 'undefined'
        ? null
        : createPortal(
            <DragOverlay adjustScale={false}>
              {activeNode ? (
                <DocumentTreeDragOverlay
                  changeState={changeStates.get(activeNode.path)}
                  currentPath={currentPath}
                  node={activeNode}
                />
              ) : null}
            </DragOverlay>,
            document.body,
          )}
    </DndContext>
  );
}

function SortableDocumentTreeItem({
  changeState,
  changeStates,
  currentPath,
  mobile,
  node,
  onToggleOpen,
  openNodes,
  pending,
  query,
  sortable,
}: {
  changeState?: DocumentTreeChangeState;
  changeStates: Map<string, DocumentTreeChangeState>;
  currentPath?: string;
  mobile: boolean;
  node: DocumentTreeNode;
  onToggleOpen: (path: string) => void;
  openNodes: Set<string>;
  pending: boolean;
  query?: string;
  sortable: boolean;
}) {
  const { name, path, document, children } = node;
  const hasChildren = children.length > 0;
  const urlPath = document ? documentPathToUrlPath(document.path) : path;
  const title = document?.title ?? name;
  const selected = currentPath !== undefined && urlPath === currentPath;
  const disabled = !sortable || pending;
  const open = openNodes.has(path);
  const tapBehavior = treeItemTapBehavior(mobile, hasChildren, open);
  const attachmentCount = document?.attachmentCount ?? 0;
  const attachmentDescriptionId = useId();
  const {
    attributes,
    isDragging,
    listeners,
    setActivatorNodeRef,
    setDraggableNodeRef,
    setDroppableNodeRef,
    transform,
    transition,
  } = useSortable({ disabled, id: nodeKey(node) });

  const handleTap = (event: MouseEvent<HTMLAnchorElement>) => {
    if (tapBehavior !== 'expand') return;
    event.preventDefault();
    onToggleOpen(path);
  };

  return (
    <li
      className={cn(
        'w-full min-w-0 max-w-full overflow-x-hidden',
        isDragging && 'opacity-0',
      )}
      ref={setDraggableNodeRef}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
      }}
    >
      <Collapsible
        className="block w-full min-w-0 max-w-full overflow-x-hidden"
        open={open}
      >
        <div
          className={cn(
            'grid w-full min-w-0 grid-cols-[1.75rem_minmax(0,1fr)] items-center rounded-lg',
            selected && 'bg-sidebar-accent text-sidebar-accent-foreground',
          )}
          data-selected={selected ? '' : undefined}
          ref={setDroppableNodeRef}
        >
          {hasChildren ? (
            <span
              aria-hidden="true"
              className={cn(
                'grid h-7 w-6 shrink-0 place-items-center justify-self-center text-sidebar-foreground/40 transition-transform max-md:h-9 max-md:w-7',
                open && 'rotate-90',
              )}
            >
              <FluentIconChevronRight
                size={mobile ? 16 : 12}
                variant="Filled"
                title={null}
              />
            </span>
          ) : (
            <span aria-hidden="true" />
          )}
          <DocumentUrlLink
            ariaExpanded={mobile && hasChildren ? open : undefined}
            attachmentDescriptionId={
              attachmentCount > 0 ? attachmentDescriptionId : undefined
            }
            changeState={changeState}
            dragDescriptionId={
              disabled ? undefined : attributes['aria-describedby']
            }
            dragListeners={disabled ? undefined : listeners}
            dragRef={disabled ? undefined : setActivatorNodeRef}
            onClick={handleTap}
            path={urlPath}
            query={query}
            selected={selected}
            sortable={!disabled}
            title={title}
          >
            {document?.icon || document?.iconImageModifiedAt ? (
              <span
                aria-hidden="true"
                className="grid shrink-0 place-items-center"
                data-icon="inline-start"
              >
                <DocumentIcon
                  className="size-3.5 text-xs max-md:size-4"
                  icon={document.icon}
                  imageSrc={getDocumentIconUrl(
                    document.path,
                    document.iconImageModifiedAt,
                  )}
                />
              </span>
            ) : null}
            <span className="min-w-0 flex-1 truncate text-left">{title}</span>
            {changeState ? (
              <DocumentChangeIndicator state={changeState} />
            ) : null}
            {attachmentCount > 0 ? (
              <Badge
                aria-hidden="true"
                title={`${attachmentCount} 个附件`}
                variant="ghost"
              >
                <PaperclipIcon aria-hidden="true" data-icon="inline-start" />
                {attachmentCount}
              </Badge>
            ) : null}
            {attachmentCount > 0 ? (
              <span className="sr-only" id={attachmentDescriptionId}>
                {attachmentCount} 个附件
              </span>
            ) : null}
          </DocumentUrlLink>
        </div>
        {hasChildren ? (
          <CollapsibleContent className="w-full min-w-0 max-w-full overflow-x-hidden pl-4">
            <DocumentTreeItems
              changeStates={changeStates}
              currentPath={currentPath}
              mobile={mobile}
              nodes={children}
              onToggleOpen={onToggleOpen}
              openNodes={openNodes}
              query={query}
              sortable={sortable}
            />
          </CollapsibleContent>
        ) : null}
      </Collapsible>
    </li>
  );
}

function DocumentTreeDragOverlay({
  changeState,
  currentPath,
  node,
}: {
  changeState?: DocumentTreeChangeState;
  currentPath?: string;
  node: DocumentTreeNode;
}) {
  const { document, name, path } = node;
  const urlPath = document ? documentPathToUrlPath(document.path) : path;
  const selected = currentPath !== undefined && urlPath === currentPath;
  const iconImageUrl = document
    ? getDocumentIconUrl(document.path, document.iconImageModifiedAt)
    : null;

  return (
    <div
      aria-hidden="true"
      className={cn(
        buttonVariants({
          variant: selected ? 'selected' : 'nav',
          size: 'nav',
          className:
            'pointer-events-none h-8 w-full min-w-0 justify-start overflow-hidden border-transparent shadow-none',
        }),
      )}
    >
      <span className="size-4 shrink-0" />
      {document?.icon || iconImageUrl ? (
        <span
          className="grid shrink-0 place-items-center"
          data-icon="inline-start"
        >
          <DocumentIcon
            className="size-3.5 text-xs"
            icon={document?.icon ?? null}
            imageSrc={iconImageUrl}
          />
        </span>
      ) : null}
      <span className="min-w-0 flex-1 truncate text-left">
        {document?.title ?? name}
      </span>
      {changeState ? <DocumentChangeIndicator state={changeState} /> : null}
    </div>
  );
}

function DocumentUrlLink({
  ariaExpanded,
  attachmentDescriptionId,
  changeState,
  children,
  dragDescriptionId,
  dragListeners,
  dragRef,
  onClick,
  path,
  query,
  selected,
  sortable,
  title,
}: {
  ariaExpanded?: boolean;
  attachmentDescriptionId?: string;
  changeState?: DocumentTreeChangeState;
  children: ReactNode;
  dragDescriptionId?: string;
  dragListeners?: DraggableSyntheticListeners;
  dragRef?: (node: HTMLElement | null) => void;
  onClick?: (event: MouseEvent<HTMLAnchorElement>) => void;
  path: string;
  query?: string;
  selected: boolean;
  sortable: boolean;
  title: string;
}) {
  return (
    <Link
      {...dragListeners}
      aria-current={selected ? 'page' : undefined}
      aria-expanded={ariaExpanded}
      aria-label={title}
      aria-describedby={
        [dragDescriptionId, attachmentDescriptionId]
          .filter(Boolean)
          .join(' ') || undefined
      }
      aria-roledescription={sortable ? '可排序笔记' : undefined}
      className={buttonVariants({
        variant: 'nav',
        size: 'nav',
        className:
          'w-full min-w-0 cursor-default justify-start overflow-hidden no-underline max-md:h-9 max-md:text-sm [&>*:first-child]:-ml-2',
      })}
      data-slot="button"
      draggable={false}
      onClick={onClick}
      onDragStart={(event) => event.preventDefault()}
      ref={dragRef}
      title={
        changeState
          ? `${title} · ${getDocumentTreeChangeLabel(changeState)}`
          : title
      }
      to={{
        pathname: urlPathToHref(path),
        search: query ? `?${createSearchParams({ q: query })}` : '',
      }}
      state={workspaceMobileSecondaryState}
    >
      {children}
    </Link>
  );
}

function DocumentChangeIndicator({
  state,
}: {
  state: DocumentTreeChangeState;
}) {
  const label = getDocumentTreeChangeLabel(state);
  return (
    <span
      className="flex h-4 shrink-0 items-center gap-1"
      aria-label={label}
      role="img"
      title={label}
    >
      {state.own ? (
        <DocumentChangeDot kind="own" status={state.own} />
      ) : state.descendants[0] ? (
        <DocumentChangeDot kind="descendant" status={state.descendants[0]} />
      ) : null}
    </span>
  );
}

function DocumentChangeDot({
  kind,
  status,
}: {
  kind: 'own' | 'descendant';
  status: DocumentChangeStatus;
}) {
  return (
    <span
      className={cn(
        'size-1.5 rounded-full',
        kind === 'descendant' && 'border',
        status === 'added' &&
          (kind === 'own' ? 'bg-success' : 'border-success'),
        status === 'modified' &&
          (kind === 'own' ? 'bg-warning' : 'border-warning'),
      )}
      aria-hidden="true"
    />
  );
}
