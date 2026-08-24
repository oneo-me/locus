export type GlobalTheme = 'light' | 'dark' | 'system';

export const DEFAULT_WORKSPACE_SIDEBAR_WIDTH = 284;
export const MIN_WORKSPACE_SIDEBAR_WIDTH = DEFAULT_WORKSPACE_SIDEBAR_WIDTH;
export const MAX_WORKSPACE_SIDEBAR_WIDTH = 420;

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
  theme: GlobalTheme;
  showGraphAttachments: boolean;
  mcpEnabled: boolean;
  workspaceSidebarWidth: number;
}

export interface UpdateGlobalSettingsInput {
  theme?: GlobalTheme;
  showGraphAttachments?: boolean;
  mcpEnabled?: boolean;
  workspaceSidebarWidth?: number;
}

export const DEFAULT_GLOBAL_SETTINGS: GlobalSettings = {
  theme: 'system',
  showGraphAttachments: true,
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
    theme: isGlobalTheme(record.theme)
      ? record.theme
      : DEFAULT_GLOBAL_SETTINGS.theme,
    showGraphAttachments:
      typeof record.showGraphAttachments === 'boolean'
        ? record.showGraphAttachments
        : DEFAULT_GLOBAL_SETTINGS.showGraphAttachments,
    mcpEnabled:
      typeof record.mcpEnabled === 'boolean'
        ? record.mcpEnabled
        : DEFAULT_GLOBAL_SETTINGS.mcpEnabled,
    workspaceSidebarWidth: normalizeWorkspaceSidebarWidth(
      record.workspaceSidebarWidth,
    ),
  };
}
