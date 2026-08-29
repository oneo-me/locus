import { getDocumentIconUrl } from '@locus/document-library/contracts/asset';
import { ArchiveIcon, ArchiveRestoreIcon } from 'lucide-react';
import { useState } from 'react';
import { useRevalidator } from 'react-router';
import type { LibraryArchiveView } from '@locus/document-library/contracts/library-api';
import { Alert, AlertDescription } from '@locus/ui/components/alert';
import { Button } from '@locus/ui/components/button';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@locus/ui/components/empty';
import { useDocumentApi } from '~/lib/document-api-context';
import { DocumentIcon } from '@locus/document-library-ui/icons';
import { SettingsPage } from './page';
import { SettingsRow, SettingsSection } from './section';

export function ArchiveSettings({
  data,
  error,
}: {
  data: LibraryArchiveView;
  error: string | null;
}) {
  const { setDocumentArchived } = useDocumentApi();
  const { revalidate } = useRevalidator();
  const [pendingPath, setPendingPath] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  const restore = async (path: string) => {
    setPendingPath(path);
    setActionError(null);
    try {
      await setDocumentArchived({ path, archived: false });
      await revalidate();
    } catch (cause) {
      setActionError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setPendingPath(null);
    }
  };

  return (
    <SettingsPage title="归档">
      <SettingsSection title="已归档笔记">
        {error ? (
          <Alert variant="destructive">
            <AlertDescription>无法读取归档：{error}</AlertDescription>
          </Alert>
        ) : data.documents.length === 0 ? (
          <Empty className="py-8">
            <EmptyMedia variant="icon">
              <ArchiveIcon aria-hidden="true" />
            </EmptyMedia>
            <EmptyHeader>
              <EmptyTitle>暂无归档笔记</EmptyTitle>
              <EmptyDescription>
                点击笔记标题左侧的图标可以归档笔记。
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        ) : (
          data.documents.map((document) => (
            <SettingsRow
              description={document.directory || '笔记库根目录'}
              key={document.path}
              label={
                <span className="flex min-w-0 items-center gap-2">
                  <DocumentIcon
                    className="size-4 shrink-0 text-sm"
                    icon={document.icon}
                    imageSrc={getDocumentIconUrl(
                      document.path,
                      document.iconImageModifiedAt,
                    )}
                  />
                  <span className="truncate">{document.title}</span>
                </span>
              }
            >
              <Button
                disabled={pendingPath !== null}
                onClick={() => void restore(document.path)}
                size="sm"
                type="button"
                variant="outline"
              >
                <ArchiveRestoreIcon
                  aria-hidden="true"
                  data-icon="inline-start"
                />
                {pendingPath === document.path ? '正在移除…' : '移除归档'}
              </Button>
            </SettingsRow>
          ))
        )}
        {actionError ? (
          <Alert variant="destructive">
            <AlertDescription>{actionError}</AlertDescription>
          </Alert>
        ) : null}
      </SettingsSection>
    </SettingsPage>
  );
}
