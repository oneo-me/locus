import { accessSync, constants, statSync } from 'node:fs';
import { homedir } from 'node:os';
import { delimiter, join } from 'node:path';
import type { AgentProviderDetection } from '../contracts/provider.js';
import type { AgentProvider } from '../providers/protocol.js';
import { resolveAgentProviderModels } from './provider-models.js';

export class AgentProviderRegistry {
  readonly #providers: AgentProvider[];
  #cachedDetection: AgentProviderDetection | null = null;

  constructor(providers: AgentProvider[]) {
    const ids = new Set<string>();
    for (const provider of providers) {
      if (ids.has(provider.id))
        throw new Error(`Agent provider 重复：${provider.id}。`);
      ids.add(provider.id);
    }
    this.#providers = [...providers];
  }

  get(id: string): AgentProvider | null {
    return this.#providers.find((provider) => provider.id === id) ?? null;
  }

  findExecutable(id: string): string | null {
    const provider = this.get(id);
    return provider
      ? findExecutable(provider.command, executableSearchDirectories())
      : null;
  }

  detect(): AgentProviderDetection {
    const directories = executableSearchDirectories();
    const providers = this.#providers.map((provider) => {
      const path = findExecutable(provider.command, directories);
      return {
        command: provider.command,
        id: provider.id,
        installed: path !== null,
        models: provider.fallbackModels.map((model) => ({ ...model })),
        name: provider.name,
        path,
        supportsTasks: Boolean(provider.createRuntime),
      };
    });
    providers.sort(
      (left, right) => Number(right.installed) - Number(left.installed),
    );
    return { checkedAt: new Date().toISOString(), providers };
  }

  async detectWithModels(
    refresh = false,
    modelCacheDirectory?: string,
  ): Promise<AgentProviderDetection> {
    if (!refresh && this.#cachedDetection)
      return structuredClone(this.#cachedDetection);
    const detection = this.detect();
    await Promise.all(
      detection.providers.map(async (probe) => {
        if (!probe.path) return;
        const provider = this.get(probe.id);
        if (!provider) return;
        probe.models = await resolveAgentProviderModels(
          provider,
          probe.path,
          modelCacheDirectory,
        );
      }),
    );
    this.#cachedDetection = detection;
    return structuredClone(detection);
  }
}

function executableSearchDirectories(): string[] {
  const home = homedir();
  const directories = [
    ...(process.env.PATH ?? '').split(delimiter),
    ...userToolDirectories(home),
    ...systemToolDirectories(),
  ];
  return [...new Set(directories.filter(Boolean))];
}

function userToolDirectories(home: string): string[] {
  if (process.platform !== 'win32') {
    return [
      join(home, '.local/bin'),
      join(home, '.bun/bin'),
      join(home, '.cargo/bin'),
      join(home, '.local/share/mise/shims'),
      join(home, '.volta/bin'),
    ];
  }
  const localAppData = process.env.LOCALAPPDATA;
  return [
    join(process.env.APPDATA ?? join(home, 'AppData/Roaming'), 'npm'),
    join(home, '.bun/bin'),
    join(home, '.cargo/bin'),
    join(home, 'scoop/shims'),
    join(home, 'AppData/Local/Microsoft/WindowsApps'),
    join(home, '.local/bin'),
    ...(localAppData
      ? [
          join(localAppData, 'Volta/bin'),
          join(localAppData, 'pnpm'),
          join(localAppData, 'Programs/nodejs'),
        ]
      : []),
  ];
}

function systemToolDirectories(): string[] {
  if (process.platform !== 'win32')
    return [
      '/opt/homebrew/bin',
      '/usr/local/bin',
      '/usr/bin',
      '/bin',
      '/usr/sbin',
      '/sbin',
    ];
  const directories: string[] = [];
  if (process.env.ProgramFiles)
    directories.push(join(process.env.ProgramFiles, 'nodejs'));
  if (process.env.SystemRoot) {
    const system32 = join(process.env.SystemRoot, 'System32');
    directories.push(join(system32, 'WindowsPowerShell/v1.0'), system32);
  }
  return directories;
}

function executableNames(command: string): string[] {
  if (process.platform !== 'win32') return [command];
  return (process.env.PATHEXT || '.COM;.EXE;.BAT;.CMD')
    .split(';')
    .map((extension) => extension.trim())
    .filter(Boolean)
    .map((extension) => `${command}${extension}`);
}

function findExecutable(command: string, directories: string[]): string | null {
  for (const directory of directories) {
    for (const name of executableNames(command)) {
      const candidate = join(directory, name);
      try {
        if (!statSync(candidate).isFile()) continue;
        accessSync(candidate, constants.X_OK);
        return candidate;
      } catch {
        continue;
      }
    }
  }
  return null;
}
