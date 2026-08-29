import { Badge } from '@locus/ui/components/badge';
import { SettingsPage } from './page';
import { SettingsRow, SettingsSection } from './section';

export function AboutSettings() {
  return (
    <SettingsPage title="关于">
      <SettingsSection title="应用">
        <SettingsRow description="Knowledge, woven together" label="Locus">
          <Badge variant="secondary">v0.1.0</Badge>
        </SettingsRow>
      </SettingsSection>
    </SettingsPage>
  );
}
