import { type CSSProperties, useState } from 'react';
import { AgentConversation } from './conversation';
import { AgentsSidebar } from './sidebar';
import { AgentsSidebarResizeHandle } from './sidebar-resize-handle';

export function AgentsShell() {
  const [sidebarWidth, setSidebarWidth] = useState(272);

  return (
    <div
      className="grid h-full min-h-0 min-w-0 grid-cols-[var(--agents-sidebar-width)_0_minmax(0,1fr)] bg-background text-sm"
      style={
        {
          '--agents-sidebar-width': `${sidebarWidth}px`,
        } as CSSProperties
      }
    >
      <AgentsSidebar />
      <AgentsSidebarResizeHandle
        onWidthChange={setSidebarWidth}
        width={sidebarWidth}
      />
      <AgentConversation />
    </div>
  );
}
