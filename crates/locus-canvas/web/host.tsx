import { Component, Suspense, lazy, type ReactNode } from 'react';
import { createRoot } from 'react-dom/client';
import { CanvasProvider } from './sdk';
import './host.css';

const note = document.documentElement.dataset.note!;
const token = btoa(Array.from(new TextEncoder().encode(note), byte => String.fromCharCode(byte)).join('')).replaceAll('+', '-').replaceAll('/', '_').replaceAll('=', '');
const Canvas = lazy(() => import(/* @vite-ignore */ `/__locus/entry.tsx?note=${token}&import`));

class CanvasError extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) { return { error }; }
  render() {
    if (this.state.error) {
      const chinese = document.documentElement.lang === 'zh-CN';
      return <main role="alert" className="locus-error"><p>{chinese ? '无法打开画布' : 'Could not open canvas'}</p>
        <pre>{this.state.error.message}</pre><button onClick={() => location.reload()}>{chinese ? '刷新' : 'Reload'}</button></main>;
    }
    return this.props.children;
  }
}

createRoot(document.getElementById('root')!).render(
  <CanvasError><CanvasProvider value={{ note }}>
    <Suspense fallback={null}><Canvas /></Suspense>
  </CanvasProvider></CanvasError>,
);
