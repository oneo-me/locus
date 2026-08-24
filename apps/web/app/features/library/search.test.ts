import { describe, expect, test } from 'vitest';
import { parseLibrarySearch } from './search';

describe('parseLibrarySearch', () => {
  test('keeps string values and drops unsafe shapes', () => {
    expect(parseLibrarySearch({ q: 'Bun' })).toEqual({
      q: 'Bun',
    });
    expect(parseLibrarySearch({ q: 42 })).toEqual({
      q: undefined,
    });
  });
});
