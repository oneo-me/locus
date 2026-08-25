import { SettingsBrowser } from '~/features/settings/browser';
import { useLocation } from 'react-router';

export default function SettingsRoute() {
  const location = useLocation();
  const libraryError =
    location.state !== null &&
    typeof location.state === 'object' &&
    'libraryError' in location.state &&
    typeof location.state.libraryError === 'string'
      ? location.state.libraryError
      : undefined;
  return <SettingsBrowser libraryError={libraryError} />;
}
