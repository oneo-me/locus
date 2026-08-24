export interface LibraryState {
  order: Record<string, number>;
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
    return { order: {} };
  }
  const input = value as Partial<LibraryState>;
  return {
    order: normalizedOrder(input.order),
  };
}
