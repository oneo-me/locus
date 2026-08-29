import {
  getMarkdownAssetName,
  isPreviewableImageAsset,
} from '@locus/document-library/contracts/asset';
import { documentIconName } from '@locus/document-library/contracts/document-icon';
import { LinkIcon } from 'lucide-react';
import { type ComponentProps, useMemo, useState } from 'react';
import { type MarkdownComponents, MarkdownRenderer } from '@locus/markdown-ui';
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
import type { MarkdownDocument } from 'comark';
import { useDocumentLibraryUi } from './runtime';

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
  const { getAssetUrl, getDocumentHref, Link } = useDocumentLibraryUi();
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
      href={getDocumentHref(resolved.articlePath)}
      title={resolved.titlePath}
    >
      {hasIcon ? (
        <span aria-hidden="true" data-icon="inline-start">
          <DocumentIcon
            className="size-full text-[0.65em]"
            icon={resolved.icon}
            imageSrc={
              resolved.iconImageModifiedAt
                ? getAssetUrl(resolved.documentPath, documentIconName, {
                    version: resolved.iconImageModifiedAt,
                  })
                : null
            }
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

function RemoteLinkIcon({ url }: { url: URL }) {
  const { getRemoteFaviconUrl } = useDocumentLibraryUi();
  const [failed, setFailed] = useState(false);
  const faviconUrl = getRemoteFaviconUrl?.(url);

  if (failed || !faviconUrl) {
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
        src={faviconUrl}
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
  const { getAssetUrl } = useDocumentLibraryUi();
  const assetName = href
    ? getMarkdownAssetName(documentPath, assets, href)
    : null;
  const resolvedHref = href
    ? assetName
      ? getAssetUrl(documentPath, assetName)
      : href
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
  content: MarkdownDocument;
  documentPath: string;
  documents: readonly WikiLinkDocument[];
}) {
  const { createMarkdownExtensions, getAssetUrl } = useDocumentLibraryUi();
  const documentsByPath = useMemo(
    () => createWikiLinkDocumentMap(documents),
    [documents],
  );
  const extensionComponents = useMemo(
    () =>
      Object.assign(
        {},
        ...(createMarkdownExtensions?.({ assets, documentPath }) ?? []).map(
          (extension) => extension.components ?? {},
        ),
      ),
    [assets, createMarkdownExtensions, documentPath],
  );
  const components = useMemo<MarkdownComponents>(
    () => ({
      a: (props: ComponentProps<'a'> & { href?: string }) => (
        <MarkdownLink {...props} assets={assets} documentPath={documentPath} />
      ),
      'locus-wiki-link': (props: { path?: string }) => (
        <WikiLink {...props} documentsByPath={documentsByPath} />
      ),
      attachments: (props: { error?: unknown; items?: unknown }) => (
        <AttachmentList {...props} documentPath={documentPath} />
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
            src={src && assetName ? getAssetUrl(documentPath, assetName) : src}
          />
        );
      },
      pre: DocumentCodeBlock,
      ...extensionComponents,
    }),
    [assets, extensionComponents, documentPath, documentsByPath, getAssetUrl],
  );

  return (
    <MarkdownRenderer
      className="space-y-5"
      components={components}
      value={content}
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
  content: MarkdownDocument;
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
