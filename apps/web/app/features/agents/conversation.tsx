import { PanelRightIcon } from 'lucide-react';
import { Button } from '@locus/ui/components/button';
import { Separator } from '@locus/ui/components/separator';
import { AgentComposer } from './composer';

export function AgentConversation() {
  return (
    <main className="flex min-h-0 min-w-0 flex-col bg-card" id="main-content">
      <header className="flex h-12 shrink-0 items-center gap-3 px-4">
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-sm">实现 Agent 工作区外壳</h2>
        </div>
        <Button
          aria-disabled="true"
          aria-label="打开右侧面板"
          size="icon-sm"
          tabIndex={-1}
          variant="ghost"
        >
          <PanelRightIcon aria-hidden="true" />
        </Button>
      </header>
      <Separator />

      <section aria-label="对话内容" className="flex min-h-0 flex-1 flex-col">
        <div className="min-h-0 flex-1" />
        <AgentComposer />
      </section>
    </main>
  );
}
