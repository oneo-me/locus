import { Button as ButtonPrimitive } from '@base-ui/react/button';
import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '../utils';

const buttonVariants = cva(
  "group/button inline-flex shrink-0 items-center justify-center rounded-lg border border-transparent bg-clip-padding text-sm font-medium whitespace-nowrap transition-colors outline-none select-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      iconSize: {
        default: '',
        sm: "[&_svg:not([class*='size-'])]:size-3.5",
      },
      variant: {
        default:
          'border-[color-mix(in_oklch,var(--primary),black_15%)] bg-linear-to-b from-[color-mix(in_oklch,var(--primary),white_10%)] to-[color-mix(in_oklch,var(--primary),black_7%)] text-primary-foreground shadow-[0_1px_2px_rgb(0_0_0/0.08),inset_0_1px_0_rgb(255_255_255/0.18)] hover:from-[color-mix(in_oklch,var(--primary),white_4%)] hover:to-[color-mix(in_oklch,var(--primary),black_12%)]',
        outline:
          'border-border hover:bg-muted hover:text-foreground aria-expanded:bg-muted aria-expanded:text-foreground dark:border-input dark:bg-input/30 dark:hover:bg-input/50',
        secondary:
          'border-border bg-linear-to-b from-background to-muted text-secondary-foreground shadow-xs hover:from-muted hover:to-[color-mix(in_srgb,var(--brand)_6%,var(--card))] aria-expanded:bg-muted aria-expanded:text-secondary-foreground',
        floating:
          'border-border bg-linear-to-b from-card to-muted text-foreground shadow-[0_3px_8px_rgb(0_0_0/0.14),inset_0_1px_0_color-mix(in_oklch,var(--foreground)_12%,transparent)] hover:from-card hover:to-[color-mix(in_srgb,var(--primary)_6%,var(--card))] hover:text-foreground',
        'secondary-selected':
          'border-primary bg-linear-to-b from-background to-muted text-primary shadow-xs hover:from-muted hover:to-[color-mix(in_srgb,var(--brand)_6%,var(--card))]',
        nav: 'text-foreground',
        selected: 'bg-sidebar-accent text-sidebar-accent-foreground',
        ghost:
          'hover:bg-muted hover:text-foreground aria-expanded:bg-muted aria-expanded:text-foreground dark:hover:bg-muted/50',
        'destructive-ghost':
          'text-muted-foreground hover:bg-destructive hover:text-white focus-visible:border-destructive/40 focus-visible:ring-destructive/20',
        destructive:
          'border-[color-mix(in_oklch,var(--destructive),black_15%)] bg-linear-to-b from-[color-mix(in_oklch,var(--destructive),white_10%)] to-[color-mix(in_oklch,var(--destructive),black_7%)] text-white shadow-[0_1px_2px_rgb(0_0_0/0.08),inset_0_1px_0_rgb(255_255_255/0.18)] hover:from-[color-mix(in_oklch,var(--destructive),white_4%)] hover:to-[color-mix(in_oklch,var(--destructive),black_12%)] focus-visible:border-destructive/40 focus-visible:ring-destructive/20',
        link: 'text-primary underline-offset-4 hover:underline',
      },
      size: {
        default:
          'h-9 gap-1.5 px-3 has-data-[icon=inline-end]:pr-2.5 has-data-[icon=inline-start]:pl-2.5',
        xs: "h-7 gap-1 rounded-[min(var(--radius-md),10px)] px-2 text-xs in-data-[slot=button-group]:rounded-lg has-data-[icon=inline-end]:pr-1.5 has-data-[icon=inline-start]:pl-1.5 [&_svg:not([class*='size-'])]:size-3.5",
        sm: "h-8 gap-1 rounded-[min(var(--radius-md),12px)] px-2.5 text-[0.8rem] in-data-[slot=button-group]:rounded-lg has-data-[icon=inline-end]:pr-2 has-data-[icon=inline-start]:pl-2 [&_svg:not([class*='size-'])]:size-3.5",
        nav: "h-7 gap-1 px-2.5 text-[0.8125rem] leading-5 font-normal [&_svg:not([class*='size-'])]:size-3.5",
        lg: 'h-10 gap-1.5 px-3.5 has-data-[icon=inline-end]:pr-3 has-data-[icon=inline-start]:pl-3',
        icon: 'size-9',
        'icon-xs':
          "size-7 rounded-[min(var(--radius-md),10px)] in-data-[slot=button-group]:rounded-lg [&_svg:not([class*='size-'])]:size-3.5",
        'icon-sm':
          'size-8 rounded-[min(var(--radius-md),12px)] in-data-[slot=button-group]:rounded-lg',
        'icon-lg': 'size-10',
      },
    },
    defaultVariants: {
      iconSize: 'default',
      variant: 'default',
      size: 'default',
    },
  },
);

function Button({
  className,
  iconSize = 'default',
  variant = 'default',
  size = 'default',
  ...props
}: ButtonPrimitive.Props & VariantProps<typeof buttonVariants>) {
  return (
    <ButtonPrimitive
      data-slot="button"
      className={cn(buttonVariants({ iconSize, variant, size, className }))}
      {...props}
    />
  );
}

export { Button, buttonVariants };
