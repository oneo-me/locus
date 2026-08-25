import { SettingsBrowser } from '~/features/settings/browser';
import { useLocation, useSearchParams } from 'react-router';

export default function SettingsRoute() {
  const location = useLocation();
  const [searchParams] = useSearchParams();
  const libraryError =
    location.state !== null &&
    typeof location.state === 'object' &&
    'libraryError' in location.state &&
    typeof location.state.libraryError === 'string'
      ? location.state.libraryError
      : undefined;
  return (
    <SettingsBrowser
      initialSection={
        searchParams.get('section') === 'library' ? 'library' : 'appearance'
      }
      libraryError={libraryError}
    />
  );
}
