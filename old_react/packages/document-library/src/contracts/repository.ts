export interface CommitLibraryChangesInput {
  message: string;
  expectedRevision: string;
  files: string[];
}

export interface CommitLibraryChangesResult {
  commit: string;
  message: string;
  changedFileCount: number;
  files: string[];
  pushed: true;
}

export type LibraryRepositoryChangeStatus =
  | 'added'
  | 'modified'
  | 'deleted'
  | 'renamed'
  | 'copied'
  | 'conflicted';

export interface LibraryRepositoryChange {
  file: string;
  previousFile?: string;
  status: LibraryRepositoryChangeStatus;
  staged: boolean;
  unstaged: boolean;
}

export interface LibraryRepositoryChanges {
  revision: string;
  changes: LibraryRepositoryChange[];
}

export interface LibraryRepositoryStatus {
  revision: string;
  clean: boolean;
  changes: {
    total: number;
    staged: number;
    unstaged: number;
    untracked: number;
    conflicted: number;
  };
}
