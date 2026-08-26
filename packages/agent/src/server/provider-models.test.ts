import assert from 'node:assert/strict';
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { discoverAgentProviderModels } from './provider-models.js';

void test('keeps the last successful provider model catalog', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'locus-model-cache-'));
  const executable = join(directory, 'cursor-agent');
  const cache = join(directory, 'model-cache');
  writeFileSync(executable, '#!/bin/sh\nprintf "model-a\\n"\n');
  chmodSync(executable, 0o755);
  try {
    const discovered = await discoverAgentProviderModels(
      'cursor',
      executable,
      cache,
    );
    assert.deepEqual(
      discovered.map((model) => model.id),
      ['model-a'],
    );

    writeFileSync(executable, '#!/bin/sh\nexit 1\n');
    const cached = await discoverAgentProviderModels(
      'cursor',
      executable,
      cache,
    );
    assert.deepEqual(cached, discovered);
  } finally {
    rmSync(directory, { force: true, recursive: true });
  }
});
