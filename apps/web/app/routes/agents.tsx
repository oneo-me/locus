import { AgentsShell } from '../features/agents/shell';

export default function AgentsRoute() {
  return <AgentsShell />;
}

export function meta() {
  return [{ title: 'Agent — Locus' }];
}
