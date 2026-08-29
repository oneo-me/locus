import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createInterface } from 'node:readline';

export interface JsonLineProcessOptions {
  args: string[];
  binary: string;
  cwd: string;
  env?: Record<string, string>;
  onExit: (message: string | null) => void;
  onMessage: (message: Record<string, unknown>) => void;
}

export class JsonLineProcess {
  readonly #child: ChildProcessWithoutNullStreams;
  #closed = false;
  #exitReported = false;
  #stderr = '';

  constructor(options: JsonLineProcessOptions) {
    this.#child = spawn(options.binary, options.args, {
      cwd: options.cwd,
      env: { ...process.env, ...options.env },
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    this.#child.stderr.setEncoding('utf8');
    this.#child.stderr.on('data', (chunk: string) => {
      this.#stderr = `${this.#stderr}${chunk}`.slice(-16_384);
    });
    const lines = createInterface({ input: this.#child.stdout });
    lines.on('line', (line) => {
      try {
        const value = JSON.parse(line) as unknown;
        if (isRecord(value)) options.onMessage(value);
      } catch {
        return;
      }
    });
    this.#child.once('error', (error) => {
      if (!this.#closed && !this.#exitReported) {
        this.#exitReported = true;
        options.onExit(error.message);
      }
    });
    this.#child.once('exit', (code, signal) => {
      if (this.#closed || this.#exitReported) return;
      this.#exitReported = true;
      const detail = this.#stderr.trim();
      options.onExit(
        detail || `进程已退出（${signal ?? code ?? 'unknown'}）。`,
      );
    });
  }

  send(value: unknown): void {
    if (this.#closed || !this.#child.stdin.writable) {
      throw new Error('Agent 进程输入流已关闭。');
    }
    this.#child.stdin.write(`${JSON.stringify(value)}\n`);
  }

  close(): void {
    if (this.#closed) return;
    this.#closed = true;
    this.#child.stdin.end();
    const timeout = setTimeout(() => {
      if (this.#child.exitCode === null) this.#child.kill();
    }, 5_000);
    timeout.unref();
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
