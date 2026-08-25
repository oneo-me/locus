import {
  archivedDocumentPaths,
  isDocumentArchived,
} from './document-archive.js';
import type { DocumentSummary } from './library-types.js';

export interface LibraryState {
  order: Record<string, number>;
  graphParticipation: Record<string, boolean>;
}

export interface DocumentGraphParticipation {
  configured: boolean;
  inheritedFrom: string | null;
  participates: boolean;
}

function normalizedGraphParticipation(value: unknown): Record<string, boolean> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  return Object.fromEntries(
    Object.entries(value)
      .map(([path, participates]) => [path.trim(), participates] as const)
      .filter(
        (entry): entry is readonly [string, boolean] =>
          entry[0].length > 0 && typeof entry[1] === 'boolean',
      ),
  );
}

function normalizedOrder(value: unknown): Record<string, number> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  return Object.fromEntries(
    Object.entries(value).filter(
      (entry): entry is [string, number] =>
        entry[0].length > 0 &&
        typeof entry[1] === 'number' &&
        Number.isFinite(entry[1]),
    ),
  );
}

export function normalizeLibraryState(value: unknown): LibraryState {
  if (!value || typeof value !== 'object') {
    return { order: {}, graphParticipation: {} };
  }
  const input = value as Partial<LibraryState>;
  return {
    order: normalizedOrder(input.order),
    graphParticipation: normalizedGraphParticipation(input.graphParticipation),
  };
}

function parentDocumentPaths(path: string): string[] {
  const segments = path.split('/').filter(Boolean);
  if (segments.at(-1) === 'index.md') segments.pop();

  const parents: string[] = [];
  while (segments.length > 0) {
    segments.pop();
    parents.push(
      segments.length > 0 ? `${segments.join('/')}/index.md` : 'index.md',
    );
  }
  return parents;
}

export function documentGraphParticipation(
  state: LibraryState,
  path: string,
): DocumentGraphParticipation {
  const configured = state.graphParticipation[path] ?? true;
  const inheritedFrom =
    parentDocumentPaths(path).find(
      (parentPath) => state.graphParticipation[parentPath] === false,
    ) ?? null;
  return {
    configured,
    inheritedFrom,
    participates: configured && inheritedFrom === null,
  };
}

export function participatesInDocumentGraph(
  state: LibraryState,
  path: string,
): boolean {
  return documentGraphParticipation(state, path).participates;
}

export function excludedDocumentGraphPaths(
  state: LibraryState,
  documents: readonly DocumentSummary[],
): string[] {
  const archivedPaths = archivedDocumentPaths(documents);
  return documents
    .map((document) => document.path)
    .filter(
      (path) =>
        isDocumentArchived(archivedPaths, path) ||
        !participatesInDocumentGraph(state, path),
    );
}
