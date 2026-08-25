export type GlobalTheme = 'light' | 'dark' | 'system';

export const DEFAULT_WORKSPACE_SIDEBAR_WIDTH = 304;
export const MIN_WORKSPACE_SIDEBAR_WIDTH = DEFAULT_WORKSPACE_SIDEBAR_WIDTH;
export const MAX_WORKSPACE_SIDEBAR_WIDTH = 440;

export function normalizeWorkspaceSidebarWidth(value: unknown): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return DEFAULT_WORKSPACE_SIDEBAR_WIDTH;
  }
  return Math.min(
    MAX_WORKSPACE_SIDEBAR_WIDTH,
    Math.max(MIN_WORKSPACE_SIDEBAR_WIDTH, Math.round(value)),
  );
}

export interface GlobalSettings {
  libraryPath: string | null;
  theme: GlobalTheme;
  mcpEnabled: boolean;
  workspaceSidebarWidth: number;
}

export interface UpdateGlobalSettingsInput {
  libraryPath?: string;
  theme?: GlobalTheme;
  mcpEnabled?: boolean;
  workspaceSidebarWidth?: number;
}

export const DEFAULT_GLOBAL_SETTINGS: GlobalSettings = {
  libraryPath: null,
  theme: 'system',
  mcpEnabled: false,
  workspaceSidebarWidth: DEFAULT_WORKSPACE_SIDEBAR_WIDTH,
};

export function isGlobalTheme(value: unknown): value is GlobalTheme {
  return value === 'light' || value === 'dark' || value === 'system';
}

export function normalizeGlobalSettings(value: unknown): GlobalSettings {
  if (!value || typeof value !== 'object') {
    return { ...DEFAULT_GLOBAL_SETTINGS };
  }

  const record = value as Record<string, unknown>;
  return {
    libraryPath:
      typeof record.libraryPath === 'string' && record.libraryPath.trim()
        ? record.libraryPath.trim()
        : DEFAULT_GLOBAL_SETTINGS.libraryPath,
    theme: isGlobalTheme(record.theme)
      ? record.theme
      : DEFAULT_GLOBAL_SETTINGS.theme,
    mcpEnabled:
      typeof record.mcpEnabled === 'boolean'
        ? record.mcpEnabled
        : DEFAULT_GLOBAL_SETTINGS.mcpEnabled,
    workspaceSidebarWidth: normalizeWorkspaceSidebarWidth(
      record.workspaceSidebarWidth,
    ),
  };
}
