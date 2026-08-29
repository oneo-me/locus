import { execFile } from 'node:child_process';
import { stat, realpath } from 'node:fs/promises';
import { homedir } from 'node:os';
import { isAbsolute, resolve } from 'node:path';
import { promisify } from 'node:util';
import { logInfo } from '~/logging';

const execFileAsync = promisify(execFile);

export interface OpenLocalDirectoryInput {
  path: string;
}

export async function openLocalPath(
  path: string,
  target: 'directory' | 'vscode',
): Promise<void> {
  if (process.platform === 'darwin') {
    await execFileAsync(
      'open',
      target === 'vscode' ? ['-a', 'Visual Studio Code', path] : [path],
    );
    return;
  }
  if (process.platform === 'win32') {
    await execFileAsync(target === 'vscode' ? 'code.cmd' : 'explorer.exe', [
      path,
    ]);
    return;
  }
  await execFileAsync(target === 'vscode' ? 'code' : 'xdg-open', [path]);
}

export async function openLocalDirectory(input: OpenLocalDirectoryInput) {
  if (!input || typeof input.path !== 'string' || !input.path.trim()) {
    throw new Error('path 必须是非空字符串。');
  }

  const requestedPath = input.path.trim();
  const expandedPath =
    requestedPath === '~'
      ? homedir()
      : /^~[/\\]/.test(requestedPath)
        ? resolve(homedir(), requestedPath.slice(2))
        : requestedPath;
  if (!isAbsolute(expandedPath)) {
    throw new Error('path 必须是绝对路径或以 ~/ 开头。');
  }

  const directory = await realpath(expandedPath).catch(() => null);
  const directoryStats = directory
    ? await stat(directory).catch(() => null)
    : null;
  if (!directory || !directoryStats) {
    throw new Error('目录不存在或无法访问。');
  }
  if (!directoryStats.isDirectory()) {
    throw new Error('path 不是目录。');
  }

  await openLocalPath(directory, 'directory');
  logInfo('system.directory.opened');
  return { ok: true as const };
}
