import type { AgentProviderId } from '@locus/agent/contracts/provider';
import { cn } from '@locus/ui/utils';
import { BotIcon } from 'lucide-react';
import ampIcon from './assets/agents/amp.svg';
import claudeIcon from './assets/agents/claude.svg';
import codexIcon from './assets/agents/codex.svg';
import cursorIcon from './assets/agents/cursor.svg';
import deepseekIcon from './assets/agents/deepseek.svg';
import fxIcon from './assets/agents/fx.svg';
import grokIcon from './assets/agents/grok.svg';
import kimiIcon from './assets/agents/kimi.svg';
import ohmypiIcon from './assets/agents/ohmypi.svg';
import opencodeIcon from './assets/agents/opencode.svg';
import piIcon from './assets/agents/pi.svg';

const AGENT_PROVIDER_ICONS: Record<string, string> = {
  amp: ampIcon,
  claude: claudeIcon,
  codex: codexIcon,
  cursor: cursorIcon,
  deepseek: deepseekIcon,
  fx: fxIcon,
  grok: grokIcon,
  kimi: kimiIcon,
  ohmypi: ohmypiIcon,
  opencode: opencodeIcon,
  pi: piIcon,
};

export function AgentProviderIcon({
  className,
  label,
  providerId,
}: {
  className?: string;
  label?: string;
  providerId: AgentProviderId;
}) {
  const icon = AGENT_PROVIDER_ICONS[providerId];
  if (!icon) {
    return (
      <BotIcon
        aria-hidden={label ? undefined : true}
        aria-label={label}
        className={className}
        role={label ? 'img' : undefined}
      />
    );
  }
  const image = `url("${icon}")`;
  return (
    <span
      aria-hidden={label ? undefined : true}
      aria-label={label}
      className={cn('inline-block size-4 shrink-0 bg-current', className)}
      role={label ? 'img' : undefined}
      style={{
        maskImage: image,
        maskPosition: 'center',
        maskRepeat: 'no-repeat',
        maskSize: 'contain',
        WebkitMaskImage: image,
        WebkitMaskPosition: 'center',
        WebkitMaskRepeat: 'no-repeat',
        WebkitMaskSize: 'contain',
      }}
    />
  );
}
