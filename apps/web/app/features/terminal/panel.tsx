import { TerminalView } from '@shellweb/core/react';
import { deleteStoredTerminalSession } from '@shellweb/core/client';
import {
  useCallback,
  createContext,
  type ReactNode,
  useContext,
  useMemo,
  useState,
} from 'react';
import { useTheme } from '@locus/ui/theme-provider';
import { cn } from '@locus/ui/utils';
import { useGlobalSettings } from '../settings/global-settings-provider';

interface TerminalPanelState {
  open: boolean;
  show: () => void;
  toggle: () => Promise<void>;
}

const TerminalPanelContext = createContext<TerminalPanelState | null>(null);
const TERMINAL_PANEL_STORAGE_KEY = 'locus.library-terminal.panel.v1';
const TERMINAL_SESSION_STORAGE_KEY = 'locus.library-terminal.v1';

const TERMINAL_THEMES = {
  dark: {
    background: '#11151c',
    foreground: '#dce2ee',
    cursor: '#8a94a3',
    selectionBackground: '#43536a',
  },
  light: {
    background: '#fafafa',
    foreground: '#18181b',
    cursor: '#52525b',
    selectionBackground: '#d4d4d8',
  },
} as const;

export function TerminalPanelProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(() => {
    if (typeof window === 'undefined') return false;
    return sessionStorage.getItem(TERMINAL_PANEL_STORAGE_KEY) === 'open';
  });
  const show = useCallback(() => {
    sessionStorage.setItem(TERMINAL_PANEL_STORAGE_KEY, 'open');
    setOpen(true);
  }, []);
  const close = useCallback(async () => {
    await deleteStoredTerminalSession({
      storageKey: TERMINAL_SESSION_STORAGE_KEY,
    });
    sessionStorage.removeItem(TERMINAL_PANEL_STORAGE_KEY);
    setOpen(false);
  }, []);
  const toggle = useCallback(async () => {
    if (open) await close();
    else show();
  }, [close, open, show]);
  const value = useMemo(
    () => ({
      open,
      show,
      toggle,
    }),
    [open, show, toggle],
  );

  return (
    <TerminalPanelContext.Provider value={value}>
      {children}
    </TerminalPanelContext.Provider>
  );
}

export function useTerminalPanel(): TerminalPanelState {
  const value = useContext(TerminalPanelContext);
  if (!value) {
    throw new Error('useTerminalPanel 必须在 TerminalPanelProvider 内使用。');
  }
  return value;
}

export function LibraryTerminalPanel() {
  const { settings } = useGlobalSettings();
  const { resolvedTheme } = useTheme();
  const terminalTheme =
    settings.terminalTheme === 'auto' ? resolvedTheme : settings.terminalTheme;
  const terminalOptions = useMemo(
    () => ({
      cursorBlink: true,
      fontFamily:
        'GeistMonoShellweb, NotoSansSymbols2Shellweb, ui-monospace, monospace',
      fontSize: 13,
      lineHeight: 1.15,
      theme: TERMINAL_THEMES[terminalTheme],
    }),
    [terminalTheme],
  );

  return (
    <section
      aria-label="文档库终端"
      className="h-full min-h-0 min-w-0 border-l border-border"
    >
      <TerminalView
        className={cn(
          'h-full min-h-0!',
          terminalTheme === 'light'
            ? 'bg-[#fafafa]! text-[#18181b]!'
            : 'bg-[#11151c]! text-[#dce2ee]!',
        )}
        showToolbar={false}
        storageKey={TERMINAL_SESSION_STORAGE_KEY}
        terminal={terminalOptions}
      />
    </section>
  );
}
