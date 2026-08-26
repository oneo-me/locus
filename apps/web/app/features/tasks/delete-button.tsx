import { Trash2Icon } from 'lucide-react';
import { type MouseEvent, useEffect, useState } from 'react';
import { Button } from '@locus/ui/components/button';
import { cn } from '@locus/ui/utils';

export function TaskDeleteButton({
  className,
  disabled,
  onDelete,
}: {
  className?: string;
  disabled?: boolean;
  onDelete: () => void;
}) {
  const [confirming, setConfirming] = useState(false);

  useEffect(() => {
    if (!confirming) return;
    const timeout = window.setTimeout(() => setConfirming(false), 3_000);
    return () => window.clearTimeout(timeout);
  }, [confirming]);

  const click = (event: MouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
    event.stopPropagation();
    if (!confirming) {
      setConfirming(true);
      return;
    }
    setConfirming(false);
    onDelete();
  };

  return (
    <Button
      aria-label={confirming ? '确认删除任务' : '删除任务'}
      className={cn(confirming && 'h-6 w-auto px-2 text-xs', className)}
      disabled={disabled}
      onClick={click}
      onMouseLeave={() => setConfirming(false)}
      size="icon-2xs"
      type="button"
      variant={confirming ? 'destructive' : 'destructive-ghost'}
    >
      {confirming ? '确认' : <Trash2Icon aria-hidden="true" />}
    </Button>
  );
}
