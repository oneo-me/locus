import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DocumentLibrary } from '../app/server/features/library/core/document-library';

const documentCount = Number.parseInt(process.argv[2] ?? '750', 10);
const idleDurationMs = Number.parseInt(process.argv[3] ?? '1500', 10);
const watchIntervalMs = process.argv[4]
  ? Number.parseInt(process.argv[4], 10)
  : undefined;

if (!Number.isInteger(documentCount) || documentCount < 1) {
  throw new Error('documentCount must be a positive integer.');
}
if (!Number.isInteger(idleDurationMs) || idleDurationMs < 100) {
  throw new Error('idleDurationMs must be at least 100.');
}
if (
  watchIntervalMs !== undefined &&
  (!Number.isInteger(watchIntervalMs) || watchIntervalMs < 100)
) {
  throw new Error('watchIntervalMs must be at least 100.');
}

const root = mkdtempSync(join(tmpdir(), 'asterweft-benchmark-library-'));
const cacheRoot = mkdtempSync(join(tmpdir(), 'asterweft-benchmark-cache-'));
function elapsedMs(start: number): number {
  return Number((performance.now() - start).toFixed(2));
}

function cpuMs(start: NodeJS.CpuUsage): number {
  const usage = process.cpuUsage(start);
  return Number(((usage.user + usage.system) / 1000).toFixed(2));
}

function rssMiB(): number {
  return Number((process.memoryUsage.rss() / 1024 / 1024).toFixed(2));
}

function wait(durationMs: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}

try {
  const body = '基准正文 '.repeat(128);
  for (let index = 0; index < documentCount; index += 1) {
    const directory = join(
      root,
      `article-${index.toString().padStart(4, '0')}`,
    );
    mkdirSync(directory);
    const previous =
      index > 0
        ? `\n[上一篇](../article-${(index - 1).toString().padStart(4, '0')})`
        : '';
    writeFileSync(
      join(directory, 'index.md'),
      `---\ntitle: Article ${index}\ntags: [benchmark]\n---\n${body}${previous}`,
    );
  }

  const rssBefore = rssMiB();
  const startupCpu = process.cpuUsage();
  const startupStart = performance.now();
  const library = await DocumentLibrary.open(root, {
    cachePath: join(cacheRoot, 'cache.sqlite'),
    watch: true,
    watchDebounceMs: 20,
    watchIntervalMs,
  });
  const startup = {
    wallMs: elapsedMs(startupStart),
    cpuMs: cpuMs(startupCpu),
    rssDeltaMiB: Number((rssMiB() - rssBefore).toFixed(2)),
  };

  const idleCpu = process.cpuUsage();
  await wait(idleDurationMs);
  const idle = {
    durationMs: idleDurationMs,
    cpuMs: cpuMs(idleCpu),
  };

  const graphFirstCpu = process.cpuUsage();
  const graphFirstStart = performance.now();
  const firstGraph = library.getDocumentGraph();
  const graphFirst = {
    wallMs: elapsedMs(graphFirstStart),
    cpuMs: cpuMs(graphFirstCpu),
    nodes: firstGraph.nodes.length,
    edges: firstGraph.edges.length,
  };

  const graphRepeatCpu = process.cpuUsage();
  const graphRepeatStart = performance.now();
  const secondGraph = library.getDocumentGraph();
  const graphRepeat = {
    wallMs: elapsedMs(graphRepeatStart),
    cpuMs: cpuMs(graphRepeatCpu),
    nodes: secondGraph.nodes.length,
    edges: secondGraph.edges.length,
  };

  const changedPath = join(root, 'article-0000', 'index.md');
  let resolveChange = () => {};
  const changed = new Promise<void>((resolve) => {
    resolveChange = resolve;
  });
  const unsubscribe = library.subscribe(resolveChange);
  const changeCpu = process.cpuUsage();
  const changeStart = performance.now();
  writeFileSync(
    changedPath,
    `---\ntitle: Changed article\ntags: [benchmark]\n---\n${body}`,
  );
  await Promise.race([
    changed,
    wait(10_000).then(() => {
      throw new Error('Timed out waiting for the external file change.');
    }),
  ]);
  const externalChange = {
    wallMs: elapsedMs(changeStart),
    cpuMs: cpuMs(changeCpu),
  };

  unsubscribe();
  library.close();

  console.log(
    JSON.stringify(
      {
        documentCount,
        watchIntervalMs: watchIntervalMs ?? 'default',
        startup,
        idle,
        graphFirst,
        graphRepeat,
        externalChange,
        finalRssMiB: rssMiB(),
      },
      null,
      2,
    ),
  );
} finally {
  rmSync(root, { recursive: true, force: true });
  rmSync(cacheRoot, { recursive: true, force: true });
}
