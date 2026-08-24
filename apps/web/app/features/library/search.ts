export interface LibrarySearch {
  q?: string;
}

export function parseLibrarySearch(
  search: Record<string, unknown>,
): LibrarySearch {
  return {
    q: typeof search.q === 'string' ? search.q : undefined,
  };
}
