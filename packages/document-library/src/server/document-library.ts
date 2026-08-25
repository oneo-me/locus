import {
  closeSync,
  type Dirent,
  existsSync,
  lstatSync,
  mkdirSync,
  openSync,
  readdirSync,
  readFileSync,
  readSync,
  realpathSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { basename, dirname, relative, resolve, sep } from 'node:path';
import { type FSWatcher, watch } from 'chokidar';
import {
  articleContentLines,
  articleMetadata,
  articleRevision,
} from './article-content.js';
import {
  articleParentPath,
  articlePathToDocumentPath,
  documentPathToArticlePath,
  normalizeArticlePath,
} from './article-paths.js';
import type {
  ArticleAttachment,
  ArticleListItem,
  CreateArticleInput,
  DeleteArticleAttachmentInput,
  DeleteArticleAttachmentResult,
  DeleteArticleInput,
  DeleteArticleResult,
  EditArticleContentInput,
  EditArticleContentResult,
  MoveArticleInput,
  MoveArticleResult,
  ReadArticleAttachmentInput,
  ReadArticleAttachmentResult,
  ReadArticleLinesInput,
  ReadArticleLinesResult,
  UpdateArticleInput,
  WriteArticleAttachmentInput,
  WriteArticleContentInput,
  WriteArticleContentResult,
} from './article-types.js';
import { DocumentCache, type ParsedCacheDocument } from './document-cache.js';
import type { DocumentGraph } from '../contracts/document-graph.js';
import { buildDocumentGraph } from './document-graph.js';
import {
  assertValidDocumentIcon,
  documentIconName,
  isDocumentIconName,
} from '../contracts/document-icon.js';
import {
  commitLibraryChanges,
  getLibraryRepositoryChanges,
  getLibraryRepositoryStatus,
} from './git-repository.js';
import type {
  CommitLibraryChangesInput,
  CommitLibraryChangesResult,
  LibraryRepositoryChanges,
  LibraryRepositoryStatus,
} from '../contracts/repository.js';
import {
  createExcerpt,
  normalizeIcon,
  parseDocument,
  serializeDocument,
  serializeDocumentArchive,
} from './markdown.js';
import {
  documentIndexName,
  isDocumentIndexPath,
  isInside,
  normalizeRelativePath,
  resolveExistingAsset,
  resolveExistingDocument,
  resolveNewAsset,
  resolveNewDocument,
} from './paths.js';
import type {
  CreateDocumentInput,
  DocumentAsset,
  DocumentContent,
  DocumentExcerpt,
  DocumentMetadata,
  DocumentSummary,
  LibraryStatus,
  ListDocumentsResult,
  SearchDocumentsResult,
  UpdateDocumentInput,
} from '../contracts/library-types.js';

const ignoredDirectories = new Set([
  '.git',
  '.idea',
  '.vscode',
  'node_modules',
]);
const maxRefreshRetries = 3;
const watchDebounceMs = 120;
const watchMaxWaitMs = 1_000;

export type LibraryChangeListener = (status: LibraryStatus) => void;

export class DocumentLibrary {
  readonly #requestedRootPath: string;
  readonly #listeners = new Set<LibraryChangeListener>();
  readonly #pendingDocumentPaths = new Set<string>();
  #rootPath = '';
  #cache: DocumentCache | null = null;
  #watcher: FSWatcher | null = null;
  #refreshTimer: ReturnType<typeof setTimeout> | null = null;
  #refreshRetryCount = 0;
  #refreshing = false;
  #refreshQueued = false;
  #fullRefreshPending = false;
  #watchBatchStartedAt: number | null = null;
  #documentSummaries: DocumentSummary[] | null = null;
  #documentGraph: DocumentGraph | null = null;
  #documentGraphExclusions = '';

  constructor(rootPath: string) {
    const normalizedRoot = rootPath.trim();
    if (!normalizedRoot) throw new Error('文档库路径不能为空。');
    this.#requestedRootPath = resolve(normalizedRoot);
  }

  static async open(rootPath: string): Promise<DocumentLibrary> {
    const library = new DocumentLibrary(rootPath);
    await library.start();
    return library;
  }

  async start(): Promise<void> {
    if (this.#cache) return;
    mkdirSync(this.#requestedRootPath, { recursive: true });
    this.#rootPath = realpathSync(this.#requestedRootPath);
    this.#cache = new DocumentCache();
    await this.refresh();
    await this.#startWatcher();
  }

  getRootPath(): string {
    if (!this.#rootPath) throw new Error('文档库尚未启动。');
    return this.#rootPath;
  }

  status(): LibraryStatus {
    const cache = this.#requireCache();
    return {
      rootPath: this.#rootPath,
      documentCount: cache.count(),
      watching: this.#watcher !== null,
      refreshing: this.#refreshing || this.#watchBatchStartedAt !== null,
      indexedAt: cache.indexedAt(),
    };
  }

  subscribe(listener: LibraryChangeListener): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  async refresh(): Promise<LibraryStatus> {
    const cache = this.#requireCache();
    if (this.#refreshing) {
      this.#refreshQueued = true;
      return this.status();
    }
    this.#refreshing = true;
    let changed = false;
    try {
      const documents = this.#scanDirectory(this.#rootPath);
      changed = !cache.matches(documents);
      if (changed) {
        cache.replaceAll(documents, new Date().toISOString());
        this.#invalidateDerivedViews();
      }
    } finally {
      this.#refreshing = false;
    }
    const status = this.status();
    if (changed) this.#notifyListeners(status);
    if (this.#refreshQueued) {
      this.#refreshQueued = false;
      return this.refresh();
    }
    return status;
  }

  listDocuments(directory = ''): ListDocumentsResult {
    const normalizedDirectory = normalizeRelativePath(directory);
    const prefix = normalizedDirectory ? `${normalizedDirectory}/` : '';
    const directories = new Set<string>();
    const documents = [];

    for (const document of this.#getDocumentSummaries()) {
      if (!document.path.startsWith(prefix)) continue;
      const rest = document.path.slice(prefix.length);
      if (rest.includes('/')) {
        const child = rest.slice(0, rest.indexOf('/'));
        directories.add(prefix + child);
      } else {
        documents.push(document);
      }
    }

    return {
      directory: normalizedDirectory,
      directories: [...directories].sort((left, right) =>
        left.localeCompare(right),
      ),
      documents,
    };
  }

  listAllDocuments() {
    return this.#getDocumentSummaries();
  }

  listAllDirectories(): string[] {
    const directories = new Set<string>();
    for (const document of this.#getDocumentSummaries()) {
      const segments = document.directory.split('/').filter(Boolean);
      for (let index = 1; index <= segments.length; index += 1) {
        directories.add(segments.slice(0, index).join('/'));
      }
    }
    return [...directories].sort((left, right) => left.localeCompare(right));
  }

  searchDocuments(query: string, limit = 50): SearchDocumentsResult {
    const normalizedQuery = query.trim();
    if (!normalizedQuery) throw new Error('query 不能为空。');
    if (!Number.isInteger(limit) || limit < 1 || limit > 200) {
      throw new Error('limit 必须是 1 到 200 之间的整数。');
    }
    const needle = normalizedQuery.toLocaleLowerCase();
    const matches: DocumentExcerpt[] = [];
    let total = 0;
    for (const document of this.#requireCache().all()) {
      const searchable = [
        document.summary.path,
        document.summary.title,
        document.summary.summary,
        document.summary.assets.join(' '),
        document.content,
      ]
        .filter((value): value is string => value !== null)
        .join('\n')
        .toLocaleLowerCase();
      if (!searchable.includes(needle)) continue;
      total += 1;
      if (matches.length < limit) {
        matches.push({
          ...document.summary,
          excerpt: createExcerpt(document.content, normalizedQuery),
        });
      }
    }
    return { query: normalizedQuery, total, results: matches };
  }

  getDocument(path: string): DocumentContent {
    const normalizedPath = normalizeRelativePath(path);
    const document = this.#requireCache().get(normalizedPath);
    if (!document) throw new Error(`文档不存在：${normalizedPath}`);
    return document;
  }

  getDocumentGraph(excludedPaths: readonly string[] = []): DocumentGraph {
    const exclusions = JSON.stringify([...new Set(excludedPaths)].sort());
    if (this.#documentGraph && this.#documentGraphExclusions === exclusions) {
      return this.#documentGraph;
    }
    const excluded = new Set(excludedPaths);
    const documents = this.#requireCache()
      .all()
      .map((document) => ({
        ...document.summary,
        content: document.content,
      }));
    this.#documentGraph = buildDocumentGraph(documents, excluded);
    this.#documentGraphExclusions = exclusions;
    return this.#documentGraph;
  }

  readAsset(path: string, assetName: string): DocumentAsset {
    const document = this.getDocument(path);
    if (!document.assets.includes(assetName)) {
      throw new Error(`文件不是该文档的附件：${assetName}`);
    }
    const target = resolveExistingAsset(this.#rootPath, path, assetName);
    const data = Uint8Array.from(readFileSync(target)).buffer;
    return {
      name: assetName,
      data,
      size: data.byteLength,
    };
  }

  readDocumentIcon(path: string): DocumentAsset {
    this.getDocument(path);
    const target = resolveExistingAsset(this.#rootPath, path, documentIconName);
    const data = Uint8Array.from(readFileSync(target)).buffer;
    return {
      name: documentIconName,
      data,
      size: data.byteLength,
    };
  }

  writeDocumentIcon(path: string, data: Uint8Array): DocumentContent {
    assertValidDocumentIcon(data);
    const document = this.getDocument(path);
    const target = resolve(
      dirname(resolveExistingDocument(this.#rootPath, document.path)),
      documentIconName,
    );
    if (existsSync(target)) {
      resolveExistingAsset(this.#rootPath, document.path, documentIconName);
    }
    this.#writeBinaryAtomically(target, documentIconName, data);
    this.#refreshDocumentSynchronously(document.path);
    return this.getDocument(document.path);
  }

  deleteDocumentIcon(path: string): DocumentContent {
    const document = this.getDocument(path);
    const target = resolveExistingAsset(
      this.#rootPath,
      document.path,
      documentIconName,
    );
    rmSync(target);
    this.#refreshDocumentSynchronously(document.path);
    return this.getDocument(document.path);
  }

  listArticleAttachments(path: string): ArticleAttachment[] {
    const document = this.#getArticle(path);
    return document.assets.map((name) => {
      const target = resolveExistingAsset(this.#rootPath, document.path, name);
      const stats = statSync(target);
      return {
        name,
        size: stats.size,
        modifiedAt: stats.mtime.toISOString(),
      };
    });
  }

  readArticleAttachment(
    input: ReadArticleAttachmentInput,
  ): ReadArticleAttachmentResult {
    const offset = input.offset ?? 0;
    const maxBytes = input.maxBytes ?? 256 * 1024;
    if (!Number.isInteger(offset) || offset < 0) {
      throw new Error('offset 必须是大于等于 0 的整数。');
    }
    if (!Number.isInteger(maxBytes) || maxBytes < 1 || maxBytes > 1024 * 1024) {
      throw new Error('maxBytes 必须是 1 到 1048576 之间的整数。');
    }

    const document = this.#getArticle(input.path);
    if (!document.assets.includes(input.name)) {
      throw new Error(`文件不是该文章的附件：${input.name}`);
    }
    const target = resolveExistingAsset(
      this.#rootPath,
      document.path,
      input.name,
    );
    const stats = statSync(target);
    if (offset > stats.size) {
      throw new Error('offset 不能超过附件大小。');
    }
    const length = Math.min(maxBytes, stats.size - offset);
    const bytes = new Uint8Array(length);
    const descriptor = openSync(target, 'r');
    let bytesRead = 0;
    try {
      bytesRead = readSync(descriptor, bytes, 0, length, offset);
    } finally {
      closeSync(descriptor);
    }
    return {
      attachment: {
        name: input.name,
        size: stats.size,
        modifiedAt: stats.mtime.toISOString(),
      },
      data: bytes.buffer.slice(0, bytesRead),
      offset,
      nextOffset: offset + bytesRead < stats.size ? offset + bytesRead : null,
    };
  }

  writeArticleAttachment(
    input: WriteArticleAttachmentInput,
  ): ArticleAttachment {
    const document = this.#getArticle(input.path);
    const target = resolveNewAsset(this.#rootPath, document.path, input.name);
    if (existsSync(target)) {
      if (!document.assets.includes(input.name)) {
        throw new Error(`同名路径不是可覆盖的附件：${input.name}`);
      }
      resolveExistingAsset(this.#rootPath, document.path, input.name);
      if (!input.overwrite) {
        throw new Error(`附件已经存在：${input.name}`);
      }
    }

    const temporary = resolve(
      dirname(target),
      `.${basename(target)}.${crypto.randomUUID()}.tmp`,
    );
    try {
      writeFileSync(temporary, input.data, { flag: 'wx' });
      renameSync(temporary, target);
    } catch {
      throw new Error(`无法写入附件：${input.name}`);
    } finally {
      if (existsSync(temporary)) rmSync(temporary);
    }
    const stats = statSync(target);
    this.#refreshDocumentSynchronously(document.path);
    return {
      name: input.name,
      size: stats.size,
      modifiedAt: stats.mtime.toISOString(),
    };
  }

  deleteArticleAttachment(
    input: DeleteArticleAttachmentInput,
  ): DeleteArticleAttachmentResult {
    const document = this.#getArticle(input.path);
    if (isDocumentIconName(input.name)) {
      throw new Error('受保护的 icon.png 不能通过附件接口删除。');
    }
    if (!document.assets.includes(input.name)) {
      throw new Error(`文件不是该文章的附件：${input.name}`);
    }
    const target = resolveExistingAsset(
      this.#rootPath,
      document.path,
      input.name,
    );
    rmSync(target);
    this.#refreshDocumentSynchronously(document.path);
    return {
      path: normalizeArticlePath(input.path),
      name: input.name,
      deleted: true,
    };
  }

  readDocument(path: string, startLine = 1, lineCount = 200) {
    if (!Number.isInteger(startLine) || startLine < 1) {
      throw new Error('startLine 必须是大于等于 1 的整数。');
    }
    if (!Number.isInteger(lineCount) || lineCount < 1 || lineCount > 500) {
      throw new Error('lineCount 必须是 1 到 500 之间的整数。');
    }
    const document = this.getDocument(path);
    const lines = document.content ? document.content.split(/\r?\n/) : [];
    return {
      path: document.path,
      startLine,
      endLine: Math.min(lines.length, startLine + lineCount - 1),
      totalLines: lines.length,
      content: lines.slice(startLine - 1, startLine - 1 + lineCount).join('\n'),
    };
  }

  listChildArticles(parentPath = ''): ArticleListItem[] {
    const normalizedParent = normalizeArticlePath(parentPath);
    if (normalizedParent) {
      this.#getArticle(normalizedParent);
    }

    const documents = this.listAllDocuments().map((summary) =>
      this.getDocument(summary.path),
    );
    const byPath = new Map(
      documents.map((document) => [
        documentPathToArticlePath(document.path),
        document,
      ]),
    );
    const childrenByParent = new Map<string, string[]>();
    for (const path of byPath.keys()) {
      if (!path) continue;
      const parent = articleParentPath(path);
      const children = childrenByParent.get(parent) ?? [];
      children.push(path);
      childrenByParent.set(parent, children);
    }

    const compareArticles = (left: string, right: string): number =>
      left.localeCompare(right);

    const build = (path: string): ArticleListItem => {
      const document = byPath.get(path);
      if (!document) throw new Error(`文章不存在：${path}`);
      const childPaths = (childrenByParent.get(path) ?? []).sort(
        compareArticles,
      );
      return {
        ...articleMetadata(document),
        childCount: childPaths.length,
      };
    };

    return (childrenByParent.get(normalizedParent) ?? [])
      .sort(compareArticles)
      .map(build);
  }

  readArticleLines(input: ReadArticleLinesInput): ReadArticleLinesResult {
    const offset = input.offset ?? 1;
    const limit = input.limit ?? 2_000;
    if (!Number.isInteger(offset) || offset < 1) {
      throw new Error('offset 必须是大于等于 1 的整数。');
    }
    if (!Number.isInteger(limit) || limit < 1 || limit > 2_000) {
      throw new Error('limit 必须是 1 到 2000 之间的整数。');
    }
    const document = this.#getArticle(input.path);
    const lines = articleContentLines(document.content);
    if (offset > lines.length && !(offset === 1 && lines.length === 0)) {
      throw new Error(
        `offset ${offset} 超出文章范围，文章共 ${lines.length} 行。`,
      );
    }
    const selected = lines.slice(offset - 1, offset - 1 + limit);
    const endLine = selected.length > 0 ? offset + selected.length - 1 : 0;
    return {
      article: articleMetadata(document),
      revision: articleRevision(document),
      offset,
      endLine,
      totalLines: lines.length,
      content: selected.join('\n'),
      nextOffset: endLine < lines.length ? endLine + 1 : null,
    };
  }

  writeArticleContent(
    input: WriteArticleContentInput,
  ): WriteArticleContentResult {
    const current = this.#getArticle(input.path);
    if (articleRevision(current) !== input.expectedRevision) {
      throw new Error('文章已发生变化，请重新读取后再更新。');
    }
    const previousTotalLines = articleContentLines(current.content).length;
    if (current.content === input.content) {
      return {
        article: articleMetadata(current),
        revision: articleRevision(current),
        changed: false,
        previousTotalLines,
        totalLines: previousTotalLines,
      };
    }
    const updated = this.updateDocument({
      path: articlePathToDocumentPath(input.path),
      content: input.content,
    });
    return {
      article: articleMetadata(updated),
      revision: articleRevision(updated),
      changed: true,
      previousTotalLines,
      totalLines: articleContentLines(updated.content).length,
    };
  }

  editArticleContent(input: EditArticleContentInput): EditArticleContentResult {
    if (!input.oldString) throw new Error('oldString 不能为空。');
    if (input.oldString === input.newString) {
      throw new Error('newString 必须与 oldString 不同。');
    }
    const current = this.#getArticle(input.path);
    if (articleRevision(current) !== input.expectedRevision) {
      throw new Error('文章已发生变化，请重新读取后再更新。');
    }
    const replacements = current.content.split(input.oldString).length - 1;
    if (replacements === 0) {
      throw new Error('文章正文中找不到 oldString。');
    }
    if (!input.replaceAll && replacements > 1) {
      throw new Error(
        'oldString 在文章正文中出现多次，请提供更多上下文或设置 replaceAll。',
      );
    }
    const content = input.replaceAll
      ? current.content.split(input.oldString).join(input.newString)
      : current.content.replace(input.oldString, input.newString);
    const updated = this.updateDocument({
      path: articlePathToDocumentPath(input.path),
      content,
    });
    return {
      article: articleMetadata(updated),
      revision: articleRevision(updated),
      replacements: input.replaceAll ? replacements : 1,
      totalLines: articleContentLines(updated.content).length,
    };
  }

  createArticle(input: CreateArticleInput) {
    const path = normalizeArticlePath(input.path);
    if (!path) throw new Error('不能创建文档库虚拟根文章。');
    const parent = articleParentPath(path);
    if (parent) this.#getArticle(parent);
    if (this.#requireCache().get(articlePathToDocumentPath(path))) {
      throw new Error(`文章已经存在：${path}`);
    }
    try {
      return this.createDocument({
        path: articlePathToDocumentPath(path),
        title: input.title,
        icon: input.icon,
        summary: input.summary,
        content: input.content,
      });
    } catch (error) {
      if ((error as Error).message.includes('index.md')) {
        throw new Error(`无法创建文章：${path}`);
      }
      throw error;
    }
  }

  updateArticle(input: UpdateArticleInput) {
    const documentPath = articlePathToDocumentPath(input.path);
    const current = this.#getArticle(input.path);
    if (articleRevision(current) !== input.expectedRevision) {
      throw new Error('文章已发生变化，请重新读取后再更新。');
    }
    const metadata = input.metadata;
    const hasMetadata =
      metadata.title !== undefined ||
      metadata.icon !== undefined ||
      metadata.summary !== undefined;
    if (!hasMetadata) {
      throw new Error('至少需要提供一个元数据更新。');
    }
    try {
      return this.updateDocument({
        path: documentPath,
        title: metadata.title,
        icon: metadata.icon,
        summary: metadata.summary,
      });
    } catch (error) {
      if ((error as Error).message.includes('index.md')) {
        throw new Error(
          `无法更新文章：${normalizeArticlePath(input.path) || '根文章'}`,
        );
      }
      throw error;
    }
  }

  moveArticle(input: MoveArticleInput): MoveArticleResult {
    const previousPath = normalizeArticlePath(input.path);
    const current = this.#getArticle(previousPath);
    if (articleRevision(current) !== input.expectedRevision) {
      throw new Error('文章已发生变化，请重新读取后再更新。');
    }
    const path = normalizeArticlePath(input.newPath);
    if (!path) {
      throw new Error('新文章路径不能是文档库虚拟根路径。');
    }
    const pathChanged = path !== previousPath;
    if (!pathChanged) {
      throw new Error('新文章路径必须与当前路径不同。');
    }
    if (!previousPath) {
      throw new Error('不能移动文档库根文章的路径。');
    }
    const descendantPrefix = `${previousPath}/`;
    if (path.startsWith(descendantPrefix)) {
      throw new Error('不能将文章移动到自己的子文章路径中。');
    }
    const parent = articleParentPath(path);
    if (parent) this.#getArticle(parent);
    if (this.#requireCache().get(articlePathToDocumentPath(path))) {
      throw new Error(`文章已经存在：${path}`);
    }

    const sourceDocument = resolveExistingDocument(
      this.#rootPath,
      articlePathToDocumentPath(previousPath),
    );
    const sourceDirectory = dirname(sourceDocument);
    const sourceEntry = lstatSync(sourceDirectory);
    const realSourceDirectory = realpathSync(sourceDirectory);
    if (
      sourceEntry.isSymbolicLink() ||
      !sourceEntry.isDirectory() ||
      realSourceDirectory === this.#rootPath ||
      !isInside(this.#rootPath, realSourceDirectory)
    ) {
      throw new Error(`文章无法安全移动：${previousPath}`);
    }

    const destinationDocument = resolveNewDocument(
      this.#rootPath,
      articlePathToDocumentPath(path),
    );
    const destinationDirectory = dirname(destinationDocument);
    const destinationParent = dirname(destinationDirectory);
    const parentEntry = lstatSync(destinationParent);
    const realDestinationParent = realpathSync(destinationParent);
    if (
      parentEntry.isSymbolicLink() ||
      !parentEntry.isDirectory() ||
      !isInside(this.#rootPath, realDestinationParent)
    ) {
      throw new Error(`文章路径无法安全访问：${path}`);
    }

    const destinationExists = existsSync(destinationDirectory);
    const isCaseOnlyRename =
      destinationExists &&
      realpathSync(destinationDirectory) === realSourceDirectory;
    if (destinationExists && !isCaseOnlyRename) {
      throw new Error(`文章已经存在：${path}`);
    }
    const movedArticles = this.listAllDocuments()
      .map((document) => documentPathToArticlePath(document.path))
      .filter(
        (articlePath) =>
          articlePath === previousPath ||
          articlePath.startsWith(descendantPrefix),
      ).length;
    const temporaryDirectory = resolve(
      dirname(realSourceDirectory),
      `.${basename(realSourceDirectory)}.${crypto.randomUUID()}.rename`,
    );
    let moved = false;

    try {
      if (isCaseOnlyRename) {
        renameSync(realSourceDirectory, temporaryDirectory);
        try {
          renameSync(temporaryDirectory, destinationDirectory);
        } catch (error) {
          renameSync(temporaryDirectory, realSourceDirectory);
          throw error;
        }
      } else {
        renameSync(realSourceDirectory, destinationDirectory);
      }
      moved = true;
      this.#refreshSynchronously();
    } catch {
      if (moved) {
        try {
          if (isCaseOnlyRename) {
            renameSync(destinationDirectory, temporaryDirectory);
            renameSync(temporaryDirectory, realSourceDirectory);
          } else {
            renameSync(destinationDirectory, realSourceDirectory);
          }
          this.#refreshSynchronously();
        } catch {}
      }
      throw new Error(`无法移动文章：${previousPath}`);
    }

    return {
      previousPath,
      path,
      movedArticles,
    };
  }

  deleteArticle(input: DeleteArticleInput): DeleteArticleResult {
    const path = normalizeArticlePath(input.path);
    if (!path) throw new Error('不能删除文档库虚拟根文章。');
    this.#getArticle(path);
    const documentPath = articlePathToDocumentPath(path);
    let target: string;
    try {
      target = resolveExistingDocument(this.#rootPath, documentPath);
    } catch {
      throw new Error(`文章不存在：${path}`);
    }
    const targetDirectory = dirname(target);
    const realDirectory = realpathSync(targetDirectory);
    const directoryEntry = lstatSync(targetDirectory);
    if (
      directoryEntry.isSymbolicLink() ||
      !directoryEntry.isDirectory() ||
      realDirectory === this.#rootPath ||
      !isInside(this.#rootPath, realDirectory)
    ) {
      throw new Error(`文章无法安全删除：${path}`);
    }

    const descendantPrefix = `${path}/`;
    const deletedArticles = this.listAllDocuments()
      .map((document) => documentPathToArticlePath(document.path))
      .filter(
        (articlePath) =>
          articlePath === path || articlePath.startsWith(descendantPrefix),
      );
    if (!input.recursive && deletedArticles.length > 1) {
      throw new Error('文章包含子文章，请显式设置 recursive: true。');
    }
    rmSync(realDirectory, { recursive: true });
    this.#deleteCachedDocumentsSynchronously(
      deletedArticles.map((articlePath) =>
        articlePathToDocumentPath(articlePath),
      ),
    );
    return {
      path,
      recursive: input.recursive ?? false,
      deletedArticles: deletedArticles.length,
    };
  }

  createDocument(input: CreateDocumentInput): DocumentContent {
    const target = resolveNewDocument(this.#rootPath, input.path);
    const normalizedPath = normalizeRelativePath(input.path);
    if (existsSync(target)) throw new Error(`文档已经存在：${normalizedPath}`);
    this.#ensureSafeParent(target);
    const title =
      input.title === undefined
        ? basename(dirname(target))
        : input.title.trim();
    if (!title) throw new Error('title 不能为空。');
    const metadata: DocumentMetadata = {
      archived: false,
      title,
      icon: normalizeIcon(input.icon),
      summary: input.summary?.trim() || null,
    };
    writeFileSync(target, serializeDocument(metadata, input.content ?? ''), {
      encoding: 'utf8',
      flag: 'wx',
    });
    this.#refreshDocumentSynchronously(normalizedPath);
    return this.getDocument(normalizedPath);
  }

  updateDocument(input: UpdateDocumentInput): DocumentContent {
    if (
      input.summary === undefined &&
      input.title === undefined &&
      input.icon === undefined &&
      input.content === undefined
    ) {
      throw new Error('至少需要提供一个要更新的字段。');
    }
    const normalizedPath = normalizeRelativePath(input.path);
    const target = resolveExistingDocument(this.#rootPath, normalizedPath);
    const current = this.getDocument(normalizedPath);
    const metadata: DocumentMetadata = {
      archived: current.archived,
      title: input.title === undefined ? current.title : input.title.trim(),
      icon: input.icon === undefined ? current.icon : normalizeIcon(input.icon),
      summary:
        input.summary === undefined
          ? current.summary
          : input.summary?.trim() || null,
    };
    if (!metadata.title) throw new Error('title 不能为空。');
    this.#writeAtomically(
      target,
      normalizedPath,
      serializeDocument(metadata, input.content ?? current.content),
    );
    this.#refreshDocumentSynchronously(normalizedPath);
    return this.getDocument(normalizedPath);
  }

  setDocumentArchived(path: string, archived: boolean): DocumentContent {
    const normalizedPath = normalizeRelativePath(path);
    const target = resolveExistingDocument(this.#rootPath, normalizedPath);
    const current = this.getDocument(normalizedPath);
    if (current.archived === archived) return current;
    const raw = readFileSync(target, 'utf8');
    this.#writeAtomically(
      target,
      normalizedPath,
      serializeDocumentArchive(raw, archived),
    );
    this.#refreshDocumentSynchronously(normalizedPath);
    return this.getDocument(normalizedPath);
  }

  commitChanges(input: CommitLibraryChangesInput): CommitLibraryChangesResult {
    const result = commitLibraryChanges(this.#rootPath, input);
    this.#notifyListeners();
    return result;
  }

  repositoryStatus(): LibraryRepositoryStatus {
    return getLibraryRepositoryStatus(this.#rootPath);
  }

  repositoryChanges(): LibraryRepositoryChanges {
    return getLibraryRepositoryChanges(this.#rootPath);
  }

  close(): void {
    if (this.#refreshTimer) clearTimeout(this.#refreshTimer);
    this.#refreshTimer = null;
    void this.#watcher?.close();
    this.#watcher = null;
    this.#pendingDocumentPaths.clear();
    this.#fullRefreshPending = false;
    this.#watchBatchStartedAt = null;
    this.#invalidateDerivedViews();
    this.#cache = null;
  }

  #requireCache(): DocumentCache {
    if (!this.#cache) throw new Error('DocumentLibrary 尚未启动。');
    return this.#cache;
  }

  #getDocumentSummaries(): DocumentSummary[] {
    this.#documentSummaries ??= this.#requireCache().summaries();
    return this.#documentSummaries;
  }

  #invalidateDerivedViews(): void {
    this.#documentSummaries = null;
    this.#documentGraph = null;
    this.#documentGraphExclusions = '';
  }

  #getArticle(path: string): DocumentContent {
    const normalizedPath = normalizeArticlePath(path);
    const document = this.#requireCache().get(
      articlePathToDocumentPath(normalizedPath),
    );
    if (!document) {
      throw new Error(
        normalizedPath ? `文章不存在：${normalizedPath}` : '根文章不存在。',
      );
    }
    return document;
  }

  #scanDirectory(directory: string): ParsedCacheDocument[] {
    const documents: ParsedCacheDocument[] = [];
    const entries = readdirSync(directory, { withFileTypes: true });
    const siblingAssets = this.#assetNames(directory, entries);
    const iconImageModifiedAt = this.#iconImageModifiedAt(directory, entries);

    for (const entry of entries) {
      if (entry.isSymbolicLink()) continue;
      const absolutePath = resolve(directory, entry.name);
      if (entry.isDirectory()) {
        if (entry.name.startsWith('.') || ignoredDirectories.has(entry.name)) {
          continue;
        }
        documents.push(...this.#scanDirectory(absolutePath));
        continue;
      }
      if (
        !entry.isFile() ||
        !isDocumentIndexPath(entry.name) ||
        (directory === this.#rootPath && entry.name === documentIndexName)
      ) {
        continue;
      }
      documents.push(
        this.#parseDocumentFile(
          absolutePath,
          siblingAssets,
          iconImageModifiedAt,
        ),
      );
    }
    return documents.sort((left, right) =>
      left.summary.path.localeCompare(right.summary.path),
    );
  }

  async #startWatcher(): Promise<void> {
    const watcher = watch(this.#rootPath, {
      ignoreInitial: true,
      persistent: true,
      usePolling: false,
      ignored: (path, stats) => {
        const name = basename(path);
        return Boolean(
          stats?.isDirectory() &&
          (name.startsWith('.') || ignoredDirectories.has(name)),
        );
      },
    });
    this.#watcher = watcher;
    watcher.on('all', (event, path) => {
      this.#queueWatchedChange(event, path);
    });
    await new Promise<void>((ready, reject) => {
      watcher.once('ready', ready);
      watcher.once('error', reject);
    }).catch((error) => {
      throw new Error(`无法监听文档库变化：${(error as Error).message}`);
    });
    watcher.on('error', (error) => {
      console.error(
        `文档库监听失败：${error instanceof Error ? error.message : String(error)}`,
      );
    });
  }

  #queueWatchedChange(event: string, path: string): void {
    const absolutePath = resolve(this.#rootPath, path);
    if (!isInside(this.#rootPath, absolutePath)) return;

    if (event === 'unlinkDir') {
      this.#fullRefreshPending = true;
      this.#scheduleRefresh();
      return;
    }
    if (event === 'addDir') return;

    if (isDocumentIndexPath(absolutePath)) {
      const documentPath = relative(this.#rootPath, absolutePath)
        .split(sep)
        .join('/');
      if (documentPath !== documentIndexName) {
        this.#pendingDocumentPaths.add(documentPath);
        this.#scheduleRefresh();
      }
      return;
    }

    if (
      isDocumentIconName(basename(absolutePath)) ||
      event === 'add' ||
      event === 'unlink'
    ) {
      const documentPath = relative(
        this.#rootPath,
        resolve(dirname(absolutePath), documentIndexName),
      )
        .split(sep)
        .join('/');
      if (documentPath !== documentIndexName) {
        this.#pendingDocumentPaths.add(documentPath);
        this.#scheduleRefresh();
      }
    }
  }

  #scheduleRefresh(retry = false): void {
    if (!retry) {
      this.#refreshRetryCount = 0;
      if (this.#watchBatchStartedAt === null) {
        this.#watchBatchStartedAt = Date.now();
        this.#notifyListeners();
      }
    }
    if (this.#refreshTimer) clearTimeout(this.#refreshTimer);
    const batchStartedAt = this.#watchBatchStartedAt ?? Date.now();
    const delay = retry
      ? watchDebounceMs
      : Math.min(
          watchDebounceMs,
          Math.max(0, watchMaxWaitMs - (Date.now() - batchStartedAt)),
        );
    this.#refreshTimer = setTimeout(() => {
      this.#refreshTimer = null;
      void this.#refreshWatchedChanges().then(
        () => {
          this.#refreshRetryCount = 0;
          this.#finishWatchedRefresh();
        },
        (error: unknown) => {
          if (!this.#cache) return;
          if (this.#refreshRetryCount < maxRefreshRetries) {
            this.#refreshRetryCount += 1;
            this.#scheduleRefresh(true);
            return;
          }
          this.#refreshRetryCount = 0;
          this.#finishWatchedRefresh();
          console.error(
            `刷新文档库失败：${error instanceof Error ? error.message : String(error)}`,
          );
        },
      );
    }, delay);
  }

  #finishWatchedRefresh(): void {
    if (this.#watchBatchStartedAt === null) return;
    this.#watchBatchStartedAt = null;
    this.#notifyListeners();
  }

  async #refreshWatchedChanges(): Promise<LibraryStatus> {
    if (this.#fullRefreshPending) {
      this.#fullRefreshPending = false;
      this.#pendingDocumentPaths.clear();
      return this.refresh();
    }

    const documentPaths = [...this.#pendingDocumentPaths];
    this.#pendingDocumentPaths.clear();
    if (documentPaths.length === 0) return this.status();

    const cache = this.#requireCache();
    const indexedAt = new Date().toISOString();
    let changed = false;
    try {
      for (const documentPath of documentPaths) {
        const target = resolve(this.#rootPath, documentPath);
        if (!existsSync(target)) {
          changed = cache.delete([documentPath], indexedAt) || changed;
          continue;
        }
        const resolvedTarget = resolveExistingDocument(
          this.#rootPath,
          documentPath,
        );
        const directory = dirname(resolvedTarget);
        const entries = readdirSync(directory, { withFileTypes: true });
        const document = this.#parseDocumentFile(
          resolvedTarget,
          this.#assetNames(directory, entries),
          this.#iconImageModifiedAt(directory, entries),
        );
        changed = cache.upsert(document, indexedAt) || changed;
      }
    } catch (error) {
      for (const documentPath of documentPaths) {
        this.#pendingDocumentPaths.add(documentPath);
      }
      throw error;
    }

    const status = this.status();
    if (changed) {
      this.#invalidateDerivedViews();
      this.#notifyListeners(status);
    }
    return status;
  }

  #refreshSynchronously(): void {
    const documents = this.#scanDirectory(this.#rootPath);
    this.#requireCache().replaceAll(documents, new Date().toISOString());
    this.#invalidateDerivedViews();
    this.#notifyListeners();
  }

  #refreshDocumentSynchronously(documentPath: string): void {
    const target = resolveExistingDocument(this.#rootPath, documentPath);
    const directory = dirname(target);
    const entries = readdirSync(directory, { withFileTypes: true });
    const document = this.#parseDocumentFile(
      target,
      this.#assetNames(directory, entries),
      this.#iconImageModifiedAt(directory, entries),
    );
    this.#requireCache().upsert(document, new Date().toISOString());
    this.#invalidateDerivedViews();
    this.#notifyListeners();
  }

  #deleteCachedDocumentsSynchronously(documentPaths: string[]): void {
    this.#requireCache().delete(documentPaths, new Date().toISOString());
    this.#invalidateDerivedViews();
    this.#notifyListeners();
  }

  #assetNames(directory: string, entries: Dirent[]): string[] {
    return entries
      .filter((entry) => {
        if (
          entry.isSymbolicLink() ||
          !entry.isFile() ||
          entry.name.startsWith('.') ||
          isDocumentIconName(entry.name) ||
          isDocumentIndexPath(entry.name)
        ) {
          return false;
        }
        return true;
      })
      .map((entry) => entry.name)
      .sort((left, right) => left.localeCompare(right));
  }

  #iconImageModifiedAt(directory: string, entries: Dirent[]): string | null {
    const entry = entries.find(
      (candidate) =>
        candidate.name === documentIconName &&
        candidate.isFile() &&
        !candidate.isSymbolicLink(),
    );
    return entry
      ? statSync(resolve(directory, documentIconName)).mtime.toISOString()
      : null;
  }

  #parseDocumentFile(
    absolutePath: string,
    siblingAssets: string[],
    iconImageModifiedAt: string | null,
  ): ParsedCacheDocument {
    const path = relative(this.#rootPath, absolutePath).split(sep).join('/');
    const document = parseDocument(
      path,
      basename(dirname(absolutePath)),
      readFileSync(absolutePath, 'utf8'),
      statSync(absolutePath),
    );
    document.summary.assets = siblingAssets;
    document.summary.iconImageModifiedAt = iconImageModifiedAt;
    return document;
  }

  #notifyListeners(status = this.status()): void {
    for (const listener of this.#listeners) listener(status);
  }

  #ensureSafeParent(target: string): void {
    const parent = dirname(target);
    mkdirSync(parent, { recursive: true });
    let cursor = parent;
    while (cursor !== this.#rootPath) {
      const entry = lstatSync(cursor);
      if (entry.isSymbolicLink() || !entry.isDirectory()) {
        throw new Error(`目录无法安全访问：${basename(cursor)}`);
      }
      const next = dirname(cursor);
      if (next === cursor || !isInside(this.#rootPath, next)) break;
      cursor = next;
    }
    if (!isInside(this.#rootPath, realpathSync(parent))) {
      throw new Error('文档路径超出文档库范围。');
    }
  }

  #writeAtomically(target: string, displayPath: string, content: string): void {
    const temporary = resolve(
      dirname(target),
      `.${basename(target)}.${crypto.randomUUID()}.tmp`,
    );
    try {
      writeFileSync(temporary, content, {
        encoding: 'utf8',
        flag: 'wx',
        mode: statSync(target).mode,
      });
      renameSync(temporary, target);
    } catch {
      throw new Error(`无法更新文档：${displayPath}`);
    } finally {
      if (existsSync(temporary)) rmSync(temporary);
    }
  }

  #writeBinaryAtomically(
    target: string,
    displayPath: string,
    content: Uint8Array,
  ): void {
    const temporary = resolve(
      dirname(target),
      `.${basename(target)}.${crypto.randomUUID()}.tmp`,
    );
    try {
      writeFileSync(temporary, content, { flag: 'wx' });
      renameSync(temporary, target);
    } catch {
      throw new Error(`无法写入文件：${displayPath}`);
    } finally {
      if (existsSync(temporary)) rmSync(temporary);
    }
  }
}
