import { expect, type Locator, test } from '@playwright/test';

interface ZoomSnapshot {
  k: number;
  x: number;
  y: number;
}

function readZoom(canvas: Locator): Promise<ZoomSnapshot> {
  return canvas.evaluate((element) => {
    const transform = (
      element as HTMLCanvasElement & {
        __zoom?: ZoomSnapshot;
      }
    ).__zoom;
    if (!transform) throw new Error('Expected a d3-zoom canvas transform.');
    return { k: transform.k, x: transform.x, y: transform.y };
  });
}

async function waitForZoom(canvas: Locator) {
  await expect
    .poll(() => canvas.evaluate((element) => '__zoom' in element))
    .toBe(true);
  await expect(canvas).toHaveAttribute('data-graph-layout-status', 'settled', {
    timeout: 15_000,
  });
  await expect
    .poll(
      () =>
        canvas.evaluate(
          (element) =>
            new Promise<number>((resolve) => {
              const readTransform = () => {
                const transform = (
                  element as HTMLCanvasElement & {
                    __zoom?: ZoomSnapshot;
                  }
                ).__zoom;
                if (!transform) {
                  throw new Error('Expected a d3-zoom canvas transform.');
                }
                return transform;
              };
              const before = readTransform();
              requestAnimationFrame(() => {
                requestAnimationFrame(() => {
                  const after = readTransform();
                  resolve(
                    Math.max(
                      Math.abs(after.k - before.k),
                      Math.abs(after.x - before.x),
                      Math.abs(after.y - before.y),
                    ),
                  );
                });
              });
            }),
        ),
      { timeout: 5_000 },
    )
    .toBe(0);
}

async function dispatchWheel(
  canvas: Locator,
  init: Pick<WheelEventInit, 'ctrlKey' | 'deltaX' | 'deltaY' | 'shiftKey'>,
) {
  await canvas.evaluate((element, eventInit) => {
    const bounds = element.getBoundingClientRect();
    element.dispatchEvent(
      new WheelEvent('wheel', {
        ...eventInit,
        bubbles: true,
        cancelable: true,
        clientX: bounds.left + bounds.width / 2,
        clientY: bounds.top + bounds.height / 2,
        deltaMode: WheelEvent.DOM_DELTA_PIXEL,
      }),
    );
  }, init);
}

async function findGraphNodePosition(canvas: Locator) {
  return canvas.evaluate((element) => {
    const bounds = element.getBoundingClientRect();
    for (let y = 4; y < bounds.height; y += 4) {
      for (let x = 4; x < bounds.width; x += 4) {
        if (y < 80 && x > bounds.width - 300) continue;
        element.dispatchEvent(
          new PointerEvent('pointermove', {
            bubbles: true,
            clientX: bounds.left + x,
            clientY: bounds.top + y,
            pointerId: 1,
            pointerType: 'mouse',
          }),
        );
        if ((element as HTMLCanvasElement).style.cursor === 'pointer') {
          return { x, y };
        }
      }
    }
    throw new Error('Expected to find an openable graph node.');
  });
}

test('opens a document node on double click without applying drag force', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const messages: string[] = [];
    (
      window as typeof window & {
        __graphWorkerMessages: string[];
      }
    ).__graphWorkerMessages = messages;
    const postMessage = Reflect.get(
      Worker.prototype,
      'postMessage',
    ) as Worker['postMessage'];
    Worker.prototype.postMessage = function (...args) {
      const message = args[0] as { type?: string };
      messages.push(message.type ?? '');
      return Reflect.apply(postMessage, this, args);
    };
  });
  await page.goto('/graph');
  const canvas = page.getByRole('img', { name: /^知识图谱，包含/ });
  await expect(canvas).toBeVisible();
  await waitForZoom(canvas);

  const nodePosition = await findGraphNodePosition(canvas);
  await page.evaluate(() => {
    (
      window as typeof window & {
        __graphWorkerMessages: string[];
      }
    ).__graphWorkerMessages.length = 0;
  });
  await canvas.click({ position: nodePosition });

  await expect(page).toHaveURL(/\/graph$/);
  await expect(page.getByRole('link', { name: /^打开笔记：/ })).toHaveCount(0);
  await expect
    .poll(() =>
      page.evaluate(() =>
        (
          window as typeof window & {
            __graphWorkerMessages: string[];
          }
        ).__graphWorkerMessages.filter(
          (type) => type === 'pin' || type === 'release',
        ),
      ),
    )
    .toEqual([]);

  await Promise.all([
    page.waitForURL((url) => url.pathname !== '/graph'),
    canvas.dblclick({ position: nodePosition }),
  ]);
});

test('zooms with the wheel and pans with the mouse', async ({ page }) => {
  await page.goto('/graph');
  const canvas = page.getByRole('img', { name: /^知识图谱，包含/ });
  await expect(canvas).toBeVisible();
  await expect(page.locator('aside header [role="status"]')).not.toContainText(
    '正在连接实时事件',
  );
  await waitForZoom(canvas);

  const initial = await readZoom(canvas);
  await dispatchWheel(canvas, {
    ctrlKey: false,
    deltaX: 0,
    deltaY: 24,
    shiftKey: false,
  });
  const zoomed = await readZoom(canvas);
  expect(zoomed.k).toBeLessThan(initial.k);

  const bounds = await canvas.boundingBox();
  if (!bounds) throw new Error('Expected graph canvas bounds.');
  await page.mouse.move(bounds.x + 20, bounds.y + 20);
  await page.mouse.down();
  await page.mouse.move(bounds.x + 46, bounds.y + 35);
  await page.mouse.up();
  const dragged = await readZoom(canvas);
  expect(dragged.k).toBeCloseTo(zoomed.k, 8);
  expect(dragged.x).toBeCloseTo(zoomed.x + 26, 8);
  expect(dragged.y).toBeCloseTo(zoomed.y + 15, 8);
});

test('uses d3 multi-touch pinch zooming on the canvas', async ({
  context,
  page,
}) => {
  await page.goto('/graph');
  const canvas = page.getByRole('img', { name: /^知识图谱，包含/ });
  await expect(canvas).toBeVisible();
  await expect(page.locator('aside header [role="status"]')).not.toContainText(
    '正在连接实时事件',
  );
  await waitForZoom(canvas);

  const bounds = await canvas.boundingBox();
  if (!bounds) throw new Error('Expected graph canvas bounds.');
  const centerX = bounds.x + bounds.width / 2;
  const centerY = bounds.y + bounds.height / 2;
  const initial = await readZoom(canvas);
  const session = await context.newCDPSession(page);

  await session.send('Input.dispatchTouchEvent', {
    touchPoints: [
      { id: 1, x: centerX - 80, y: centerY },
      { id: 2, x: centerX + 80, y: centerY },
    ],
    type: 'touchStart',
  });
  await session.send('Input.dispatchTouchEvent', {
    touchPoints: [
      { id: 1, x: centerX - 40, y: centerY },
      { id: 2, x: centerX + 40, y: centerY },
    ],
    type: 'touchMove',
  });
  await session.send('Input.dispatchTouchEvent', {
    touchPoints: [],
    type: 'touchEnd',
  });

  const pinched = await readZoom(canvas);
  expect(pinched.k).toBeCloseTo(initial.k / 2, 5);
});
