import { Switch } from '@locus/ui/components/switch';
import { useGlobalSettings } from '../settings/global-settings-provider';

export function GraphAttachmentsSwitch() {
  const { settings, updateSettings } = useGlobalSettings();

  return (
    <Switch
      aria-label="显示附件"
      checked={settings.showGraphAttachments}
      onCheckedChange={(checked) => {
        void updateSettings({ showGraphAttachments: checked }).catch(() => {});
      }}
    />
  );
}

export function GraphAttachmentsControl() {
  return (
    <div className="flex items-start justify-between gap-3 text-sm">
      <div className="min-w-0">
        <p>显示附件</p>
        <p className="mt-0.5 text-xs text-muted-foreground">
          在知识图谱中显示文档同目录附件节点。
        </p>
      </div>
      <GraphAttachmentsSwitch />
    </div>
  );
}
