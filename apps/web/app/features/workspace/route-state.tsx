import type { LibraryWorkspaceView } from '@locus/document-library/contracts/library-api';
import { FileTextIcon } from 'lucide-react';
import { Button } from '@locus/ui/components/button';
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@locus/ui/components/empty';

export function WorkspaceDocumentLandingState({
  data,
}: {
  data: LibraryWorkspaceView;
}) {
  const navigationIsEmpty = data.documents.length === 0;
  const libraryIsEmpty = data.status.documentCount === 0;
  const title = libraryIsEmpty ? '文档库还是空的' : '所有笔记均已归档';
  const description = libraryIsEmpty
    ? `在 ${data.status.rootPath} 下新建文章目录，并在其中加入 index.md。`
    : '前往设置中的归档分类，可以移除笔记的归档状态。';

  return (
    <main
      className="h-full min-h-0 min-w-0 overflow-x-hidden overflow-y-auto overscroll-contain bg-card text-sm"
      id="main-content"
    >
      {navigationIsEmpty ? (
        <Empty className="mx-auto mt-[18vh] w-[calc(100%-3rem)] max-w-xl">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <FileTextIcon aria-hidden="true" />
            </EmptyMedia>
            <EmptyTitle>{title}</EmptyTitle>
            <EmptyDescription>{description}</EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : null}
    </main>
  );
}

export function WorkspaceErrorState({ error }: { error: Error }) {
  return (
    <main className="mx-auto mt-[18vh] w-[calc(100%-3rem)] max-w-xl">
      <Empty>
        <EmptyHeader>
          <EmptyTitle>文档库暂时无法读取</EmptyTitle>
          <EmptyDescription>{error.message}</EmptyDescription>
        </EmptyHeader>
        <EmptyContent>
          <Button type="button" onClick={() => window.location.reload()}>
            重新加载
          </Button>
        </EmptyContent>
      </Empty>
    </main>
  );
}
