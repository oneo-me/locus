import { accessSync, constants, statSync } from 'node:fs';
import { homedir } from 'node:os';
import { delimiter, join } from 'node:path';
import {
  AGENT_PROVIDERS,
  type AgentProviderDetection,
  type AgentProviderId,
} from '../contracts/provider.js';
import {
  discoverAgentProviderModels,
  fallbackAgentProviderModels,
} from './provider-models.js';

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
  if (process.platform !== 'win32') {
    return [
      '/opt/homebrew/bin',
      '/usr/local/bin',
      '/usr/bin',
      '/bin',
      '/usr/sbin',
      '/sbin',
    ];
  }
  const directories: string[] = [];
  if (process.env.ProgramFiles) {
    directories.push(join(process.env.ProgramFiles, 'nodejs'));
  }
  if (process.env.SystemRoot) {
    const system32 = join(process.env.SystemRoot, 'System32');
    directories.push(join(system32, 'WindowsPowerShell/v1.0'), system32);
  }
  return directories;
}

function executableNames(command: string): string[] {
  if (process.platform !== 'win32') return [command];
  const extensions = (process.env.PATHEXT || '.COM;.EXE;.BAT;.CMD')
    .split(';')
    .map((extension) => extension.trim())
    .filter(Boolean);
  return extensions.map((extension) => `${command}${extension}`);
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

export function findAgentProviderExecutable(
  providerId: AgentProviderId,
): string | null {
  const provider = AGENT_PROVIDERS.find(
    (candidate) => candidate.id === providerId,
  );
  return provider
    ? findExecutable(provider.command, executableSearchDirectories())
    : null;
}

export function detectAgentProviders(): AgentProviderDetection {
  const directories = executableSearchDirectories();
  const providers = AGENT_PROVIDERS.map((provider) => {
    const path = findExecutable(provider.command, directories);
    return {
      ...provider,
      installed: path !== null,
      models: fallbackAgentProviderModels(provider.id),
      path,
    };
  });
  providers.sort(
    (left, right) => Number(right.installed) - Number(left.installed),
  );
  return {
    checkedAt: new Date().toISOString(),
    providers,
  };
}

let cachedDetection: AgentProviderDetection | null = null;

export async function detectAgentProvidersWithModels(
  refresh = false,
  modelCacheDirectory?: string,
): Promise<AgentProviderDetection> {
  if (!refresh && cachedDetection) return structuredClone(cachedDetection);
  const detection = detectAgentProviders();
  await Promise.all(
    detection.providers.map(async (provider) => {
      if (!provider.path) return;
      provider.models = await discoverAgentProviderModels(
        provider.id,
        provider.path,
        modelCacheDirectory,
      );
    }),
  );
  cachedDetection = detection;
  return structuredClone(detection);
}
