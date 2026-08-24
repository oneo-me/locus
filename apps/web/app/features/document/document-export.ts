export const documentExportRootSelector = '[data-document-export-root]';
export const documentExportExcludeAttribute = 'data-document-export-exclude';
export const documentExportContentAttribute = 'data-document-export-content';

const maximumCanvasDimension = 16_384;
const maximumCanvasArea = 16_777_216;
const documentImageExportPadding = 16;
const documentImageExportTimeout = 30_000;
const documentExportRenderTimeout = 15_000;
const readyDocumentRenderSelector = '[data-document-content-status="ready"]';
const renderedDocumentSelector = '.comark-content';

function pixelValue(value: string): number {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function documentExportContentWidth(
  layoutWidth: number,
  paddingLeft: string,
  paddingRight: string,
): number {
  return Math.max(
    1,
    layoutWidth - pixelValue(paddingLeft) - pixelValue(paddingRight),
  );
}

function freezeDocumentExportMotion(element: HTMLElement) {
  for (const candidate of [
    element,
    ...element.querySelectorAll<HTMLElement>('*'),
  ]) {
    candidate.style.setProperty('animation', 'none', 'important');
    candidate.style.setProperty('transition', 'none', 'important');
  }
}

const transparentExportImage =
  'data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=';

function blobDataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(reader.error ?? new Error('无法读取图片。'));
    reader.onload = () => {
      if (typeof reader.result === 'string') resolve(reader.result);
      else reject(new Error('无法读取图片。'));
    };
    reader.readAsDataURL(blob);
  });
}

interface DocumentExportImage {
  clip?: {
    height: number;
    radius: number;
    width: number;
    x: number;
    y: number;
  };
  height: number;
  objectFit: string;
  objectPosition: string;
  source: HTMLImageElement;
  sourceSvg?: string;
  width: number;
  x: number;
  y: number;
}

interface DocumentExportShadowLayer {
  blur: number;
  color: string;
  offsetX: number;
  offsetY: number;
  spread: number;
}

function imageClip(
  image: HTMLImageElement,
  root: HTMLElement,
  rootBounds: DOMRect,
): DocumentExportImage['clip'] {
  const imageBounds = image.getBoundingClientRect();
  for (let candidate: HTMLElement | null = image; candidate;) {
    const style = getComputedStyle(candidate);
    const radius = Math.max(
      pixelValue(style.borderTopLeftRadius),
      pixelValue(style.borderTopRightRadius),
      pixelValue(style.borderBottomRightRadius),
      pixelValue(style.borderBottomLeftRadius),
    );
    const clips =
      candidate === image ||
      ['hidden', 'clip'].includes(style.overflowX) ||
      ['hidden', 'clip'].includes(style.overflowY);
    if (clips && radius > 0) {
      const bounds = candidate.getBoundingClientRect();
      const borderLeft = pixelValue(style.borderLeftWidth);
      const borderRight = pixelValue(style.borderRightWidth);
      const borderTop = pixelValue(style.borderTopWidth);
      const borderBottom = pixelValue(style.borderBottomWidth);
      const left = Math.max(imageBounds.left, bounds.left + borderLeft);
      const right = Math.min(imageBounds.right, bounds.right - borderRight);
      const top = Math.max(imageBounds.top, bounds.top + borderTop);
      const bottom = Math.min(imageBounds.bottom, bounds.bottom - borderBottom);
      return {
        height: Math.max(0, bottom - top),
        radius: Math.max(
          0,
          radius - Math.max(borderLeft, borderRight, borderTop, borderBottom),
        ),
        width: Math.max(0, right - left),
        x: left - rootBounds.left,
        y: top - rootBounds.top,
      };
    }
    if (candidate === root) break;
    candidate = candidate.parentElement;
  }
  return undefined;
}

async function prepareDocumentExportImages(
  element: HTMLElement,
  timeout: number,
): Promise<{ cleanup: () => void; images: DocumentExportImage[] }> {
  const images = [...element.querySelectorAll('img')];
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);
  const objectUrls = new Set<string>();
  const sources = new Map<
    string,
    Promise<{ image: HTMLImageElement; svg?: string }>
  >();

  try {
    await Promise.all([
      globalThis.document?.fonts?.ready,
      ...images.map(async (image) => {
        image.loading = 'eager';
        const source =
          image.dataset.documentExportImageSource ||
          image.currentSrc ||
          image.src;
        if (!source) throw new Error('导出图片缺少来源。');

        let pending = sources.get(source);
        if (!pending) {
          pending = fetch(source, {
            cache: 'no-store',
            credentials: 'same-origin',
            signal: controller.signal,
          }).then(
            async (
              response,
            ): Promise<{ image: HTMLImageElement; svg?: string }> => {
              if (!response.ok) {
                throw new Error(`无法加载导出图片：${response.status}`);
              }
              const blob = await response.blob();
              const isSvg = blob.type.includes('image/svg+xml');
              const imageUrl = isSvg
                ? await blobDataUrl(blob)
                : URL.createObjectURL(blob);
              if (imageUrl.startsWith('blob:')) objectUrls.add(imageUrl);
              const loaded = element.ownerDocument.createElement('img');
              loaded.decoding = 'async';
              loaded.src = imageUrl;
              await loaded.decode();
              return {
                image: loaded,
                svg: isSvg ? await blob.text() : undefined,
              };
            },
          );
          sources.set(source, pending);
        }
        await pending;
      }),
    ]);

    const loadedSources = new Map(
      await Promise.all(
        [...sources].map(
          async ([source, pending]) => [source, await pending] as const,
        ),
      ),
    );
    const rootBounds = element.getBoundingClientRect();
    const preparedImages = images.map((image): DocumentExportImage => {
      const source =
        image.dataset.documentExportImageSource ||
        image.currentSrc ||
        image.src;
      const loaded = loadedSources.get(source);
      if (!loaded) throw new Error('导出图片尚未完成加载。');
      const bounds = image.getBoundingClientRect();
      const style = getComputedStyle(image);
      Object.assign(image.style, {
        height: `${bounds.height}px`,
        maxHeight: 'none',
        maxWidth: 'none',
        visibility: 'hidden',
        width: `${bounds.width}px`,
      });
      image.srcset = '';
      image.src = transparentExportImage;

      return {
        clip: imageClip(image, element, rootBounds),
        height: bounds.height,
        objectFit: style.objectFit,
        objectPosition: style.objectPosition,
        source: loaded.image,
        sourceSvg: loaded.svg,
        width: bounds.width,
        x: bounds.left - rootBounds.left,
        y: bounds.top - rootBounds.top,
      };
    });

    return {
      cleanup: () => {
        for (const objectUrl of objectUrls) URL.revokeObjectURL(objectUrl);
      },
      images: preparedImages,
    };
  } catch (error) {
    for (const objectUrl of objectUrls) URL.revokeObjectURL(objectUrl);
    if (controller.signal.aborted) {
      throw new Error('等待文章图片加载超时。');
    }
    throw error;
  } finally {
    clearTimeout(timeoutId);
    controller.abort();
  }
}

async function prepareHighResolutionSvgImages(
  images: DocumentExportImage[],
  pixelRatio: number,
) {
  await Promise.all(
    images.map(async (image) => {
      if (!image.sourceSvg) return;
      const document = new DOMParser().parseFromString(
        image.sourceSvg,
        'image/svg+xml',
      );
      const svg = document.documentElement;
      if (svg.localName !== 'svg' || !svg.hasAttribute('viewBox')) return;

      const scale = Math.max(
        1,
        (image.width * pixelRatio) / image.source.naturalWidth,
        (image.height * pixelRatio) / image.source.naturalHeight,
      );
      if (scale <= 1) return;
      svg.setAttribute(
        'width',
        `${Math.ceil(image.source.naturalWidth * scale)}`,
      );
      svg.setAttribute(
        'height',
        `${Math.ceil(image.source.naturalHeight * scale)}`,
      );
      const source = image.source.ownerDocument.createElement('img');
      source.decoding = 'async';
      source.src = await blobDataUrl(
        new Blob([new XMLSerializer().serializeToString(svg)], {
          type: 'image/svg+xml;charset=utf-8',
        }),
      );
      await source.decode();
      image.source = source;
    }),
  );
}

async function withDocumentExportTimeout<T>(
  promise: Promise<T>,
  timeout: number,
): Promise<T> {
  let timeoutId: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      promise,
      new Promise<T>((_, reject) => {
        timeoutId = setTimeout(
          () => reject(new Error('生成文章图片超时。')),
          timeout,
        );
      }),
    ]);
  } finally {
    clearTimeout(timeoutId);
  }
}

function sanitizeCanvasExportCss(value: string) {
  return value
    .replace(/@import[^;]+;/gi, '')
    .replace(
      /url\(\s*(?:'([^']*)'|"([^"]*)"|([^)]*))\s*\)/gi,
      (match, singleQuoted, doubleQuoted, unquoted) =>
        /^(?:data:|blob:)/i.test(
          String(singleQuoted ?? doubleQuoted ?? unquoted).trim(),
        )
          ? match
          : 'none',
    );
}

function sanitizeCanvasExportDocument(document: Document) {
  for (const blocked of document.querySelectorAll(
    'script,iframe,frame,object,embed,link,meta,base',
  )) {
    blocked.remove();
  }
  for (const element of document.querySelectorAll<HTMLElement>('*')) {
    for (let index = element.attributes.length - 1; index >= 0; index -= 1) {
      const attribute = element.attributes.item(index);
      if (!attribute) continue;
      const name = attribute.name.toLowerCase();
      const value = attribute.value.trim();
      if (
        name.startsWith('on') ||
        name === 'nonce' ||
        name === 'srcdoc' ||
        name === 'srcset' ||
        (['src', 'poster'].includes(name) &&
          !/^(?:data:|blob:)/i.test(value)) ||
        (['href', 'xlink:href'].includes(name) &&
          element.namespaceURI === 'http://www.w3.org/2000/svg' &&
          !/^(?:#|data:|blob:)/i.test(value))
      ) {
        element.removeAttribute(attribute.name);
      }
    }
    if (element.hasAttribute('style')) {
      element.setAttribute(
        'style',
        sanitizeCanvasExportCss(element.getAttribute('style') ?? ''),
      );
    }
  }
  for (const style of document.querySelectorAll('style')) {
    style.textContent = sanitizeCanvasExportCss(style.textContent ?? '');
  }
}

function splitCssList(value: string): string[] {
  const items: string[] = [];
  let depth = 0;
  let start = 0;
  for (let index = 0; index < value.length; index += 1) {
    const character = value[index];
    if (character === '(') depth += 1;
    else if (character === ')') depth = Math.max(0, depth - 1);
    else if (character === ',' && depth === 0) {
      items.push(value.slice(start, index).trim());
      start = index + 1;
    }
  }
  items.push(value.slice(start).trim());
  return items.filter(Boolean);
}

function documentExportShadowLayers(value: string) {
  return splitCssList(value).flatMap<DocumentExportShadowLayer>((layer) => {
    if (/\binset\b/i.test(layer)) return [];
    const color =
      layer.match(/(?:rgba?|hsla?|color)\([^)]*\)|#[\da-f]{3,8}/i)?.[0] ?? '';
    if (!color || /rgba?\([^)]*,\s*0(?:\.0+)?\s*\)$/i.test(color)) {
      return [];
    }
    const lengths = [
      ...layer.replace(color, '').matchAll(/-?(?:\d+\.?\d*|\.\d+)px/g),
    ].map(([length]) => pixelValue(length));
    const [offsetX, offsetY, blur = 0, spread = 0] = lengths;
    return offsetX === undefined || offsetY === undefined
      ? []
      : [{ blur, color, offsetX, offsetY, spread }];
  });
}

function nextDocumentExportPaint(): Promise<void> {
  return new Promise((resolve) =>
    requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
  );
}

function canvasExportContentHeight(document: Document): number {
  const rootTop = document.documentElement.getBoundingClientRect().top;
  let bottom = document.body.getBoundingClientRect().bottom;
  for (const candidate of document.body.querySelectorAll<HTMLElement>('*')) {
    const style = getComputedStyle(candidate);
    if (style.display === 'none') continue;
    bottom = Math.max(
      bottom,
      candidate.getBoundingClientRect().bottom + pixelValue(style.marginBottom),
    );
  }
  return Math.max(1, Math.ceil(bottom - rootTop));
}

function inlineCanvasExportStyles(
  sourceRoot: HTMLElement,
  destinationRoot: HTMLElement,
) {
  const sources = [
    sourceRoot,
    ...sourceRoot.querySelectorAll<HTMLElement>('*'),
  ];
  const destinations = [
    destinationRoot,
    ...destinationRoot.querySelectorAll<HTMLElement>('*'),
  ];
  for (let index = 0; index < sources.length; index += 1) {
    const source = sources[index];
    const destination = destinations[index];
    if (!source || !destination) continue;
    const style = getComputedStyle(source);
    for (
      let propertyIndex = 0;
      propertyIndex < style.length;
      propertyIndex += 1
    ) {
      const property = style.item(propertyIndex);
      destination.style.setProperty(property, style.getPropertyValue(property));
    }
  }
}

async function replaceCanvasFramesWithStaticDocuments(
  element: HTMLElement,
  timeout: number,
) {
  const containers = [
    ...element.querySelectorAll<HTMLElement>('[data-canvas-container]'),
  ];
  const surfaces: HTMLElement[] = [];
  const layoutFrames: HTMLIFrameElement[] = [];
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);
  try {
    await Promise.all(
      containers.map(async (container) => {
        const frame = container.querySelector<HTMLIFrameElement>(
          '[data-canvas-frame]',
        );
        const snapshot = container.querySelector<HTMLImageElement>(
          '[data-canvas-export-snapshot]',
        );
        if (!snapshot) throw new Error('画布缺少导出快照。');

        const response = await fetch(snapshot.src, {
          cache: 'no-store',
          signal: controller.signal,
        });
        if (!response.ok) {
          throw new Error(`无法加载画布快照：${response.status}`);
        }
        const document = new DOMParser().parseFromString(
          await response.text(),
          'image/svg+xml',
        );
        const svg = document.documentElement;
        const staticDocument =
          svg.querySelector('foreignObject')?.firstElementChild;
        const sourceWidth = pixelValue(svg.getAttribute('width') ?? '');
        const sourceHeight = pixelValue(svg.getAttribute('height') ?? '');
        if (!staticDocument || sourceWidth <= 0 || sourceHeight <= 0) {
          throw new Error('画布快照内容无效。');
        }
        sanitizeCanvasExportDocument(document);

        Object.assign(container.style, {
          inlineSize: '100%',
          marginInline: '0',
        });
        const targetWidth = container.clientWidth;
        const sourceRoot = staticDocument as HTMLElement;
        Object.assign(sourceRoot.style, {
          height: 'auto',
          width: '100%',
        });
        const layoutFrame = element.ownerDocument.createElement('iframe');
        layoutFrames.push(layoutFrame);
        layoutFrame.setAttribute('sandbox', 'allow-same-origin');
        layoutFrame.width = `${Math.ceil(targetWidth)}`;
        layoutFrame.height = `${Math.ceil(sourceHeight)}`;
        Object.assign(layoutFrame.style, {
          border: '0',
          height: `${sourceHeight}px`,
          left: '-100000px',
          pointerEvents: 'none',
          position: 'fixed',
          top: '0',
          visibility: 'hidden',
          width: `${targetWidth}px`,
        });
        const loaded = new Promise<void>((resolve, reject) => {
          layoutFrame.addEventListener('load', () => resolve(), { once: true });
          layoutFrame.addEventListener(
            'error',
            () => reject(new Error('无法建立画布导出布局。')),
            { once: true },
          );
        });
        layoutFrame.srcdoc =
          '<!doctype html><html><head></head><body></body></html>';
        element.ownerDocument.body.append(layoutFrame);
        await loaded;
        const layoutDocument = layoutFrame.contentDocument;
        if (!layoutDocument) throw new Error('无法读取画布导出布局。');
        layoutDocument.replaceChild(
          layoutDocument.importNode(sourceRoot, true),
          layoutDocument.documentElement,
        );
        await layoutDocument.fonts?.ready;
        await nextDocumentExportPaint();
        const targetHeight = canvasExportContentHeight(layoutDocument);
        layoutFrame.style.height = `${targetHeight}px`;
        layoutFrame.height = `${targetHeight}`;
        layoutDocument.documentElement.style.height = `${targetHeight}px`;
        await nextDocumentExportPaint();
        const renderedRoot = layoutDocument.documentElement;
        const staticRoot = element.ownerDocument.importNode(renderedRoot, true);
        inlineCanvasExportStyles(renderedRoot, staticRoot);

        const host = element.ownerDocument.createElement('div');
        Object.assign(host.style, {
          display: 'block',
          height: `${targetHeight}px`,
          overflow: 'hidden',
          width: '100%',
        });
        const surface = element.ownerDocument.createElement('div');
        Object.assign(surface.style, {
          height: `${targetHeight}px`,
          width: `${targetWidth}px`,
        });
        surface.append(staticRoot);
        surfaces.push(surface);
        host.attachShadow({ mode: 'open' }).append(surface);
        frame?.remove();
        snapshot.remove();
        container.append(host);
      }),
    );
    await nextDocumentExportPaint();
    for (const surface of surfaces) {
      for (const candidate of surface.querySelectorAll<HTMLElement>('*')) {
        const style = getComputedStyle(candidate);
        const layers = documentExportShadowLayers(style.boxShadow);
        if (layers.length === 0) continue;
        const bounds = candidate.getBoundingClientRect();
        if (bounds.width <= 0 || bounds.height <= 0) continue;
        candidate.style.setProperty('box-shadow', 'none', 'important');
        const parent = candidate.parentElement;
        if (!parent) continue;
        if (getComputedStyle(parent).position === 'static') {
          parent.style.position = 'relative';
        }
        parent.style.isolation = 'isolate';
        const parentBounds = parent.getBoundingClientRect();
        const left = bounds.left - parentBounds.left - parent.clientLeft;
        const top = bounds.top - parentBounds.top - parent.clientTop;
        for (let index = layers.length - 1; index >= 0; index -= 1) {
          const layer = layers[index];
          if (!layer) continue;
          const proxy = element.ownerDocument.createElement('span');
          Object.assign(proxy.style, {
            backgroundColor:
              style.backgroundColor === 'rgba(0, 0, 0, 0)'
                ? '#000'
                : style.backgroundColor,
            borderBottomLeftRadius: `${Math.max(0, pixelValue(style.borderBottomLeftRadius) + layer.spread)}px`,
            borderBottomRightRadius: `${Math.max(0, pixelValue(style.borderBottomRightRadius) + layer.spread)}px`,
            borderTopLeftRadius: `${Math.max(0, pixelValue(style.borderTopLeftRadius) + layer.spread)}px`,
            borderTopRightRadius: `${Math.max(0, pixelValue(style.borderTopRightRadius) + layer.spread)}px`,
            display: 'block',
            filter: `drop-shadow(${layer.offsetX}px ${layer.offsetY}px ${layer.blur / 2}px ${layer.color})`,
            height: `${bounds.height + layer.spread * 2}px`,
            left: `${left - layer.spread}px`,
            pointerEvents: 'none',
            position: 'absolute',
            top: `${top - layer.spread}px`,
            width: `${bounds.width + layer.spread * 2}px`,
            zIndex: '-1',
          });
          parent.prepend(proxy);
        }
      }
    }
    await nextDocumentExportPaint();
  } catch (error) {
    if (controller.signal.aborted) {
      throw new Error('等待画布快照加载超时。');
    }
    throw error;
  } finally {
    for (const layoutFrame of layoutFrames) layoutFrame.remove();
    clearTimeout(timeoutId);
    controller.abort();
  }
}

async function createDocumentExportNode(
  element: HTMLElement,
  padding: number,
  timeout: number,
) {
  const ownerDocument = element.ownerDocument;
  const content =
    element.querySelector<HTMLElement>(`[${documentExportContentAttribute}]`) ??
    element;
  const computedStyle = getComputedStyle(content);
  const contentWidth = documentExportContentWidth(
    content.clientWidth,
    computedStyle.paddingLeft,
    computedStyle.paddingRight,
  );
  const host = ownerDocument.createElement('div');
  host.ariaHidden = 'true';
  Object.assign(host.style, {
    left: '0',
    pointerEvents: 'none',
    position: 'fixed',
    top: '0',
    transform: 'translateX(-100%)',
  });

  const clone = element.cloneNode(true) as HTMLElement;
  for (const excluded of clone.querySelectorAll(
    `[${documentExportExcludeAttribute}]`,
  )) {
    excluded.remove();
  }
  Object.assign(clone.style, {
    boxSizing: 'border-box',
    backgroundColor: 'var(--card)',
    color: 'var(--foreground)',
    containerType: 'inline-size',
    height: 'auto',
    margin: '0',
    maxWidth: 'none',
    minHeight: '0',
    padding: `${padding}px`,
    width: `${Math.ceil(contentWidth + padding * 2)}px`,
  });
  const clonedContent = clone.querySelector<HTMLElement>(
    `[${documentExportContentAttribute}]`,
  );
  if (clonedContent) {
    Object.assign(clonedContent.style, {
      margin: '0',
      maxWidth: 'none',
      width: `${Math.ceil(contentWidth)}px`,
    });
  }
  freezeDocumentExportMotion(clone);
  host.append(clone);
  ownerDocument.body.append(host);

  try {
    await withDocumentExportTimeout(
      replaceCanvasFramesWithStaticDocuments(clone, timeout),
      timeout,
    );
    freezeDocumentExportMotion(clone);
    const preparedImages = await withDocumentExportTimeout(
      prepareDocumentExportImages(clone, timeout),
      timeout,
    );
    return {
      cleanup: () => {
        preparedImages.cleanup();
        host.remove();
      },
      height: Math.ceil(clone.scrollHeight),
      images: preparedImages.images,
      node: clone,
      width: Math.ceil(clone.scrollWidth),
    };
  } catch (error) {
    host.remove();
    throw error;
  }
}

export function documentExportPixelRatio(
  width: number,
  height: number,
  devicePixelRatio = globalThis.devicePixelRatio ?? 1,
): number {
  if (width <= 0 || height <= 0) return 1;

  return Math.min(
    Math.max(1, devicePixelRatio),
    2,
    maximumCanvasDimension / width,
    maximumCanvasDimension / height,
    Math.sqrt(maximumCanvasArea / (width * height)),
  );
}

function imagePosition(value: string, freeSpace: number): number {
  if (value === 'left' || value === 'top') return 0;
  if (value === 'right' || value === 'bottom') return freeSpace;
  if (value.endsWith('%')) return (freeSpace * pixelValue(value)) / 100;
  if (value.endsWith('px')) return pixelValue(value);
  return freeSpace / 2;
}

function drawDocumentExportImage(
  context: CanvasRenderingContext2D,
  image: DocumentExportImage,
) {
  const sourceWidth = image.source.naturalWidth;
  const sourceHeight = image.source.naturalHeight;
  if (
    sourceWidth <= 0 ||
    sourceHeight <= 0 ||
    image.width <= 0 ||
    image.height <= 0
  ) {
    return;
  }

  const clip = image.clip ?? {
    height: image.height,
    radius: 0,
    width: image.width,
    x: image.x,
    y: image.y,
  };
  let x = image.x;
  let y = image.y;
  let renderedWidth = image.width;
  let renderedHeight = image.height;
  if (image.objectFit !== 'fill') {
    const containScale = Math.min(
      image.width / sourceWidth,
      image.height / sourceHeight,
    );
    const scale =
      image.objectFit === 'cover'
        ? Math.max(image.width / sourceWidth, image.height / sourceHeight)
        : image.objectFit === 'none'
          ? 1
          : image.objectFit === 'scale-down'
            ? Math.min(1, containScale)
            : containScale;
    renderedWidth = sourceWidth * scale;
    renderedHeight = sourceHeight * scale;
    const [horizontalPosition = '50%', verticalPosition = '50%'] =
      image.objectPosition.split(/\s+/);
    x += imagePosition(horizontalPosition, image.width - renderedWidth);
    y += imagePosition(verticalPosition, image.height - renderedHeight);
  }

  if (clip.radius > 0) {
    const supersample = Math.max(2, context.getTransform().a * 2);
    const mask = image.source.ownerDocument.createElement('canvas');
    mask.width = Math.ceil(clip.width * supersample);
    mask.height = Math.ceil(clip.height * supersample);
    const maskContext = mask.getContext('2d');
    if (!maskContext) return;
    maskContext.imageSmoothingEnabled = true;
    maskContext.imageSmoothingQuality = 'high';
    maskContext.setTransform(
      supersample,
      0,
      0,
      supersample,
      -clip.x * supersample,
      -clip.y * supersample,
    );
    maskContext.beginPath();
    maskContext.roundRect(clip.x, clip.y, clip.width, clip.height, clip.radius);
    maskContext.clip();
    maskContext.drawImage(image.source, x, y, renderedWidth, renderedHeight);
    context.drawImage(mask, clip.x, clip.y, clip.width, clip.height);
    return;
  }

  context.save();
  context.beginPath();
  context.rect(clip.x, clip.y, clip.width, clip.height);
  context.clip();
  context.drawImage(image.source, x, y, renderedWidth, renderedHeight);
  context.restore();
}

function canvasBlob(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (blob) resolve(blob);
      else reject(new Error('无法生成文档图片。'));
    }, 'image/png');
  });
}

export function shouldIncludeDocumentExportNode(node: Node): boolean {
  const candidate = node as Partial<HTMLElement>;
  return (
    typeof candidate.hasAttribute !== 'function' ||
    !candidate.hasAttribute(documentExportExcludeAttribute)
  );
}

async function waitForDocumentExportRendering(
  element: HTMLElement,
  timeoutDuration = documentExportRenderTimeout,
) {
  if (typeof MutationObserver === 'undefined') return false;

  return new Promise<boolean>((resolve) => {
    let stabilityCheck = 0;
    const finish = (ready: boolean) => {
      clearTimeout(timeout);
      observer.disconnect();
      resolve(ready);
    };
    const check = () => {
      stabilityCheck += 1;
      const checkId = stabilityCheck;
      if (
        !element.querySelector(readyDocumentRenderSelector) ||
        !element.querySelector(renderedDocumentSelector) ||
        element.querySelector(
          '[data-canvas-container]:not([data-canvas-export-status="ready"])',
        )
      ) {
        return;
      }

      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          if (
            checkId === stabilityCheck &&
            element.querySelector(readyDocumentRenderSelector) &&
            element.querySelector(renderedDocumentSelector) &&
            !element.querySelector(
              '[data-canvas-container]:not([data-canvas-export-status="ready"])',
            )
          ) {
            finish(true);
          }
        });
      });
    };
    const observer = new MutationObserver(check);
    const timeout = setTimeout(() => finish(false), timeoutDuration);

    observer.observe(element, {
      attributes: true,
      attributeFilter: ['data-canvas-export-status'],
      childList: true,
      subtree: true,
    });

    check();
  });
}

async function renderDocumentPng(element: HTMLElement): Promise<Blob> {
  const deadline = Date.now() + documentImageExportTimeout;
  const remainingTime = () => Math.max(1, deadline - Date.now());
  const renderingReady = await waitForDocumentExportRendering(
    element,
    Math.min(documentExportRenderTimeout, remainingTime()),
  );
  if (!renderingReady) {
    throw new Error('文章尚未完成渲染。');
  }
  const prepared = await createDocumentExportNode(
    element,
    documentImageExportPadding,
    remainingTime(),
  );
  try {
    const { toCanvas } = await import('html-to-image');
    const backgroundColor = getComputedStyle(prepared.node).backgroundColor;
    const pixelRatio = documentExportPixelRatio(
      prepared.width,
      prepared.height,
    );
    await withDocumentExportTimeout(
      prepareHighResolutionSvgImages(prepared.images, pixelRatio),
      remainingTime(),
    );
    const canvas = await withDocumentExportTimeout(
      toCanvas(prepared.node, {
        backgroundColor,
        filter: shouldIncludeDocumentExportNode,
        height: prepared.height,
        includeQueryParams: true,
        pixelRatio,
        preferredFontFormat: 'woff2',
        skipAutoScale: true,
        width: prepared.width,
      }),
      remainingTime(),
    );
    const context = canvas.getContext('2d');
    if (!context) throw new Error('无法绘制文档图片。');
    context.imageSmoothingEnabled = true;
    context.imageSmoothingQuality = 'high';
    context.setTransform(
      canvas.width / prepared.width,
      0,
      0,
      canvas.height / prepared.height,
      0,
      0,
    );
    for (const image of prepared.images) {
      drawDocumentExportImage(context, image);
    }
    context.resetTransform();
    return await withDocumentExportTimeout(canvasBlob(canvas), remainingTime());
  } finally {
    prepared.cleanup();
  }
}

export function documentImageFilename(title: string): string {
  const normalizedTitle = Array.from(title.trim(), (character) =>
    character.charCodeAt(0) < 32 ? '-' : character,
  ).join('');
  const safeTitle = normalizedTitle
    .replace(/[<>:"/\\|?*]/g, '-')
    .replace(/\.+$/g, '')
    .trim();
  return `${safeTitle || 'locus-note'}.png`;
}

export async function downloadDocumentAsImage(
  element: HTMLElement,
  filename: string,
): Promise<boolean> {
  try {
    const blob = await renderDocumentPng(element);
    const url = URL.createObjectURL(blob);
    const link = element.ownerDocument.createElement('a');
    link.download = filename;
    link.href = url;
    link.style.display = 'none';
    element.ownerDocument.body.append(link);
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 0);
    return true;
  } catch (error) {
    console.error('下载文档图片失败。', error);
    return false;
  }
}

export async function copyDocumentAsImage(
  element: HTMLElement,
  clipboard: Pick<Clipboard, 'write'> | undefined = globalThis.navigator
    ?.clipboard,
  ClipboardItemConstructor:
    | typeof ClipboardItem
    | undefined = globalThis.ClipboardItem,
): Promise<boolean> {
  if (!clipboard || !ClipboardItemConstructor) return false;

  try {
    const png = renderDocumentPng(element);
    await clipboard.write([
      new ClipboardItemConstructor({
        'image/png': png,
      }),
    ]);
    return true;
  } catch (error) {
    console.error('复制文档图片失败。', error);
    return false;
  }
}
