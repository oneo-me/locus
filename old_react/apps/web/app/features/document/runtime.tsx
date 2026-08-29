import {
  getAssetUrl as createAssetUrl,
  getMarkdownAssetName,
} from '@locus/document-library/contracts/asset';
import { documentIconName } from '@locus/document-library/contracts/document-icon';
import { urlPathToHref } from '@locus/document-library/contracts/library-route';
import type { LibrarySelectionSourceView } from '@locus/document-library/contracts/library-api';
import {
  createCanvasMarkdownExtension,
  type ResolveCanvasUrl,
} from '@locus/canvas-ui';
import {
  type DocumentLibraryUiRuntime,
  type DocumentLinkProps,
  type DocumentMarkdownExtensionContext,
} from '@locus/document-library-ui';
import { parseDocumentMarkdown } from '@locus/document-library-ui/markdown-parser';
import { useMemo } from 'react';
import { Link, useNavigate, useRevalidator } from 'react-router';
import { copyText } from '~/lib/clipboard';
import { useDocumentApi } from '~/lib/document-api-context';

const canvasParserExtension = createCanvasMarkdownExtension();

function RouterDocumentLink({ href, ...props }: DocumentLinkProps) {
  return <Link {...props} to={href} />;
}

function getAssetUrl(
  documentPath: string,
  assetName: string,
  options?: { thumbnail?: boolean; version?: string | null },
): string {
  const url = new URL(createAssetUrl(documentPath, assetName), location.origin);
  if (options?.thumbnail) url.searchParams.set('thumbnail', '1');
  if (options?.version) url.searchParams.set('v', options.version);
  return `${url.pathname}${url.search}`;
}

function getRemoteFaviconUrl(url: URL): string {
  const search = new URLSearchParams({
    domain_url: url.origin,
    sz: '32',
  });
  return `https://www.google.com/s2/favicons?${search}`;
}

async function loadAssetAsDataUrl(url: string): Promise<string | null> {
  try {
    const response = await fetch(url);
    if (!response.ok) return null;
    const blob = await response.blob();
    return await new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.addEventListener('error', () => reject(reader.error));
      reader.addEventListener('load', () => {
        if (typeof reader.result === 'string') resolve(reader.result);
        else reject(new Error('无法读取画布附件。'));
      });
      reader.readAsDataURL(blob);
    });
  } catch {
    return null;
  }
}

function resolveCanvasDocumentIconUrl(
  source: string,
  documentPath: string,
): string | null {
  if (
    !source ||
    source.startsWith('/') ||
    source.startsWith('//') ||
    /^[a-z][a-z\d+.-]*:/i.test(source)
  ) {
    return null;
  }

  const pathEnd = source.search(/[?#]/);
  const encodedPath = pathEnd < 0 ? source : source.slice(0, pathEnd);
  let decodedPath: string;
  try {
    decodedPath = decodeURIComponent(encodedPath).normalize('NFC');
  } catch {
    return null;
  }

  const segments = decodedPath.replace(/^\.\//, '').split('/');
  if (
    segments.length === 0 ||
    segments.some(
      (segment) => !segment || segment === '.' || segment === '..',
    ) ||
    segments.at(-1)?.toLowerCase() !== documentIconName
  ) {
    return null;
  }

  const directory = documentPath.endsWith('/index.md')
    ? documentPath.slice(0, -'/index.md'.length)
    : documentPath === 'index.md'
      ? ''
      : null;
  if (directory === null) return null;

  const iconDirectory = [directory, ...segments.slice(0, -1)]
    .filter(Boolean)
    .join('/');
  return getAssetUrl(
    iconDirectory ? `${iconDirectory}/index.md` : 'index.md',
    documentIconName,
  );
}

function createCanvasExtensions(
  { assets, documentPath }: DocumentMarkdownExtensionContext,
  openLocalDirectory: (path: string) => Promise<unknown>,
) {
  const resolveUrl: ResolveCanvasUrl = (source) => {
    const assetName = getMarkdownAssetName(documentPath, assets, source);
    return assetName
      ? getAssetUrl(documentPath, assetName)
      : resolveCanvasDocumentIconUrl(source, documentPath);
  };

  return [
    createCanvasMarkdownExtension({
      assetOrigins: [location.origin],
      loadAssetAsDataUrl,
      onOpenDirectory: openLocalDirectory,
      resolveUrl,
    }),
  ];
}

export async function prepareLibrarySelectionView(
  source: LibrarySelectionSourceView,
) {
  const { assetImageSizes, selected, ...selection } = source;
  if (!selected) return { ...selection, selected: null };
  return {
    ...selection,
    selected: {
      ...selected,
      content: await parseDocumentMarkdown({
        assetImageSizes,
        assets: selected.assets,
        content: selected.content,
        documentPath: selected.path,
        extensions: [canvasParserExtension],
      }),
    },
  };
}

export function useDocumentLibraryUiRuntime(): DocumentLibraryUiRuntime {
  const api = useDocumentApi();
  const navigate = useNavigate();
  const revalidator = useRevalidator();

  return useMemo(
    () => ({
      Link: RouterDocumentLink,
      copyReference: copyText,
      createMarkdownExtensions: (context) =>
        createCanvasExtensions(context, async (path) => {
          await api.openLocalDirectory({ path });
        }),
      getAssetUrl,
      getDocumentHref: urlPathToHref,
      getRemoteFaviconUrl,
      openDocument: async (documentPath, target) => {
        await api.openDocument({ path: documentPath, target });
      },
      setDocumentArchived: async (documentPath) => {
        await api.setDocumentArchived({
          archived: true,
          path: documentPath,
        });
        await navigate('/notes');
      },
      setDocumentGraphParticipation: async (documentPath, participates) => {
        const result = await api.setDocumentGraphParticipation({
          participates,
          path: documentPath,
        });
        await revalidator.revalidate();
        return result.participates;
      },
      updateDocumentIconImage: async (documentPath, dataBase64) => {
        await api.updateDocumentIconImage({
          dataBase64,
          path: documentPath,
        });
        await revalidator.revalidate();
      },
    }),
    [api, navigate, revalidator],
  );
}
