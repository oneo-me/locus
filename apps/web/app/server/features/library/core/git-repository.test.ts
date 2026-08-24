import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from '~/server/test/node-process';
import { afterEach, describe, expect, test } from 'vitest';
import {
  commitLibraryChanges,
  GitPushError,
  getLibraryRepositoryChanges,
  getLibraryRepositoryStatus,
} from './git-repository.js';

const roots: string[] = [];

function temporaryRepository(): string {
  const root = mkdtempSync(join(tmpdir(), 'asterweft-git-'));
  const remote = mkdtempSync(join(tmpdir(), 'asterweft-git-remote-'));
  roots.push(root, remote);
  execFileSync('git', ['init', '--bare', '--quiet'], { cwd: remote });
  execFileSync('git', ['init', '--quiet'], { cwd: root });
  execFileSync('git', ['config', 'user.name', 'Asterweft Test'], { cwd: root });
  execFileSync('git', ['config', 'user.email', 'test@asterweft.local'], {
    cwd: root,
  });
  writeFileSync(join(root, 'index.md'), '---\ntitle: Notes\n---\n');
  execFileSync('git', ['add', '--all'], { cwd: root });
  execFileSync('git', ['commit', '--quiet', '-m', 'docs: initialize notes'], {
    cwd: root,
  });
  execFileSync('git', ['remote', 'add', 'origin', remote], { cwd: root });
  execFileSync('git', ['push', '--quiet', '--set-upstream', 'origin', 'HEAD'], {
    cwd: root,
  });
  return root;
}

afterEach(() => {
  for (const root of roots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

describe('Git document library commits', () => {
  test('updates the revision across a large set of working tree changes', () => {
    const root = temporaryRepository();
    const bulk = join(root, 'bulk');
    mkdirSync(bulk);
    for (let index = 0; index < 300; index += 1) {
      writeFileSync(
        join(bulk, `${String(index).padStart(3, '0')}.md`),
        `draft-${String(index).padStart(3, '0')}`,
      );
    }

    const first = getLibraryRepositoryChanges(root);
    writeFileSync(join(bulk, '157.md'), 'final-157');
    const second = getLibraryRepositoryChanges(root);

    expect(first.changes).toHaveLength(300);
    expect(second.changes).toHaveLength(300);
    expect(second.revision).not.toBe(first.revision);
  });

  test('lists concrete changes and commits only selected files', () => {
    const root = temporaryRepository();
    writeFileSync(join(root, 'index.md'), '---\ntitle: Notes\n---\nUpdated\n');
    execFileSync('git', ['add', 'index.md'], { cwd: root });
    mkdirSync(join(root, 'guide'));
    writeFileSync(join(root, 'guide', 'index.md'), '---\ntitle: Guide\n---\n');
    const status = getLibraryRepositoryStatus(root);
    expect(status).toMatchObject({
      clean: false,
      changes: {
        total: 2,
        staged: 1,
        unstaged: 0,
        untracked: 1,
        conflicted: 0,
      },
    });
    expect(status.revision).toHaveLength(64);
    expect(JSON.stringify(status)).not.toContain('index.md');
    expect(getLibraryRepositoryChanges(root)).toEqual({
      revision: status.revision,
      changes: [
        {
          file: 'index.md',
          status: 'modified',
          staged: true,
          unstaged: false,
        },
        {
          file: 'guide/index.md',
          status: 'added',
          staged: false,
          unstaged: true,
        },
      ],
    });

    const result = commitLibraryChanges(root, {
      expectedRevision: status.revision,
      message: 'docs(notes): update guide',
      files: ['guide/index.md'],
    });

    expect(result.commit).toMatch(/^[a-f0-9]{40,64}$/);
    expect(result.message).toBe('docs(notes): update guide');
    expect(result.changedFileCount).toBe(1);
    expect(result.files).toEqual(['guide/index.md']);
    expect(result.pushed).toBe(true);
    expect(
      execFileSync('git', ['log', '-1', '--format=%s'], {
        cwd: root,
        encoding: 'utf8',
      }).trim(),
    ).toBe('docs(notes): update guide');
    expect(
      execFileSync('git', ['status', '--porcelain'], {
        cwd: root,
        encoding: 'utf8',
      }),
    ).toBe('M  index.md\n');
    expect(
      execFileSync('git', ['show', '--format=', '--name-only', 'HEAD'], {
        cwd: root,
        encoding: 'utf8',
      }).trim(),
    ).toBe('guide/index.md');
    expect(
      execFileSync('git', ['rev-parse', '@{upstream}'], {
        cwd: root,
        encoding: 'utf8',
      }).trim(),
    ).toBe(result.commit);
  });

  test('rejects invalid messages, clean repositories, and nested roots', () => {
    const root = temporaryRepository();
    writeFileSync(join(root, 'index.md'), 'Changed\n');
    const initialStatus = getLibraryRepositoryStatus(root);
    expect(() =>
      commitLibraryChanges(root, {
        expectedRevision: initialStatus.revision,
        message: 'update notes',
        files: ['index.md'],
      }),
    ).toThrow('Conventional Commit');
    expect(() =>
      commitLibraryChanges(root, {
        expectedRevision: initialStatus.revision,
        message: 'docs: 更新笔记',
        files: ['index.md'],
      }),
    ).toThrow('Conventional Commit');
    expect(() =>
      commitLibraryChanges(root, {
        expectedRevision: initialStatus.revision,
        message: 'docs: update notes\n\n更新正文',
        files: ['index.md'],
      }),
    ).toThrow('Conventional Commit');
    expect(() =>
      commitLibraryChanges(root, {
        expectedRevision: initialStatus.revision,
        message: 'docs: update notes',
        files: ['not-changed.md'],
      }),
    ).toThrow('不在当前 Git 变更中');

    writeFileSync(join(root, 'index.md'), 'Changed again\n');
    expect(() =>
      commitLibraryChanges(root, {
        expectedRevision: initialStatus.revision,
        message: 'docs: update notes',
        files: ['index.md'],
      }),
    ).toThrow('重新读取状态');

    const latestStatus = getLibraryRepositoryStatus(root);
    commitLibraryChanges(root, {
      expectedRevision: latestStatus.revision,
      message: 'docs: update notes',
      files: ['index.md'],
    });
    const cleanStatus = getLibraryRepositoryStatus(root);
    expect(cleanStatus.clean).toBe(true);
    expect(() =>
      commitLibraryChanges(root, {
        expectedRevision: cleanStatus.revision,
        message: 'docs: update notes again',
        files: ['index.md'],
      }),
    ).toThrow('没有可提交');

    const nested = join(root, 'nested');
    mkdirSync(nested);
    expect(() => getLibraryRepositoryStatus(nested)).toThrow('仓库根目录');
  });

  test('reports rename pairs and commits a selected rename as one change', () => {
    const root = temporaryRepository();
    execFileSync('git', ['mv', 'index.md', 'notes.md'], { cwd: root });
    const changes = getLibraryRepositoryChanges(root);

    expect(changes.changes).toEqual([
      {
        file: 'notes.md',
        previousFile: 'index.md',
        status: 'renamed',
        staged: true,
        unstaged: false,
      },
    ]);

    const result = commitLibraryChanges(root, {
      expectedRevision: changes.revision,
      message: 'docs: rename notes',
      files: ['notes.md'],
    });

    expect(result.files).toEqual(['notes.md']);
    expect(result.changedFileCount).toBe(1);
    expect(getLibraryRepositoryStatus(root).clean).toBe(true);
  });

  test('requires an upstream before creating a commit', () => {
    const root = mkdtempSync(join(tmpdir(), 'asterweft-git-no-upstream-'));
    roots.push(root);
    execFileSync('git', ['init', '--quiet'], { cwd: root });
    execFileSync('git', ['config', 'user.name', 'Asterweft Test'], {
      cwd: root,
    });
    execFileSync('git', ['config', 'user.email', 'test@asterweft.local'], {
      cwd: root,
    });
    writeFileSync(join(root, 'index.md'), '---\ntitle: Notes\n---\n');
    execFileSync('git', ['add', '--all'], { cwd: root });
    execFileSync('git', ['commit', '--quiet', '-m', 'docs: initialize notes'], {
      cwd: root,
    });
    const before = execFileSync('git', ['rev-parse', 'HEAD'], {
      cwd: root,
      encoding: 'utf8',
    }).trim();
    writeFileSync(join(root, 'index.md'), 'Changed\n');
    const status = getLibraryRepositoryStatus(root);

    expect(() =>
      commitLibraryChanges(root, {
        expectedRevision: status.revision,
        message: 'docs: update notes',
        files: ['index.md'],
      }),
    ).toThrow('上游');
    expect(
      execFileSync('git', ['rev-parse', 'HEAD'], {
        cwd: root,
        encoding: 'utf8',
      }).trim(),
    ).toBe(before);
  });

  test('preserves the local commit identifier when the upstream rejects a push', () => {
    const root = temporaryRepository();
    const remote = execFileSync(
      'git',
      ['config', '--get', 'remote.origin.url'],
      { cwd: root, encoding: 'utf8' },
    ).trim();
    const hook = join(remote, 'hooks', 'pre-receive');
    writeFileSync(hook, '#!/bin/sh\nexit 1\n');
    chmodSync(hook, 0o755);
    writeFileSync(join(root, 'index.md'), 'Rejected upstream update\n');
    const status = getLibraryRepositoryStatus(root);

    let error: unknown;
    try {
      commitLibraryChanges(root, {
        expectedRevision: status.revision,
        message: 'docs: update rejected notes',
        files: ['index.md'],
      });
    } catch (cause) {
      error = cause;
    }

    expect(error).toBeInstanceOf(GitPushError);
    const commit = (error as GitPushError).commit;
    expect(commit).toMatch(/^[a-f0-9]{40,64}$/);
    expect(
      execFileSync('git', ['rev-parse', 'HEAD'], {
        cwd: root,
        encoding: 'utf8',
      }).trim(),
    ).toBe(commit);
    expect(
      execFileSync('git', ['rev-parse', '@{upstream}'], {
        cwd: root,
        encoding: 'utf8',
      }).trim(),
    ).not.toBe(commit);
  });
});
