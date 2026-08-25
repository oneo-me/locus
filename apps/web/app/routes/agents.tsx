import { BotIcon } from 'lucide-react';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@locus/ui/components/empty';

export default function AgentsRoute() {
  return (
    <main
      className="grid h-full min-h-0 place-items-center bg-card px-6"
      id="main-content"
    >
      <Empty className="max-w-lg">
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <BotIcon aria-hidden="true" />
          </EmptyMedia>
          <EmptyTitle>Agent</EmptyTitle>
          <EmptyDescription>Agent 工作区将在这里显示。</EmptyDescription>
        </EmptyHeader>
      </Empty>
    </main>
  );
}
