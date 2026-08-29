export class NativeTitleRefresh {
  #state: 'idle' | 'resolved' | 'running' = 'idle';
  #token: symbol | null = null;

  start(
    delays: readonly number[],
    lookup: () => string | null,
    onTitle: (title: string) => void,
  ): void {
    if (this.#state !== 'idle') return;
    this.#state = 'running';
    const token = Symbol();
    this.#token = token;
    void this.#run(token, delays, lookup, onTitle);
  }

  resolve(): void {
    this.#state = 'resolved';
    this.#token = null;
  }

  close(): void {
    this.#state = 'idle';
    this.#token = null;
  }

  async #run(
    token: symbol,
    delays: readonly number[],
    lookup: () => string | null,
    onTitle: (title: string) => void,
  ): Promise<void> {
    for (const delay of delays) {
      if (delay > 0) await refreshDelay(delay);
      if (this.#token !== token) return;
      let title: string | null;
      try {
        title = lookup()?.trim() || null;
      } catch {
        continue;
      }
      if (!title) continue;
      this.resolve();
      onTitle(title);
      return;
    }
    if (this.#token === token) {
      this.#state = 'idle';
      this.#token = null;
    }
  }
}

function refreshDelay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => {
    const timer = setTimeout(resolve, milliseconds);
    timer.unref();
  });
}
