import type { ComarkPlugin, ElementNode, Node } from 'comark';
import { CircleAlertIcon, ImageOffIcon } from 'lucide-react';
import {
  type ComponentProps,
  type CSSProperties,
  useEffect,
  useMemo,
  useState,
} from 'react';
import {
  Alert,
  AlertDescription,
  AlertTitle,
} from '@locus/ui/components/alert';
import { Button } from '@locus/ui/components/button';
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@locus/ui/components/dialog';
import {
  getAssetUrl,
  getAssetThumbnailUrl,
  getMarkdownAssetName,
  isBrowserViewableAsset,
  isPreviewableImageAsset,
} from '@locus/document-library/contracts/asset';
import { cn } from '@locus/ui/utils';
import type { ImageSize } from '@locus/document-library/contracts/library-types';

const invalidGalleryMessage =
  '请在 ::gallery 中只放置当前文章的本地图片附件，并为每张图片填写替代文本。';
const invalidAttachmentsMessage =
  '请在 ::attachments 中只放置当前文章的本地普通附件，并使用文件名作为链接文字。';

interface AttachmentImageItem {
  alt: string;
  name: string;
}

interface GalleryItem extends AttachmentImageItem, ImageSize {}

interface AttachmentItem {
  name: string;
}

function isElement(node: Node | undefined): node is ElementNode {
  return Array.isArray(node) && typeof node[0] === 'string';
}

function textContent(node: Node): string {
  if (typeof node === 'string') return node;
  if (!isElement(node)) return '';
  return (node.slice(2) as Node[]).map((child) => textContent(child)).join('');
}

function containerElements(
  node: ElementNode,
  expectedTag: 'a' | 'img',
): ElementNode[] | undefined {
  const elements: ElementNode[] = [];

  for (const child of node.slice(2) as Node[]) {
    if (typeof child === 'string') {
      if (child.trim()) return;
      continue;
    }
    if (!isElement(child) || child[0] !== 'p') return;

    for (const content of child.slice(2) as Node[]) {
      if (typeof content === 'string') {
        if (content.trim()) return;
        continue;
      }
      if (!isElement(content) || content[0] !== expectedTag) return;
      elements.push(content);
    }
  }

  return elements.length > 0 ? elements : undefined;
}

function prepareGallery(
  node: ElementNode,
  documentPath: string,
  assets: string[],
  assetImageSizes: Record<string, ImageSize>,
) {
  const images = containerElements(node, 'img');
  const items = images?.flatMap<GalleryItem>((image) => {
    const source = typeof image[1].src === 'string' ? image[1].src : '';
    const alt = typeof image[1].alt === 'string' ? image[1].alt.trim() : '';
    const name = getMarkdownAssetName(documentPath, assets, source);
    const size = name ? assetImageSizes[name] : undefined;
    return name && alt && size && isPreviewableImageAsset(name)
      ? [{ alt, name, ...size }]
      : [];
  });

  if (!images || !items || items.length !== images.length) {
    delete node[1].items;
    node[1].error = invalidGalleryMessage;
  } else {
    node[1].items = items;
    delete node[1].error;
  }
  node.splice(2);
}

function prepareAttachments(
  node: ElementNode,
  documentPath: string,
  assets: string[],
) {
  const links = containerElements(node, 'a');
  const items = links?.flatMap<AttachmentItem>((link) => {
    const source = typeof link[1].href === 'string' ? link[1].href : '';
    const name = getMarkdownAssetName(documentPath, assets, source);
    const label = textContent(link).trim().normalize('NFC');
    return name &&
      !isPreviewableImageAsset(name) &&
      label === name.normalize('NFC')
      ? [{ name }]
      : [];
  });

  if (!links || !items || items.length !== links.length) {
    delete node[1].items;
    node[1].error = invalidAttachmentsMessage;
  } else {
    node[1].items = items;
    delete node[1].error;
  }
  node.splice(2);
}

function prepareAttachmentViews(
  node: Node,
  documentPath: string,
  assets: string[],
  assetImageSizes: Record<string, ImageSize>,
) {
  if (!isElement(node)) return;
  if (node[0] === 'gallery') {
    prepareGallery(node, documentPath, assets, assetImageSizes);
    return;
  }
  if (node[0] === 'attachments') {
    prepareAttachments(node, documentPath, assets);
    return;
  }
  for (const child of node.slice(2) as Node[]) {
    prepareAttachmentViews(child, documentPath, assets, assetImageSizes);
  }
}

export function createAttachmentViewsPlugin({
  assetImageSizes,
  assets,
  documentPath,
}: {
  assetImageSizes: Record<string, ImageSize>;
  assets: string[];
  documentPath: string;
}): ComarkPlugin {
  return {
    name: 'locus-attachment-views',
    post(state) {
      if (!state.tree) return;
      for (const node of state.tree.nodes) {
        prepareAttachmentViews(node, documentPath, assets, assetImageSizes);
      }
    },
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function galleryItems(value: unknown): GalleryItem[] | undefined {
  if (!Array.isArray(value) || value.length === 0) return;
  const items = value.flatMap<GalleryItem>((item) =>
    isRecord(item) &&
    typeof item.name === 'string' &&
    typeof item.alt === 'string' &&
    typeof item.height === 'number' &&
    typeof item.width === 'number' &&
    item.name &&
    item.alt &&
    item.height > 0 &&
    item.width > 0
      ? [
          {
            alt: item.alt,
            height: item.height,
            name: item.name,
            width: item.width,
          },
        ]
      : [],
  );
  return items.length === value.length ? items : undefined;
}

function attachmentItems(value: unknown): AttachmentItem[] | undefined {
  if (!Array.isArray(value) || value.length === 0) return;
  const items = value.flatMap<AttachmentItem>((item) =>
    isRecord(item) && typeof item.name === 'string' && item.name
      ? [{ name: item.name }]
      : [],
  );
  return items.length === value.length ? items : undefined;
}

function AttachmentViewError({
  kind,
  message,
}: {
  kind: 'attachments' | 'gallery';
  message: string;
}) {
  return (
    <Alert data-not-typeset variant="destructive">
      <CircleAlertIcon aria-hidden="true" />
      <AlertTitle>
        {kind === 'gallery' ? '无法显示图库' : '无法显示附件'}
      </AlertTitle>
      <AlertDescription>{message}</AlertDescription>
    </Alert>
  );
}

function GalleryThumbnail({
  item,
  documentPath,
  onSelect,
}: {
  item: GalleryItem;
  documentPath: string;
  onSelect: () => void;
}) {
  const [failed, setFailed] = useState(false);

  return (
    <Button
      aria-label={`查看图片：${item.alt}`}
      className="flex size-full overflow-hidden p-0"
      onClick={onSelect}
      type="button"
      variant="ghost"
    >
      {failed ? (
        <ImageOffIcon aria-hidden="true" />
      ) : (
        <img
          alt=""
          className="size-full object-cover"
          decoding="async"
          height={item.height}
          loading="lazy"
          onError={() => setFailed(true)}
          src={getAssetThumbnailUrl(documentPath, item.name)}
          width={item.width}
        />
      )}
    </Button>
  );
}

function OriginalAttachmentImage({
  item,
  documentPath,
}: {
  item: AttachmentImageItem;
  documentPath: string;
}) {
  const [naturalSize, setNaturalSize] = useState({ height: 0, width: 0 });
  const [viewportSize, setViewportSize] = useState({ height: 0, width: 0 });

  useEffect(() => {
    const viewport = window.visualViewport;
    const updateViewportSize = () =>
      setViewportSize({
        height: (viewport?.height ?? window.innerHeight) - 32,
        width: (viewport?.width ?? window.innerWidth) - 32,
      });

    updateViewportSize();
    window.addEventListener('resize', updateViewportSize);
    viewport?.addEventListener('resize', updateViewportSize);
    return () => {
      window.removeEventListener('resize', updateViewportSize);
      viewport?.removeEventListener('resize', updateViewportSize);
    };
  }, []);

  const scale =
    naturalSize.width && naturalSize.height
      ? Math.min(
          1,
          viewportSize.width / naturalSize.width,
          viewportSize.height / naturalSize.height,
        )
      : 0;

  return (
    <img
      alt={item.alt}
      className="block object-contain"
      onLoad={(event) =>
        setNaturalSize({
          height: event.currentTarget.naturalHeight,
          width: event.currentTarget.naturalWidth,
        })
      }
      src={getAssetUrl(documentPath, item.name)}
      style={{
        height: naturalSize.height * scale,
        width: naturalSize.width * scale,
      }}
    />
  );
}

function AttachmentImageDialog({
  item,
  documentPath,
  onOpenChange,
  open,
}: {
  item: AttachmentImageItem;
  documentPath: string;
  onOpenChange: (open: boolean) => void;
  open: boolean;
}) {
  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      {open ? (
        <DialogContent
          className="inset-0 top-0 left-0 size-full max-w-none translate-x-0 translate-y-0 place-items-center gap-0 rounded-none bg-transparent p-0 text-white shadow-none ring-0 sm:max-w-none data-open:zoom-in-100 data-closed:zoom-out-100"
          onClick={() => onOpenChange(false)}
          overlayClassName="bg-black/80"
        >
          <DialogTitle className="sr-only">{item.alt}</DialogTitle>
          <div className="flex size-full min-h-0 min-w-0 items-center justify-center overflow-hidden p-4">
            <OriginalAttachmentImage documentPath={documentPath} item={item} />
          </div>
        </DialogContent>
      ) : null}
    </Dialog>
  );
}

export function InlineAttachmentImage({
  alt,
  className,
  documentPath,
  name,
  onError,
  ...props
}: Omit<ComponentProps<'img'>, 'src'> & {
  documentPath: string;
  name: string;
}) {
  const [failed, setFailed] = useState(false);
  const [open, setOpen] = useState(false);
  const description = alt?.trim() || name;
  const item = { alt: description, name };

  return (
    <>
      <Button
        aria-label={`查看原图：${description}`}
        className="block h-auto max-w-full overflow-hidden p-0"
        data-not-typeset
        onClick={() => setOpen(true)}
        type="button"
        variant="ghost"
      >
        {failed ? (
          <ImageOffIcon aria-hidden="true" />
        ) : (
          <img
            {...props}
            alt=""
            className={cn('h-auto max-w-full', className)}
            decoding={props.decoding ?? 'async'}
            loading={props.loading ?? 'lazy'}
            onError={(event) => {
              setFailed(true);
              onError?.(event);
            }}
            src={getAssetThumbnailUrl(documentPath, name)}
          />
        )}
      </Button>
      <AttachmentImageDialog
        documentPath={documentPath}
        item={item}
        onOpenChange={setOpen}
        open={open}
      />
    </>
  );
}

export function AttachmentGallery({
  documentPath,
  error,
  items: itemValue,
}: {
  documentPath: string;
  error?: unknown;
  items?: unknown;
}) {
  const items = useMemo(() => galleryItems(itemValue), [itemValue]);
  const [selected, setSelected] = useState<GalleryItem | null>(null);

  if (typeof error === 'string') {
    return <AttachmentViewError kind="gallery" message={error} />;
  }
  if (!items) {
    return (
      <AttachmentViewError kind="gallery" message={invalidGalleryMessage} />
    );
  }

  return (
    <section
      aria-label={`图片附件，共 ${items.length} 张`}
      className="document-adaptive-block"
      data-attachment-gallery
      data-not-typeset
    >
      <ul className="flex list-none flex-wrap gap-2 p-0 after:-ml-2 after:h-0 after:basis-0 after:grow-[999999] after:content-['']">
        {items.map((item, index) => {
          const aspectRatio = item.width / item.height;
          const style = {
            '--gallery-item-mobile-width': `${aspectRatio * 120}px`,
            '--gallery-item-width': `${aspectRatio * 180}px`,
            aspectRatio: `${item.width} / ${item.height}`,
            flexGrow: aspectRatio,
          } as CSSProperties;
          return (
            <li
              className="min-w-0 max-w-full shrink basis-[var(--gallery-item-mobile-width)] overflow-hidden sm:basis-[var(--gallery-item-width)]"
              data-gallery-height={item.height}
              data-gallery-width={item.width}
              key={`${item.name}:${index}`}
              style={style}
            >
              <GalleryThumbnail
                documentPath={documentPath}
                item={item}
                onSelect={() => setSelected(item)}
              />
            </li>
          );
        })}
      </ul>
      {selected ? (
        <AttachmentImageDialog
          documentPath={documentPath}
          item={selected}
          onOpenChange={(open) => {
            if (!open) setSelected(null);
          }}
          open
        />
      ) : null}
    </section>
  );
}

function MiddleTruncatedFilename({ value }: { value: string }) {
  const [start, end] = splitFilenameForMiddleEllipsis(value);

  if (!end) return <span className="truncate">{start}</span>;
  return (
    <span aria-hidden="true" className="flex min-w-0">
      <span className="min-w-[4ch] truncate">{start}</span>
      <span className="shrink-0">{end}</span>
    </span>
  );
}

export function splitFilenameForMiddleEllipsis(
  value: string,
): [start: string, end: string] {
  const characters = Array.from(value);
  if (characters.length <= 20) return [value, ''];

  const tailLength = Math.min(
    12,
    Math.max(8, Math.floor(characters.length * 0.35)),
  );
  return [
    characters.slice(0, -tailLength).join(''),
    characters.slice(-tailLength).join(''),
  ];
}

function assetTypeLabel(name: string): string {
  const extensionStart = name.lastIndexOf('.');
  if (extensionStart <= 0 || extensionStart === name.length - 1) return 'FILE';
  return name
    .slice(extensionStart + 1)
    .toUpperCase()
    .slice(0, 4);
}

export function AttachmentList({
  documentPath,
  error,
  items: itemValue,
}: {
  documentPath: string;
  error?: unknown;
  items?: unknown;
}) {
  const items = attachmentItems(itemValue);
  if (typeof error === 'string') {
    return <AttachmentViewError kind="attachments" message={error} />;
  }
  if (!items) {
    return (
      <AttachmentViewError
        kind="attachments"
        message={invalidAttachmentsMessage}
      />
    );
  }

  return (
    <section aria-label="普通附件" data-attachment-list data-not-typeset>
      <ul className="grid grid-cols-1 gap-px overflow-hidden rounded-lg border bg-border">
        {items.map((item, index) => {
          const browserViewable = isBrowserViewableAsset(item.name);
          return (
            <li className="min-w-0 bg-background" key={`${item.name}:${index}`}>
              <a
                aria-label={
                  browserViewable
                    ? `在新标签页打开附件：${item.name}`
                    : `下载附件：${item.name}`
                }
                className="flex min-w-0 items-center gap-3 px-3 py-2.5 text-foreground no-underline transition-colors hover:bg-muted/60 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
                download={browserViewable ? undefined : item.name}
                href={getAssetUrl(documentPath, item.name)}
                rel={browserViewable ? 'noopener noreferrer' : undefined}
                target={browserViewable ? '_blank' : undefined}
                title={item.name}
              >
                <span
                  aria-hidden="true"
                  className="grid size-9 shrink-0 place-items-center rounded-md bg-muted px-1 text-[0.625rem] font-semibold text-muted-foreground"
                >
                  {assetTypeLabel(item.name)}
                </span>
                <span className="min-w-0 flex-1 text-foreground">
                  <MiddleTruncatedFilename value={item.name} />
                </span>
              </a>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
