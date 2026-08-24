import { mkdir, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { expect, type Locator, type Page, test } from '@playwright/test';

const nestedFixturePath = resolve(
  import.meta.dirname,
  '../fixtures/library/live-nested',
);

const secondNestedFixturePath = resolve(
  import.meta.dirname,
  '../fixtures/library/live-sibling',
);
async function expectControlHeight(locator: Locator, height: number) {
  await expect
    .poll(async () => {
      const bounds = await locator.boundingBox();
      return bounds ? Math.round(bounds.height) : null;
    })
    .toBe(height);
}

async function expectSquareControl(locator: Locator, size: number) {
  await expect
    .poll(async () => {
      const bounds = await locator.boundingBox();
      return bounds
        ? [Math.round(bounds.width), Math.round(bounds.height)]
        : null;
    })
    .toEqual([size, size]);
}

async function expectNoPageOverflow(page: import('@playwright/test').Page) {
  const dimensions = await page.evaluate(() => ({
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
  expect(dimensions.scrollWidth).toBe(dimensions.clientWidth);
}

async function expectNoHorizontalOverflow(locator: Locator) {
  await expect(locator).toHaveCSS('overflow-x', 'hidden');
  await expect
    .poll(() =>
      locator.evaluate((element) => element.scrollWidth <= element.clientWidth),
    )
    .toBe(true);
}

async function swipeFromLeftEdge(page: Page) {
  const viewport = page.viewportSize();
  const y = Math.round((viewport?.height ?? 800) / 2);
  await page.mouse.move(4, y);
  await page.mouse.down();
  await page.mouse.move(64, y + 4, { steps: 6 });
  await page.mouse.up();
}

async function expectDenseDocumentTypography(page: Page) {
  const title = page.locator('[data-document-export-title]');
  await expect(title).toHaveCSS('font-size', '20px');
  await expect(title).toHaveCSS('line-height', '28px');
  await expect(title).toHaveCSS('font-weight', '500');

  const typeset = page.locator('.typeset-document');
  await expect(typeset).toHaveCSS('font-size', '14px');
  await expect(typeset).toHaveCSS('line-height', '22.4px');
  await expect(typeset).toHaveCSS('font-weight', '400');

  await typeset.evaluate((element) => {
    const probe = document.createElement('div');
    probe.dataset.typographyProbe = '';
    for (let level = 1; level <= 6; level += 1) {
      const heading = document.createElement(`h${level}`);
      heading.dataset.headingLevel = String(level);
      heading.textContent = `标题 ${level}`;
      probe.append(heading);
    }
    element.append(probe);
  });

  const headingStyles = [
    ['20px', '28px'],
    ['18px', '28px'],
    ['16px', '24px'],
    ['14px', '22px'],
    ['14px', '22px'],
    ['14px', '22px'],
  ] as const;
  for (const [index, [fontSize, lineHeight]] of headingStyles.entries()) {
    const heading = typeset.locator(`[data-heading-level="${index + 1}"]`);
    await expect(heading).toHaveCSS('font-size', fontSize);
    await expect(heading).toHaveCSS('line-height', lineHeight);
    await expect(heading).toHaveCSS('font-weight', '500');
  }

  const h6 = typeset.locator('[data-heading-level="6"]');
  await expect(h6).toHaveCSS('letter-spacing', 'normal');
  await expect(h6).toHaveCSS('text-transform', 'none');
}

test('uses the same dense document typography on small and desktop viewports', async ({
  page,
}) => {
  for (const viewport of [
    { width: 390, height: 844 },
    { width: 1280, height: 800 },
  ]) {
    await page.setViewportSize(viewport);
    await page.goto('/guide');
    await expectDenseDocumentTypography(page);
  }
});

test('only enables document tree sorting at desktop widths', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');

  const documentLink = page.getByRole('link', {
    exact: true,
    name: '路由验证',
  });
  await expect(documentLink).not.toHaveAttribute('aria-roledescription');

  await page.setViewportSize({ width: 768, height: 844 });
  await expect(documentLink).toHaveAttribute(
    'aria-roledescription',
    '可排序笔记',
  );

  await page.setViewportSize({ width: 767, height: 844 });
  await expect(documentLink).not.toHaveAttribute('aria-roledescription');
});

test('uses primary and secondary workspace panes below 768px', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');

  const primaryPane = page.locator('[data-workspace-mobile-pane="primary"]');
  const secondaryPane = page.locator(
    '[data-workspace-mobile-pane="secondary"]',
  );
  const homeLink = primaryPane.getByRole('link', {
    exact: true,
    name: 'Asterweft 首页',
  });
  const settingsButton = primaryPane.getByRole('button', { name: '设置' });
  const search = primaryPane.getByRole('textbox', { name: '搜索文档' });
  const documentTree = primaryPane.getByRole('navigation', {
    name: '文档树',
  });
  const graphLink = primaryPane.getByRole('link', {
    exact: true,
    name: '知识图谱',
  });

  await expect(primaryPane).toBeVisible();
  await expect(secondaryPane).toBeHidden();
  await expect(primaryPane).toHaveCSS('padding', '8px');
  await expect(primaryPane).toHaveCSS('border-radius', '0px');
  await expect(primaryPane).toHaveCSS('border-width', '0px');
  await expect(primaryPane).toHaveCSS('box-shadow', 'none');
  const primaryBounds = await primaryPane.boundingBox();
  expect(primaryBounds).toMatchObject({ x: 0, y: 0, width: 390, height: 844 });
  await expect(
    page.getByRole('separator', {
      name: '调整文档侧栏宽度',
      includeHidden: true,
    }),
  ).toBeHidden();
  await expectControlHeight(homeLink, 36);
  await expectSquareControl(graphLink, 36);
  await expectSquareControl(settingsButton, 36);
  await expectControlHeight(search.locator('..'), 32);
  await expectNoHorizontalOverflow(documentTree);

  await search.fill('路由验证');
  await search.press('Enter');
  await expect(page).toHaveURL('/?q=%E8%B7%AF%E7%94%B1%E9%AA%8C%E8%AF%81');

  const documentLink = primaryPane.getByRole('link', {
    exact: true,
    name: '路由验证',
  });
  await expectControlHeight(documentLink, 36);
  await expect(documentLink).toHaveCSS('font-size', '14px');
  await expect(documentLink).toHaveCSS('font-weight', '400');
  const brandTitle = homeLink.getByText('Asterweft');
  await expect(brandTitle).toHaveCSS('font-size', '14px');
  await expect(brandTitle).toHaveCSS('line-height', '20px');
  await expect(brandTitle).toHaveCSS('font-weight', '500');
  const connectionStatus = homeLink.locator('[role="status"]');
  await expect(connectionStatus).toHaveCSS('font-size', '12px');
  await expect(connectionStatus).toHaveCSS('line-height', '16px');
  await expect(connectionStatus).toHaveCSS('font-weight', '400');
  await documentLink.click();

  await expect(page).toHaveURL('/guide?q=%E8%B7%AF%E7%94%B1%E9%AA%8C%E8%AF%81');
  await expect(primaryPane).toBeHidden();
  await expect(secondaryPane).toBeVisible();
  await expect(page.getByRole('button', { name: '返回文档导航' })).toHaveCount(
    0,
  );
  const edgeSwipe = page.getByRole('button', { name: '展开文档导航' });
  await expect(edgeSwipe).toBeVisible();
  await expect(edgeSwipe).toHaveCSS('width', '10px');
  const phoneMedia = await page.evaluate(
    () =>
      window.matchMedia(
        '(max-width: 767px) and (hover: none) and (pointer: coarse)',
      ).matches,
  );
  await expectSquareControl(
    page.getByRole('button', {
      name: phoneMedia ? '下载图片' : '复制为图片',
    }),
    32,
  );
  await expect(
    page.getByRole('button', {
      name: phoneMedia ? '复制为图片' : '下载图片',
      includeHidden: true,
    }),
  ).toBeHidden();
  await expect(
    page.getByRole('button', { name: '是否展示文章差异', includeHidden: true }),
  ).toBeHidden();
  await expect(page.locator('#main-content')).toHaveCSS(
    'border-left-width',
    '0px',
  );
  await expect(page.locator('.typeset-document')).toHaveCSS(
    'font-size',
    '14px',
  );
  await expect(page.locator('.typeset-document')).toHaveCSS(
    'font-weight',
    '400',
  );
  expect(
    await page
      .locator('.typeset-document')
      .evaluate((element) => getComputedStyle(element).fontFamily),
  ).toContain('Geist Variable');
  const articleBounds = await page
    .locator('[data-document-export-content]')
    .boundingBox();
  expect(articleBounds?.x).toBe(16);
  expect((articleBounds?.x ?? 0) + (articleBounds?.width ?? 0)).toBe(374);
  for (const block of [
    page.locator('[data-attachment-gallery]'),
    page.locator('figure.code-block'),
    page.locator('table'),
  ]) {
    const bounds = await block.boundingBox();
    expect(bounds?.x).toBe(16);
    expect((bounds?.x ?? 0) + (bounds?.width ?? 0)).toBe(374);
  }
  await expectNoPageOverflow(page);

  await swipeFromLeftEdge(page);
  await expect(page).toHaveURL('/guide?q=%E8%B7%AF%E7%94%B1%E9%AA%8C%E8%AF%81');
  await expect(primaryPane).toBeVisible();
  await expect(search).toHaveValue('路由验证');
  await expect(documentLink).toBeFocused();

  await graphLink.click();
  await expect(page).toHaveURL('/graph?q=%E8%B7%AF%E7%94%B1%E9%AA%8C%E8%AF%81');
  await expect(primaryPane).toBeHidden();
  await expect(secondaryPane).toBeVisible();
  const graphSettings = page.getByRole('button', { name: '知识图谱设置' });
  await expectSquareControl(graphSettings, 32);
  const graphSettingsBefore = await graphSettings.boundingBox();
  expect(
    (graphSettingsBefore?.x ?? 0) + (graphSettingsBefore?.width ?? 0),
  ).toBe(382);
  await page.locator('canvas').dispatchEvent('wheel', {
    deltaMode: 0,
    deltaX: 12,
    deltaY: 24,
  });
  expect(await graphSettings.boundingBox()).toEqual(graphSettingsBefore);
  await expectNoPageOverflow(page);

  await swipeFromLeftEdge(page);
  await expect(primaryPane).toBeVisible();
  await expect(secondaryPane).toBeHidden();
});

test('uses settings categories as the small-screen primary pane', async ({
  page,
}) => {
  await page.setViewportSize({ width: 430, height: 932 });
  await page.goto('/');
  await page.getByRole('button', { exact: true, name: '设置' }).click();

  const dialog = page.getByRole('dialog');
  const primaryPane = dialog.locator('[data-settings-mobile-pane="primary"]');
  const secondaryPane = dialog.locator(
    '[data-settings-mobile-pane="secondary"]',
  );
  await expect(primaryPane).toBeVisible();
  await expect(secondaryPane).toBeHidden();

  const graphCategory = primaryPane.getByRole('button', {
    exact: true,
    name: '知识图谱',
  });
  await expectControlHeight(graphCategory, 28);
  await expect(graphCategory).toHaveCSS('font-weight', '400');
  await expectSquareControl(
    primaryPane.getByRole('button', { name: '关闭' }),
    32,
  );
  await expectControlHeight(
    primaryPane.getByRole('textbox', { name: '搜索设置' }).locator('..'),
    32,
  );
  await graphCategory.click();

  await expect(primaryPane).toBeHidden();
  await expect(secondaryPane).toBeVisible();
  const settingsBack = secondaryPane.getByRole('button', {
    name: '返回设置分类',
  });
  await expectSquareControl(settingsBack, 32);
  await expectSquareControl(
    secondaryPane.getByRole('button', { name: '关闭' }),
    32,
  );
  await expect(
    secondaryPane.getByRole('heading', { exact: true, name: '知识图谱' }),
  ).toBeVisible();
  await expectNoPageOverflow(page);

  await settingsBack.click();
  await expect(primaryPane).toBeVisible();
  await expect(secondaryPane).toBeHidden();
  await expect(graphCategory).toBeFocused();
});

test('keeps desktop split panes at and above 768px', async ({
  page,
  request,
}) => {
  await page.setViewportSize({ width: 768, height: 1024 });
  await page.goto('/');

  await expect(
    page.locator('[data-workspace-mobile-pane="primary"]'),
  ).toBeVisible();
  await expect(
    page.locator('[data-workspace-mobile-pane="secondary"]'),
  ).toBeVisible();
  await expect(page.getByRole('button', { name: '返回文档导航' })).toHaveCount(
    0,
  );
  await expect(
    page.getByRole('button', { name: '展开文档导航', includeHidden: true }),
  ).toBeHidden();

  const documentLink = page.getByRole('link', {
    exact: true,
    name: '路由验证',
  });
  const documentRow = documentLink.locator('..');
  await documentLink.click();
  await expect(documentRow).toHaveAttribute('data-selected', '');
  await expect(documentLink).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  expect(
    await documentRow.evaluate(
      (element) => getComputedStyle(element).backgroundColor,
    ),
  ).not.toBe('rgba(0, 0, 0, 0)');

  await page.getByRole('link', { name: 'Asterweft 首页' }).click();
  await expect(documentRow).not.toHaveAttribute('data-selected', '');
  await expect(documentRow).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');

  await page.getByRole('button', { exact: true, name: '设置' }).click();
  const dialog = page.getByRole('dialog');
  await expect(
    dialog.locator('[data-settings-mobile-pane="primary"]'),
  ).toBeVisible();
  await expect(
    dialog.locator('[data-settings-mobile-pane="secondary"]'),
  ).toBeVisible();
  await expect(
    dialog.getByRole('button', { name: '返回设置分类' }),
  ).toBeHidden();
  await expectNoPageOverflow(page);
  await dialog.getByRole('button', { name: '关闭' }).click();

  await page.setViewportSize({ width: 1280, height: 800 });
  await expectNoPageOverflow(page);

  const sidebar = page.getByRole('complementary', { name: '文档导航' });
  const documentTree = sidebar.getByRole('navigation', { name: '文档树' });
  await expectNoHorizontalOverflow(documentTree);
  await documentTree.evaluate((element) => {
    const overflowProbe = document.createElement('div');
    overflowProbe.dataset.scrollOverflowProbe = '';
    overflowProbe.style.height = '1200px';
    element.append(overflowProbe);
  });
  expect(
    await documentTree.evaluate((element) => ({
      overflowY: getComputedStyle(element).overflowY,
      scrollable: element.scrollHeight > element.clientHeight,
    })),
  ).toEqual({ overflowY: 'auto', scrollable: true });
  await documentTree.evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  expect(
    await documentTree.evaluate((element) => element.scrollTop),
  ).toBeGreaterThan(0);
  await documentTree
    .locator('[data-scroll-overflow-probe]')
    .evaluate((element) => element.remove());
  const resizeHandle = page.getByRole('separator', {
    name: '调整文档侧栏宽度',
  });
  await expect(resizeHandle).toBeVisible();
  const idleHandleColor = await resizeHandle.evaluate(
    (element) => getComputedStyle(element, '::after').backgroundColor,
  );
  await resizeHandle.hover();
  expect(
    await resizeHandle.evaluate(
      (element) => getComputedStyle(element, '::after').backgroundColor,
    ),
  ).toBe(idleHandleColor);
  const initialSidebar = await sidebar.boundingBox();
  const handle = await resizeHandle.boundingBox();
  expect(initialSidebar).not.toBeNull();
  expect(handle).not.toBeNull();

  try {
    const minimumSaved = page.waitForResponse(
      (response) =>
        response.url().endsWith('/api/settings') &&
        response.request().method() === 'PATCH',
    );
    await resizeHandle.focus();
    await resizeHandle.press('ArrowLeft');
    await minimumSaved;
    await expect
      .poll(async () => (await sidebar.boundingBox())?.width)
      .toBe(initialSidebar?.width);

    const settingsSaved = page.waitForResponse(
      (response) =>
        response.url().endsWith('/api/settings') &&
        response.request().method() === 'PATCH',
    );
    const handleY = (handle?.y ?? 0) + (handle?.height ?? 0) / 2;
    await resizeHandle.hover();
    await page.mouse.down();
    await page.mouse.move(
      (handle?.x ?? 0) + (handle?.width ?? 0) / 2 + 60,
      handleY,
    );
    await page.mouse.up();
    await settingsSaved;

    await expect
      .poll(async () => (await sidebar.boundingBox())?.width)
      .toBe((initialSidebar?.width ?? 0) + 60);
    await page.reload();
    await expect
      .poll(async () => (await sidebar.boundingBox())?.width)
      .toBe((initialSidebar?.width ?? 0) + 60);
  } finally {
    await request.patch('/api/settings', {
      data: { workspaceSidebarWidth: 284 },
    });
  }
});

test('hides phone-only actions from display and focus', async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== 'mobile-chromium');

  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/guide');
  expect(
    await page.evaluate(
      () =>
        window.matchMedia(
          '(max-width: 767px) and (hover: none) and (pointer: coarse)',
        ).matches,
    ),
  ).toBe(true);

  await expect(
    page.getByRole('button', { name: '复制文章引用', includeHidden: true }),
  ).toBeHidden();
  await expect(page.getByRole('button', { name: '下载图片' })).toBeVisible();
  await expect(
    page.getByRole('button', { name: '复制为图片', includeHidden: true }),
  ).toBeHidden();
  await expect(
    page.getByRole('button', {
      name: '是否展示文章差异',
      includeHidden: true,
    }),
  ).toBeHidden();

  const downloadStarted = page.waitForEvent('download');
  await page.getByRole('button', { name: '下载图片' }).click();
  const download = await downloadStarted;
  expect(download.suggestedFilename()).toBe('路由验证.png');
});

test('expands collapsed parent nodes and views expanded or leaf nodes on mobile', async ({
  page,
}) => {
  await rm(nestedFixturePath, { force: true, recursive: true });

  try {
    await mkdir(resolve(nestedFixturePath, 'alpha'), { recursive: true });
    await writeFile(
      resolve(nestedFixturePath, 'index.md'),
      '---\ntitle: 实时嵌套\n---\n父级正文',
    );
    await writeFile(
      resolve(nestedFixturePath, 'alpha/index.md'),
      '---\ntitle: 嵌套甲\n---\n甲正文',
    );

    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/');

    const primaryPane = page.locator('[data-workspace-mobile-pane="primary"]');
    const parentLink = primaryPane.getByRole('link', {
      exact: true,
      name: '实时嵌套',
    });
    const alphaLink = primaryPane.getByRole('link', {
      exact: true,
      name: '嵌套甲',
    });
    await expect(alphaLink).toBeHidden();

    await parentLink.click();
    await expect(page).toHaveURL('/');
    await expect(primaryPane).toBeVisible();
    await expect(alphaLink).toBeVisible();

    await parentLink.click();
    await expect(page).toHaveURL('/live-nested');
    await expect(primaryPane).toBeHidden();

    await swipeFromLeftEdge(page);
    await expect(primaryPane).toBeVisible();
    await expect(alphaLink).toBeVisible();
  } finally {
    await rm(nestedFixturePath, { force: true, recursive: true });
  }
});

test('collapses sibling branches on mobile like desktop when expanding a node', async ({
  page,
}) => {
  await rm(nestedFixturePath, { force: true, recursive: true });
  await rm(secondNestedFixturePath, { force: true, recursive: true });

  try {
    await mkdir(resolve(nestedFixturePath, 'alpha'), { recursive: true });
    await writeFile(
      resolve(nestedFixturePath, 'index.md'),
      '---\ntitle: 实时嵌套\n---\n父级正文',
    );
    await writeFile(
      resolve(nestedFixturePath, 'alpha/index.md'),
      '---\ntitle: 嵌套甲\n---\n甲正文',
    );
    await mkdir(resolve(secondNestedFixturePath, 'beta'), { recursive: true });
    await writeFile(
      resolve(secondNestedFixturePath, 'index.md'),
      '---\ntitle: 实时同级\n---\n同级正文',
    );
    await writeFile(
      resolve(secondNestedFixturePath, 'beta/index.md'),
      '---\ntitle: 同级乙\n---\n乙正文',
    );

    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/');

    const primaryPane = page.locator('[data-workspace-mobile-pane="primary"]');
    const parentLink = primaryPane.getByRole('link', {
      exact: true,
      name: '实时嵌套',
    });
    const alphaLink = primaryPane.getByRole('link', {
      exact: true,
      name: '嵌套甲',
    });
    const siblingParentLink = primaryPane.getByRole('link', {
      exact: true,
      name: '实时同级',
    });
    const siblingChildLink = primaryPane.getByRole('link', {
      exact: true,
      name: '同级乙',
    });

    await parentLink.click();
    await expect(alphaLink).toBeVisible();
    await siblingParentLink.click();
    await expect(siblingChildLink).toBeVisible();
    await expect(alphaLink).toBeHidden();
    await expect(page).toHaveURL('/');

    await parentLink.click();
    await expect(alphaLink).toBeVisible();
    await expect(siblingChildLink).toBeHidden();

    await alphaLink.click();
    await expect(page).toHaveURL('/live-nested/alpha');

    await swipeFromLeftEdge(page);
    await expect(primaryPane).toBeVisible();
    await expect(alphaLink).toHaveAttribute('aria-current', 'page');
    await expect(siblingParentLink).toBeVisible();
  } finally {
    await rm(nestedFixturePath, { force: true, recursive: true });
    await rm(secondNestedFixturePath, { force: true, recursive: true });
  }
});
