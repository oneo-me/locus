import assert from 'node:assert/strict';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import test from 'node:test';
import { readOpenCodeSessionTitle } from './opencode-title.js';

void test('reads a generated OpenCode title from its native session store', () => {
  const directory = mkdtempSync(join(tmpdir(), 'locus-opencode-title-'));
  const database = new DatabaseSync(join(directory, 'opencode.db'));
  database.exec(
    'CREATE TABLE session (id TEXT PRIMARY KEY, title TEXT NOT NULL)',
  );
  database
    .prepare('INSERT INTO session (id, title) VALUES (?, ?)')
    .run('session-1', '  Refactor task titles  ');
  database.close();

  assert.equal(
    readOpenCodeSessionTitle('session-1', directory),
    'Refactor task titles',
  );
});

void test('ignores missing and placeholder OpenCode titles', () => {
  const directory = mkdtempSync(join(tmpdir(), 'locus-opencode-title-'));
  const database = new DatabaseSync(join(directory, 'opencode.db'));
  database.exec(
    'CREATE TABLE session (id TEXT PRIMARY KEY, title TEXT NOT NULL)',
  );
  database
    .prepare('INSERT INTO session (id, title) VALUES (?, ?)')
    .run('session-1', 'New session - 2026-08-26T18:33:35.122Z');
  database.close();

  assert.equal(readOpenCodeSessionTitle('session-1', directory), null);
  assert.equal(readOpenCodeSessionTitle('missing', directory), null);
});
