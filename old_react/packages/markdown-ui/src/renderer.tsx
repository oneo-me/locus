import { MarkdownDocument } from '@comark/react';
import type { MarkdownDocument as MarkdownDocumentValue } from 'comark';
import type { ComponentProps } from 'react';
import { memo } from 'react';
import { cn } from '@locus/ui/utils';

export type MarkdownComponents = NonNullable<
  ComponentProps<typeof MarkdownDocument>['components']
>;

const baseComponents: MarkdownComponents = {};

export interface MarkdownRendererProps {
  className?: string;
  components?: MarkdownComponents;
  value: MarkdownDocumentValue;
}

export const MarkdownRenderer = memo(
  ({ className, components, value }: MarkdownRendererProps) => (
    <MarkdownDocument
      className={cn(
        'size-full [&>*:first-child]:mt-0 [&>*:last-child]:mb-0',
        className,
      )}
      components={{ ...baseComponents, ...components }}
      value={value}
    />
  ),
);

MarkdownRenderer.displayName = 'MarkdownRenderer';
