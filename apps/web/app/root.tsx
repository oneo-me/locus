import {
  isRouteErrorResponse,
  Links,
  Meta,
  Outlet,
  Scripts,
  ScrollRestoration,
} from 'react-router';
import type { Route } from './+types/root';
import '@locus/ui/styles.css';

export const links: Route.LinksFunction = () => [
  {
    rel: 'icon',
    type: 'image/png',
    sizes: '32x32',
    href: '/icons/locus-mark-32.png',
  },
  {
    rel: 'icon',
    type: 'image/png',
    sizes: '16x16',
    href: '/icons/locus-mark-16.png',
  },
];

export function Layout({ children }: { children: React.ReactNode }) {
  return (
    <html
      className="overflow-hidden scroll-smooth font-sans motion-reduce:scroll-auto"
      lang="zh-CN"
      suppressHydrationWarning
    >
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta name="color-scheme" content="light dark" />
        <meta
          name="theme-color"
          content="#f7f9fc"
          media="(prefers-color-scheme: light)"
        />
        <meta
          name="theme-color"
          content="#121920"
          media="(prefers-color-scheme: dark)"
        />
        <meta
          name="description"
          content="Knowledge, woven together. 浏览与阅读本地 Markdown 笔记。"
        />
        <Meta />
        <Links />
        <script
          dangerouslySetInnerHTML={{
            __html: `(()=>{try{const theme=localStorage.getItem('locus-theme');const preference=theme==='light'||theme==='dark'?theme:'system';const dark=preference==='dark'||(preference==='system'&&window.matchMedia('(prefers-color-scheme: dark)').matches);document.querySelector('meta[name="color-scheme"]').content=preference==='system'?'light dark':preference;document.documentElement.classList.add(dark?'dark':'light')}catch{}})()`,
          }}
        />
      </head>
      <body className="h-dvh min-w-0 overflow-hidden bg-background text-foreground">
        {children}
        <ScrollRestoration />
        <Scripts />
      </body>
    </html>
  );
}

export default function App() {
  return <Outlet />;
}

export function ErrorBoundary({ error }: Route.ErrorBoundaryProps) {
  let message = '出错了';
  let details = '发生了意外错误。';

  if (isRouteErrorResponse(error)) {
    message = error.status === 404 ? '404' : '错误';
    details =
      error.status === 404 ? '找不到请求的页面。' : error.statusText || details;
  }

  return (
    <main className="container mx-auto p-4 pt-16">
      <h1>{message}</h1>
      <p>{details}</p>
    </main>
  );
}

export function meta() {
  return [{ title: 'Locus — Knowledge, woven together' }];
}
