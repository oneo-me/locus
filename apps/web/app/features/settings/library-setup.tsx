import { Link } from 'react-router';
import { buttonVariants } from '@locus/ui/components/button';
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from '@locus/ui/components/empty';

export function LibrarySetup({ error }: { error?: string }) {
  return (
    <main className="grid h-full min-h-0 place-items-center bg-card px-6">
      <Empty className="max-w-lg">
        <EmptyHeader>
          <EmptyTitle>需要设置文档库</EmptyTitle>
          <EmptyDescription>
            {error ?? '请先在设置页面选择本地 Markdown 文档库。'}
          </EmptyDescription>
        </EmptyHeader>
        <EmptyContent>
          <Link
            className={buttonVariants({ variant: 'default' })}
            state={{ libraryError: error }}
            to="/settings"
          >
            打开设置
          </Link>
        </EmptyContent>
      </Empty>
    </main>
  );
}
