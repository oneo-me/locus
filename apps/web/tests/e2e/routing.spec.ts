import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { expect, type Page, test } from '@playwright/test';

const guideFixturePath = resolve(
  import.meta.dirname,
  '../fixtures/library/guide/index.md',
);

const liveSignalsFixturePath = resolve(
  import.meta.dirname,
  '../fixtures/library/live-signals',
);

async function copiedImageColorStats(
  page: Page,
  color: readonly [number, number, number],
) {
  return page.evaluate(async ([targetRed, targetGreen, targetBlue]) => {
    const item = (await navigator.clipboard.read())[0];
    if (!item) throw new Error('Expected a copied article image.');
    const png = await item.getType('image/png');
    const image = await createImageBitmap(png);
    const canvas = document.createElement('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    const context = canvas.getContext('2d');
    if (!context) throw new Error('Expected an image canvas.');
    context.drawImage(image, 0, 0);
    const sourcePixels = context.getImageData(
      0,
      0,
      canvas.width,
      canvas.height,
    ).data;
    const matches = new Uint8Array(canvas.width * canvas.height);
    let matchedPixels = 0;
    for (let index = 0; index < sourcePixels.length; index += 4) {
      if (
        Math.abs((sourcePixels[index] ?? 0) - targetRed) <= 8 &&
        Math.abs((sourcePixels[index + 1] ?? 0) - targetGreen) <= 8 &&
        Math.abs((sourcePixels[index + 2] ?? 0) - targetBlue) <= 8
      ) {
        matches[index / 4] = 1;
        matchedPixels += 1;
      }
    }
    const queue = new Int32Array(matches.length);
    let components = 0;
    for (let start = 0; start < matches.length; start += 1) {
      if (matches[start] !== 1) continue;
      let head = 0;
      let tail = 1;
      let componentPixels = 0;
      queue[0] = start;
      matches[start] = 0;
      while (head < tail) {
        const current = queue[head] ?? 0;
        head += 1;
        componentPixels += 1;
        const x = current % canvas.width;
        for (const neighbor of [
          x > 0 ? current - 1 : -1,
          x + 1 < canvas.width ? current + 1 : -1,
          current - canvas.width,
          current + canvas.width,
        ]) {
          if (neighbor >= 0 && matches[neighbor] === 1) {
            matches[neighbor] = 0;
            queue[tail] = neighbor;
            tail += 1;
          }
        }
      }
      if (componentPixels > 50) components += 1;
    }
    return { components, pixels: matchedPixels };
  }, color);
}

test('updates sidebar documents and attachment counts from external file changes', async ({
  page,
}) => {
  await rm(liveSignalsFixturePath, { force: true, recursive: true });

  try {
    await page.goto('/');
    await mkdir(liveSignalsFixturePath);
    await writeFile(
      resolve(liveSignalsFixturePath, 'index.md'),
      '---\ntitle: 实时信号\n---\n正文',
    );
    await writeFile(resolve(liveSignalsFixturePath, 'first.txt'), 'first');
    await writeFile(
      resolve(liveSignalsFixturePath, 'icon.png'),
      await readFile(
        resolve(import.meta.dirname, '../fixtures/library/guide/icon.png'),
      ),
    );

    const navigationLink = page.getByRole('link', {
      exact: true,
      name: '实时信号',
    });
    await expect(navigationLink).toBeVisible();
    await expect(navigationLink.getByTitle('1 个附件')).toHaveText(/^\s*1\s*$/);

    await rm(resolve(liveSignalsFixturePath, 'first.txt'));
    await expect(navigationLink.getByTitle('1 个附件')).toHaveCount(0);

    await writeFile(resolve(liveSignalsFixturePath, 'second.txt'), 'second');
    await expect(navigationLink.getByTitle('1 个附件')).toHaveText(/^\s*1\s*$/);

    await navigationLink.click();
    await expect(page).toHaveURL('/live-signals');
    await rm(liveSignalsFixturePath, { recursive: true });
    await expect(navigationLink).toHaveCount(0);
  } finally {
    await rm(liveSignalsFixturePath, { force: true, recursive: true });
  }
});

test('returns home from the full brand area with settings as a separate action', async ({
  page,
}) => {
  await page.goto('/guide');

  const header = page.locator('aside header');
  const homeLink = header.getByRole('link', {
    exact: true,
    name: 'Asterweft 首页',
  });
  await expect(homeLink).toHaveAttribute('href', '/');
  await expect(homeLink).toHaveCSS('cursor', 'default');
  await expect(homeLink.locator('img')).toBeVisible();
  await expect(homeLink.getByRole('status')).toBeVisible();
  await expect(page.locator('#main-content')).toBeVisible();
  await expect(header.getByRole('button', { name: '设置' })).toBeVisible();
  await expect(header.getByRole('link', { name: '知识图谱' })).toBeVisible();

  await homeLink.getByRole('status').click();
  await expect(page).toHaveURL('/');
});

test('redirects the former PWA launch URL to the home page', async ({
  page,
}) => {
  await page.goto('/document');

  await expect(page).toHaveURL('/');
  await expect(page.locator('#main-content')).toBeVisible();
});

test('opens the design system from settings in a separate page', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { exact: true, name: '设置' }).click();

  const designSystemLink = page
    .getByRole('dialog')
    .getByRole('link', { exact: true, name: '设计系统' });
  await expect(designSystemLink).toHaveAttribute('href', '/design-system');
  await expect(designSystemLink).toHaveAttribute('target', '_blank');

  const popupPromise = page.waitForEvent('popup');
  await designSystemLink.click();
  const designSystemPage = await popupPromise;

  await expect(designSystemPage).toHaveURL('/design-system');
  await expect(
    designSystemPage.getByRole('heading', { name: '设计系统' }),
  ).toBeAttached();
  await expect(
    designSystemPage.getByRole('complementary', { name: '文档导航' }),
  ).toHaveCount(0);
  await expect(
    designSystemPage.getByText('Button 按钮', { exact: true }),
  ).toBeVisible();
});

test('keeps the rendered article surface while switching articles', async ({
  page,
}) => {
  await page.goto('/guide');
  await expect(
    page.getByLabel('文档元数据').getByRole('heading', { name: '路由验证' }),
  ).toBeVisible();

  await page.evaluate(() => {
    const main = document.querySelector('#main-content');
    if (!main) throw new Error('Expected the document surface.');
    const titles: Array<string | null> = [];
    new MutationObserver(() => {
      titles.push(
        document.querySelector('[aria-label="文档元数据"] h1')?.textContent ??
          null,
      );
    }).observe(main, {
      characterData: true,
      childList: true,
      subtree: true,
    });
    (
      window as typeof window & {
        articleSwitchState?: {
          main: Element;
          titles: Array<string | null>;
        };
      }
    ).articleSwitchState = { main, titles };
  });

  const articleNavigationRequests: string[] = [];
  page.on('request', (request) => {
    const url = new URL(request.url());
    if (url.pathname.endsWith('.data')) {
      articleNavigationRequests.push(`${url.pathname}${url.search}`);
    }
  });
  const articleDataResponsePromise = page.waitForResponse(
    (response) => new URL(response.url()).pathname === '/canvas.data',
  );

  await page.getByRole('link', { exact: true, name: '画布' }).click();
  const articleDataResponse = await articleDataResponsePromise;
  const articleDataBody = await articleDataResponse.text();
  await expect(page).toHaveURL('/canvas');
  await expect(
    page.getByLabel('文档元数据').getByRole('heading', { name: '画布' }),
  ).toBeVisible();

  const state = await page.evaluate(() => {
    const current = (
      window as typeof window & {
        articleSwitchState?: {
          main: Element;
          titles: Array<string | null>;
        };
      }
    ).articleSwitchState;
    return {
      mainPreserved: current?.main === document.querySelector('#main-content'),
      titles: current?.titles ?? [],
    };
  });

  expect(state.mainPreserved).toBe(true);
  expect(state.titles).not.toContain(null);
  expect(articleNavigationRequests).toEqual([
    '/canvas.data?_routes=routes%2Fworkspace-document',
  ]);
  expect(articleDataBody).toContain('selectedPath');
  expect(articleDataBody).not.toContain('repositoryChanges');
});

test('opens articles and serves protected files through the attachment API', async ({
  page,
  request,
}) => {
  await page.goto('/guide');

  await expect(
    page.getByLabel('文档元数据').getByRole('heading', { name: '路由验证' }),
  ).toBeVisible();
  const guideNavigationLink = page.getByRole('link', {
    name: '路由验证',
    exact: true,
  });
  await expect(guideNavigationLink.getByTitle('4 个附件')).toHaveText(
    /^\s*4\s*$/,
  );
  await expect(guideNavigationLink).toHaveAccessibleDescription(/4 个附件/);
  await expect(
    page.getByRole('button', { name: '文档图标' }).locator('img'),
  ).toHaveAttribute('src', /^\/api\/assets\?path=guide%2Ficon\.png&v=/);
  await expect(
    page.getByRole('link', { name: '下载附件：report.txt' }),
  ).toHaveAttribute('href', '/api/assets?path=guide%2Freport.txt');
  await expect(
    page.getByRole('button', { name: /^查看图片：图库测试图片/ }),
  ).toHaveCount(16);
  await expect(
    page
      .getByRole('button', { name: '查看图片：图库测试图片 01' })
      .locator('img'),
  ).toHaveAttribute('width', /^\d+$/);
  const wideBlockLayout = await page.evaluate(() => {
    const article = document.querySelector<HTMLElement>(
      '[data-document-export-content]',
    );
    const main = document.querySelector<HTMLElement>('#main-content');
    const blocks = {
      code: document.querySelector<HTMLElement>('figure.code-block'),
      gallery: document.querySelector<HTMLElement>('[data-attachment-gallery]'),
      table: document.querySelector<HTMLElement>('table'),
    };
    if (!article || !main || Object.values(blocks).some((block) => !block)) {
      return null;
    }
    const articleBounds = article.getBoundingClientRect();
    const mainBounds = main.getBoundingClientRect();
    return {
      articleWidth: articleBounds.width,
      blocks: Object.fromEntries(
        Object.entries(blocks).map(([name, block]) => {
          const bounds = block!.getBoundingClientRect();
          return [name, { left: bounds.left, width: bounds.width }];
        }),
      ),
      mainLeft: mainBounds.left,
      mainWidth: main.clientWidth,
    };
  });
  expect(wideBlockLayout).not.toBeNull();
  const maximumBlockWidth = (wideBlockLayout?.mainWidth ?? 0) - 32;
  const mainCenter =
    (wideBlockLayout?.mainLeft ?? 0) + (wideBlockLayout?.mainWidth ?? 0) / 2;
  for (const block of Object.values(wideBlockLayout?.blocks ?? {})) {
    expect(block.left + block.width / 2).toBeCloseTo(mainCenter, 0);
    expect(block.width).toBeGreaterThanOrEqual(
      wideBlockLayout?.articleWidth ?? 0,
    );
    expect(block.width).toBeLessThanOrEqual(maximumBlockWidth + 0.5);
  }
  expect(wideBlockLayout?.blocks.code?.width).toBeCloseTo(
    wideBlockLayout?.articleWidth ?? 0,
    0,
  );
  expect(wideBlockLayout?.blocks.gallery?.width).toBeGreaterThan(
    wideBlockLayout?.articleWidth ?? 0,
  );
  expect(wideBlockLayout?.blocks.table?.width).toBeGreaterThan(
    wideBlockLayout?.articleWidth ?? 0,
  );
  const galleryLayout = await page
    .getByRole('region', { name: '图片附件，共 16 张' })
    .evaluate((gallery) => {
      const galleryBounds = gallery.getBoundingClientRect();
      const rows = new Map<number, DOMRect[]>();
      for (const button of gallery.querySelectorAll('button')) {
        const bounds = button.getBoundingClientRect();
        const top = Math.round(bounds.top);
        rows.set(top, [...(rows.get(top) ?? []), bounds]);
      }
      const orderedRows = [...rows.entries()]
        .sort(([topA], [topB]) => topA - topB)
        .map(([, bounds]) => bounds.sort((a, b) => a.left - b.left));
      return {
        galleryLeft: galleryBounds.left,
        galleryRight: galleryBounds.right,
        itemRatios: [...gallery.querySelectorAll('button')].map((button) => {
          const bounds = button.getBoundingClientRect();
          const item = button.closest<HTMLElement>('[data-gallery-width]');
          const width = Number(item?.dataset.galleryWidth);
          const height = Number(item?.dataset.galleryHeight);
          return {
            box: bounds.width / bounds.height,
            source: width / height,
          };
        }),
        rows: orderedRows.map((bounds) => ({
          bottom: bounds[0]?.bottom ?? 0,
          heights: bounds.map((item) => item.height),
          left: bounds[0]?.left ?? 0,
          right: bounds.at(-1)?.right ?? 0,
          top: bounds[0]?.top ?? 0,
        })),
      };
    });
  expect(galleryLayout.rows.length).toBeGreaterThan(1);
  expect(galleryLayout.itemRatios[0]?.source).toBeCloseTo(1 / 3, 2);
  expect(galleryLayout.itemRatios[1]?.source).toBeCloseTo(4 / 3, 2);
  for (const ratio of galleryLayout.itemRatios) {
    expect(ratio.box).toBeCloseTo(ratio.source, 2);
  }
  for (const row of galleryLayout.rows.slice(0, -1)) {
    expect(Math.abs(row.left - galleryLayout.galleryLeft)).toBeLessThan(0.5);
    expect(Math.abs(row.right - galleryLayout.galleryRight)).toBeLessThan(0.5);
    expect(Math.max(...row.heights) - Math.min(...row.heights)).toBeLessThan(
      0.5,
    );
  }
  const finalGalleryRow = galleryLayout.rows.at(-1);
  expect(finalGalleryRow).toBeDefined();
  expect(finalGalleryRow!.right).toBeLessThan(galleryLayout.galleryRight);
  expect(finalGalleryRow!.top - galleryLayout.rows.at(-2)!.bottom).toBeCloseTo(
    8,
  );
  await expect(page.getByText('unreferenced.txt')).toHaveCount(0);

  await page.getByRole('button', { name: '查看图片：图库测试图片 01' }).click();
  const imageDialog = page.getByRole('dialog');
  await expect(
    imageDialog.getByRole('img', { name: '图库测试图片 01' }),
  ).toBeVisible();
  const previewImageSize = await imageDialog
    .getByRole('img', { name: '图库测试图片 01' })
    .evaluate((image) => {
      const element = image as HTMLImageElement;
      const bounds = element.getBoundingClientRect();
      return {
        height: bounds.height,
        naturalHeight: element.naturalHeight,
        naturalWidth: element.naturalWidth,
        width: bounds.width,
      };
    });
  expect(previewImageSize.width).toBeLessThanOrEqual(
    previewImageSize.naturalWidth,
  );
  expect(previewImageSize.height).toBeLessThanOrEqual(
    previewImageSize.naturalHeight,
  );
  expect(previewImageSize.width).toBeLessThanOrEqual(1568);
  expect(previewImageSize.height).toBeLessThanOrEqual(968);
  await expect(imageDialog).toHaveClass(/inset-0/);
  await expect(imageDialog).toHaveClass(/bg-transparent/);
  await expect(page.locator('[data-slot="dialog-overlay"]')).toHaveClass(
    /bg-black\/80/,
  );
  await imageDialog.getByRole('img', { name: '图库测试图片 01' }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await page.getByRole('button', { name: '查看图片：图库测试图片 01' }).click();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);

  const inlineLink = page.getByRole('link', { name: '打开报告' });
  await expect(inlineLink).toHaveClass(/document-inline-link/);
  const inlineLinkStyle = await inlineLink.evaluate((element) => {
    const style = window.getComputedStyle(element);
    const paragraph = element.closest('p');
    if (!paragraph) throw new Error('Expected the inline link in a paragraph.');
    return {
      backgroundColor: style.backgroundColor,
      backgroundImage: style.backgroundImage,
      borderStyle: style.borderStyle,
      borderWidth: style.borderWidth,
      color: style.color,
      fontSize: style.fontSize,
      marginBlockStart: Number.parseFloat(style.marginBlockStart),
      paddingInlineStart: Number.parseFloat(style.paddingInlineStart),
      paragraphColor: window.getComputedStyle(paragraph).color,
      paragraphFontSize: window.getComputedStyle(paragraph).fontSize,
      paragraphHeight: paragraph.getBoundingClientRect().height,
      paragraphLineHeight: Number.parseFloat(
        window.getComputedStyle(paragraph).lineHeight,
      ),
      verticalAlign: style.verticalAlign,
    };
  });
  expect(Number.parseFloat(inlineLinkStyle.fontSize)).toBeCloseTo(
    Number.parseFloat(inlineLinkStyle.paragraphFontSize) * 0.95,
    1,
  );
  expect(inlineLinkStyle.color).toBe(inlineLinkStyle.paragraphColor);
  expect(inlineLinkStyle.backgroundImage).toContain('linear-gradient');
  expect(inlineLinkStyle.marginBlockStart).toBeLessThan(0);
  expect(inlineLinkStyle.paddingInlineStart).toBeGreaterThan(0);
  expect(inlineLinkStyle.borderStyle).toBe('solid');
  expect(inlineLinkStyle.borderWidth).toBe('1px');
  expect(inlineLinkStyle.verticalAlign).toBe('middle');
  expect(inlineLinkStyle.paragraphHeight).toBeCloseTo(
    inlineLinkStyle.paragraphLineHeight,
    0,
  );

  await inlineLink.hover();
  await expect
    .poll(() =>
      inlineLink.evaluate(
        (element) => window.getComputedStyle(element).backgroundColor,
      ),
    )
    .not.toBe(inlineLinkStyle.backgroundColor);

  const iconResponse = await request.get('/api/assets?path=guide%2Ficon.png');
  expect(iconResponse.status()).toBe(200);
  expect(iconResponse.headers()['content-type']).toBe('image/png');

  const attachmentResponse = await request.get(
    '/api/assets?path=guide%2Freport.txt',
  );
  expect(attachmentResponse.status()).toBe(200);
  expect(await attachmentResponse.text()).toContain('direct attachment route');

  const graphResponse = await request.get('/api/library/graph');
  expect(graphResponse.status()).toBe(200);
  const graph = (await graphResponse.json()) as {
    graph: { nodes: Array<{ kind: string; path: string }> };
  };
  expect(graph.graph.nodes).toContainEqual(
    expect.objectContaining({
      kind: 'attachment',
      path: 'guide/unreferenced.txt',
    }),
  );
});

test('lays out gallery proportions before client hydration', async ({
  page,
}) => {
  await page.route('**/*', async (route) => {
    if (route.request().resourceType() === 'script') {
      await route.abort();
      return;
    }
    await route.continue();
  });
  await page.goto('/guide');

  const buttons = page.getByRole('button', {
    name: /^查看图片：图库测试图片/,
  });
  await expect(buttons).toHaveCount(16);
  const ratios = await buttons.evaluateAll((items) =>
    items.map((button) => {
      const bounds = button.getBoundingClientRect();
      const item = button.closest<HTMLElement>('[data-gallery-width]');
      return {
        box: bounds.width / bounds.height,
        source:
          Number(item?.dataset.galleryWidth) /
          Number(item?.dataset.galleryHeight),
      };
    }),
  );

  expect(ratios[0]?.source).toBeCloseTo(1 / 3, 2);
  expect(ratios[1]?.source).toBeCloseTo(4 / 3, 2);
  for (const ratio of ratios) {
    expect(ratio.box).toBeCloseTo(ratio.source, 2);
  }
});

test('keeps the server-rendered article while live events connect', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const state = {
      galleryReplacements: 0,
      galleryTops: [] as number[],
      sawArticleLoadingState: false,
    };
    let gallery: Element | null = null;
    const sample = () => {
      const nextGallery = document.querySelector<HTMLElement>(
        '[data-attachment-gallery]',
      );
      if (nextGallery) {
        if (gallery && nextGallery !== gallery) {
          state.galleryReplacements += 1;
        }
        gallery = nextGallery;
        const bounds = nextGallery.getBoundingClientRect();
        if (bounds.width > 0 && bounds.height > 0) {
          state.galleryTops.push(bounds.top);
        }
      }
      state.sawArticleLoadingState ||= Boolean(
        document.querySelector('[data-document-content-status="loading"]'),
      );
      requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
    Object.assign(window, { articleStabilityState: state });
  });
  await page.goto('/guide');
  await expect(page.locator('aside header [role="status"]')).toContainText(
    '事件流已连接。',
  );
  await page.waitForFunction(() =>
    [
      ...document.querySelectorAll<HTMLImageElement>(
        '[data-attachment-gallery] img',
      ),
    ].every((image) => image.complete),
  );
  await page.evaluate(() => document.fonts.ready);
  const state = await page.evaluate(
    () =>
      (
        window as typeof window & {
          articleStabilityState: {
            galleryReplacements: number;
            galleryTops: number[];
            sawArticleLoadingState: boolean;
          };
        }
      ).articleStabilityState,
  );

  expect(state.galleryReplacements).toBe(0);
  expect(state.sawArticleLoadingState).toBe(false);
  expect(state.galleryTops.length).toBeGreaterThan(0);
  expect(
    Math.max(...state.galleryTops) - Math.min(...state.galleryTops),
  ).toBeLessThan(0.5);
});

test('copies an article image in the current theme with inline attachments', async ({
  context,
  page,
}) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await page.goto('/guide');
  await expect(page.locator('aside header [role="status"]')).not.toContainText(
    '正在连接实时事件',
  );
  await page.evaluate(() => {
    document.documentElement.classList.remove('light');
    document.documentElement.classList.add('dark');
  });
  await expect(page.locator('html')).toHaveClass(/dark/);

  const imageAction = page.getByRole('button', { name: '复制为图片' });
  const exportRoot = page.locator('[data-document-export-root]');
  await expect(imageAction).toBeVisible();
  await expect(
    page.getByLabel('文档元数据').getByRole('heading', { name: '路由验证' }),
  ).toBeVisible();
  await expect(
    exportRoot.locator('[data-document-header-atmosphere]'),
  ).toHaveCount(1);
  await expect(imageAction.locator('xpath=..')).toHaveAttribute(
    'data-document-export-exclude',
    'true',
  );
  await expect(page.getByRole('button', { name: '返回文档导航' })).toHaveCount(
    0,
  );
  await expect(
    page.getByRole('region', { name: '普通附件', exact: true }),
  ).not.toHaveAttribute('data-document-export-exclude');

  await imageAction.click();
  await expect(imageAction).toHaveAttribute('title', '已复制为图片');
  const copiedImage = await page.evaluate(async () => {
    const content = document.querySelector<HTMLElement>(
      '[data-document-export-content]',
    );
    const attachments = content?.querySelector<HTMLElement>(
      '[aria-label="普通附件"]',
    );
    const item = (await navigator.clipboard.read())[0];
    if (!content || !attachments || !item) {
      throw new Error('Expected a copied article image and attachments.');
    }
    const colorCanvas = document.createElement('canvas');
    colorCanvas.width = 1;
    colorCanvas.height = 1;
    const colorContext = colorCanvas.getContext('2d');
    if (!colorContext) throw new Error('Expected a color canvas.');
    colorContext.fillStyle = getComputedStyle(content).color;
    colorContext.fillRect(0, 0, 1, 1);
    const themedTextColor = colorContext.getImageData(0, 0, 1, 1).data;
    const png = await item.getType('image/png');
    const image = await createImageBitmap(png);
    const canvas = document.createElement('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    const context = canvas.getContext('2d');
    if (!context) throw new Error('Expected an image canvas.');
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    let themedTextPixels = 0;
    for (let index = 0; index < pixels.length; index += 4) {
      const red = pixels[index] ?? 0;
      const green = pixels[index + 1] ?? 0;
      const blue = pixels[index + 2] ?? 0;
      if (
        Math.abs(red - (themedTextColor[0] ?? 0)) <= 8 &&
        Math.abs(green - (themedTextColor[1] ?? 0)) <= 8 &&
        Math.abs(blue - (themedTextColor[2] ?? 0)) <= 8
      ) {
        themedTextPixels += 1;
      }
    }
    return {
      articleHeight: content.scrollHeight,
      articleWidth: content.clientWidth,
      attachmentHeight: Math.ceil(attachments.getBoundingClientRect().height),
      background: [pixels[0] ?? 0, pixels[1] ?? 0, pixels[2] ?? 0],
      height: image.height,
      themedTextPixels,
      width: image.width,
    };
  });
  expect(copiedImage.width).toBeGreaterThan(copiedImage.articleWidth);
  const exportPadding = copiedImage.width - copiedImage.articleWidth;
  expect(exportPadding).toBe(32);
  expect(copiedImage.attachmentHeight).toBeGreaterThan(0);
  expect(copiedImage.height).toBeGreaterThan(
    copiedImage.articleHeight + exportPadding - copiedImage.attachmentHeight,
  );
  expect(copiedImage.themedTextPixels).toBeGreaterThan(1_000);
  expect(copiedImage.background.every((channel) => channel < 100)).toBe(true);
});

test('loads every gallery image and isolates image data between article exports', async ({
  context,
  page,
}) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await page.goto('/guide');
  const originalGalleryAssets = new Set<string>();
  page.on('response', (response) => {
    const url = new URL(response.url());
    const path = url.searchParams.get('path');
    if (
      url.pathname === '/api/assets' &&
      !url.searchParams.has('thumbnail') &&
      (path === 'guide/gallery.svg' || path === 'guide/portrait.svg')
    ) {
      originalGalleryAssets.add(path);
    }
  });

  const imageAction = page.getByRole('button', { name: '复制为图片' });
  await imageAction.click();
  await expect(
    page.getByRole('status', { exact: true, name: '正在生成图片' }),
  ).toBeVisible();
  const pendingAction = page.getByRole('button', { name: '正在生成图片' });
  await expect(pendingAction).toBeVisible();
  await expect(pendingAction).toHaveAttribute('aria-busy', 'true');
  const referenceAction = page.getByRole('button', { name: '复制文章引用' });
  await expect(referenceAction).toBeEnabled();
  await expect(imageAction).toHaveAttribute('title', '已复制为图片');
  await expect(imageAction).toBeEnabled();
  const galleryColor = await copiedImageColorStats(page, [37, 99, 235]);
  expect(galleryColor.pixels).toBeGreaterThan(1_000);
  expect(galleryColor.components).toBeGreaterThanOrEqual(15);
  expect(originalGalleryAssets).toEqual(
    new Set(['guide/gallery.svg', 'guide/portrait.svg']),
  );

  await page.getByRole('link', { exact: true, name: '单图导出' }).click();
  await expect(page).toHaveURL('/single-image');
  await expect(
    page.getByRole('button', { name: '查看原图：独立导出图片' }),
  ).toBeVisible();

  await imageAction.click();
  await expect(imageAction).toHaveAttribute('title', '已复制为图片');
  await expect(imageAction).toBeEnabled();
  expect(
    (await copiedImageColorStats(page, [0, 230, 118])).pixels,
  ).toBeGreaterThan(10_000);
});

test('keeps the latest sidebar title while navigating after a file event', async ({
  page,
}) => {
  const original = await readFile(guideFixturePath, 'utf8');
  const renamed = original.replace('title: 路由验证', 'title: 事件重命名');

  try {
    await page.goto('/');
    await page.reload();
    await expect(
      page.getByRole('link', { name: '路由验证', exact: true }),
    ).toBeVisible();

    await writeFile(guideFixturePath, renamed);
    const renamedLink = page.getByRole('link', {
      name: '事件重命名',
      exact: true,
    });
    await expect(renamedLink).toBeVisible();

    await page.evaluate(() => {
      const navigation = document.querySelector('nav[aria-label="文档树"]');
      if (!navigation) throw new Error('Expected the document tree.');
      const titles = {
        document: [] as string[],
        sidebar: [navigation.textContent ?? ''],
      };
      new MutationObserver(() => {
        titles.sidebar.push(navigation.textContent ?? '');
      }).observe(navigation, {
        characterData: true,
        childList: true,
        subtree: true,
      });
      (
        window as typeof window & {
          titleSnapshots?: { document: string[]; sidebar: string[] };
        }
      ).titleSnapshots = titles;
      new MutationObserver(() => {
        const documentTitle = document.querySelector(
          '[aria-label="文档元数据"] h1',
        )?.textContent;
        if (documentTitle) titles.document.push(documentTitle);
      }).observe(document.body, {
        characterData: true,
        childList: true,
        subtree: true,
      });
    });

    await renamedLink.click();
    await expect(page).toHaveURL(/\/guide$/);
    await expect(
      page
        .getByLabel('文档元数据')
        .getByRole('heading', { name: '事件重命名' }),
    ).toBeVisible();

    const titleSnapshots = await page.evaluate(
      () =>
        (
          window as typeof window & {
            titleSnapshots?: { document: string[]; sidebar: string[] };
          }
        ).titleSnapshots ?? { document: [], sidebar: [] },
    );
    expect(
      titleSnapshots.sidebar.some((title) => title.includes('路由验证')),
    ).toBe(false);
    expect(
      titleSnapshots.document.some((title) => title.includes('路由验证')),
    ).toBe(false);
  } finally {
    await writeFile(guideFixturePath, original);
  }
});

test('updates the currently open article after an external file change', async ({
  page,
}) => {
  const original = await readFile(guideFixturePath, 'utf8');
  const updated = original
    .replace('title: 路由验证', 'title: 当前文章已更新')
    .replace('此文章用于 Playwright 路由验收。', '外部文件变更已经展示。');

  try {
    await page.goto('/guide');
    await expect(
      page.getByLabel('文档元数据').getByRole('heading', {
        name: '路由验证',
      }),
    ).toBeVisible();

    await writeFile(guideFixturePath, updated);

    await expect(
      page.getByLabel('文档元数据').getByRole('heading', {
        name: '当前文章已更新',
      }),
    ).toBeVisible();
    await expect(page.getByText('外部文件变更已经展示。')).toBeVisible();
  } finally {
    await writeFile(guideFixturePath, original);
  }
});

test('does not cache dynamic document responses', async ({ page }) => {
  const documentResponse = await page.goto('/guide');
  const dataResponsePromise = page.waitForResponse(
    (response) => new URL(response.url()).pathname === '/canvas.data',
  );
  await page.getByRole('link', { exact: true, name: '画布' }).click();
  const dataResponse = await dataResponsePromise;

  expect(documentResponse?.headers()['cache-control']).toBe(
    'private, no-store',
  );
  expect(dataResponse.headers()['cache-control']).toBe('private, no-store');
});

test('searches without reloading the workspace or interrupting events', async ({
  page,
}) => {
  await page.goto('/');
  const eventStatus = page.locator('aside header [role="status"]');
  await expect(eventStatus).not.toContainText('正在连接实时事件');

  await page.evaluate(() => {
    const sidebar = document.querySelector('aside');
    if (!sidebar) throw new Error('Expected the workspace sidebar.');
    const searchStatuses: string[] = [];
    const eventStatus = document.querySelector('aside header [role="status"]');
    if (!eventStatus) throw new Error('Expected the graph event status.');
    new MutationObserver(() => {
      searchStatuses.push(eventStatus.textContent ?? '');
    }).observe(eventStatus, {
      characterData: true,
      childList: true,
      subtree: true,
    });
    (
      window as typeof window & {
        searchTestState?: {
          sidebar: Element;
          eventStatuses: string[];
        };
      }
    ).searchTestState = {
      sidebar,
      eventStatuses: searchStatuses,
    };
  });

  await page.getByRole('textbox', { name: '搜索文档' }).fill('路由验证');
  await page.getByRole('textbox', { name: '搜索文档' }).press('Enter');

  await expect(page).toHaveURL('/?q=%E8%B7%AF%E7%94%B1%E9%AA%8C%E8%AF%81');
  await expect(
    page.getByRole('link', { name: '路由验证', exact: true }),
  ).toBeVisible();

  const workspaceState = await page.evaluate(() => {
    const state = (
      window as typeof window & {
        searchTestState?: {
          sidebar: Element;
          eventStatuses: string[];
        };
      }
    ).searchTestState;
    return {
      sidebarPreserved: state?.sidebar === document.querySelector('aside'),
      eventStatuses: state?.eventStatuses ?? [],
    };
  });
  expect(workspaceState.sidebarPreserved).toBe(true);
  expect(
    workspaceState.eventStatuses.some((status) =>
      status.includes('连接中断，正在重试'),
    ),
  ).toBe(false);
});

test('preserves settings, graph events, manifest, and MCP server routes', async ({
  page,
  request,
}) => {
  let graphEventsResponse:
    | { contentType: string | undefined; status: number }
    | undefined;

  page.on('response', (response) => {
    const path = new URL(response.url()).pathname;
    if (path === '/api/graph-events') {
      graphEventsResponse = {
        contentType: response.headers()['content-type'],
        status: response.status(),
      };
    }
  });

  await page.goto('/');
  await expect(
    page.getByRole('complementary', { name: '文档导航' }),
  ).toBeVisible();

  const libraryResponse = await request.get('/api/library');
  expect(libraryResponse.status()).toBe(200);

  const settingsResponse = await request.get('/api/settings');
  expect(settingsResponse.status()).toBe(200);

  const graphApiResponse = await request.get('/api/library/graph');
  expect(graphApiResponse.status()).toBe(200);

  await page.goto('/graph');
  await expect.poll(() => graphEventsResponse?.status).toBe(200);
  expect(graphEventsResponse?.contentType).toContain('text/event-stream');

  const manifestResponse = await request.get('/manifest');
  expect(manifestResponse.status()).toBe(200);
  expect(manifestResponse.headers()['content-type']).toContain(
    'application/manifest+json',
  );
  expect((await manifestResponse.json()).start_url).toBe('/');

  const mcpResponse = await request.post('/mcp', {
    data: {
      jsonrpc: '2.0',
      id: 1,
      method: 'initialize',
      params: {
        protocolVersion: '2025-11-25',
        capabilities: {},
        clientInfo: {
          name: 'asterweft-routing-e2e',
          version: '1.0.0',
        },
      },
    },
    headers: {
      accept: 'application/json, text/event-stream',
      'content-type': 'application/json',
    },
  });

  expect([200, 404]).toContain(mcpResponse.status());
  if (mcpResponse.status() === 200) {
    expect(await mcpResponse.text()).toContain('"jsonrpc":"2.0"');
  } else {
    expect(await mcpResponse.text()).toBe('Not Found');
  }
});
