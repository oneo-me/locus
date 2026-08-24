import type { UpdateGlobalSettingsInput } from '~/contracts/global-settings';
import {
  getGlobalSettings,
  updateGlobalSettings,
} from '~/server/features/settings/service';
import { jsonError, jsonOk, readJsonBody } from '~/server/http/json';
import type { Route } from './+types/api.settings';

export async function loader() {
  try {
    return jsonOk(getGlobalSettings());
  } catch (error) {
    return jsonError(error);
  }
}

export async function action({ request }: Route.ActionArgs) {
  if (request.method !== 'PATCH' && request.method !== 'POST') {
    return jsonError(new Error('Method Not Allowed'), 405);
  }
  try {
    const body = await readJsonBody<UpdateGlobalSettingsInput>(request);
    return jsonOk(updateGlobalSettings(body));
  } catch (error) {
    return jsonError(error);
  }
}
