import assert from 'node:assert/strict';
import { test } from 'node:test';
import { providerPromptContent } from './prompt-content.js';

void test('quotes a leading absolute path without changing slash commands', () => {
  assert.equal(
    providerPromptContent('/Users/oneo/image.jpeg 调查这个项目', []),
    '`/Users/oneo/image.jpeg` 调查这个项目',
  );
  assert.equal(
    providerPromptContent('/review 检查改动', [{ name: 'review' }]),
    '/review 检查改动',
  );
  assert.equal(
    providerPromptContent('查看 /Users/oneo/image.jpeg', []),
    '查看 /Users/oneo/image.jpeg',
  );
});
