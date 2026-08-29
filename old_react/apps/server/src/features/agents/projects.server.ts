import { existsSync, statSync } from 'node:fs';
import { homedir } from 'node:os';
import { isAbsolute, resolve } from 'node:path';
import type {
  AgentProject,
  AgentProjectList,
} from '@locus/agent/contracts/conversation';
import {
  archivedDocumentPaths,
  isDocumentArchived,
} from '@locus/document-library/contracts/document-archive';
import { getLibrary } from '~/features/library/instance.server';

export async function listAgentProjects(): Promise<AgentProjectList> {
  const library = await getLibrary();
  const documents = library.listAllDocuments();
  const archivedPaths = archivedDocumentPaths(documents);
  const documentProjects = documents
    .filter(
      (document) =>
        /^projects\/[^/]+\/index\.md$/.test(document.path) &&
        !isDocumentArchived(archivedPaths, document.path),
    )
    .flatMap((summary) => {
      const document = library.getDocument(summary.path);
      const source = document.additionalMetadata.source;
      if (typeof source !== 'string' || !source.trim()) return [];
      return [
        {
          id: summary.path,
          name: summary.title,
          sourcePath: resolveSourcePath(source),
        },
      ];
    })
    .sort((left, right) => left.name.localeCompare(right.name));
  return {
    projects: [
      {
        id: 'locus://library',
        name: '文档库',
        sourcePath: library.getRootPath(),
      },
      ...documentProjects,
    ],
  };
}

export async function getAgentProject(id: string): Promise<AgentProject> {
  const project = (await listAgentProjects()).projects.find(
    (candidate) => candidate.id === id,
  );
  if (!project) throw new Error('项目不存在或不可用于 Agent。');
  if (
    !existsSync(project.sourcePath) ||
    !statSync(project.sourcePath).isDirectory()
  ) {
    throw new Error('项目 source 不是可用的本地目录。');
  }
  return project;
}

function resolveSourcePath(source: string): string {
  const trimmed = source.trim();
  if (trimmed === '~') return homedir();
  if (trimmed.startsWith('~/')) return resolve(homedir(), trimmed.slice(2));
  if (!isAbsolute(trimmed))
    throw new Error('项目 source 必须是绝对路径或 ~/ 路径。');
  return resolve(trimmed);
}
