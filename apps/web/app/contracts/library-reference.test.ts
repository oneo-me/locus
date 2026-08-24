import { describe, expect, test } from 'vitest';
import {
  libraryReferenceScheme,
  stripLibraryReferenceScheme,
} from './library-reference';

describe('library reference scheme', () => {
  test('defines the canonical asterweft:// prefix', () => {
    expect(libraryReferenceScheme).toBe('asterweft://');
  });

  test('strips the scheme prefix from references', () => {
    expect(stripLibraryReferenceScheme('asterweft://projects')).toBe(
      'projects',
    );
    expect(stripLibraryReferenceScheme('asterweft://projects/asterweft')).toBe(
      'projects/asterweft',
    );
    expect(stripLibraryReferenceScheme('asterweft://')).toBe('');
  });

  test('accepts the scheme case-insensitively while preserving the path', () => {
    expect(stripLibraryReferenceScheme('ASTERWEFT://Projects')).toBe(
      'Projects',
    );
  });

  test('leaves bare paths unchanged', () => {
    expect(stripLibraryReferenceScheme('projects')).toBe('projects');
    expect(stripLibraryReferenceScheme('')).toBe('');
    expect(stripLibraryReferenceScheme('asterweft:/projects')).toBe(
      'asterweft:/projects',
    );
  });
});
