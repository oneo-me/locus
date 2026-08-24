import { dirname } from 'node:path';
import { parse as parseYaml, stringify as stringifyYaml } from 'yaml';
import {
  type DocumentMetadata,
  type DocumentMetadataKey,
  type DocumentSummary,
  documentMetadataKeys,
} from '../contracts/library-types.js';

type Frontmatter = Record<string, unknown>;

export interface ParsedDocument {
  summary: DocumentSummary;
  content: string;
}

function optionalString(value: unknown): string | null {
  if (typeof value === 'string') return value.trim() || null;
  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value);
  }
  if (value instanceof Date) return value.toISOString();
  return null;
}

function splitFrontmatter(raw: string): {
  attributes: Frontmatter;
  content: string;
  issue: string | null;
} {
  const normalized = raw.replace(/^\uFEFF/, '');
  const match = normalized.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);

  if (!match) {
    return {
      attributes: {},
      content: normalized,
      issue: '缺少有效的 YAML frontmatter。',
    };
  }

  try {
    const parsed = parseYaml(match[1] ?? '') as Frontmatter | null;
    return {
      attributes: parsed && typeof parsed === 'object' ? parsed : {},
      content: normalized.slice(match[0].length),
      issue: null,
    };
  } catch (error) {
    return {
      attributes: {},
      content: normalized.slice(match[0].length),
      issue: `YAML 解析失败：${(error as Error).message}`,
    };
  }
}

export function extractDocumentContent(raw: string): string {
  return splitFrontmatter(raw).content;
}

function countWords(content: string): number {
  const latinWords = content.match(/[\p{Letter}\p{Number}]+/gu) ?? [];
  return latinWords.length;
}

export function parseDocument(
  path: string,
  fallbackTitle: string,
  raw: string,
  stats: { mtime: Date; size: number },
): ParsedDocument {
  const { attributes, content, issue } = splitFrontmatter(raw);
  const metadataKeys = documentMetadataKeys.filter((key) =>
    Object.hasOwn(attributes, key),
  );

  return {
    summary: {
      path,
      directory: dirname(path) === '.' ? '' : dirname(path),
      title: optionalString(attributes.title) ?? fallbackTitle,
      icon: optionalString(attributes.icon),
      iconImageModifiedAt: null,
      metadataKeys,
      summary: optionalString(attributes.summary),
      assets: [],
      modifiedAt: stats.mtime.toISOString(),
      size: stats.size,
      wordCount: countWords(content),
      issue,
    },
    content,
  };
}

export function normalizeIcon(icon: string | null | undefined): string | null {
  return icon?.trim() || null;
}

function frontmatterValue(
  key: DocumentMetadataKey,
  metadata: DocumentMetadata,
): unknown {
  const value = metadata[key];
  if (value === null || value === undefined) return undefined;
  return value;
}

export function serializeDocument(
  metadata: DocumentMetadata,
  content: string,
): string {
  const frontmatter: Frontmatter = {};
  for (const key of documentMetadataKeys) {
    const value = frontmatterValue(key, metadata);
    if (value !== undefined) frontmatter[key] = value;
  }
  const yaml = stringifyYaml(frontmatter, { indent: 2 }).trimEnd();
  return `---\n${yaml}\n---\n${content.replace(/^\uFEFF/, '')}`;
}

export function createExcerpt(content: string, query: string): string {
  const plain = content
    .replace(/```[\s\S]*?```/g, ' ')
    .replace(/[#>*_`~[\]()!-]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
  if (!plain) return '';
  const index = plain.toLocaleLowerCase().indexOf(query.toLocaleLowerCase());
  const start = Math.max(0, index < 0 ? 0 : index - 70);
  const end = Math.min(plain.length, start + 180);
  return `${start > 0 ? '…' : ''}${plain.slice(start, end)}${end < plain.length ? '…' : ''}`;
}
