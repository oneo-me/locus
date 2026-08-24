import { spawn } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  readdirSync,
  renameSync,
  rmSync,
  statSync,
} from 'node:fs';
import { basename, join, resolve } from 'node:path';

export const projectRoot = resolve(import.meta.dirname, '..');
const maximumWebEntryBytes = 500 * 1024;
const defaultBuildDirectory = join(projectRoot, 'build');

function assertBuildDirectory(outputDirectory: string): void {
  const resolved = resolve(outputDirectory);
  const relative = resolved.slice(projectRoot.length + 1);
  if (
    !resolved.startsWith(`${projectRoot}/`) ||
    !(
      basename(resolved) === 'build' || basename(resolved).startsWith('.build')
    ) ||
    relative.includes('/../')
  ) {
    throw new Error(`拒绝写入非项目构建目录：${resolved}`);
  }
}

async function runReactRouterBuild(): Promise<string> {
  if (existsSync(defaultBuildDirectory)) {
    rmSync(defaultBuildDirectory, { recursive: true, force: true });
  }

  const child = spawn('pnpm', ['exec', 'react-router', 'build'], {
    cwd: projectRoot,
    env: {
      ...process.env,
      NODE_ENV: 'production',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  async function readStream(
    stream: NodeJS.ReadableStream | null,
  ): Promise<string> {
    if (!stream) return '';
    const chunks: Buffer[] = [];
    for await (const chunk of stream) {
      chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
    }
    return Buffer.concat(chunks).toString('utf8');
  }

  const [stdout, stderr, exitCode] = await Promise.all([
    readStream(child.stdout),
    readStream(child.stderr),
    new Promise<number>((resolveExit) => {
      child.on('close', (code) => resolveExit(code ?? 1));
    }),
  ]);
  const output = [stdout.trimEnd(), stderr.trimEnd()]
    .filter(Boolean)
    .join('\n');
  if (exitCode !== 0) {
    throw new Error(
      [output, `React Router 构建失败（退出码 ${exitCode}）。`]
        .filter(Boolean)
        .join('\n'),
    );
  }
  if (
    !existsSync(join(defaultBuildDirectory, 'client')) ||
    !existsSync(join(defaultBuildDirectory, 'server'))
  ) {
    throw new Error('React Router 构建完成，但缺少 client 或 server 产物。');
  }
  return output;
}

function assertEntryBudget(clientDirectory: string): void {
  const assetsDirectory = join(clientDirectory, 'assets');
  if (!existsSync(assetsDirectory)) {
    throw new Error('构建完成，但缺少 client/assets。');
  }

  const entryName = readdirSync(assetsDirectory).find(
    (name) =>
      name.startsWith('entry.client-') &&
      name.endsWith('.js') &&
      !name.endsWith('.map'),
  );
  if (!entryName) {
    throw new Error('构建完成，但缺少 entry.client 脚本。');
  }
  const entryAsset = join(assetsDirectory, entryName);
  const entryBytes = statSync(entryAsset).size;
  if (entryBytes > maximumWebEntryBytes) {
    throw new Error(
      `Web 入口 ${basename(entryAsset)} 为 ${entryBytes} B，超过 500 KiB 验收上限。`,
    );
  }
}

export async function buildProduction(
  outputDirectory: string,
): Promise<string> {
  assertBuildDirectory(outputDirectory);
  const output = await runReactRouterBuild();
  assertEntryBudget(join(defaultBuildDirectory, 'client'));

  if (resolve(outputDirectory) === resolve(defaultBuildDirectory)) {
    return output;
  }

  if (existsSync(outputDirectory)) {
    rmSync(outputDirectory, { recursive: true, force: true });
  }
  mkdirSync(resolve(outputDirectory, '..'), { recursive: true });
  renameSync(defaultBuildDirectory, outputDirectory);
  return output;
}
