import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { accessSync, constants, lstatSync, realpathSync } from 'node:fs';
import { delimiter, join, resolve } from 'node:path';

export interface CommitLibraryChangesInput {
  message: string;
  expectedRevision: string;
  files: string[];
}

export interface CommitLibraryChangesResult {
  commit: string;
  message: string;
  changedFileCount: number;
  files: string[];
  pushed: true;
}

export type LibraryRepositoryChangeStatus =
  | 'added'
  | 'modified'
  | 'deleted'
  | 'renamed'
  | 'copied'
  | 'conflicted';

export interface LibraryRepositoryChange {
  file: string;
  previousFile?: string;
  status: LibraryRepositoryChangeStatus;
  staged: boolean;
  unstaged: boolean;
}

export interface LibraryRepositoryChanges {
  revision: string;
  changes: LibraryRepositoryChange[];
}

export interface LibraryRepositoryStatus {
  revision: string;
  clean: boolean;
  changes: {
    total: number;
    staged: number;
    unstaged: number;
    untracked: number;
    conflicted: number;
  };
}

const conventionalCommitHeader =
  /^[a-zA-Z][a-zA-Z0-9-]*(?:\([^()\r\n]+\))?!?: [^\r\n]+$/;
const englishCommitMessage = /^[\n\x20-\x7e]+$/;
const hashObjectBatchSize = 256;

function validateCommitMessage(message: string): string {
  if (
    !message ||
    message !== message.trim() ||
    message.includes('\0') ||
    message.length > 20_000 ||
    !englishCommitMessage.test(message)
  ) {
    throw new Error('提交信息不是有效的 Conventional Commit。');
  }
  const lines = message.split('\n');
  if (
    !conventionalCommitHeader.test(lines[0] ?? '') ||
    (lines.length > 1 && lines[1] !== '')
  ) {
    throw new Error('提交信息不是有效的 Conventional Commit。');
  }
  return message;
}

interface GitResult {
  stderr: string;
  status: number;
  stdout: string;
}

export class GitPushError extends Error {
  readonly commit: string;

  constructor(commit: string) {
    super('Git 提交已创建，但无法推送到配置的上游。');
    this.name = 'GitPushError';
    this.commit = commit;
  }
}

function whichExecutable(command: string): string | undefined {
  for (const directory of (process.env.PATH ?? '').split(delimiter)) {
    if (!directory) continue;
    const candidate = join(directory, command);
    try {
      accessSync(candidate, constants.X_OK);
      return candidate;
    } catch {
      // Keep searching PATH entries.
    }
  }
  return undefined;
}

function runGit(rootPath: string, args: string[], input?: string): GitResult {
  if (!whichExecutable('git')) {
    throw new Error('Git 不可用。');
  }
  try {
    const result = spawnSync(
      'git',
      ['-c', `safe.directory=${rootPath}`, '-C', rootPath, ...args],
      {
        encoding: 'buffer',
        env: {
          ...process.env,
          GIT_TERMINAL_PROMPT: '0',
          LC_ALL: 'C',
        },
        input: input === undefined ? undefined : Buffer.from(input),
        maxBuffer: 10 * 1024 * 1024,
      },
    );
    return {
      stderr: new TextDecoder().decode(result.stderr ?? Buffer.alloc(0)),
      status: result.status ?? 1,
      stdout: new TextDecoder().decode(result.stdout ?? Buffer.alloc(0)),
    };
  } catch (error) {
    const detail = error instanceof Error ? error.message.trim() : '';
    throw new Error(detail ? `Git 不可用：${detail}` : 'Git 不可用。');
  }
}

function requireSuccessfulGit(
  rootPath: string,
  args: string[],
  failureMessage: string,
  input?: string,
): GitResult {
  const result = runGit(rootPath, args, input);
  if (result.status !== 0) {
    const detail = result.stderr.trim() || result.stdout.trim();
    throw new Error(detail ? `${failureMessage} ${detail}` : failureMessage);
  }
  return result;
}

function requireRepositoryRoot(rootPath: string): void {
  const topLevel = requireSuccessfulGit(
    rootPath,
    ['rev-parse', '--show-toplevel'],
    '文档库目录不是 Git 仓库。',
  ).stdout.trim();
  let realTopLevel: string;
  try {
    realTopLevel = realpathSync(topLevel);
  } catch {
    throw new Error('文档库目录不是 Git 仓库。');
  }
  if (realTopLevel !== realpathSync(rootPath)) {
    throw new Error('文档库目录必须是 Git 仓库根目录。');
  }
}

interface RepositoryChange {
  indexStatus: string;
  worktreeStatus: string;
  path: string;
  previousPath?: string;
}

const conflictStatuses = new Set(['DD', 'AU', 'UD', 'UA', 'DU', 'AA', 'UU']);

function parseRepositoryChanges(output: string): RepositoryChange[] {
  const fields = output.split('\0');
  const changes: RepositoryChange[] = [];
  for (let index = 0; index < fields.length; index += 1) {
    const field = fields[index];
    if (!field) continue;
    const indexStatus = field[0] ?? ' ';
    const worktreeStatus = field[1] ?? ' ';
    changes.push({
      indexStatus,
      worktreeStatus,
      path: field.slice(3),
      ...((indexStatus === 'R' ||
        indexStatus === 'C' ||
        worktreeStatus === 'R' ||
        worktreeStatus === 'C') &&
      fields[index + 1]
        ? { previousPath: fields[index + 1] }
        : {}),
    });
    if (
      indexStatus === 'R' ||
      indexStatus === 'C' ||
      worktreeStatus === 'R' ||
      worktreeStatus === 'C'
    ) {
      index += 1;
    }
  }
  return changes;
}

function repositoryRevision(
  rootPath: string,
  statusOutput: string,
  changes: RepositoryChange[],
): string {
  const hash = createHash('sha256');
  hash.update(statusOutput);
  const head = runGit(rootPath, ['rev-parse', '--verify', 'HEAD']);
  hash.update(String(head.status));
  hash.update(head.stdout);
  hash.update(
    requireSuccessfulGit(
      rootPath,
      ['ls-files', '--stage', '-z'],
      '无法读取 Git 索引。',
    ).stdout,
  );

  const paths = [...new Set(changes.map((change) => change.path))].sort();
  const readablePaths = paths.filter((path) => {
    try {
      lstatSync(resolve(rootPath, path));
      return true;
    } catch {
      return false;
    }
  });
  hash.update(JSON.stringify(paths));
  for (
    let offset = 0;
    offset < readablePaths.length;
    offset += hashObjectBatchSize
  ) {
    const batch = readablePaths.slice(offset, offset + hashObjectBatchSize);
    const content = runGit(rootPath, [
      'hash-object',
      '--no-filters',
      '--',
      ...batch,
    ]);
    hash.update(JSON.stringify(batch));
    hash.update(String(content.status));
    hash.update(content.stdout);
  }
  return hash.digest('hex');
}

function repositoryChangeStatus({
  indexStatus,
  worktreeStatus,
}: RepositoryChange): LibraryRepositoryChangeStatus {
  const status = `${indexStatus}${worktreeStatus}`;
  if (conflictStatuses.has(status)) return 'conflicted';
  if (indexStatus === 'R' || worktreeStatus === 'R') return 'renamed';
  if (indexStatus === 'C' || worktreeStatus === 'C') return 'copied';
  if (indexStatus === 'A' || worktreeStatus === 'A' || status === '??') {
    return 'added';
  }
  if (indexStatus === 'D' || worktreeStatus === 'D') return 'deleted';
  return 'modified';
}

function toPublicRepositoryChange(
  change: RepositoryChange,
): LibraryRepositoryChange {
  const untracked = change.indexStatus === '?' && change.worktreeStatus === '?';
  return {
    file: change.path,
    ...(change.previousPath ? { previousFile: change.previousPath } : {}),
    status: repositoryChangeStatus(change),
    staged: !untracked && change.indexStatus !== ' ',
    unstaged: untracked || change.worktreeStatus !== ' ',
  };
}

interface PushTarget {
  remote: string;
  remoteRef: string;
}

function resolvePushTarget(rootPath: string): PushTarget {
  const branch = runGit(rootPath, [
    'symbolic-ref',
    '--quiet',
    '--short',
    'HEAD',
  ]);
  if (branch.status !== 0 || !branch.stdout.trim()) {
    throw new Error('Git 当前分支没有配置可推送的上游。');
  }
  const branchName = branch.stdout.trim();
  const remote = runGit(rootPath, [
    'config',
    '--get',
    `branch.${branchName}.remote`,
  ]);
  const remoteRef = runGit(rootPath, [
    'config',
    '--get',
    `branch.${branchName}.merge`,
  ]);
  if (
    remote.status !== 0 ||
    remoteRef.status !== 0 ||
    !remote.stdout.trim() ||
    !remoteRef.stdout.trim()
  ) {
    throw new Error('Git 当前分支没有配置可推送的上游。');
  }
  return {
    remote: remote.stdout.trim(),
    remoteRef: remoteRef.stdout.trim(),
  };
}

export function getLibraryRepositoryStatus(
  rootPath: string,
): LibraryRepositoryStatus {
  const repositoryChanges = getLibraryRepositoryChanges(rootPath);
  const records = repositoryChanges.changes;
  let staged = 0;
  let unstaged = 0;
  let untracked = 0;
  let conflicted = 0;
  for (const record of records) {
    if (record.status === 'added' && !record.staged) untracked += 1;
    if (record.status === 'conflicted') {
      conflicted += 1;
      continue;
    }
    if (record.staged) staged += 1;
    if (record.unstaged && !(record.status === 'added' && !record.staged)) {
      unstaged += 1;
    }
  }
  return {
    revision: repositoryChanges.revision,
    clean: records.length === 0,
    changes: {
      total: records.length,
      staged,
      unstaged,
      untracked,
      conflicted,
    },
  };
}

export function getLibraryRepositoryChanges(
  rootPath: string,
): LibraryRepositoryChanges {
  requireRepositoryRoot(rootPath);
  const statusOutput = requireSuccessfulGit(
    rootPath,
    ['status', '--porcelain=v1', '-z', '--untracked-files=all'],
    '无法读取 Git 仓库状态。',
  ).stdout;
  const records = parseRepositoryChanges(statusOutput);
  return {
    revision: repositoryRevision(rootPath, statusOutput, records),
    changes: records.map(toPublicRepositoryChange),
  };
}

function selectRepositoryChanges(
  files: string[],
  changes: LibraryRepositoryChange[],
): LibraryRepositoryChange[] {
  if (!Array.isArray(files) || files.length === 0) {
    throw new Error('至少需要选择一个要提交的文件。');
  }
  const available = new Map(changes.map((change) => [change.file, change]));
  const selected: LibraryRepositoryChange[] = [];
  const seen = new Set<string>();
  for (const file of files) {
    if (
      typeof file !== 'string' ||
      !file ||
      file !== file.trim() ||
      file.includes('\0')
    ) {
      throw new Error('提交文件必须是有效的仓库相对路径。');
    }
    if (seen.has(file)) throw new Error('提交文件不能重复。');
    const change = available.get(file);
    if (!change) throw new Error('提交文件不在当前 Git 变更中。');
    seen.add(file);
    selected.push(change);
  }
  return selected;
}

export function commitLibraryChanges(
  rootPath: string,
  input: CommitLibraryChangesInput,
): CommitLibraryChangesResult {
  const message = validateCommitMessage(input.message);
  const repositoryChanges = getLibraryRepositoryChanges(rootPath);
  if (repositoryChanges.revision !== input.expectedRevision) {
    throw new Error('Git 仓库已发生变化，请重新读取状态后再提交。');
  }
  if (
    repositoryChanges.changes.some((change) => change.status === 'conflicted')
  ) {
    throw new Error('Git 仓库存在未解决的冲突。');
  }
  if (repositoryChanges.changes.length === 0) {
    throw new Error('Git 仓库没有可提交的更新。');
  }
  const selected = selectRepositoryChanges(
    input.files,
    repositoryChanges.changes,
  );
  const pushTarget = resolvePushTarget(rootPath);
  const stagingPathspecs = selected.map((change) => change.file);
  const commitPathspecs = [
    ...new Set(
      selected.flatMap((change) => [
        change.file,
        ...(change.previousFile ? [change.previousFile] : []),
      ]),
    ),
  ];

  requireSuccessfulGit(
    rootPath,
    ['--literal-pathspecs', 'add', '--all', '--', ...stagingPathspecs],
    '无法暂存文档库更新。',
  );
  const staged = runGit(rootPath, [
    '--literal-pathspecs',
    'diff',
    '--cached',
    '--quiet',
    '--exit-code',
    '--',
    ...commitPathspecs,
  ]);
  if (staged.status === 0) throw new Error('Git 仓库没有可提交的更新。');
  if (staged.status !== 1) throw new Error('无法检查已暂存的文档库更新。');

  requireSuccessfulGit(
    rootPath,
    [
      '--literal-pathspecs',
      'commit',
      '--only',
      '--cleanup=verbatim',
      '--file',
      '-',
      '--',
      ...commitPathspecs,
    ],
    '无法创建 Git 提交。',
    `${message}\n`,
  );
  const commit = requireSuccessfulGit(
    rootPath,
    ['rev-parse', 'HEAD'],
    '无法读取新建的 Git 提交。',
  ).stdout.trim();
  const pushed = runGit(rootPath, [
    'push',
    '--porcelain',
    pushTarget.remote,
    `HEAD:${pushTarget.remoteRef}`,
  ]);
  if (pushed.status !== 0) throw new GitPushError(commit);

  return {
    commit,
    message,
    changedFileCount: selected.length,
    files: selected.map((change) => change.file),
    pushed: true,
  };
}
