import { Switch as SwitchPrimitive } from '@base-ui/react/switch';

import { cn } from '../utils';

function Switch({
  className,
  size = 'default',
  ...props
}: SwitchPrimitive.Root.Props & {
  size?: 'sm' | 'default';
}) {
  const isDefault = size === 'default';

  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      data-size={size}
      className={cn(
        'peer group/switch relative inline-flex shrink-0 items-center rounded-full transition-colors outline-none select-none',
        'after:absolute after:-inset-x-3 after:-inset-y-2',
        'focus-visible:ring-3 focus-visible:ring-ring/50',
        'data-checked:bg-primary data-unchecked:bg-input dark:data-unchecked:bg-input/80',
        'aria-invalid:data-checked:bg-destructive aria-invalid:data-unchecked:bg-destructive/45',
        'dark:aria-invalid:data-unchecked:bg-destructive/55',
        'data-disabled:cursor-not-allowed data-disabled:opacity-50',
        isDefault ? 'h-6 w-11 p-[3px]' : 'h-[1.125rem] w-8 p-0.5',
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className={cn(
          'pointer-events-none block shrink-0 rounded-full bg-background',
          'shadow-[0_1px_2px_rgb(0_0_0/0.12),0_0_1px_rgb(0_0_0/0.06)]',
          'transition-transform duration-150 ease-out will-change-transform',
          'dark:data-checked:bg-primary-foreground dark:data-unchecked:bg-foreground',
          isDefault
            ? 'size-[1.125rem] group-data-checked/switch:translate-x-5'
            : 'size-3.5 group-data-checked/switch:translate-x-3.5',
        )}
      />
    </SwitchPrimitive.Root>
  );
}

export { Switch };
