import {
  CheckCircle2Icon,
  CircleIcon,
  LoaderCircleIcon,
  RefreshCwIcon,
} from 'lucide-react';
import { useState } from 'react';
import type { AgentProviderDetection } from '@locus/agent/contracts/provider';
import { Alert, AlertDescription } from '@locus/ui/components/alert';
import { Button } from '@locus/ui/components/button';
import { apiGet } from '~/lib/api-client';
import { AgentProviderIcon } from '~/features/agent/provider-icon';
import { SettingsPage } from './page';
import { SettingsRow, SettingsSection } from './section';

export function AgentSettings({
  initialDetection,
  initialError,
}: {
  initialDetection: AgentProviderDetection;
  initialError: string | null;
}) {
  const [detection, setDetection] = useState(initialDetection);
  const [error, setError] = useState(initialError);
  const [checking, setChecking] = useState(false);
  const installedCount = detection.providers.filter(
    (provider) => provider.installed,
  ).length;

  const refresh = async () => {
    setChecking(true);
    setError(null);
    try {
      setDetection(
        await apiGet<AgentProviderDetection>('/api/agent-providers', {
          refresh: '1',
        }),
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setChecking(false);
    }
  };

  return (
    <SettingsPage title="Agents">
      <div className="flex flex-col gap-4">
        {error ? (
          <Alert variant="destructive">
            <AlertDescription>Agent 检测失败：{error}</AlertDescription>
          </Alert>
        ) : null}
        <SettingsSection
          action={
            <div className="flex items-center gap-2">
              {detection.checkedAt ? (
                <span className="text-xs text-muted-foreground">
                  {formatDetectionTime(detection.checkedAt)}
                </span>
              ) : null}
              <Button
                disabled={checking}
                onClick={() => void refresh()}
                size="sm"
                type="button"
                variant="outline"
              >
                {checking ? (
                  <LoaderCircleIcon
                    aria-hidden="true"
                    className="animate-spin"
                    data-icon="inline-start"
                  />
                ) : (
                  <RefreshCwIcon aria-hidden="true" data-icon="inline-start" />
                )}
                {checking ? '正在检测' : '重新检测'}
              </Button>
            </div>
          }
          title={`本机 CLI · ${installedCount}/${detection.providers.length}`}
        >
          {detection.providers.map((provider) => (
            <SettingsRow
              description={
                provider.installed
                  ? `${provider.path} · ${provider.models.length} 个模型`
                  : `检测命令：${provider.command}`
              }
              key={provider.id}
              label={
                <span className="flex items-center gap-2">
                  <AgentProviderIcon
                    className="text-muted-foreground"
                    providerId={provider.id}
                  />
                  {provider.name}
                </span>
              }
            >
              <span
                className={
                  provider.installed
                    ? 'flex items-center gap-1.5 text-xs text-success'
                    : 'flex items-center gap-1.5 text-xs text-muted-foreground'
                }
              >
                {provider.installed ? (
                  <CheckCircle2Icon aria-hidden="true" className="size-3.5" />
                ) : (
                  <CircleIcon aria-hidden="true" className="size-3.5" />
                )}
                {provider.installed ? '已检测' : '未检测到'}
              </span>
            </SettingsRow>
          ))}
        </SettingsSection>
      </div>
    </SettingsPage>
  );
}

function formatDetectionTime(value: string): string {
  return `检测于 ${new Intl.DateTimeFormat('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  }).format(new Date(value))}`;
}
