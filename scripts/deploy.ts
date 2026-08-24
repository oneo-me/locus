import { execFileSync, spawn, spawnSync } from 'node:child_process';
import {
  closeSync,
  existsSync,
  openSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { resolve } from 'node:path';
import { createInterface } from 'node:readline/promises';
import {
  buildCandidate,
  buildDirectory,
  commitReplacement,
  moveCandidateIntoPlace,
  recoverInterruptedReplacement,
  rollbackReplacement,
} from './build.ts';

interface ManagedProcess {
  pid: number;
  port: number;
}

function parsePort(value: string | undefined) {
  if (value === undefined) return 3001;
  if (!/^\d+$/.test(value)) {
    throw new Error('端口必须是 1 到 65535 之间的整数。');
  }
  const port = Number(value);
  if (port < 1 || port > 65535) {
    throw new Error('端口必须是 1 到 65535 之间的整数。');
  }
  return port;
}

function readProcess(directory: string): ManagedProcess | undefined {
  const metadataPath = resolve(directory, 'server.json');
  if (!existsSync(metadataPath)) return undefined;
  try {
    const metadata = JSON.parse(readFileSync(metadataPath, 'utf8'));
    if (
      Number.isSafeInteger(metadata.pid) &&
      metadata.pid > 0 &&
      Number.isSafeInteger(metadata.port) &&
      metadata.port >= 1 &&
      metadata.port <= 65535
    ) {
      return metadata;
    }
  } catch {}
  return undefined;
}

function isRunning(pid: number) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function getProcessCommand(pid: number) {
  return String(
    execFileSync('ps', ['-p', String(pid), '-o', 'command='], {
      encoding: 'utf8',
    }),
  ).trim();
}

function isManagedProcess(pid: number, directory: string) {
  if (!isRunning(pid)) return false;
  try {
    return getProcessCommand(pid).includes(
      resolve(directory, 'dist/server.js'),
    );
  } catch {
    return false;
  }
}

function findListeningProcesses(port: number): number[] {
  const result = spawnSync(
    'lsof',
    ['-nP', '-t', `-iTCP:${port}`, '-sTCP:LISTEN'],
    { encoding: 'utf8' },
  );
  if (result.error) throw result.error;
  if (result.status !== 0 && result.status !== 1) {
    throw new Error(`无法检查端口 ${port} 的占用情况。`);
  }
  return [
    ...new Set(
      String(result.stdout)
        .split('\n')
        .map((value) => Number.parseInt(value, 10))
        .filter((pid) => Number.isSafeInteger(pid) && pid > 0),
    ),
  ];
}

async function waitForExit(pid: number, timeout: number) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (!isRunning(pid)) return true;
    await new Promise((resolveWait) => setTimeout(resolveWait, 100));
  }
  return !isRunning(pid);
}

async function terminateProcess(pid: number) {
  if (!isRunning(pid)) return;
  process.kill(pid, 'SIGTERM');
  if (!(await waitForExit(pid, 5000))) {
    process.kill(pid, 'SIGKILL');
    if (!(await waitForExit(pid, 1000))) {
      throw new Error(`无法结束进程 ${pid}。`);
    }
  }
}

async function stopManagedProcess(directory: string) {
  const metadata = readProcess(directory);
  const metadataPath = resolve(directory, 'server.json');
  if (!metadata || !isManagedProcess(metadata.pid, directory)) {
    rmSync(metadataPath, { force: true });
    return undefined;
  }
  await terminateProcess(metadata.pid);
  rmSync(metadataPath, { force: true });
  return metadata;
}

async function confirmForeignProcesses(
  port: number,
  log: (message: string) => void,
) {
  const managed = readProcess(buildDirectory);
  const foreignProcesses = findListeningProcesses(port).filter(
    (pid) =>
      !managed || pid !== managed.pid || !isManagedProcess(pid, buildDirectory),
  );
  if (foreignProcesses.length === 0) return;

  const descriptions = foreignProcesses.map((pid) => {
    let command: string = '无法读取命令';
    try {
      command = getProcessCommand(pid);
    } catch {}
    return `PID ${pid}: ${command}`;
  });
  log(`端口 ${port} 被非受管进程占用：\n${descriptions.join('\n')}`);

  const prompt = createInterface({
    input: process.stdin,
    output: process.stdout,
  });
  let answer: string;
  try {
    answer = await prompt.question('是否结束以上进程并继续部署？输入 y 确认：');
  } finally {
    prompt.close();
  }
  if (answer.trim().toLowerCase() !== 'y') {
    throw new Error('用户取消部署。');
  }

  log('用户已确认结束端口占用进程。');
  for (const pid of foreignProcesses) await terminateProcess(pid);
  if (findListeningProcesses(port).length > 0) {
    throw new Error(`端口 ${port} 仍被占用。`);
  }
}

async function waitForHealth(port: number, pid: number) {
  const deadline = Date.now() + 5000;
  while (Date.now() < deadline) {
    if (!isRunning(pid)) return false;
    try {
      const response = await fetch(`http://127.0.0.1:${port}/api/health`, {
        signal: AbortSignal.timeout(500),
      });
      if (response.status === 200 && (await response.json()).status === 'ok') {
        return true;
      }
    } catch {}
    await new Promise((resolveWait) => setTimeout(resolveWait, 100));
  }
  return false;
}

async function startBackgroundProcess(directory: string, port: number) {
  const entryPath = resolve(directory, 'dist/server.js');
  const runtimeLogPath = resolve(directory, 'runtime.log');
  const runtimeLog = openSync(runtimeLogPath, 'a');
  const child = spawn(process.execPath, [entryPath, String(port)], {
    cwd: directory,
    detached: true,
    stdio: ['ignore', runtimeLog, runtimeLog],
  });
  closeSync(runtimeLog);
  child.unref();
  const pid = child.pid;
  if (pid === undefined) {
    throw new Error('无法获取正式服务的进程 ID。');
  }
  writeFileSync(
    resolve(directory, 'server.json'),
    `${JSON.stringify({ pid, port }, null, 2)}\n`,
    'utf8',
  );

  if (!(await waitForHealth(port, pid))) {
    await stopManagedProcess(directory);
    throw new Error(`正式服务启动失败，请查看 ${runtimeLogPath}。`);
  }
  return pid;
}

recoverInterruptedReplacement();
const port = parsePort(process.argv[2]);
const candidate = await buildCandidate();
let promoted = false;
let hadPreviousBuild = false;
let previousProcess: ManagedProcess | undefined;

try {
  candidate.log(`准备部署到端口 ${port}。`);
  await confirmForeignProcesses(port, candidate.log);
  previousProcess = await stopManagedProcess(buildDirectory);

  try {
    hadPreviousBuild = moveCandidateIntoPlace(candidate.directory);
    promoted = true;
    const pid = await startBackgroundProcess(buildDirectory, port);
    candidate.log(`正式服务已在后台启动，PID ${pid}。`);
  } catch (error) {
    if (promoted) {
      rollbackReplacement(candidate.directory, hadPreviousBuild);
      promoted = false;
    }
    if (previousProcess) {
      await startBackgroundProcess(buildDirectory, previousProcess.port);
    }
    throw error;
  }

  commitReplacement();
  candidate.log('部署完成。');
  await candidate.close();
} catch (error) {
  if (!promoted) await candidate.discard(error);
  throw error;
}
