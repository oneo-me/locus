import { redirect } from 'react-router';

export function clientLoader() {
  throw redirect('/notes');
}

export default function AppIndexRoute() {
  return null;
}
