import { GraphAttachmentsSwitch } from '../graph/attachments-control';
import { SettingsPage } from './page';
import { SettingsRow, SettingsSection } from './section';

export function GraphSettings() {
  return (
    <SettingsPage title="知识图谱">
      <SettingsSection title="显示">
        <SettingsRow
          description="在知识图谱中显示文档同目录附件节点。"
          label="显示附件"
        >
          <GraphAttachmentsSwitch />
        </SettingsRow>
      </SettingsSection>
    </SettingsPage>
  );
}
