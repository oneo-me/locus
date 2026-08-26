import assert from 'node:assert/strict';
import test from 'node:test';
import { NativeTitleRefresh } from './native-title-refresh.js';

void test('walks past misses and resolves the first native title', async () => {
  const refresh = new NativeTitleRefresh();
  let attempts = 0;
  const title = await new Promise<string>((resolve) => {
    refresh.start(
      [0, 0, 0, 0],
      () => {
        attempts += 1;
        if (attempts === 1) throw new Error('not written yet');
        if (attempts === 2) return null;
        if (attempts === 3) return '   ';
        return '  Native title  ';
      },
      resolve,
    );
  });

  assert.equal(title, 'Native title');
  assert.equal(attempts, 4);
  refresh.start(
    [0],
    () => 'Later title',
    () => assert.fail(),
  );
  assert.equal(attempts, 4);
});

void test('can restart after a schedule runs dry', async () => {
  const refresh = new NativeTitleRefresh();
  let attempts = 0;
  refresh.start(
    [0, 0],
    () => {
      attempts += 1;
      return null;
    },
    assert.fail,
  );
  await Promise.resolve();

  const title = await new Promise<string>((resolve) => {
    refresh.start([0], () => 'Found later', resolve);
  });
  assert.equal(title, 'Found later');
  assert.equal(attempts, 2);
});

void test('can be explicitly rearmed for a provider that streams retitles', async () => {
  const refresh = new NativeTitleRefresh();
  const first = await new Promise<string>((resolve) => {
    refresh.start(
      [0],
      () => 'First title',
      (title) => {
        resolve(title);
        refresh.close();
      },
    );
  });
  const second = await new Promise<string>((resolve) => {
    refresh.start([0], () => 'Second title', resolve);
  });

  assert.equal(first, 'First title');
  assert.equal(second, 'Second title');
});
