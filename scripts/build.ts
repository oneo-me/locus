import { spawn } from 'node:child_process';
import { once } from 'node:events';
import {
  cpSync,
  createWriteStream,
  existsSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { homedir, platform } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { stripVTControlCharacters } from 'node:util';
import type { WriteStream } from 'node:fs';

export const repositoryDirectory = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '..',
);
export const buildDirectory = resolve(repositoryDirectory, '.build');
export const previousBuildDirectory = resolve(
  repositoryDirectory,
  '.build.previous',
);

const serverDistDirectory = resolve(repositoryDirectory, 'apps/server/dist');
const webBuildDirectory = resolve(repositoryDirectory, 'apps/web/build');

export interface CandidateBuild {
  directory: string;
  log: (message: string) => void;
  close: () => Promise<void>;
  discard: (error?: unknown) => Promise<void>;
}

function formatError(error: unknown) {
  if (error instanceof Error) return error.message;
  if (
    typeof error === 'string' ||
    typeof error === 'number' ||
    typeof error === 'bigint' ||
    typeof error === 'boolean'
  ) {
    return String(error);
  }
  return JSON.stringify(error) ?? '未知错误';
}

function resolveStoreDirectory() {
  const storeVersion = 'v11';
  if (platform() === 'darwin') {
    return join(homedir(), 'Library/pnpm/store', storeVersion);
  }
  if (platform() === 'win32') {
    return join(
      process.env.LOCALAPPDATA ?? join(homedir(), 'AppData/Local'),
      'pnpm/store',
      storeVersion,
    );
  }
  return join(
    process.env.XDG_DATA_HOME ?? join(homedir(), '.local/share'),
    'pnpm/store',
    storeVersion,
  );
}

function writeLog(stream: WriteStream, message: string) {
  const line = `[${new Date().toISOString()}] ${message}\n`;
  process.stdout.write(line);
  stream.write(line);
}

function pipeOutput(
  source: NodeJS.ReadableStream,
  destination: NodeJS.WriteStream,
  log: WriteStream,
) {
  source.on('data', (chunk) => {
    destination.write(chunk);
    log.write(stripVTControlCharacters(chunk.toString()));
  });
}

async function runPnpm(
  args: string[],
  buildLog: WriteStream,
  forceBuiltInCommand = false,
) {
  const child = spawn(
    'pnpm',
    [
      ...(forceBuiltInCommand ? ['pm'] : []),
      `--config.store-dir=${resolveStoreDirectory()}`,
      ...args,
    ],
    {
      cwd: repositoryDirectory,
      stdio: ['inherit', 'pipe', 'pipe'],
    },
  );
  pipeOutput(child.stdout, process.stdout, buildLog);
  pipeOutput(child.stderr, process.stderr, buildLog);
  const [code, signal] = await once(child, 'exit');
  if (code !== 0) {
    throw new Error(
      `pnpm ${args.join(' ')} 执行失败${signal ? `（${signal}）` : ''}。`,
    );
  }
}

function prepareManifest(directory: string) {
  const packagePath = resolve(directory, 'package.json');
  const manifest = JSON.parse(readFileSync(packagePath, 'utf8'));
  manifest.scripts = { start: 'node dist/server.js' };
  delete manifest.devDependencies;
  delete manifest.files;
  delete manifest.optionalDependencies;
  writeFileSync(packagePath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
  rmSync(resolve(directory, 'pnpm-workspace.yaml'));
}

async function closeServer(server: import('node:http').Server) {
  if (!server.listening) return;
  await new Promise<void>((resolveClose, reject) => {
    server.close((error) => (error ? reject(error) : resolveClose()));
  });
}

async function verifyCandidate(directory: string) {
  const entryPath = resolve(directory, 'dist/server.js');
  const moduleUrl = `${pathToFileURL(entryPath).href}?verify=${Date.now()}`;
  const { startServer } = await import(moduleUrl);
  const server = startServer(0);

  try {
    if (!server.listening) await once(server, 'listening');
    const address = server.address();
    if (!address || typeof address === 'string') {
      throw new Error('候选服务没有可用的 TCP 地址。');
    }

    const baseUrl = `http://127.0.0.1:${address.port}`;
    const request = (path: string) =>
      fetch(`${baseUrl}${path}`, { signal: AbortSignal.timeout(5000) });
    const health = await request('/api/health');
    if (health.status !== 200 || (await health.json()).status !== 'ok') {
      throw new Error('候选服务健康检查失败。');
    }

    const index = await request('/');
    const indexHtml = await index.text();
    if (
      index.status !== 200 ||
      !index.headers.get('content-type')?.startsWith('text/html') ||
      index.headers.get('cache-control') !== 'no-cache'
    ) {
      throw new Error('候选服务无法正确提供 SPA 首页。');
    }

    const spaRoute = await request('/workspace/deploy-verification');
    if (
      spaRoute.status !== 200 ||
      !spaRoute.headers.get('content-type')?.startsWith('text/html') ||
      spaRoute.headers.get('cache-control') !== 'no-cache'
    ) {
      throw new Error('候选服务的 SPA fallback 不可用。');
    }

    for (const path of ['/api', '/api/missing', '/mcp/missing']) {
      const response = await request(path);
      if (
        response.status !== 404 ||
        !response.headers.get('content-type')?.startsWith('application/json')
      ) {
        throw new Error(`候选服务错误处理了服务端路径 ${path}。`);
      }
    }

    const assetPath = indexHtml.match(/\/assets\/[^"']+\.(?:css|js)/)?.[0];
    if (!assetPath) throw new Error('候选 SPA 没有可验证的静态资源。');
    const asset = await request(assetPath);
    if (
      asset.status !== 200 ||
      asset.headers.get('cache-control') !==
        'public, max-age=31536000, immutable'
    ) {
      throw new Error('候选服务的静态资源缓存策略无效。');
    }
  } finally {
    await closeServer(server);
  }
}

function closeLog(stream: WriteStream) {
  return new Promise<void>((resolveClose, reject) => {
    stream.once('error', reject);
    stream.end(resolveClose);
  });
}

export function recoverInterruptedReplacement() {
  if (!existsSync(buildDirectory) && existsSync(previousBuildDirectory)) {
    renameSync(previousBuildDirectory, buildDirectory);
  }
}

export async function buildCandidate(): Promise<CandidateBuild> {
  const directory = mkdtempSync(`${buildDirectory}-`);
  rmSync(directory, { recursive: true });
  const temporaryLogPath = `${directory}.log`;
  let buildLog = createWriteStream(temporaryLogPath, {
    flags: 'a',
  });
  const log = (message: string) => writeLog(buildLog, message);
  let closed = false;
  const close = async () => {
    if (closed) return;
    closed = true;
    await closeLog(buildLog);
  };
  const discard = async (error?: unknown) => {
    if (error !== undefined) {
      log(`构建失败：${formatError(error)}`);
    }
    await close();
    rmSync(directory, { recursive: true, force: true });
    rmSync(temporaryLogPath, { force: true });
  };

  try {
    log('开始构建。');
    rmSync(serverDistDirectory, { recursive: true, force: true });
    rmSync(webBuildDirectory, { recursive: true, force: true });
    await runPnpm(['--filter', '@locus/web', 'build'], buildLog);
    await runPnpm(['--filter', '@locus/server', 'build'], buildLog);
    cpSync(
      resolve(webBuildDirectory, 'client'),
      resolve(serverDistDirectory, 'client'),
      { recursive: true },
    );
    await runPnpm(
      ['--filter', '@locus/server', 'deploy', directory, '--prod'],
      buildLog,
      true,
    );
    await closeLog(buildLog);
    renameSync(temporaryLogPath, resolve(directory, 'build.log'));
    buildLog = createWriteStream(resolve(directory, 'build.log'), {
      flags: 'a',
    });
    prepareManifest(directory);
    await verifyCandidate(directory);
    log('候选服务验证通过。');
    return { directory, log, close, discard };
  } catch (error) {
    await discard(error);
    throw error;
  } finally {
    rmSync(serverDistDirectory, { recursive: true, force: true });
    rmSync(webBuildDirectory, { recursive: true, force: true });
  }
}

export function moveCandidateIntoPlace(candidateDirectory: string) {
  rmSync(previousBuildDirectory, { recursive: true, force: true });
  const hadPreviousBuild = existsSync(buildDirectory);
  if (hadPreviousBuild) renameSync(buildDirectory, previousBuildDirectory);
  try {
    renameSync(candidateDirectory, buildDirectory);
  } catch (error) {
    if (hadPreviousBuild && !existsSync(buildDirectory)) {
      renameSync(previousBuildDirectory, buildDirectory);
    }
    throw error;
  }
  return hadPreviousBuild;
}

export function rollbackReplacement(
  candidateDirectory: string,
  hadPreviousBuild: boolean,
) {
  if (existsSync(buildDirectory))
    renameSync(buildDirectory, candidateDirectory);
  if (hadPreviousBuild && existsSync(previousBuildDirectory)) {
    renameSync(previousBuildDirectory, buildDirectory);
  }
}

export function commitReplacement() {
  rmSync(previousBuildDirectory, { recursive: true, force: true });
}

async function build() {
  recoverInterruptedReplacement();
  const candidate = await buildCandidate();
  try {
    moveCandidateIntoPlace(candidate.directory);
    commitReplacement();
    candidate.log('构建完成。');
    await candidate.close();
  } catch (error) {
    await candidate.discard(error);
    throw error;
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  await build();
}
