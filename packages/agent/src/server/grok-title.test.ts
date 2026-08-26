import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { readGrokSessionTitle } from './grok-title.js';

void test('reads Grok generated_title from its native session summary', () => {
  const home = mkdtempSync(join(tmpdir(), 'locus-grok-title-'));
  const sessionId = '01a03e26-8bfe-7ef0-990a-b506e60d2781';
  const sessionDirectory = join(home, 'sessions', 'workspace', sessionId);
  mkdirSync(sessionDirectory, { recursive: true });
  writeFileSync(
    join(sessionDirectory, 'summary.json'),
    JSON.stringify({ generated_title: '  Investigate title sources  ' }),
  );

  assert.equal(
    readGrokSessionTitle(sessionId, home),
    'Investigate title sources',
  );
});

void test('ignores invalid ids and missing Grok titles', () => {
  const home = mkdtempSync(join(tmpdir(), 'locus-grok-title-'));
  assert.equal(readGrokSessionTitle('../session', home), null);
  assert.equal(
    readGrokSessionTitle('01a03e26-8bfe-7ef0-990a-b506e60d2781', home),
    null,
  );
});
