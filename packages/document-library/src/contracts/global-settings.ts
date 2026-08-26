export type GlobalTheme = 'light' | 'dark' | 'system';
export type TerminalTheme = 'auto' | 'light' | 'dark';

export interface TaskDraftPreference {
  modelId: string | null;
  projectId: string | null;
  providerId: string | null;
}

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
  defaultDocumentPath: string | null;
  theme: GlobalTheme;
  commandPaletteDefaultExpanded: boolean;
  showGraphAttachments: boolean;
  mcpEnabled: boolean;
  taskDraft: TaskDraftPreference;
  workspaceSidebarWidth: number;
  terminalShell: string | null;
  terminalStartupCommand: string;
  terminalTheme: TerminalTheme;
}

export interface UpdateGlobalSettingsInput {
  libraryPath?: string;
  defaultDocumentPath?: string | null;
  theme?: GlobalTheme;
  commandPaletteDefaultExpanded?: boolean;
  showGraphAttachments?: boolean;
  mcpEnabled?: boolean;
  taskDraft?: TaskDraftPreference;
  workspaceSidebarWidth?: number;
  terminalShell?: string | null;
  terminalStartupCommand?: string;
  terminalTheme?: TerminalTheme;
}

export const DEFAULT_GLOBAL_SETTINGS: GlobalSettings = {
  libraryPath: null,
  defaultDocumentPath: null,
  theme: 'system',
  commandPaletteDefaultExpanded: false,
  showGraphAttachments: true,
  mcpEnabled: false,
  taskDraft: {
    modelId: null,
    projectId: null,
    providerId: null,
  },
  workspaceSidebarWidth: DEFAULT_WORKSPACE_SIDEBAR_WIDTH,
  terminalShell: null,
  terminalStartupCommand: '',
  terminalTheme: 'auto',
};

export function isGlobalTheme(value: unknown): value is GlobalTheme {
  return value === 'light' || value === 'dark' || value === 'system';
}

export function isTerminalTheme(value: unknown): value is TerminalTheme {
  return value === 'auto' || value === 'light' || value === 'dark';
}

function normalizePreferenceId(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function normalizeTaskDraft(value: unknown): TaskDraftPreference {
  const record =
    value && typeof value === 'object'
      ? (value as Record<string, unknown>)
      : {};
  return {
    modelId: normalizePreferenceId(record.modelId),
    projectId: normalizePreferenceId(record.projectId),
    providerId: normalizePreferenceId(record.providerId),
  };
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
    defaultDocumentPath:
      typeof record.defaultDocumentPath === 'string' &&
      record.defaultDocumentPath.trim()
        ? record.defaultDocumentPath.trim()
        : DEFAULT_GLOBAL_SETTINGS.defaultDocumentPath,
    theme: isGlobalTheme(record.theme)
      ? record.theme
      : DEFAULT_GLOBAL_SETTINGS.theme,
    commandPaletteDefaultExpanded:
      typeof record.commandPaletteDefaultExpanded === 'boolean'
        ? record.commandPaletteDefaultExpanded
        : DEFAULT_GLOBAL_SETTINGS.commandPaletteDefaultExpanded,
    showGraphAttachments:
      typeof record.showGraphAttachments === 'boolean'
        ? record.showGraphAttachments
        : DEFAULT_GLOBAL_SETTINGS.showGraphAttachments,
    mcpEnabled:
      typeof record.mcpEnabled === 'boolean'
        ? record.mcpEnabled
        : DEFAULT_GLOBAL_SETTINGS.mcpEnabled,
    taskDraft: normalizeTaskDraft(record.taskDraft),
    workspaceSidebarWidth: normalizeWorkspaceSidebarWidth(
      record.workspaceSidebarWidth,
    ),
    terminalShell:
      typeof record.terminalShell === 'string' && record.terminalShell.trim()
        ? record.terminalShell.trim()
        : DEFAULT_GLOBAL_SETTINGS.terminalShell,
    terminalStartupCommand:
      typeof record.terminalStartupCommand === 'string'
        ? record.terminalStartupCommand.trim()
        : DEFAULT_GLOBAL_SETTINGS.terminalStartupCommand,
    terminalTheme: isTerminalTheme(record.terminalTheme)
      ? record.terminalTheme
      : DEFAULT_GLOBAL_SETTINGS.terminalTheme,
  };
}
