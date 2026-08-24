import type {
  GlobalSettings,
  UpdateGlobalSettingsInput,
} from '~/contracts/global-settings';
import { apiGet, apiSend } from '~/lib/api-client';

export async function getGlobalSettings(): Promise<GlobalSettings> {
  return apiGet<GlobalSettings>('/api/settings');
}

export async function updateGlobalSettings(
  data: UpdateGlobalSettingsInput,
): Promise<GlobalSettings> {
  return apiSend<GlobalSettings>('/api/settings', 'PATCH', data);
}
