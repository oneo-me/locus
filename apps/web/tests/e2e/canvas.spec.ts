import { expect, test } from '@playwright/test';

test('renders an interactive canvas inline across the note pane', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto('/canvas');

  const canvasContainer = page.locator('[data-canvas-container]').first();
  const canvas = page.locator('iframe[data-canvas-frame]').first();
  const frame = page.frameLocator('iframe[data-canvas-frame]').first();
  await expect(frame.locator('body')).toHaveAttribute(
    'data-script-ran',
    'true',
  );
  await canvasContainer.scrollIntoViewIfNeeded();
  await expect(canvasContainer).toBeVisible();
  await expect(canvasContainer).toHaveCSS('border-radius', '0px');
  await expect(canvasContainer).toHaveCSS('border-width', '0px');
  await expect(canvas).toBeVisible();
  await expect
    .poll(async () => Number(await canvas.getAttribute('height')))
    .toBe(640);
  await expect(canvas).toHaveAttribute('sandbox', 'allow-scripts');
  await expect(canvasContainer.locator('figcaption')).toHaveCount(0);
  await expect(canvasContainer.getByRole('button')).toHaveCount(0);

  await expect(
    frame.getByRole('heading', { name: '嵌入式页面' }),
  ).toBeVisible();
  const counter = frame.getByRole('button', { name: '点击 0 次' });
  await expect(counter).toBeVisible();
  await expect(counter).toHaveCSS(
    'background-color',
    'oklch(0.5 0.134 242.749)',
  );
  await counter.click();
  await expect(
    frame.getByRole('button', { name: '点击 1 次' }),
  ).toHaveAttribute('data-inline-handler', 'true');
  await expect(page.locator('html')).not.toHaveAttribute(
    'data-canvas-compromised',
    'true',
  );
  await expect(frame.locator('script[src]')).toHaveCount(1);
  const runtimeUrl = await frame.locator('script[src]').getAttribute('src');
  expect(runtimeUrl).not.toBeNull();
  expect(new URL(runtimeUrl ?? '', page.url()).origin).toBe(
    new URL(page.url()).origin,
  );
  expect(runtimeUrl).not.toContain('cdn.jsdelivr.net');

  const dimensions = await page.evaluate(() => {
    const article = document.querySelector<HTMLElement>(
      '[data-document-export-content]',
    );
    const main = document.querySelector<HTMLElement>('#main-content');
    const container = document.querySelector<HTMLElement>(
      '[data-canvas-container]',
    );
    if (!article || !main || !container) return null;
    const articleBounds = article.getBoundingClientRect();
    const mainBounds = main.getBoundingClientRect();
    const containerBounds = container.getBoundingClientRect();
    return {
      articleWidth: articleBounds.width,
      containerLeft: containerBounds.left,
      containerWidth: containerBounds.width,
      mainLeft: mainBounds.left,
      mainWidth: main.clientWidth,
    };
  });
  expect(dimensions).not.toBeNull();
  expect(dimensions?.containerWidth).toBeCloseTo(
    (dimensions?.mainWidth ?? 0) - 32,
    0,
  );
  expect(dimensions?.containerLeft).toBeCloseTo(
    (dimensions?.mainLeft ?? 0) + 16,
    0,
  );
  expect(dimensions?.containerWidth ?? 0).toBeGreaterThan(
    dimensions?.articleWidth ?? 0,
  );

  await page.evaluate(() => {
    document.documentElement.classList.remove('dark');
    document.documentElement.classList.add('light');
  });
  await expect(frame.locator('html')).toHaveClass(/\blight\b/);
  const lightBackground = await frame
    .locator('main')
    .evaluate((element) => getComputedStyle(element).backgroundColor);
  await page.evaluate(() => {
    document.documentElement.classList.remove('light');
    document.documentElement.classList.add('dark');
  });
  await expect(frame.locator('html')).toHaveClass(/\bdark\b/);
  await expect
    .poll(() =>
      frame
        .locator('main')
        .evaluate((element) => getComputedStyle(element).backgroundColor),
    )
    .not.toBe(lightBackground);
});

test('sizes a canvas from its content and resolves note attachments', async ({
  page,
}) => {
  await page.goto('/canvas');

  const container = page.locator('[data-canvas-container]').nth(1);
  const canvas = container.locator('iframe[data-canvas-frame]');
  const frame = page.frameLocator('iframe[data-canvas-frame]').nth(1);
  const image = frame.getByRole('img', { name: '画布附件示例' });

  await expect(image).toBeVisible();
  await expect
    .poll(() =>
      image.evaluate((element) => (element as HTMLImageElement).naturalWidth),
    )
    .toBeGreaterThan(0);
  await expect
    .poll(async () => Number(await canvas.getAttribute('height')))
    .toBe(240);
});

test('includes canvas content when copying a note as an image', async ({
  context,
  page,
}) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await page.goto('/canvas');

  const imageAction = page.getByRole('button', { name: '复制为图片' });
  await expect(
    page.locator('[data-canvas-container][data-canvas-export-status="ready"]'),
  ).toHaveCount(3);
  const documentLayout = await page.evaluate(() => {
    const article = document.querySelector<HTMLElement>(
      '[data-document-export-content]',
    );
    if (!article) throw new Error('Expected document export content.');
    return {
      articleWidth: article.clientWidth,
      pageWidth: document.documentElement.scrollWidth,
    };
  });
  await imageAction.click();
  await expect(imageAction).toHaveAttribute('title', '已复制为图片');

  const copiedImage = await page.evaluate(async () => {
    const item = (await navigator.clipboard.read())[0];
    if (!item) throw new Error('Expected a copied document image.');
    const png = await item.getType('image/png');
    const image = await createImageBitmap(png);
    const canvas = document.createElement('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    const context = canvas.getContext('2d');
    if (!context) throw new Error('Expected a 2D canvas.');
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    let count = 0;
    let cyanPixels = 0;
    let cyanLeft = canvas.width;
    let cyanRight = 0;
    let cyanTop = canvas.height;
    let cyanBottom = 0;
    let canvasSurfaceLeft = canvas.width;
    const responsiveMarkers: Array<{ x: number; y: number }> = [];
    let longestCanvasSurfaceRun = 0;
    let currentCanvasSurfaceRun = 0;
    let currentCanvasSurfaceStart = 0;
    for (let index = 0; index < pixels.length; index += 4) {
      const red = pixels[index] ?? 0;
      const green = pixels[index + 1] ?? 0;
      const blue = pixels[index + 2] ?? 0;
      const pixel = index / 4;
      const x = pixel % canvas.width;
      if (
        Math.abs(red - 240) <= 2 &&
        Math.abs(green - 249) <= 2 &&
        Math.abs(blue - 255) <= 2
      ) {
        if (currentCanvasSurfaceRun === 0) currentCanvasSurfaceStart = x;
        currentCanvasSurfaceRun += 1;
        if (currentCanvasSurfaceRun > longestCanvasSurfaceRun) {
          longestCanvasSurfaceRun = currentCanvasSurfaceRun;
          canvasSurfaceLeft = currentCanvasSurfaceStart;
        }
      } else {
        currentCanvasSurfaceRun = 0;
      }
      if (red > 240 && green < 30 && blue > 100 && blue < 160) count += 1;
      if (
        Math.abs(red - 168) <= 4 &&
        Math.abs(green - 85) <= 4 &&
        Math.abs(blue - 247) <= 4
      ) {
        responsiveMarkers.push({ x, y: Math.floor(pixel / canvas.width) });
      }
      if (
        Math.abs(red - 56) <= 8 &&
        Math.abs(green - 189) <= 8 &&
        Math.abs(blue - 248) <= 8
      ) {
        const y = Math.floor(pixel / canvas.width);
        cyanPixels += 1;
        cyanLeft = Math.min(cyanLeft, x);
        cyanRight = Math.max(cyanRight, x);
        cyanTop = Math.min(cyanTop, y);
        cyanBottom = Math.max(cyanBottom, y);
      }
    }
    const markerYCoordinates = [
      ...new Set(responsiveMarkers.map((marker) => marker.y)),
    ].sort((left, right) => left - right);
    let markerRows = 0;
    let previousMarkerY = Number.NEGATIVE_INFINITY;
    for (const markerY of markerYCoordinates) {
      if (markerY - previousMarkerY > 4) markerRows += 1;
      previousMarkerY = markerY;
    }
    return {
      attachmentAspect:
        cyanPixels > 0
          ? (cyanRight - cyanLeft + 1) / (cyanBottom - cyanTop + 1)
          : 0,
      attachmentPixels: cyanPixels,
      canvasSurfaceLeft,
      height: image.height,
      markerPixels: count,
      responsiveMarkerPixels: responsiveMarkers.length,
      responsiveMarkerRows: markerRows,
      width: image.width,
    };
  });
  expect(copiedImage.markerPixels).toBeGreaterThan(100);
  expect(copiedImage.responsiveMarkerPixels).toBeGreaterThan(1_000);
  expect(copiedImage.responsiveMarkerRows).toBe(3);
  expect(copiedImage.attachmentPixels).toBeGreaterThan(100);
  expect(copiedImage.attachmentAspect).toBeGreaterThan(1);
  expect(copiedImage.attachmentAspect).toBeLessThan(1.4);
  expect(copiedImage.canvasSurfaceLeft).toBeGreaterThanOrEqual(14);
  expect(copiedImage.canvasSurfaceLeft).toBeLessThanOrEqual(17);
  expect(copiedImage.width).toBe(documentLayout.articleWidth + 32);
  expect(copiedImage.height).toBeGreaterThan(copiedImage.width);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    documentLayout.pageWidth,
  );
});

test.describe('high-density canvas export', () => {
  test.use({ deviceScaleFactor: 2 });

  test('renders canvas documents at the final PNG density', async ({
    context,
    page,
  }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await page.goto('/canvas');
    const articleWidth = await page
      .locator('[data-document-export-content]')
      .evaluate((article) => article.clientWidth);
    const imageAction = page.getByRole('button', { name: '复制为图片' });
    await imageAction.click();
    await expect(imageAction).toHaveAttribute('title', '已复制为图片');

    const dimensions = await page.evaluate(async () => {
      const item = (await navigator.clipboard.read())[0];
      if (!item) throw new Error('Expected a copied document image.');
      const image = await createImageBitmap(await item.getType('image/png'));
      return { height: image.height, width: image.width };
    });
    expect(dimensions.width).toBe((articleWidth + 32) * 2);
    expect(dimensions.height).toBeGreaterThan(dimensions.width);
  });
});

test('keeps an inline canvas within the note pane on a narrow viewport', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/canvas');

  const canvas = page.locator('iframe[data-canvas-frame]').first();
  const frame = page.frameLocator('iframe[data-canvas-frame]').first();
  await expect(frame.locator('body')).toHaveAttribute(
    'data-script-ran',
    'true',
  );
  await canvas.scrollIntoViewIfNeeded();
  await expect(canvas).toBeVisible();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          document.documentElement.scrollWidth -
          document.documentElement.clientWidth,
      ),
    )
    .toBe(0);
});
