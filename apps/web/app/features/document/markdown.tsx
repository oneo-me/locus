import {
  getDocumentIconUrl,
  getMarkdownAssetName,
  isPreviewableImageAsset,
  resolveMarkdownAssetUrl,
} from '@locus/document-library/contracts/asset';
import { LinkIcon } from 'lucide-react';
import { type ComponentProps, useMemo, useState } from 'react';
import { Link } from 'react-router';
import {
  type MarkdownComponents,
  MarkdownRenderer,
} from '../../components/markdown/renderer';
import { CanvasBlock } from '../../components/markdown/canvas';
import { cn } from '@locus/ui/utils';
import {
  AttachmentGallery,
  AttachmentList,
  InlineAttachmentImage,
} from './attachment-views';
import { DocumentIcon } from './icons';
import {
  createWikiLinkDocumentMap,
  resolveWikiLink,
  type WikiLinkDocument,
} from './wiki-link';
import type { ComarkTree } from 'comark';
import type { SerializedMarkdownTree } from '@locus/document-library/contracts/library-api';

function DocumentCodeBlock({
  children,
  className,
  language,
}: ComponentProps<'pre'> & { language?: string }) {
  return (
    <figure className="code-block document-adaptive-block rendered-block">
      <figcaption className="rendered-block__label">
        {language || 'text'}
      </figcaption>
      <pre className={className} data-language={language || 'text'}>
        {children}
      </pre>
    </figure>
  );
}

function WikiLink({
  documentsByPath,
  path,
}: {
  documentsByPath: ReadonlyMap<string, WikiLinkDocument>;
  path?: string;
}) {
  const rawPath = typeof path === 'string' ? path : '';
  const resolved = resolveWikiLink(rawPath, documentsByPath);
  if (!resolved) {
    return (
      <span
        className="document-inline-link"
        data-invalid
        title={`未找到笔记：${rawPath || '（空路径）'}`}
      >
        {rawPath || '[[]]'}
      </span>
    );
  }

  const hasIcon = Boolean(resolved.icon || resolved.iconImageModifiedAt);

  return (
    <Link
      className="document-inline-link"
      title={resolved.titlePath}
      to={resolved.href}
    >
      {hasIcon ? (
        <span aria-hidden="true" data-icon="inline-start">
          <DocumentIcon
            className="size-full text-[0.65em]"
            icon={resolved.icon}
            imageSrc={getDocumentIconUrl(
              resolved.documentPath,
              resolved.iconImageModifiedAt,
            )}
          />
        </span>
      ) : null}
      {resolved.label}
    </Link>
  );
}

function remoteHttpUrl(href: string): URL | null {
  try {
    const url = new URL(href);
    return url.protocol === 'http:' || url.protocol === 'https:' ? url : null;
  } catch {
    return null;
  }
}

function remoteFaviconUrl(url: URL): string {
  const search = new URLSearchParams({
    domain_url: url.origin,
    sz: '32',
  });
  return `https://www.google.com/s2/favicons?${search}`;
}

function RemoteLinkIcon({ url }: { url: URL }) {
  const [failed, setFailed] = useState(false);

  if (failed) {
    return (
      <span aria-hidden="true" data-icon="inline-start">
        <LinkIcon className="size-full" />
      </span>
    );
  }

  return (
    <span aria-hidden="true" data-icon="inline-start">
      <img
        alt=""
        className="size-full rounded-[2px]"
        decoding="async"
        loading="lazy"
        onError={() => setFailed(true)}
        referrerPolicy="no-referrer"
        src={remoteFaviconUrl(url)}
      />
    </span>
  );
}

function MarkdownLink({
  assets,
  children,
  documentPath,
  href,
  ...props
}: ComponentProps<'a'> & {
  assets: string[];
  documentPath: string;
  href?: string;
}) {
  const resolvedHref = href
    ? resolveMarkdownAssetUrl(documentPath, assets, href)
    : undefined;
  const remoteUrl = resolvedHref ? remoteHttpUrl(resolvedHref) : null;

  return (
    <a
      {...props}
      className={cn('document-inline-link', props.className)}
      href={resolvedHref}
      rel={remoteUrl ? 'noopener noreferrer' : props.rel}
      target={remoteUrl ? '_blank' : props.target}
    >
      {remoteUrl ? <RemoteLinkIcon url={remoteUrl} /> : null}
      {children}
    </a>
  );
}

function DocumentMarkdown({
  assets,
  content,
  documentPath,
  documents,
}: {
  assets: string[];
  content: SerializedMarkdownTree;
  documentPath: string;
  documents: readonly WikiLinkDocument[];
}) {
  const documentsByPath = useMemo(
    () => createWikiLinkDocumentMap(documents),
    [documents],
  );
  const components = useMemo<MarkdownComponents>(
    () => ({
      a: (props: ComponentProps<'a'> & { href?: string }) => (
        <MarkdownLink {...props} assets={assets} documentPath={documentPath} />
      ),
      'asterweft-wiki-link': (props: { path?: string }) => (
        <WikiLink {...props} documentsByPath={documentsByPath} />
      ),
      attachments: (props: { error?: unknown; items?: unknown }) => (
        <AttachmentList {...props} documentPath={documentPath} />
      ),
      canvas: (props: {
        error?: unknown;
        height?: unknown;
        source?: unknown;
      }) => (
        <CanvasBlock {...props} assets={assets} documentPath={documentPath} />
      ),
      gallery: (props: { error?: unknown; items?: unknown }) => (
        <AttachmentGallery {...props} documentPath={documentPath} />
      ),
      img: ({
        alt,
        src,
        ...props
      }: ComponentProps<'img'> & { src?: string }) => {
        const assetName = src
          ? getMarkdownAssetName(documentPath, assets, src)
          : null;
        return assetName && isPreviewableImageAsset(assetName) ? (
          <InlineAttachmentImage
            {...props}
            alt={alt ?? ''}
            documentPath={documentPath}
            name={assetName}
          />
        ) : (
          <img
            {...props}
            alt={alt ?? ''}
            src={
              src
                ? resolveMarkdownAssetUrl(documentPath, assets, src)
                : undefined
            }
          />
        );
      },
      pre: DocumentCodeBlock,
    }),
    [assets, documentPath, documentsByPath],
  );

  return (
    <MarkdownRenderer
      className="space-y-5"
      components={components}
      tree={content as ComarkTree}
    />
  );
}

export function MarkdownContent({
  assets,
  content,
  documentPath,
  documents = [],
}: {
  assets: string[];
  content: SerializedMarkdownTree;
  documentPath: string;
  documents?: readonly WikiLinkDocument[];
}) {
  return (
    <div
      className="typeset typeset-document w-full [overflow-wrap:anywhere] [text-wrap:pretty] [&_a:not([data-slot=badge]):not(.document-inline-link)]:text-primary [&_:is(h1,h2,h3,h4,h5,h6)]:[text-wrap:balance]"
      data-document-content-status="ready"
    >
      <DocumentMarkdown
        assets={assets}
        content={content}
        documentPath={documentPath}
        documents={documents}
      />
    </div>
  );
}
