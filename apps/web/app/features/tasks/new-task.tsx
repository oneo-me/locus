import type { AgentProject } from '@locus/agent/contracts/conversation';
import { CheckIcon } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@locus/ui/components/button';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@locus/ui/components/popover';
import logoUrl from '~/assets/logo.png';

export function NewTask({
  onProjectChange,
  pending,
  projectId,
  projects,
}: {
  onProjectChange: (id: string) => void;
  pending: boolean;
  projectId: string | null;
  projects: AgentProject[];
}) {
  const [open, setOpen] = useState(false);
  const project = projects.find((item) => item.id === projectId);

  return (
    <div className="text-center">
      <img
        alt=""
        className="mx-auto size-8 rounded-full"
        draggable={false}
        height={64}
        src={logoUrl}
        width={64}
      />
      <div className="mt-3 flex flex-wrap items-baseline justify-center gap-1 text-xl font-medium">
        <span>想在</span>
        <Popover onOpenChange={setOpen} open={open}>
          <PopoverTrigger
            className="max-w-64 truncate border-b border-dashed border-muted-foreground px-1 outline-none hover:border-foreground focus-visible:rounded-sm focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50"
            disabled={pending}
          >
            {project?.name ?? '选择项目'}
          </PopoverTrigger>
          <PopoverContent
            align="center"
            aria-label="选择项目"
            className="w-64 gap-0.5 p-1"
            side="bottom"
          >
            {projects.map((project) => (
              <Button
                aria-pressed={project.id === projectId}
                className="w-full min-w-0 justify-start"
                key={project.id}
                onClick={() => {
                  onProjectChange(project.id);
                  setOpen(false);
                }}
                size="nav"
                variant={project.id === projectId ? 'selected' : 'ghost'}
              >
                <span className="min-w-0 flex-1 truncate text-left">
                  {project.name}
                </span>
                {project.id === projectId && (
                  <CheckIcon aria-hidden="true" data-icon="inline-end" />
                )}
              </Button>
            ))}
          </PopoverContent>
        </Popover>
        <span>中构建什么？</span>
      </div>
    </div>
  );
}
