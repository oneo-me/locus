import type { ComarkPlugin, ElementNode, Node } from 'comark';
import { CircleAlertIcon } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
// @ts-ignore Vite resolves the browser script as a public asset URL.
import tailwindBrowserScriptUrl from '@tailwindcss/browser?url';
import {
  Alert,
  AlertDescription,
  AlertTitle,
} from '@locus/ui/components/alert';
import type { MarkdownUiExtension } from '@locus/markdown-ui';

const invalidCanvasContentMessage =
  '请在 ::canvas 区域中仅放置一个 html fenced code block。';
const canvasThemeMessageType = 'locus:canvas-theme';
const canvasRenderMessageType = 'locus:canvas-render';
const canvasResizeMessageType = 'locus:canvas-resize';
const canvasSnapshotMessageType = 'locus:canvas-snapshot';
const canvasActionMessageType = 'locus:canvas-action';
const defaultCanvasHeight = 160;
interface CanvasDescriptor {
  source: string;
}

export type ResolveCanvasUrl = (
  source: string,
  attribute: 'href' | 'poster' | 'src',
) => string | null;

function isElement(node: Node | undefined): node is ElementNode {
  return Array.isArray(node) && typeof node[0] === 'string';
}

function textContent(node: Node): string {
  if (typeof node === 'string') return node;
  if (!isElement(node)) return '';
  return (node.slice(2) as Node[]).map((child) => textContent(child)).join('');
}

function prepareCanvas(node: Node) {
  if (!isElement(node)) return;

  if (node[0] === 'canvas') {
    const children = node.slice(2) as Node[];
    const pre = children.length === 1 ? children[0] : undefined;
    const code = isElement(pre) && pre[0] === 'pre' ? pre[2] : undefined;
    const language =
      isElement(pre) && typeof pre[1].language === 'string'
        ? pre[1].language.toLowerCase()
        : '';

    if (language === 'html' && isElement(code) && code[0] === 'code') {
      node[1].source = textContent(code);
      delete node[1].error;
    } else {
      delete node[1].source;
      node[1].error = invalidCanvasContentMessage;
    }
    node.splice(2);
    return;
  }

  for (const child of node.slice(2) as Node[]) {
    prepareCanvas(child);
  }
}

export function createCanvasPlugin(): ComarkPlugin {
  return {
    name: 'locus-canvas',
    post(state) {
      if (!state.tree) return;
      for (const node of state.tree.nodes) prepareCanvas(node);
    },
  };
}

function canvasDescriptor({
  source,
}: {
  source?: unknown;
}): CanvasDescriptor | undefined {
  const resolvedSource = typeof source === 'string' ? source : undefined;
  if (!resolvedSource) return;
  return { source: resolvedSource };
}

function CanvasError({
  describedBy,
  message,
  title = '无法显示画布',
}: {
  describedBy?: string;
  message: string;
  title?: string;
}) {
  return (
    <Alert
      aria-describedby={describedBy}
      className="mt-5"
      data-not-typeset
      variant="destructive"
    >
      <CircleAlertIcon aria-hidden="true" />
      <AlertTitle>{title}</AlertTitle>
      <AlertDescription>{message}</AlertDescription>
    </Alert>
  );
}

export interface CanvasBlockProps {
  'aria-describedby'?: string;
  assetOrigins?: readonly string[];
  error?: unknown;
  loadAssetAsDataUrl?: (url: string) => Promise<string | null>;
  onOpenDirectory?: (path: string) => Promise<unknown>;
  resolveUrl?: ResolveCanvasUrl;
  source?: unknown;
}

export type CanvasMarkdownExtensionOptions = Omit<
  CanvasBlockProps,
  'aria-describedby' | 'error' | 'source'
>;

export function createCanvasMarkdownExtension(
  options: CanvasMarkdownExtensionOptions = {},
): MarkdownUiExtension {
  return {
    components: {
      canvas: (props: CanvasBlockProps) => (
        <CanvasBlock {...props} {...options} />
      ),
    },
    elementNames: ['canvas'],
    plugins: [createCanvasPlugin()],
  };
}

export function CanvasBlock({
  'aria-describedby': describedBy,
  assetOrigins = [],
  error,
  loadAssetAsDataUrl,
  onOpenDirectory,
  resolveUrl,
  source,
}: CanvasBlockProps) {
  if (typeof error === 'string') {
    return <CanvasError describedBy={describedBy} message={error} />;
  }

  const canvas = canvasDescriptor({ source });
  if (!canvas) {
    return (
      <CanvasError
        describedBy={describedBy}
        message={invalidCanvasContentMessage}
      />
    );
  }

  return (
    <CanvasSurface
      canvas={canvas}
      describedBy={describedBy}
      assetOrigins={assetOrigins}
      loadAssetAsDataUrl={loadAssetAsDataUrl}
      onOpenDirectory={onOpenDirectory}
      resolveUrl={resolveUrl}
    />
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

function canvasDirectoryAction(href: string): { path: string } | null {
  try {
    const url = new URL(href);
    const path = url.searchParams.get('path')?.trim();
    if (
      url.protocol !== 'locus-action:' ||
      url.hostname !== 'open-directory' ||
      (url.pathname !== '' && url.pathname !== '/') ||
      url.searchParams.getAll('path').length !== 1 ||
      !path
    ) {
      return null;
    }
    return { path };
  } catch {
    return null;
  }
}

function sanitizeCanvasFragment(
  source: string,
  resolveUrl?: ResolveCanvasUrl,
): string {
  const template = document.createElement('template');
  template.innerHTML = source;

  for (const element of template.content.querySelectorAll(
    'iframe, frame, object, embed, link, meta, base',
  )) {
    element.remove();
  }

  for (const element of template.content.querySelectorAll('*')) {
    let opensExternalLink = false;
    for (const attribute of Array.from(element.attributes)) {
      const name = attribute.name.toLowerCase();
      if (name === 'nonce' || name === 'srcdoc' || name === 'target') {
        element.removeAttribute(attribute.name);
        continue;
      }
      if (name === 'src' || name === 'poster') {
        const resolved = resolveUrl?.(
          attribute.value,
          name as 'poster' | 'src',
        );
        if (resolved) element.setAttribute(attribute.name, resolved);
        else element.removeAttribute(attribute.name);
        continue;
      }
      if (
        (name === 'href' || name === 'xlink:href') &&
        attribute.value &&
        !attribute.value.startsWith('#')
      ) {
        const resolved = resolveUrl?.(attribute.value, 'href');
        if (resolved) {
          element.setAttribute(attribute.name, resolved);
        } else if (
          name === 'href' &&
          element.tagName === 'A' &&
          remoteHttpUrl(attribute.value)
        ) {
          opensExternalLink = true;
        } else if (
          name === 'href' &&
          element.tagName === 'A' &&
          canvasDirectoryAction(attribute.value)
        ) {
          continue;
        } else {
          element.removeAttribute(attribute.name);
        }
      }
    }
    if (opensExternalLink) {
      element.setAttribute('rel', 'noopener noreferrer');
      element.setAttribute('target', '_blank');
    }
  }

  return template.innerHTML;
}

type CanvasTheme = 'dark' | 'light';

function currentCanvasTheme(): CanvasTheme {
  return document.documentElement.classList.contains('dark') ? 'dark' : 'light';
}

function canvasContentSecurityPolicy(assetOrigins: readonly string[]): string {
  const assetSources = assetOrigins.join(' ');
  return [
    "default-src 'none'",
    `img-src data: blob: ${assetSources}`,
    "style-src 'unsafe-inline'",
    `script-src 'unsafe-inline' ${assetSources}`,
    'font-src data:',
    `media-src data: blob: ${assetSources}`,
    "connect-src 'none'",
    "frame-src 'none'",
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'none'",
  ].join('; ');
}

function canvasRuntime(): string {
  return `<script>(function(){
var resizeType=${JSON.stringify(canvasResizeMessageType)};
var snapshotType=${JSON.stringify(canvasSnapshotMessageType)};
var renderType=${JSON.stringify(canvasRenderMessageType)};
var actionType=${JSON.stringify(canvasActionMessageType)};
var scheduled=false;
function contentHeight(){
  var body=document.body;
  if(!body)return 1;
  var bottom=body.getBoundingClientRect().bottom;
  var nodes=body.querySelectorAll('*');
  for(var index=0;index<nodes.length;index+=1){
    var candidate=nodes[index];
    var style=getComputedStyle(candidate);
    if(style.display==='none')continue;
    var rect=candidate.getBoundingClientRect();
    var marginBottom=parseFloat(style.marginBottom)||0;
    bottom=Math.max(bottom,rect.bottom+marginBottom);
  }
  return Math.max(1,Math.ceil(bottom));
}
function copyRuntimeState(source,clone){
  var sourceInputs=source.querySelectorAll('input');
  var cloneInputs=clone.querySelectorAll('input');
  for(var index=0;index<sourceInputs.length;index+=1){
    cloneInputs[index].setAttribute('value',sourceInputs[index].value);
    if(sourceInputs[index].checked)cloneInputs[index].setAttribute('checked','');
    else cloneInputs[index].removeAttribute('checked');
  }
  var sourceTextareas=source.querySelectorAll('textarea');
  var cloneTextareas=clone.querySelectorAll('textarea');
  for(var textareaIndex=0;textareaIndex<sourceTextareas.length;textareaIndex+=1){
    cloneTextareas[textareaIndex].textContent=sourceTextareas[textareaIndex].value;
  }
  var sourceOptions=source.querySelectorAll('option');
  var cloneOptions=clone.querySelectorAll('option');
  for(var optionIndex=0;optionIndex<sourceOptions.length;optionIndex+=1){
    if(sourceOptions[optionIndex].selected)cloneOptions[optionIndex].setAttribute('selected','');
    else cloneOptions[optionIndex].removeAttribute('selected');
  }
  var sourceCanvases=source.querySelectorAll('canvas');
  var cloneCanvases=clone.querySelectorAll('canvas');
  for(var canvasIndex=0;canvasIndex<sourceCanvases.length;canvasIndex+=1){
    try{
      var image=document.createElement('img');
      image.src=sourceCanvases[canvasIndex].toDataURL();
      image.width=sourceCanvases[canvasIndex].width;
      image.height=sourceCanvases[canvasIndex].height;
      cloneCanvases[canvasIndex].replaceWith(image);
    }catch{}
  }
}
function snapshot(height){
  var width=Math.max(1,document.documentElement.clientWidth);
  var clone=document.documentElement.cloneNode(true);
  copyRuntimeState(document,clone);
  clone.setAttribute('xmlns','http://www.w3.org/1999/xhtml');
  clone.style.width=width+'px';
  clone.style.height=height+'px';
  var blocked=clone.querySelectorAll('script,meta,link,base');
  for(var blockedIndex=0;blockedIndex<blocked.length;blockedIndex+=1)blocked[blockedIndex].remove();
  var elements=clone.querySelectorAll('*');
  for(var elementIndex=0;elementIndex<elements.length;elementIndex+=1){
    elements[elementIndex].style.setProperty('animation','none','important');
    elements[elementIndex].style.setProperty('transition','none','important');
  }
  var markup=new XMLSerializer().serializeToString(clone);
  var svg='<svg xmlns="http://www.w3.org/2000/svg" width="'+width+'" height="'+height+'" viewBox="0 0 '+width+' '+height+'"><foreignObject width="100%" height="100%">'+markup+'</foreignObject></svg>';
  parent.postMessage({height:height,svg:svg,type:snapshotType},'*');
}
function render(){
  scheduled=false;
  var height=contentHeight();
  parent.postMessage({height:height,type:resizeType},'*');
  snapshot(height);
}
function schedule(){
  if(scheduled)return;
  scheduled=true;
  setTimeout(render,0);
}
function start(){
  new ResizeObserver(schedule).observe(document.documentElement);
  new ResizeObserver(schedule).observe(document.body);
  new MutationObserver(schedule).observe(document.documentElement,{attributes:true,characterData:true,childList:true,subtree:true});
  document.addEventListener('load',schedule,true);
  document.addEventListener('click',function(event){
    var target=event.target;
    if(!(target instanceof Element))return;
    var link=target.closest('a[href^="locus-action:"]');
    if(!link)return;
    event.preventDefault();
    parent.postMessage({href:link.getAttribute('href'),type:actionType},'*');
  });
  window.addEventListener('resize',schedule);
  window.addEventListener('message',function(event){
    if(event.source===parent&&event.data&&event.data.type===renderType)schedule();
  });
  schedule();
}
if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',start,{once:true});
else start();
})();</script>`;
}

async function inlineCanvasAssets(
  source: string,
  loadAssetAsDataUrl?: (url: string) => Promise<string | null>,
): Promise<string> {
  if (!loadAssetAsDataUrl) return source;
  const template = document.createElement('template');
  template.innerHTML = source;
  const references = [
    ...template.content.querySelectorAll<HTMLElement>('[src], [poster]'),
  ].flatMap((element) =>
    ['src', 'poster'].flatMap((attribute) => {
      const sourceUrl = element.getAttribute(attribute);
      return sourceUrl ? [{ attribute, element, sourceUrl }] : [];
    }),
  );
  await Promise.all(
    references.map(async ({ attribute, element, sourceUrl }) => {
      const dataUrl = await loadAssetAsDataUrl(sourceUrl);
      if (dataUrl) element.setAttribute(attribute, dataUrl);
    }),
  );
  return template.innerHTML;
}

function canvasDocument(
  source: string,
  theme: CanvasTheme,
  assetOrigins: readonly string[],
): string {
  return [
    '<!doctype html>',
    `<html class="${theme}" lang="zh-CN" style="color-scheme:${theme}">`,
    '<head>',
    '<meta charset="utf-8">',
    `<meta http-equiv="Content-Security-Policy" content="${canvasContentSecurityPolicy(assetOrigins)}">`,
    '<meta name="viewport" content="width=device-width, initial-scale=1">',
    '<style type="text/tailwindcss">@custom-variant dark (&:where(.dark, .dark *));</style>',
    `<script>window.addEventListener("message",function(event){if(event.source!==window.parent)return;var data=event.data;if(!data||data.type!=="${canvasThemeMessageType}")return;var theme=data.theme==="dark"?"dark":"light";document.documentElement.classList.remove("light","dark");document.documentElement.classList.add(theme);document.documentElement.style.colorScheme=theme;});</script>`,
    `<script src="${tailwindBrowserScriptUrl}"></script>`,
    canvasRuntime(),
    '</head>',
    `<body>${source}</body>`,
    '</html>',
  ].join('');
}

function CanvasSurface({
  assetOrigins,
  canvas,
  describedBy,
  loadAssetAsDataUrl,
  onOpenDirectory,
  resolveUrl,
}: {
  assetOrigins: readonly string[];
  canvas: CanvasDescriptor;
  describedBy?: string;
  loadAssetAsDataUrl?: (url: string) => Promise<string | null>;
  onOpenDirectory?: (path: string) => Promise<unknown>;
  resolveUrl?: ResolveCanvasUrl;
}) {
  const frameRef = useRef<HTMLIFrameElement>(null);
  const snapshotUrlRef = useRef<string | null>(null);
  const [frameHeight, setFrameHeight] = useState(defaultCanvasHeight);
  const [snapshotUrl, setSnapshotUrl] = useState<string>();
  const [sourceDocument, setSourceDocument] = useState<string>();
  const [actionError, setActionError] = useState<string>();

  useEffect(() => {
    let current = true;
    const sanitizedSource = sanitizeCanvasFragment(canvas.source, resolveUrl);
    void inlineCanvasAssets(sanitizedSource, loadAssetAsDataUrl).then(
      (preparedSource) => {
        if (!current) return;
        setSourceDocument(
          canvasDocument(preparedSource, currentCanvasTheme(), assetOrigins),
        );
      },
    );
    return () => {
      current = false;
    };
  }, [assetOrigins, canvas.source, loadAssetAsDataUrl, resolveUrl]);

  useEffect(() => {
    function receiveCanvasMessage(event: MessageEvent) {
      if (event.source !== frameRef.current?.contentWindow) return;
      const data = event.data as Record<string, unknown> | null;
      if (!data || typeof data !== 'object') return;

      if (
        data.type === canvasActionMessageType &&
        typeof data.href === 'string'
      ) {
        const action = canvasDirectoryAction(data.href);
        if (!action || !onOpenDirectory) return;
        setActionError(undefined);
        void onOpenDirectory(action.path).catch((error: unknown) => {
          const message =
            error instanceof Error ? error.message : String(error);
          setActionError(`打开目录失败：${message}`);
        });
        return;
      }

      if (
        data.type === canvasResizeMessageType &&
        typeof data.height === 'number' &&
        Number.isInteger(data.height) &&
        data.height > 0
      ) {
        setFrameHeight(data.height);
        return;
      }

      if (
        data.type === canvasSnapshotMessageType &&
        typeof data.svg === 'string' &&
        data.svg.startsWith('<svg')
      ) {
        const nextUrl = URL.createObjectURL(
          new Blob([data.svg], { type: 'image/svg+xml;charset=utf-8' }),
        );
        if (snapshotUrlRef.current) URL.revokeObjectURL(snapshotUrlRef.current);
        snapshotUrlRef.current = nextUrl;
        setSnapshotUrl(nextUrl);
      }
    }

    window.addEventListener('message', receiveCanvasMessage);
    return () => window.removeEventListener('message', receiveCanvasMessage);
  }, [onOpenDirectory]);

  useEffect(
    () => () => {
      if (snapshotUrlRef.current) URL.revokeObjectURL(snapshotUrlRef.current);
    },
    [],
  );

  const syncCanvasTheme = useCallback(() => {
    const contentWindow = frameRef.current?.contentWindow;
    contentWindow?.postMessage(
      {
        theme: currentCanvasTheme(),
        type: canvasThemeMessageType,
      },
      '*',
    );
    contentWindow?.postMessage({ type: canvasRenderMessageType }, '*');
  }, []);

  useEffect(() => {
    const observer = new MutationObserver(syncCanvasTheme);
    observer.observe(document.documentElement, {
      attributeFilter: ['class'],
      attributes: true,
    });
    syncCanvasTheme();

    return () => observer.disconnect();
  }, [syncCanvasTheme]);

  return (
    <figure
      aria-describedby={describedBy}
      aria-label="画布"
      className="canvas-block document-wide-block"
      data-canvas-container
      data-canvas-export-status={snapshotUrl ? 'ready' : 'pending'}
      data-not-typeset
    >
      <iframe
        className="block w-full border-0"
        data-canvas-frame
        height={frameHeight}
        onLoad={syncCanvasTheme}
        referrerPolicy="no-referrer"
        ref={frameRef}
        sandbox="allow-popups allow-popups-to-escape-sandbox allow-scripts"
        srcDoc={sourceDocument}
        title="画布"
      />
      {actionError ? (
        <CanvasError message={actionError} title="无法打开目录" />
      ) : null}
      {snapshotUrl ? (
        <img
          alt=""
          aria-hidden="true"
          className="hidden w-full"
          data-canvas-export-snapshot
          hidden
          src={snapshotUrl}
        />
      ) : null}
    </figure>
  );
}
