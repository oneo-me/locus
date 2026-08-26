import {
  type KeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  useRef,
} from 'react';

const MIN_SIDEBAR_WIDTH = 180;
const MAX_SIDEBAR_WIDTH = 420;
const KEYBOARD_RESIZE_STEP = 12;

function normalizeSidebarWidth(width: number) {
  return Math.min(MAX_SIDEBAR_WIDTH, Math.max(MIN_SIDEBAR_WIDTH, width));
}

export function AgentsSidebarResizeHandle({
  onWidthChange,
  width,
}: {
  onWidthChange: (width: number) => void;
  width: number;
}) {
  const drag = useRef<{ originWidth: number; originX: number } | null>(null);

  const handlePointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    drag.current = { originWidth: width, originX: event.clientX };
    event.currentTarget.setPointerCapture(event.pointerId);
    event.preventDefault();
  };

  const handlePointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!drag.current) return;
    onWidthChange(
      normalizeSidebarWidth(
        drag.current.originWidth + event.clientX - drag.current.originX,
      ),
    );
  };

  const handlePointerUp = (event: ReactPointerEvent<HTMLDivElement>) => {
    drag.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    let nextWidth: number | null = null;
    if (event.key === 'ArrowLeft') nextWidth = width - KEYBOARD_RESIZE_STEP;
    if (event.key === 'ArrowRight') nextWidth = width + KEYBOARD_RESIZE_STEP;
    if (event.key === 'Home') nextWidth = MIN_SIDEBAR_WIDTH;
    if (event.key === 'End') nextWidth = MAX_SIDEBAR_WIDTH;
    if (nextWidth === null) return;

    event.preventDefault();
    onWidthChange(normalizeSidebarWidth(nextWidth));
  };

  return (
    <div
      aria-label="调整对话侧栏宽度"
      aria-orientation="vertical"
      aria-valuemax={MAX_SIDEBAR_WIDTH}
      aria-valuemin={MIN_SIDEBAR_WIDTH}
      aria-valuenow={width}
      className="relative z-20 h-full w-3 -translate-x-1/2 cursor-col-resize touch-none outline-none after:absolute after:inset-y-0 after:left-1/2 after:w-px after:-translate-x-1/2 after:bg-border after:content-[''] focus-visible:after:bg-primary"
      onKeyDown={handleKeyDown}
      onLostPointerCapture={() => {
        drag.current = null;
      }}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      role="separator"
      tabIndex={0}
    />
  );
}
