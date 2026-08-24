import { spawn } from 'node:child_process';
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';

interface LighthouseReport {
  audits: Record<string, { displayValue?: string; numericValue?: number }>;
  categories: { performance: { score: number | null } };
}

interface LighthousePageResult {
  path: string;
  performance: { median: number; runs: number[] };
  firstContentfulPaint?: string;
  largestContentfulPaint?: string;
  totalBlockingTime?: string;
  cumulativeLayoutShift?: string;
  totalByteWeight?: string;
  unusedJavaScript?: string;
}

const projectRoot = resolve(import.meta.dirname, '..');
const serverEntry = join(projectRoot, 'build', 'server', 'index.js');
const serveCli = join(
  projectRoot,
  'node_modules',
  '@react-router',
  'serve',
  'dist',
  'cli.js',
);
if (!existsSync(serverEntry)) {
  throw new Error('缺少 Web 生产构建；请先运行 pnpm run build。');
}

const port = Number.parseInt(
  process.env.ASTERWEFT_LIGHTHOUSE_PORT ?? '4178',
  10,
);
if (!Number.isSafeInteger(port) || port < 1 || port > 65_535) {
  throw new Error('ASTERWEFT_LIGHTHOUSE_PORT 必须是有效端口。');
}

const runs = Number.parseInt(process.env.ASTERWEFT_LIGHTHOUSE_RUNS ?? '3', 10);
if (!Number.isSafeInteger(runs) || runs < 1 || runs > 10) {
  throw new Error('ASTERWEFT_LIGHTHOUSE_RUNS 必须是 1–10。');
}

const paths = (process.env.ASTERWEFT_LIGHTHOUSE_PATHS ?? '/,/guide')
  .split(',')
  .map((path) => path.trim())
  .filter(Boolean);
if (
  paths.length === 0 ||
  paths.some((path) => !path.startsWith('/') || path.startsWith('//'))
) {
  throw new Error('ASTERWEFT_LIGHTHOUSE_PATHS 必须是逗号分隔的站内绝对路径。');
}

const temporaryRoot = mkdtempSync(join(tmpdir(), 'asterweft-lighthouse-'));
const libraryPath = join(temporaryRoot, 'library');
const homePath = join(temporaryRoot, 'home');
const reportsPath = join(temporaryRoot, 'reports');
cpSync(join(projectRoot, 'tests', 'fixtures', 'library'), libraryPath, {
  recursive: true,
});
mkdirSync(homePath);
mkdirSync(reportsPath);

async function run(command: string[], cwd = projectRoot) {
  const [executable, ...args] = command;
  if (!executable) throw new Error('缺少命令。');
  const child = spawn(executable, args, {
    cwd,
    stdio: ['ignore', 'inherit', 'inherit'],
  });
  const exitCode = await new Promise<number>((resolveExit) => {
    child.on('close', (code) => resolveExit(code ?? 1));
  });
  if (exitCode !== 0) {
    throw new Error(`${command.join(' ')} 退出码为 ${exitCode}。`);
  }
}

await run(['git', 'init', '--quiet'], libraryPath);
await run(['git', 'add', '.'], libraryPath);
await run(
  [
    'git',
    '-c',
    'user.name=Asterweft Lighthouse',
    '-c',
    'user.email=lighthouse@asterweft.local',
    'commit',
    '--quiet',
    '-m',
    'fixture',
  ],
  libraryPath,
);

const server = spawn(process.execPath, [serveCli, serverEntry], {
  cwd: projectRoot,
  env: {
    ...process.env,
    ASTERWEFT_HOME: homePath,
    HOST: '127.0.0.1',
    ASTERWEFT_LIBRARY_PATH: libraryPath,
    NODE_ENV: 'production',
    PORT: String(port),
  },
  stdio: 'inherit',
});

const serverExit = new Promise<number>((resolveExit) => {
  server.on('exit', (code) => resolveExit(code ?? 1));
});

const origin = `http://127.0.0.1:${port}`;
try {
  let ready = false;
  for (let attempt = 0; attempt < 100; attempt += 1) {
    try {
      ready = (await fetch(origin)).ok;
    } catch {
      // The process may still be starting.
    }
    if (ready) break;
    await sleep(50);
  }
  if (!ready) throw new Error(`生产服务未能在 ${origin} 启动。`);

  const chromePath =
    process.env.CHROME_PATH ??
    (existsSync('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome')
      ? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
      : undefined);
  const results: LighthousePageResult[] = [];
  for (const [pathIndex, path] of paths.entries()) {
    const scores: number[] = [];
    const measurements: LighthouseReport[] = [];
    for (let runIndex = 0; runIndex < runs; runIndex += 1) {
      const reportPath = join(
        reportsPath,
        `report-${pathIndex + 1}-${runIndex + 1}.json`,
      );
      const command = [
        join(projectRoot, 'node_modules', '.bin', 'lighthouse'),
        new URL(path, origin).href,
        '--preset=desktop',
        '--only-categories=performance',
        '--output=json',
        `--output-path=${reportPath}`,
        '--quiet',
      ];
      if (chromePath) command.push(`--chrome-path=${chromePath}`);
      await run(command);

      const report = JSON.parse(
        readFileSync(reportPath, 'utf8'),
      ) as LighthouseReport;
      const score = report.categories.performance.score;
      if (score === null) {
        throw new Error(`Lighthouse 未返回 ${path} 的 Performance 分数。`);
      }
      scores.push(score * 100);
      measurements.push(report);
    }

    const orderedScores = [...scores].sort((left, right) => left - right);
    const median = orderedScores[Math.floor(orderedScores.length / 2)] ?? 0;
    const report = measurements[scores.indexOf(median)];
    const cumulativeLayoutShift =
      report?.audits['cumulative-layout-shift']?.numericValue ?? 0;
    results.push({
      path,
      performance: { median, runs: scores },
      firstContentfulPaint:
        report?.audits['first-contentful-paint']?.displayValue,
      largestContentfulPaint:
        report?.audits['largest-contentful-paint']?.displayValue,
      totalBlockingTime: report?.audits['total-blocking-time']?.displayValue,
      cumulativeLayoutShift:
        report?.audits['cumulative-layout-shift']?.displayValue,
      totalByteWeight: report?.audits['total-byte-weight']?.displayValue,
      unusedJavaScript: report?.audits['unused-javascript']?.displayValue,
    });
    if (median < 90) {
      throw new Error(
        `Lighthouse ${path} Performance 中位数 ${median} 低于 90。`,
      );
    }
    if (cumulativeLayoutShift > 0.1) {
      throw new Error(
        `Lighthouse ${path} CLS ${cumulativeLayoutShift} 高于 0.1。`,
      );
    }
  }
  console.log(JSON.stringify({ pages: results }, null, 2));
} finally {
  server.kill();
  await serverExit;
  rmSync(temporaryRoot, { force: true, recursive: true });
}
