import type { AgentProject } from '@locus/agent/contracts/conversation';
import {
  isAcpAgentProviderId,
  type AgentProviderId,
  type AgentProviderProbe,
} from '@locus/agent/contracts/provider';
import { FolderIcon } from 'lucide-react';
import { cn } from '@locus/ui/utils';
import { AgentProviderIcon } from '~/features/agent/provider-icon';
import { userPath } from './user-path';

export function NewTask({
  onProjectChange,
  onProviderChange,
  pending,
  projectId,
  projects,
  providerId,
  providers,
}: {
  onProjectChange: (id: string) => void;
  onProviderChange: (id: AgentProviderId) => void;
  pending: boolean;
  projectId: string | null;
  projects: AgentProject[];
  providerId: AgentProviderId | null;
  providers: AgentProviderProbe[];
}) {
  const agents = providers.filter(
    (provider) => provider.installed && isAcpAgentProviderId(provider.id),
  );

  return (
    <div className="m-auto grid w-full max-w-2xl gap-8 md:grid-cols-2">
      <section>
        <h2 className="mb-2 px-2 text-sm">项目</h2>
        <div className="flex flex-col gap-1">
          {projects.map((project) => (
            <button
              aria-pressed={project.id === projectId}
              className={cn(
                'flex min-w-0 items-start gap-2 rounded-lg px-3 py-2.5 text-left hover:bg-accent',
                project.id === projectId && 'bg-accent',
              )}
              disabled={pending}
              key={project.id}
              onClick={() => onProjectChange(project.id)}
              type="button"
            >
              <FolderIcon className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
              <span className="min-w-0">
                <span className="block truncate">{project.name}</span>
                <span className="block truncate text-xs text-muted-foreground">
                  {userPath(project.sourcePath)}
                </span>
              </span>
            </button>
          ))}
        </div>
      </section>
      <section>
        <h2 className="mb-2 px-2 text-sm">Agent</h2>
        <div className="flex flex-col gap-1">
          {agents.map((provider) => (
            <button
              aria-pressed={provider.id === providerId}
              className={cn(
                'flex items-center gap-2 rounded-lg px-3 py-2.5 text-left hover:bg-accent',
                provider.id === providerId && 'bg-accent',
              )}
              disabled={pending}
              key={provider.id}
              onClick={() => onProviderChange(provider.id)}
              type="button"
            >
              <AgentProviderIcon
                className="text-muted-foreground"
                providerId={provider.id}
              />
              {provider.name}
            </button>
          ))}
        </div>
      </section>
    </div>
  );
}
