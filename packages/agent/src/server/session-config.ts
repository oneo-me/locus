import type { SessionConfigOption } from '@agentclientprotocol/sdk';
import type { AgentSessionConfig } from '../contracts/conversation.js';

export function normalizeSessionConfig(
  options: SessionConfigOption[] | null | undefined,
): AgentSessionConfig[] {
  return (options ?? []).map((option) => {
    const base = {
      category: option.category ?? null,
      id: option.id,
      name: option.name,
    };
    if (option.type === 'boolean') {
      return { ...base, currentValue: option.currentValue, type: option.type };
    }
    return {
      ...base,
      currentValue: option.currentValue,
      options: option.options
        .flatMap((item) => ('options' in item ? item.options : [item]))
        .map(({ name, value }) => ({ name, value })),
      type: option.type,
    };
  });
}
