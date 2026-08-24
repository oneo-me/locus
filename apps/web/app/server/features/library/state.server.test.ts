import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { resolveLibraryStatePath } from '~/server/features/settings/global-paths.server';
import { afterEach, describe, expect, test } from 'vitest';
import {
  documentGraphParticipation,
  excludedDocumentGraphPaths,
  participatesInDocumentGraph,
} from './state';
import {
  normalizeLibraryState,
  readLibraryState,
  updateDocumentGraphParticipation,
  updateLibraryDocumentOrder,
} from './state.server';

const temporaryDirectories: string[] = [];

function temporaryDirectory(prefix: string): string {
  const directory = mkdtempSync(join(tmpdir(), prefix));
  temporaryDirectories.push(directory);
  return directory;
}

afterEach(() => {
  for (const directory of temporaryDirectories.splice(0)) {
    rmSync(directory, { recursive: true, force: true });
  }
});

describe('library tree state', () => {
  test('stores tree state at the fixed global path', () => {
    const home = temporaryDirectory('asterweft-home-');
    updateLibraryDocumentOrder(
      [
        { path: 'beta/index.md', order: 0 },
        { path: 'alpha/index.md', order: 1 },
      ],
      'development',
      home,
    );
    updateDocumentGraphParticipation(
      'guide/setup/index.md',
      false,
      'development',
      home,
    );

    expect(readLibraryState('development', home)).toEqual({
      order: {
        'beta/index.md': 0,
        'alpha/index.md': 1,
      },
      graphParticipation: {
        'guide/setup/index.md': false,
      },
    });
    expect(
      JSON.parse(
        readFileSync(resolveLibraryStatePath('development', home), 'utf8'),
      ),
    ).toEqual({
      order: {
        'beta/index.md': 0,
        'alpha/index.md': 1,
      },
      graphParticipation: {
        'guide/setup/index.md': false,
      },
    });
  });

  test('normalizes malformed state and ignores invalid values', () => {
    expect(
      normalizeLibraryState({
        order: {
          'valid/index.md': 2,
          invalid: 'first',
          infinite: Number.POSITIVE_INFINITY,
        },
        graphParticipation: {
          'valid/index.md': false,
          ' enabled/index.md ': true,
          invalid: 'false',
        },
      }),
    ).toEqual({
      order: { 'valid/index.md': 2 },
      graphParticipation: {
        'valid/index.md': false,
        'enabled/index.md': true,
      },
    });
  });

  test('defaults graph participation to true and persists explicit changes', () => {
    const home = temporaryDirectory('asterweft-home-');
    const initial = readLibraryState('development', home);

    expect(participatesInDocumentGraph(initial, 'notes/index.md')).toBe(true);

    const excluded = updateDocumentGraphParticipation(
      'notes/index.md',
      false,
      'development',
      home,
    );
    expect(participatesInDocumentGraph(excluded, 'notes/index.md')).toBe(false);

    const included = updateDocumentGraphParticipation(
      'notes/index.md',
      true,
      'development',
      home,
    );
    expect(participatesInDocumentGraph(included, 'notes/index.md')).toBe(true);
  });

  test('inherits disabled graph participation through every descendant', () => {
    const state = normalizeLibraryState({
      graphParticipation: {
        'guide/index.md': false,
        'guide/setup/index.md': true,
        'other/index.md': true,
      },
    });

    expect(documentGraphParticipation(state, 'guide/setup/index.md')).toEqual({
      configured: true,
      inheritedFrom: 'guide/index.md',
      participates: false,
    });
    expect(
      participatesInDocumentGraph(state, 'guide/setup/deep/index.md'),
    ).toBe(false);
    expect(participatesInDocumentGraph(state, 'other/index.md')).toBe(true);
    expect(
      excludedDocumentGraphPaths(state, [
        'guide/index.md',
        'guide/setup/index.md',
        'guide/setup/deep/index.md',
        'other/index.md',
      ]),
    ).toEqual([
      'guide/index.md',
      'guide/setup/index.md',
      'guide/setup/deep/index.md',
    ]);
  });

  test('falls back to empty state when the state file is invalid', () => {
    const home = temporaryDirectory('asterweft-home-');
    const statePath = resolveLibraryStatePath('development', home);

    updateLibraryDocumentOrder(
      [{ path: 'notes/index.md', order: 0 }],
      'development',
      home,
    );
    writeFileSync(statePath, '{broken', 'utf8');

    expect(readLibraryState('development', home)).toEqual({
      order: {},
      graphParticipation: {},
    });
  });
});
