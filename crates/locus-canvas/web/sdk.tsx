import { createContext, useContext, useSyncExternalStore } from 'react';
import exportedResources from 'virtual:locus-resources';

export interface CanvasContext {
  note: string;
}

export const CanvasProvider = createContext<CanvasContext | null>(null);

export function useCanvas(): CanvasContext {
  const context = useContext(CanvasProvider);
  if (!context) throw new Error('useCanvas must be used inside a Locus canvas');
  return context;
}

export function resourceUrl(address: string): string {
  if (exportedResources) {
    const target = exportedResources[address];
    if (!target) throw new Error(`Resource is not included in this export: ${address}`);
    return new URL(target, document.baseURI).href;
  }
  return `/__locus/resource?uri=${encodeURIComponent(address)}`;
}

export async function readText(address: string): Promise<string> {
  const response = await fetch(resourceUrl(address));
  if (!response.ok) throw new Error(`Could not read ${address}: ${response.status}`);
  return response.text();
}

function subscribeTheme(listener: () => void) {
  const observer = new MutationObserver(listener);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] });
  return () => observer.disconnect();
}

export function useTheme(): 'dark' | 'light' {
  return useSyncExternalStore(subscribeTheme,
    () => document.documentElement.classList.contains('dark') ? 'dark' : 'light', () => 'light');
}

export function subscribeResource(address: string, listener: () => void): () => void {
  const receive = (event: CustomEvent<{ uri: string }>) => {
    if (event.detail.uri === address.split('?')[0]) listener();
  };
  window.addEventListener('locus:resource-changed', receive as EventListener);
  return () => window.removeEventListener('locus:resource-changed', receive as EventListener);
}

if (import.meta.hot) {
  const receive = (detail: { uri: string }) => window.dispatchEvent(new CustomEvent('locus:resource-changed', { detail }));
  import.meta.hot.on('locus:resource-changed', receive);
  import.meta.hot.dispose(() => import.meta.hot?.off('locus:resource-changed', receive));
}
