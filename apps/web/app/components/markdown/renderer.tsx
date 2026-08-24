import { ComarkRenderer } from '@comark/react';
import type { ComarkTree } from 'comark';
import type { ComponentProps } from 'react';
import { memo } from 'react';
import { cn } from '@locus/ui/utils';

export type MarkdownComponents = NonNullable<
  ComponentProps<typeof ComarkRenderer>['components']
>;

const baseComponents: MarkdownComponents = {};

export interface MarkdownRendererProps {
  className?: string;
  components?: MarkdownComponents;
  tree: ComarkTree;
}

export const MarkdownRenderer = memo(
  ({ className, components, tree }: MarkdownRendererProps) => (
    <ComarkRenderer
      className={cn(
        'size-full [&>*:first-child]:mt-0 [&>*:last-child]:mb-0',
        className,
      )}
      components={{ ...baseComponents, ...components }}
      tree={tree}
    />
  ),
);

MarkdownRenderer.displayName = 'MarkdownRenderer';
