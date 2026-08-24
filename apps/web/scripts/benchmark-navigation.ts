import { chromium, type Request } from '@playwright/test';

interface NavigationTarget {
  path: string;
  title: string;
}

interface ServerFunctionMeasurement {
  bytes: number;
  durationMs: number;
  url: string;
}

const baseUrl = (
  process.env.ASTERWEFT_BENCHMARK_URL ?? 'http://127.0.0.1:3000'
).replace(/\/$/, '');

const initialTarget: NavigationTarget = {
  path: '/projects/asterweft/development',
  title: '开发与运行',
};

const targets: NavigationTarget[] = [
  {
    path: '/projects/asterweft/architecture',
    title: '架构设计',
  },
  {
    path: '/projects/asterweft/development',
    title: '开发与运行',
  },
  {
    path: '/projects/asterweft/coding',
    title: '编码规范',
  },
  {
    path: '/projects/asterweft/architecture',
    title: '架构设计',
  },
];

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
const requestStartedAt = new Map<Request, number>();
const serverFunctions: ServerFunctionMeasurement[] = [];
const pendingServerFunctionMeasurements = new Set<Promise<void>>();
let eventStreamConnections = 0;

page.on('request', (request) => {
  const url = new URL(request.url());
  if (url.pathname === '/api/graph-events') eventStreamConnections += 1;
  if (request.headers()['x-tsr-serverfn'] === 'true') {
    requestStartedAt.set(request, performance.now());
  }
});

page.on('requestfinished', (request) => {
  const startedAt = requestStartedAt.get(request);
  if (startedAt === undefined) return;
  requestStartedAt.delete(request);
  const measurement = (async () => {
    const response = await request.response();
    if (!response) return;
    const body = await response.body();
    serverFunctions.push({
      bytes: body.byteLength,
      durationMs: performance.now() - startedAt,
      url: new URL(request.url()).pathname,
    });
  })();
  pendingServerFunctionMeasurements.add(measurement);
  void measurement.finally(() => {
    pendingServerFunctionMeasurements.delete(measurement);
  });
});

const startupStartedAt = performance.now();
await page.goto(`${baseUrl}${initialTarget.path}`, {
  waitUntil: 'domcontentloaded',
});
await page
  .getByLabel('文档元数据')
  .getByRole('heading', { name: initialTarget.title })
  .waitFor();
const startupMs = performance.now() - startupStartedAt;

await page
  .locator('aside header [role="status"]')
  .filter({ hasNotText: '正在连接实时事件' })
  .waitFor();
await page.waitForTimeout(100);
await Promise.all(pendingServerFunctionMeasurements);

const switches = [];
for (const target of targets) {
  const serverFunctionOffset = serverFunctions.length;
  const startedAt = performance.now();
  await page.locator(`nav a[href="${target.path}"]`).click();
  await page
    .getByLabel('文档元数据')
    .getByRole('heading', { name: target.title })
    .waitFor();
  await page.waitForTimeout(0);
  await Promise.all(pendingServerFunctionMeasurements);
  switches.push({
    path: target.path,
    title: target.title,
    durationMs: performance.now() - startedAt,
    serverFunctions: serverFunctions.slice(serverFunctionOffset),
  });
}

console.log(
  JSON.stringify(
    {
      baseUrl,
      eventStreamConnections,
      startupMs,
      switches,
    },
    null,
    2,
  ),
);

await browser.close();
