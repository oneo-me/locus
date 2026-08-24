import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DocumentLibrary } from '~/server/features/library/core/document-library';
import { execFileSync } from '~/server/test/node-process';
import { afterEach, describe, expect, test } from 'vitest';
import { createGraphEventsResponse } from './graph-events';
import type { GraphEventSnapshot } from './types';

const roots: string[] = [];

function temporaryPath(prefix: string): string {
  const path = mkdtempSync(join(tmpdir(), prefix));
  roots.push(path);
  return path;
}

function initializeRepository(root: string): void {
  execFileSync('git', ['init', '--quiet'], { cwd: root });
  execFileSync('git', ['config', 'user.name', 'Asterweft Test'], { cwd: root });
  execFileSync('git', ['config', 'user.email', 'test@asterweft.local'], {
    cwd: root,
  });
  execFileSync('git', ['add', '--all'], { cwd: root });
  execFileSync('git', ['commit', '--quiet', '-m', 'docs: initialize notes'], {
    cwd: root,
  });
}

function snapshotFromChunk(chunk?: Uint8Array): GraphEventSnapshot {
  if (!chunk) throw new Error('Expected an SSE data chunk.');
  const frame = new TextDecoder().decode(chunk);
  const data = frame.match(/^data: (.+)\n\n$/)?.[1];
  if (!data) throw new Error(`Expected an SSE data frame, received: ${frame}`);
  return JSON.parse(data) as GraphEventSnapshot;
}

afterEach(() => {
  for (const root of roots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

describe('graph event stream', () => {
  test('coalesces rapid changes without sending selected article content', async () => {
    const root = temporaryPath('asterweft-events-');
    const cacheRoot = temporaryPath('asterweft-events-cache-');
    mkdirSync(join(root, 'notes'));
    writeFileSync(
      join(root, 'notes', 'index.md'),
      '---\ntitle: Notes\n---\nInitial',
    );
    initializeRepository(root);
    const library = await DocumentLibrary.open(root, {
      cachePath: join(cacheRoot, 'cache.sqlite'),
      watch: false,
    });
    const abort = new AbortController();
    const response = await createGraphEventsResponse(
      new Request('http://localhost/api/graph-events?doc=notes/index.md', {
        signal: abort.signal,
      }),
      {
        heartbeatIntervalMs: 60_000,
        library,
        readState: () => ({
          graphParticipation: {},
          order: {},
        }),
        snapshotDebounceMs: 20,
      },
    );
    const reader = response.body?.getReader();
    if (!reader) throw new Error('Expected an SSE response body.');

    const initial = await reader.read();
    expect(initial.done).toBe(false);
    const initialSnapshot = snapshotFromChunk(initial.value);
    expect(initialSnapshot.graph).toBeUndefined();
    expect(typeof initialSnapshot.repositoryRevision).toBe('string');
    expect(JSON.stringify(initialSnapshot)).not.toContain('Initial');

    library.updateDocument({
      path: 'notes/index.md',
      title: 'First update',
    });
    library.updateDocument({
      path: 'notes/index.md',
      title: 'Latest update',
    });

    const update = await reader.read();
    expect(update.done).toBe(false);
    expect(snapshotFromChunk(update.value).navigation.documents[0]?.title).toBe(
      'Latest update',
    );

    abort.abort();
    await reader.cancel();
    library.close();
  });

  test('sends article deletions and attachment count changes', async () => {
    const root = temporaryPath('asterweft-events-');
    const cacheRoot = temporaryPath('asterweft-events-cache-');
    mkdirSync(join(root, 'notes'));
    writeFileSync(
      join(root, 'notes', 'index.md'),
      '---\ntitle: Notes\n---\nInitial',
    );
    initializeRepository(root);
    const library = await DocumentLibrary.open(root, {
      cachePath: join(cacheRoot, 'cache.sqlite'),
      watch: false,
    });
    const abort = new AbortController();
    const response = await createGraphEventsResponse(
      new Request('http://localhost/api/graph-events', {
        signal: abort.signal,
      }),
      {
        heartbeatIntervalMs: 60_000,
        library,
        readState: () => ({
          graphParticipation: {},
          order: {},
        }),
        snapshotDebounceMs: 20,
      },
    );
    const reader = response.body?.getReader();
    if (!reader) throw new Error('Expected an SSE response body.');

    const initial = await reader.read();
    expect(
      snapshotFromChunk(initial.value).navigation.documents[0]?.attachmentCount,
    ).toBe(0);

    library.writeArticleAttachment({
      path: 'notes',
      name: 'brief.pdf',
      data: new TextEncoder().encode('brief'),
    });
    const attachmentAdded = await reader.read();
    expect(
      snapshotFromChunk(attachmentAdded.value).navigation.documents[0]
        ?.attachmentCount,
    ).toBe(1);

    library.deleteArticleAttachment({ path: 'notes', name: 'brief.pdf' });
    const attachmentDeleted = await reader.read();
    expect(
      snapshotFromChunk(attachmentDeleted.value).navigation.documents[0]
        ?.attachmentCount,
    ).toBe(0);

    library.deleteArticle({ path: 'notes' });
    const articleDeleted = await reader.read();
    expect(
      snapshotFromChunk(articleDeleted.value).navigation.documents,
    ).toEqual([]);

    abort.abort();
    await reader.cancel();
    library.close();
  });
});
