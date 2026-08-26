import { spawn, type ChildProcess } from 'node:child_process';
import { createServer } from 'node:net';
import { realpath } from 'node:fs/promises';

const START_TIMEOUT = 10_000;
const REQUEST_TIMEOUT = 10_000;

export class OpenCodeServer {
  readonly baseUrl: string;
  readonly #child: ChildProcess;
  #closed = false;

  private constructor(child: ChildProcess, port: number) {
    this.#child = child;
    this.baseUrl = `http://127.0.0.1:${port}`;
  }

  static async start(binary: string, cwd: string): Promise<OpenCodeServer> {
    const port = await reservePort();
    const child = spawn(
      binary,
      ['serve', '--hostname', '127.0.0.1', '--port', String(port)],
      {
        cwd,
        env: {
          ...process.env,
          OPENCODE_SERVER_PASSWORD: '',
          OPENCODE_SERVER_USERNAME: 'opencode',
        },
        stdio: 'ignore',
      },
    );
    let startupError: Error | null = null;
    child.once('error', (error) => {
      startupError = error;
    });
    const server = new OpenCodeServer(child, port);
    const startedAt = Date.now();
    while (Date.now() - startedAt < START_TIMEOUT) {
      if (startupError) throw startupError;
      if (child.exitCode !== null) {
        throw new Error(`OpenCode server 启动时退出（${child.exitCode}）。`);
      }
      try {
        await server.request('GET', '/global/health', undefined, 1_000);
        return server;
      } catch {
        await delay(40);
      }
    }
    await server.close();
    throw new Error('OpenCode server 启动超时。');
  }

  isAlive(): boolean {
    return !this.#closed && this.#child.exitCode === null;
  }

  async request(
    method: string,
    path: string,
    body?: unknown,
    timeout = REQUEST_TIMEOUT,
  ): Promise<unknown> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      body: body === undefined ? undefined : JSON.stringify(body),
      headers:
        body === undefined ? undefined : { 'content-type': 'application/json' },
      method,
      signal: AbortSignal.timeout(timeout),
    });
    if (!response.ok) {
      const detail = (await response.text()).trim();
      throw new Error(
        detail || `OpenCode 请求失败（HTTP ${response.status}）。`,
      );
    }
    if (response.status === 204) return null;
    const text = await response.text();
    return text ? JSON.parse(text) : null;
  }

  async close(): Promise<void> {
    if (this.#closed) return;
    this.#closed = true;
    if (this.#child.exitCode !== null) return;
    const exited = new Promise<void>((resolve) =>
      this.#child.once('exit', () => resolve()),
    );
    this.#child.kill();
    const stopped = await Promise.race([
      exited.then(() => true),
      delay(5_000).then(() => false),
    ]);
    if (!stopped) {
      this.#child.kill('SIGKILL');
      await exited;
    }
  }
}

interface PoolEntry {
  refs: number;
  releasing: Promise<void> | null;
  server: OpenCodeServer;
}

export interface OpenCodeServerLease {
  release(): void;
  server: OpenCodeServer;
}

const pool = new Map<string, Promise<PoolEntry>>();

export async function acquireOpenCodeServer(
  binary: string,
  cwd: string,
): Promise<OpenCodeServerLease> {
  const key = `${await realpath(binary)}\0${await realpath(cwd)}`;
  while (true) {
    let pending = pool.get(key);
    if (!pending) {
      pending = OpenCodeServer.start(binary, cwd).then((server) => ({
        refs: 0,
        releasing: null,
        server,
      }));
      pool.set(key, pending);
      void pending.catch(() => {
        if (pool.get(key) === pending) pool.delete(key);
      });
    }
    const entry = await pending;
    if (entry.releasing) {
      await entry.releasing;
      continue;
    }
    if (!entry.server.isAlive()) {
      if (pool.get(key) === pending) pool.delete(key);
      continue;
    }
    entry.refs += 1;
    let released = false;
    return {
      release() {
        if (released) return;
        released = true;
        entry.refs -= 1;
        if (entry.refs !== 0) return;
        entry.releasing = entry.server.close().finally(() => {
          if (pool.get(key) === pending) pool.delete(key);
        });
      },
      server: entry.server,
    };
  }
}

async function reservePort(): Promise<number> {
  const server = createServer();
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  if (!address || typeof address === 'string') {
    server.close();
    throw new Error('无法为 OpenCode server 分配端口。');
  }
  await new Promise<void>((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
  return address.port;
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
