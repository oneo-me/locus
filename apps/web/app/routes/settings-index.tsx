import { redirect } from 'react-router';

export function clientLoader() {
  throw redirect('/settings/general');
}

export default function SettingsIndexRoute() {
  return null;
}
