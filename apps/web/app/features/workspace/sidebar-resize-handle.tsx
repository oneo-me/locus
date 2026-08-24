import {
  MAX_WORKSPACE_SIDEBAR_WIDTH,
  MIN_WORKSPACE_SIDEBAR_WIDTH,
  normalizeWorkspaceSidebarWidth,
} from '@locus/document-library/contracts/global-settings';
import {
  type KeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  useRef,
} from 'react';

const keyboardResizeStep = 12;

export function WorkspaceSidebarResizeHandle({
  onWidthChange,
  onWidthCommit,
  width,
}: {
  onWidthChange: (width: number) => void;
  onWidthCommit: (width: number) => void;
  width: number;
}) {
  const drag = useRef<{ originWidth: number; originX: number } | null>(null);
  const widthRef = useRef(width);
  widthRef.current = width;

  const finishDrag = () => {
    if (!drag.current) return;
    drag.current = null;
    onWidthCommit(widthRef.current);
  };

  const handlePointerDown = (event: ReactPointerEvent<HTMLHRElement>) => {
    if (event.button !== 0) return;
    drag.current = { originWidth: width, originX: event.clientX };
    event.currentTarget.setPointerCapture(event.pointerId);
    event.preventDefault();
  };

  const handlePointerMove = (event: ReactPointerEvent<HTMLHRElement>) => {
    if (!drag.current) return;
    onWidthChange(
      normalizeWorkspaceSidebarWidth(
        drag.current.originWidth + event.clientX - drag.current.originX,
      ),
    );
  };

  const handlePointerUp = (event: ReactPointerEvent<HTMLHRElement>) => {
    finishDrag();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLHRElement>) => {
    let nextWidth: number | null = null;
    if (event.key === 'ArrowLeft') nextWidth = width - keyboardResizeStep;
    if (event.key === 'ArrowRight') nextWidth = width + keyboardResizeStep;
    if (event.key === 'Home') nextWidth = MIN_WORKSPACE_SIDEBAR_WIDTH;
    if (event.key === 'End') nextWidth = MAX_WORKSPACE_SIDEBAR_WIDTH;
    if (nextWidth === null) return;

    event.preventDefault();
    const normalized = normalizeWorkspaceSidebarWidth(nextWidth);
    onWidthChange(normalized);
    onWidthCommit(normalized);
  };

  return (
    <hr
      aria-label="调整文档侧栏宽度"
      aria-orientation="vertical"
      aria-valuemax={MAX_WORKSPACE_SIDEBAR_WIDTH}
      aria-valuemin={MIN_WORKSPACE_SIDEBAR_WIDTH}
      aria-valuenow={width}
      className="group relative z-20 m-0 hidden h-full w-3 -translate-x-1/2 cursor-col-resize touch-none border-0 bg-transparent outline-none after:absolute after:inset-y-0 after:left-1/2 after:w-px after:-translate-x-1/2 after:bg-border after:content-[''] focus-visible:after:bg-primary md:block"
      onKeyDown={handleKeyDown}
      onLostPointerCapture={finishDrag}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      tabIndex={0}
    />
  );
}
