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
                'flex h-8 min-w-0 cursor-default items-center gap-1.5 rounded-lg px-2 text-left disabled:opacity-50',
                project.id === projectId && 'bg-accent',
              )}
              disabled={pending}
              key={project.id}
              onClick={() => onProjectChange(project.id)}
              type="button"
            >
              <FolderIcon className="size-3.5 shrink-0 text-muted-foreground" />
              <span className="min-w-0 flex-1 truncate">{project.name}</span>
              <span className="max-w-[50%] truncate text-xs text-muted-foreground">
                {userPath(project.sourcePath)}
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
                'flex h-8 cursor-default items-center gap-1.5 rounded-lg px-2 text-left disabled:opacity-50',
                provider.id === providerId && 'bg-accent',
              )}
              disabled={pending}
              key={provider.id}
              onClick={() => onProviderChange(provider.id)}
              type="button"
            >
              <AgentProviderIcon
                className="size-3.5 text-muted-foreground"
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
