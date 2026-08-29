import assert from 'node:assert/strict';
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { discoverModels } from './models.js';

void test('reads Codex reasoning efforts from model/list', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'locus-codex-models-'));
  const executable = join(directory, 'codex');
  writeFileSync(
    executable,
    `#!/usr/bin/env node
import { createInterface } from 'node:readline';
const send = (value) => process.stdout.write(JSON.stringify(value) + '\\n');
createInterface({ input: process.stdin }).on('line', (line) => {
  const value = JSON.parse(line);
  if (value.id === 0) send({ id: 0, result: {} });
  if (value.method === 'model/list') send({ id: value.id, result: { data: [{ model: 'gpt-test', displayName: 'GPT Test', isDefault: true, defaultReasoningEffort: 'high', supportedReasoningEfforts: [{ reasoningEffort: 'low' }, { reasoningEffort: 'high' }] }] } });
});
`,
  );
  chmodSync(executable, 0o755);
  try {
    const models = await discoverModels(executable);
    assert.deepEqual(models[0]?.reasoningEfforts, [
      { id: 'low', name: '低' },
      { id: 'high', name: '高' },
    ]);
    assert.equal(models[0]?.defaultReasoningEffort, 'high');
  } finally {
    rmSync(directory, { force: true, recursive: true });
  }
});
