import { Trash2Icon } from 'lucide-react';
import { type MouseEvent, useEffect, useState } from 'react';
import { Button } from '@locus/ui/components/button';
import { cn } from '@locus/ui/utils';

export function TaskDeleteButton({
  className,
  disabled,
  onDelete,
  size = 'icon-2xs',
}: {
  className?: string;
  disabled?: boolean;
  onDelete: () => void;
  size?: 'icon-2xs' | 'icon-sm' | 'icon-xs';
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
      className={cn(confirming && 'w-auto px-2', className)}
      disabled={disabled}
      onClick={click}
      size={confirming ? 'xs' : size}
      type="button"
      variant={confirming ? 'destructive' : 'destructive-ghost'}
    >
      {confirming ? '确认' : <Trash2Icon aria-hidden="true" />}
    </Button>
  );
}
