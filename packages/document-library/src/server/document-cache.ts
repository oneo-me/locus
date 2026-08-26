import type {
  DocumentContent,
  DocumentSummary,
} from '../contracts/library-types.js';

interface CachedDocument {
  summary: DocumentSummary;
  additionalMetadata: Record<string, unknown>;
  content: string;
}

function matchesParsed(
  cached: CachedDocument,
  document: ParsedCacheDocument,
): boolean {
  return (
    cached.content === document.content &&
    JSON.stringify(cached.additionalMetadata) ===
      JSON.stringify(document.additionalMetadata) &&
    JSON.stringify(cached.summary) === JSON.stringify(document.summary)
  );
}

export class DocumentCache {
  readonly #documents = new Map<string, CachedDocument>();
  #indexedAt: string | null = null;

  replaceAll(documents: ParsedCacheDocument[], indexedAt: string): void {
    this.#documents.clear();
    for (const document of documents) {
      this.#documents.set(document.summary.path, document);
    }
    this.#indexedAt = indexedAt;
  }

  upsert(document: ParsedCacheDocument, indexedAt: string): boolean {
    const cached = this.#documents.get(document.summary.path);
    if (cached && matchesParsed(cached, document)) return false;
    this.#documents.set(document.summary.path, document);
    this.#indexedAt = indexedAt;
    return true;
  }

  delete(paths: string[], indexedAt: string): boolean {
    let changed = false;
    for (const path of paths) changed = this.#documents.delete(path) || changed;
    if (!changed) return false;
    this.#indexedAt = indexedAt;
    return true;
  }

  all(): CachedDocument[] {
    return [...this.#documents.values()].sort((left, right) =>
      left.summary.path.localeCompare(right.summary.path),
    );
  }

  summaries(): DocumentSummary[] {
    return this.all().map((document) => document.summary);
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
    const document = this.#documents.get(path);
    if (!document) return null;
    return {
      ...document.summary,
      additionalMetadata: document.additionalMetadata,
      content: document.content,
    };
  }

  count(): number {
    return this.#documents.size;
  }

  indexedAt(): string | null {
    return this.#indexedAt;
  }
}

export interface ParsedCacheDocument {
  summary: DocumentSummary;
  additionalMetadata: Record<string, unknown>;
  content: string;
}
