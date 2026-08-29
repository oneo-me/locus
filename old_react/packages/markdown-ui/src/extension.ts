import type { ComarkPlugin } from 'comark';
import type { MarkdownComponents } from './renderer';

export interface MarkdownUiExtension {
  components?: MarkdownComponents;
  elementNames?: readonly string[];
  plugins?: readonly ComarkPlugin[];
}
