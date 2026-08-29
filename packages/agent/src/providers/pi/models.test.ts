import assert from 'node:assert/strict';
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { discoverModels } from './models.js';

void test('reads model-specific thinking levels and current defaults', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'locus-pi-models-'));
  const executable = join(directory, 'pi');
  writeFileSync(
    executable,
    `#!/usr/bin/env node
import { createInterface } from 'node:readline';
const send = (value) => process.stdout.write(JSON.stringify(value) + '\\n');
createInterface({ input: process.stdin }).on('line', (line) => {
  const value = JSON.parse(line);
  if (value.type === 'get_available_models') send({ id: value.id, data: { models: [{ provider: 'test', id: 'reasoner', name: 'Reasoner', reasoning: true, thinkingLevelMap: { xhigh: 'xhigh', max: null } }] } });
  if (value.type === 'get_state') send({ id: value.id, data: { model: { provider: 'test', id: 'reasoner' }, thinkingLevel: 'xhigh' } });
});
`,
  );
  chmodSync(executable, 0o755);
  try {
    const models = await discoverModels(executable);
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
