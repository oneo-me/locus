import {
  DEFAULT_GLOBAL_SETTINGS,
  type GlobalSettings,
} from '@locus/document-library/contracts/global-settings';
import * as React from 'react';
import { useDocumentApi } from '~/lib/document-api-context';

type GlobalSettingsContextValue = {
  settings: GlobalSettings;
  updateSettings: (patch: Partial<GlobalSettings>) => Promise<GlobalSettings>;
};

const GlobalSettingsContext =
  React.createContext<GlobalSettingsContextValue | null>(null);

export function GlobalSettingsProvider({
  children,
  initialSettings = DEFAULT_GLOBAL_SETTINGS,
}: {
  children: React.ReactNode;
  initialSettings?: GlobalSettings;
}) {
  const { updateGlobalSettings } = useDocumentApi();
  const [settings, setSettings] =
    React.useState<GlobalSettings>(initialSettings);
  const settingsRef = React.useRef(settings);
  const mutationRef = React.useRef(0);

  const updateSettings = React.useCallback(
    async (patch: Partial<GlobalSettings>) => {
      const previous = settingsRef.current;
      const optimistic = { ...previous, ...patch };
      const mutation = mutationRef.current + 1;
      mutationRef.current = mutation;
      settingsRef.current = optimistic;
      setSettings(optimistic);
      try {
        const saved = await updateGlobalSettings(patch);
        if (mutationRef.current === mutation) {
          settingsRef.current = saved;
          setSettings(saved);
        }
        return saved;
      } catch (error) {
        if (mutationRef.current === mutation) {
          settingsRef.current = previous;
          setSettings(previous);
        }
        throw error;
      }
    },
    [updateGlobalSettings],
  );

  const value = React.useMemo(
    () => ({ settings, updateSettings }),
    [settings, updateSettings],
  );

  return (
    <GlobalSettingsContext.Provider value={value}>
      {children}
    </GlobalSettingsContext.Provider>
  );
}

export function useGlobalSettings(): GlobalSettingsContextValue {
  const context = React.useContext(GlobalSettingsContext);
  if (!context) {
    return {
      settings: DEFAULT_GLOBAL_SETTINGS,
      updateSettings: async () => DEFAULT_GLOBAL_SETTINGS,
    };
  }
  return context;
}
