import { openSqliteDatabase, type SqliteDatabase } from './sqlite-runtime.js';
import type {
  DocumentContent,
  DocumentSummary,
} from '../contracts/library-types.js';

interface CacheRow {
  path: string;
  directory: string;
  title: string;
  icon: string | null;
  icon_image_modified_at: string | null;
  summary: string | null;
  assets: string;
  metadata_keys: string;
  modified_at: string;
  size: number;
  word_count: number;
  issue: string | null;
  content: string;
}

interface CachedDocument {
  summary: DocumentSummary;
  content: string;
}

type CacheSummaryRow = Omit<CacheRow, 'content'>;

function summaryFromRow(row: CacheSummaryRow): DocumentSummary {
  return {
    path: row.path,
    directory: row.directory,
    title: row.title,
    icon: row.icon,
    iconImageModifiedAt: row.icon_image_modified_at ?? null,
    metadataKeys: JSON.parse(
      row.metadata_keys,
    ) as DocumentSummary['metadataKeys'],
    summary: row.summary,
    assets: JSON.parse(row.assets) as string[],
    modifiedAt: row.modified_at,
    size: row.size,
    wordCount: row.word_count,
    issue: row.issue,
  };
}

function fromRow(row: CacheRow): CachedDocument {
  return { summary: summaryFromRow(row), content: row.content };
}

function matchesParsed(
  cached: CachedDocument,
  document: ParsedCacheDocument,
): boolean {
  return (
    cached.content === document.content &&
    JSON.stringify(cached.summary) === JSON.stringify(document.summary)
  );
}

export class DocumentCache {
  readonly #database: SqliteDatabase;

  constructor(path: string) {
    this.#database = openSqliteDatabase(path);
    this.#database.exec('PRAGMA journal_mode = WAL;');
    this.#database.exec('PRAGMA synchronous = NORMAL;');
    this.#database.exec(`
      CREATE TABLE IF NOT EXISTS documents (
        path TEXT PRIMARY KEY,
        directory TEXT NOT NULL,
        title TEXT NOT NULL,
        icon TEXT,
        icon_image_modified_at TEXT,
        summary TEXT,
        assets TEXT NOT NULL DEFAULT '[]',
        metadata_keys TEXT NOT NULL DEFAULT '[]',
        modified_at TEXT NOT NULL,
        size INTEGER NOT NULL,
        word_count INTEGER NOT NULL,
        issue TEXT,
        content TEXT NOT NULL
      );
      CREATE INDEX IF NOT EXISTS documents_directory_idx ON documents(directory);
      CREATE TABLE IF NOT EXISTS cache_state (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
      );
    `);
    const columns = this.#database
      .prepare('PRAGMA table_info(documents)')
      .all() as Array<{ name: string }>;
    if (!columns.some((column) => column.name === 'icon')) {
      this.#database.exec('ALTER TABLE documents ADD COLUMN icon TEXT');
    }
    if (!columns.some((column) => column.name === 'icon_image_modified_at')) {
      this.#database.exec(
        'ALTER TABLE documents ADD COLUMN icon_image_modified_at TEXT',
      );
    }
    if (!columns.some((column) => column.name === 'assets')) {
      this.#database.exec(
        "ALTER TABLE documents ADD COLUMN assets TEXT NOT NULL DEFAULT '[]'",
      );
    }
    if (!columns.some((column) => column.name === 'metadata_keys')) {
      this.#database.exec(
        "ALTER TABLE documents ADD COLUMN metadata_keys TEXT NOT NULL DEFAULT '[]'",
      );
    }
    if (!columns.some((column) => column.name === 'summary')) {
      this.#database.exec('ALTER TABLE documents ADD COLUMN summary TEXT');
    }
    if (columns.some((column) => column.name === 'headings')) {
      this.#database.exec('ALTER TABLE documents DROP COLUMN headings');
    }
    if (columns.some((column) => column.name === 'tags')) {
      this.#database.exec('ALTER TABLE documents DROP COLUMN tags');
    }
    if (columns.some((column) => column.name === 'description')) {
      this.#database.exec('ALTER TABLE documents DROP COLUMN description');
    }
    if (columns.some((column) => column.name === 'source')) {
      this.#database.exec('ALTER TABLE documents DROP COLUMN source');
    }
    if (columns.some((column) => column.name === 'extra_attributes')) {
      this.#database.exec('ALTER TABLE documents DROP COLUMN extra_attributes');
    }
    if (columns.some((column) => column.name === 'create_at')) {
      this.#database.exec('ALTER TABLE documents DROP COLUMN create_at');
    }
    if (columns.some((column) => column.name === 'update_at')) {
      this.#database.exec('ALTER TABLE documents DROP COLUMN update_at');
    }
  }

  replaceAll(documents: ParsedCacheDocument[], indexedAt: string): void {
    const insert = this.#database.prepare(`
      INSERT INTO documents (
        path, directory, title, icon, icon_image_modified_at,
        summary, assets, metadata_keys,
        modified_at, size, word_count, issue, content
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);
    this.#database.exec('BEGIN IMMEDIATE;');
    try {
      this.#database.exec('DELETE FROM documents;');
      for (const document of documents) {
        const summary = document.summary;
        insert.run(
          summary.path,
          summary.directory,
          summary.title,
          summary.icon,
          summary.iconImageModifiedAt,
          summary.summary,
          JSON.stringify(summary.assets),
          JSON.stringify(summary.metadataKeys),
          summary.modifiedAt,
          summary.size,
          summary.wordCount,
          summary.issue,
          document.content,
        );
      }
      this.#database
        .prepare(
          'INSERT OR REPLACE INTO cache_state (key, value) VALUES (?, ?)',
        )
        .run('indexedAt', indexedAt);
      this.#database.exec('COMMIT;');
    } catch (error) {
      this.#database.exec('ROLLBACK;');
      throw error;
    }
  }

  upsert(document: ParsedCacheDocument, indexedAt: string): boolean {
    const cached = this.get(document.summary.path);
    if (cached) {
      const { content, ...summary } = cached;
      if (matchesParsed({ summary, content }, document)) return false;
    }
    const summary = document.summary;
    this.#database.exec('BEGIN IMMEDIATE;');
    try {
      this.#database
        .prepare(
          `
            INSERT OR REPLACE INTO documents (
              path, directory, title, icon, icon_image_modified_at,
              summary, assets, metadata_keys,
              modified_at, size, word_count, issue, content
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
          `,
        )
        .run(
          summary.path,
          summary.directory,
          summary.title,
          summary.icon,
          summary.iconImageModifiedAt,
          summary.summary,
          JSON.stringify(summary.assets),
          JSON.stringify(summary.metadataKeys),
          summary.modifiedAt,
          summary.size,
          summary.wordCount,
          summary.issue,
          document.content,
        );
      this.#setIndexedAt(indexedAt);
      this.#database.exec('COMMIT;');
    } catch (error) {
      this.#database.exec('ROLLBACK;');
      throw error;
    }
    return true;
  }

  delete(paths: string[], indexedAt: string): boolean {
    if (paths.length === 0) return false;
    const remove = this.#database.prepare(
      'DELETE FROM documents WHERE path = ?',
    );
    let changed = false;
    this.#database.exec('BEGIN IMMEDIATE;');
    try {
      for (const path of paths) {
        const result = remove.run(path) as { changes?: number };
        changed = changed || (result.changes ?? 0) > 0;
      }
      if (!changed) {
        this.#database.exec('ROLLBACK;');
        return false;
      }
      this.#setIndexedAt(indexedAt);
      this.#database.exec('COMMIT;');
    } catch (error) {
      this.#database.exec('ROLLBACK;');
      throw error;
    }
    return true;
  }

  all(): CachedDocument[] {
    const rows = this.#database
      .prepare('SELECT * FROM documents ORDER BY path')
      .all() as CacheRow[];
    return rows.map(fromRow);
  }

  summaries(): DocumentSummary[] {
    const rows = this.#database
      .prepare(
        `SELECT
          path, directory, title, icon, icon_image_modified_at,
          summary, assets, metadata_keys,
          modified_at, size, word_count, issue
        FROM documents
        ORDER BY path`,
      )
      .all() as CacheSummaryRow[];
    return rows.map(summaryFromRow);
  }

  matches(documents: ParsedCacheDocument[]): boolean {
    const cached = this.all();
    if (cached.length !== documents.length) return false;
    return cached.every((document, index) => {
      const candidate = documents[index];
      return candidate ? matchesParsed(document, candidate) : false;
    });
  }

  get(path: string): DocumentContent | null {
    const row = this.#database
      .prepare('SELECT * FROM documents WHERE path = ?')
      .get(path) as CacheRow | undefined;
    if (!row) return null;
    const document = fromRow(row);
    return {
      ...document.summary,
      content: document.content,
    };
  }

  count(): number {
    const row = this.#database
      .prepare('SELECT COUNT(*) AS count FROM documents')
      .get() as { count: number } | undefined;
    return row?.count ?? 0;
  }

  indexedAt(): string | null {
    const row = this.#database
      .prepare('SELECT value FROM cache_state WHERE key = ?')
      .get('indexedAt') as { value: string } | undefined;
    return row?.value ?? null;
  }

  close(): void {
    this.#database.close();
  }

  #setIndexedAt(indexedAt: string): void {
    this.#database
      .prepare('INSERT OR REPLACE INTO cache_state (key, value) VALUES (?, ?)')
      .run('indexedAt', indexedAt);
  }
}

export interface ParsedCacheDocument {
  summary: DocumentSummary;
  content: string;
}
