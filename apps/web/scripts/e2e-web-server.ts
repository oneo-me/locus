import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';
import {
  requireE2ELibraryPath,
  requireIsolatedE2EPort,
} from './e2e-web-server-config';

const parentPid = process.ppid;
const projectRoot = resolve(import.meta.dirname, '..');
const e2eHome = mkdtempSync(resolve(tmpdir(), 'asterweft-e2e-home-'));
const libraryPath = requireE2ELibraryPath(process.env.ASTERWEFT_LIBRARY_PATH);
const webPort = requireIsolatedE2EPort(process.argv.slice(2));
const developmentServer = spawn('pnpm', ['run', 'dev'], {
  cwd: projectRoot,
  detached: true,
  env: {
    ...process.env,
    ASTERWEFT_E2E: '1',
    ASTERWEFT_GIT_DISABLED: '1',
    ASTERWEFT_HOME: e2eHome,
    ASTERWEFT_WEB_HOST: '127.0.0.1',
    ASTERWEFT_WEB_PORT: String(webPort),
    HOST: '127.0.0.1',
    ASTERWEFT_LIBRARY_PATH: libraryPath,
    NODE_ENV: 'development',
    PORT: String(webPort),
  },
  stdio: 'inherit',
});

if (developmentServer.pid === undefined) {
  throw new Error('无法启动 E2E 开发服务。');
}

let serverExited = false;
let stopping: Promise<never> | undefined;
const serverExit = new Promise<number>((resolveExit) => {
  developmentServer.on('exit', (code) => {
    serverExited = true;
    resolveExit(code ?? 1);
  });
});

function cleanupE2EHome(): void {
  rmSync(e2eHome, { recursive: true, force: true });
}

function signalServer(signal: NodeJS.Signals): void {
  if (serverExited || developmentServer.pid === undefined) return;

  try {
    if (process.platform === 'win32') {
      developmentServer.kill(signal);
    } else {
      process.kill(-developmentServer.pid, signal);
    }
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ESRCH') throw error;
  }
}

async function stopServer(
  signal: NodeJS.Signals,
  exitCode: number,
): Promise<never> {
  if (stopping) return stopping;

  stopping = (async () => {
    clearInterval(parentWatch);
    signalServer(signal);

    const stoppedGracefully = await Promise.race([
      serverExit.then(() => true),
      sleep(3_000).then(() => false),
    ]);
    if (!stoppedGracefully) {
      signalServer('SIGKILL');
      await serverExit;
    }

    cleanupE2EHome();
    process.exit(exitCode);
  })();

  return stopping;
}

function parentIsAlive(): boolean {
  if (process.ppid !== parentPid) return false;

  try {
    process.kill(parentPid, 0);
    return true;
  } catch {
    return false;
  }
}

const parentWatch = setInterval(() => {
  if (parentIsAlive()) return;

  console.error(
    `[E2E WebServer] Parent process ${parentPid} exited; stopping development server.`,
  );
  void stopServer('SIGTERM', 0);
}, 500);
parentWatch.unref();

for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP'] as const) {
  process.on(signal, () => {
    void stopServer(signal, 0);
  });
}

process.on('exit', () => {
  signalServer('SIGKILL');
});

const exitCode = await serverExit;
clearInterval(parentWatch);
cleanupE2EHome();

if (!stopping) {
  process.exit(exitCode);
}
