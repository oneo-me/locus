import * as React from 'react';

export type Theme = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'dark' | 'light';

export function colorSchemeForTheme(theme: Theme): string {
  return theme === 'system' ? 'light dark' : theme;
}

/**
 * 与 `index.html` 中首屏内联脚本共用的存储键，
 * 两侧必须保持一致。localStorage 仅作首屏防闪缓存；
 * 权威来源是 `~/.oneo/asterweft/settings.json`。
 */
export const THEME_STORAGE_KEY = 'asterweft-theme';

type ThemeProviderProps = {
  children: React.ReactNode;
  defaultTheme?: Theme;
  disableTransitionOnChange?: boolean;
  onThemeChange?: (theme: Theme) => unknown;
};

type ThemeProviderState = {
  theme: Theme;
  resolvedTheme: ResolvedTheme;
  setTheme: (theme: Theme) => void;
};

const COLOR_SCHEME_QUERY = '(prefers-color-scheme: dark)';

const ThemeProviderContext = React.createContext<ThemeProviderState | null>(
  null,
);

const DEFAULT_THEME_STATE: ThemeProviderState = {
  theme: 'system',
  resolvedTheme: 'light',
  setTheme: () => {},
};

// 布局效应在绘制前应用主题类，并保证先于子组件的被动效应执行，
// 使知识图谱等读取计算样式的消费者始终拿到新主题。
const useIsomorphicLayoutEffect =
  typeof window === 'undefined' ? React.useEffect : React.useLayoutEffect;

function getSystemTheme(): ResolvedTheme {
  if (window.matchMedia(COLOR_SCHEME_QUERY).matches) {
    return 'dark';
  }

  return 'light';
}

function readStoredTheme(): Theme | null {
  if (typeof window === 'undefined') return null;

  try {
    const value = window.localStorage.getItem(THEME_STORAGE_KEY);
    return value === 'light' || value === 'dark' || value === 'system'
      ? value
      : null;
  } catch {
    return null;
  }
}

function writeStoredTheme(theme: Theme) {
  try {
    window.localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // 存储不可用时仅保持当前会话内的主题。
  }
}

function writeDocumentColorScheme(theme: Theme) {
  const meta = document.querySelector<HTMLMetaElement>(
    'meta[name="color-scheme"]',
  );
  if (meta) meta.content = colorSchemeForTheme(theme);
}

function disableTransitionsTemporarily() {
  const style = document.createElement('style');
  style.appendChild(
    document.createTextNode(
      '*,*::before,*::after{-webkit-transition:none!important;transition:none!important}',
    ),
  );
  document.head.appendChild(style);

  return () => {
    window.getComputedStyle(document.body);
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        style.remove();
      });
    });
  };
}

export function ThemeProvider({
  children,
  defaultTheme = 'system',
  disableTransitionOnChange = true,
  onThemeChange,
}: ThemeProviderProps) {
  const [theme, setThemeState] = React.useState<Theme>(defaultTheme);
  // 保持稳定初值，真实系统偏好在布局效应中同步（见下）。
  const [systemTheme, setSystemTheme] = React.useState<ResolvedTheme>('light');
  const resolvedTheme = theme === 'system' ? systemTheme : theme;

  useIsomorphicLayoutEffect(() => {
    // 首屏类名已由内联脚本应用；这里先把 React 状态同步到存储与系统偏好，
    // 并在同一帧用「即将生效」的解析主题写回 class，避免先盖成 light 再闪暗色。
    const nextSystemTheme = getSystemTheme();
    if (nextSystemTheme !== systemTheme) {
      setSystemTheme(nextSystemTheme);
    }

    const storedTheme = readStoredTheme();
    const nextTheme = storedTheme ?? theme;
    if (storedTheme !== null && storedTheme !== theme) {
      setThemeState(storedTheme);
    }

    const nextResolved: ResolvedTheme =
      nextTheme === 'system' ? nextSystemTheme : nextTheme;

    const root = document.documentElement;
    const restoreTransitions = disableTransitionOnChange
      ? disableTransitionsTemporarily()
      : null;

    root.classList.remove('light', 'dark');
    root.classList.add(nextResolved);
    writeDocumentColorScheme(nextTheme);

    restoreTransitions?.();
  }, [disableTransitionOnChange, systemTheme, theme]);

  React.useEffect(() => {
    const mediaQuery = window.matchMedia(COLOR_SCHEME_QUERY);
    const handleChange = () => setSystemTheme(getSystemTheme());
    mediaQuery.addEventListener('change', handleChange);

    return () => {
      mediaQuery.removeEventListener('change', handleChange);
    };
  }, []);

  const setTheme = React.useCallback(
    (nextTheme: Theme) => {
      setThemeState(nextTheme);
      writeStoredTheme(nextTheme);
      void onThemeChange?.(nextTheme);
    },
    [onThemeChange],
  );

  const value = React.useMemo<ThemeProviderState>(
    () => ({ theme, resolvedTheme, setTheme }),
    [theme, resolvedTheme, setTheme],
  );

  return (
    <ThemeProviderContext.Provider value={value}>
      {children}
    </ThemeProviderContext.Provider>
  );
}

export function useTheme(): ThemeProviderState {
  const context = React.useContext(ThemeProviderContext);
  if (!context) {
    return DEFAULT_THEME_STATE;
  }
  return context;
}
