import type { LibraryWorkspaceView } from '~/contracts/library-api';
import FluentIconDocument from '@oneo/fluentui-icons-like-react/FluentIconDocument';
import { Button } from '~/components/ui/button';
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '~/components/ui/empty';

export function WorkspaceDocumentLandingState({
  data,
}: {
  data: LibraryWorkspaceView;
}) {
  const searchIsEmpty = data.search?.total === 0;
  const libraryIsEmpty = data.documents.length === 0;
  const title = searchIsEmpty
    ? '没有匹配的文章'
    : libraryIsEmpty
      ? '文档库还是空的'
      : '选择一篇搜索结果';
  const description = searchIsEmpty
    ? '换一个关键词，或清除搜索查看全部文章。'
    : libraryIsEmpty
      ? `在 ${data.status.rootPath} 下新建文章目录，并在其中加入 index.md。`
      : '从左侧搜索结果中选择一篇文章开始阅读。';

  return (
    <main
      className="h-full min-h-0 min-w-0 overflow-x-hidden overflow-y-auto overscroll-contain bg-card text-sm"
      id="main-content"
    >
      {data.search || libraryIsEmpty ? (
        <Empty className="mx-auto mt-[18vh] w-[calc(100%-3rem)] max-w-xl">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <FluentIconDocument aria-hidden="true" title={null} />
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
