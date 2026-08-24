import { parseWikiLinkArticlePath } from '~/contracts/wiki-link-path';
import { articlePathToDocumentPath } from './article-paths.js';

export { parseWikiLinkArticlePath } from '~/contracts/wiki-link-path';

const wikiLinkPattern = /\[\[([^\]]+)\]\]/g;

function stripCodeRegions(content: string): string {
  return content
    .replace(/```[\s\S]*?```/g, ' ')
    .replace(/~~~[\s\S]*?~~~/g, ' ')
    .replace(/`[^`\n]+`/g, ' ');
}

/** 按出现顺序提取正文中的 wiki 文章路径（含重复，跳过代码区）。 */
export function extractWikiLinkArticlePaths(content: string): string[] {
  const paths: string[] = [];
  for (const match of stripCodeRegions(content).matchAll(wikiLinkPattern)) {
    const articlePath = parseWikiLinkArticlePath(match[1] ?? '');
    if (articlePath === null) continue;
    paths.push(articlePath);
  }
  return paths;
}

/** 按出现顺序提取 wiki 目标文档路径（含重复）。 */
export function extractWikiLinkDocumentPaths(content: string): string[] {
  return extractWikiLinkArticlePaths(content).map(articlePathToDocumentPath);
}
