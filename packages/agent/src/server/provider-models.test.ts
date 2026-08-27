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
    const models = await discoverAgentProviderModels('codex', executable);
    assert.deepEqual(models[0]?.reasoningEfforts, [
      { id: 'low', name: '低' },
      { id: 'high', name: '高' },
    ]);
    assert.equal(models[0]?.defaultReasoningEffort, 'high');
  } finally {
    rmSync(directory, { force: true, recursive: true });
  }
});

void test('reads Pi model-specific thinking levels and current defaults', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'locus-pi-models-'));
  const executable = join(directory, 'pi');
  writeFileSync(
    executable,
    `#!/usr/bin/env node
import { createInterface } from 'node:readline';
const send = (value) => process.stdout.write(JSON.stringify(value) + '\\n');
createInterface({ input: process.stdin }).on('line', (line) => {
  const value = JSON.parse(line);
  if (value.type === 'get_available_models') send({ id: value.id, type: 'response', success: true, data: { models: [{ provider: 'test', id: 'reasoner', name: 'Reasoner', reasoning: true, thinkingLevelMap: { xhigh: 'xhigh', max: null } }] } });
  if (value.type === 'get_state') send({ id: value.id, type: 'response', success: true, data: { model: { provider: 'test', id: 'reasoner' }, thinkingLevel: 'xhigh' } });
});
`,
  );
  chmodSync(executable, 0o755);
  try {
    const models = await discoverAgentProviderModels('pi', executable);
    assert.equal(models[0]?.isDefault, true);
    assert.equal(models[0]?.defaultReasoningEffort, 'xhigh');
    assert.deepEqual(
      models[0]?.reasoningEfforts.map((effort) => effort.id),
      ['none', 'minimal', 'low', 'medium', 'high', 'xhigh'],
    );
  } finally {
    rmSync(directory, { force: true, recursive: true });
  }
});
