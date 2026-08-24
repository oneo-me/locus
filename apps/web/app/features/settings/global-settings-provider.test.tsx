import { DEFAULT_GLOBAL_SETTINGS } from '~/contracts/global-settings';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, test } from 'vitest';
import {
  GlobalSettingsProvider,
  useGlobalSettings,
} from './global-settings-provider';

function GraphAttachmentsValue() {
  const { settings } = useGlobalSettings();
  return <span>{String(settings.showGraphAttachments)}</span>;
}

describe('GlobalSettingsProvider', () => {
  test('uses server-loaded settings for the initial render', () => {
    const markup = renderToStaticMarkup(
      <GlobalSettingsProvider
        initialSettings={{
          ...DEFAULT_GLOBAL_SETTINGS,
          showGraphAttachments: false,
        }}
      >
        <GraphAttachmentsValue />
      </GlobalSettingsProvider>,
    );

    expect(markup).toBe('<span>false</span>');
  });
});
