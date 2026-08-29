import { ampProvider } from '@locus/agent/providers/amp';
import { claudeProvider } from '@locus/agent/providers/claude';
import { codexProvider } from '@locus/agent/providers/codex';
import { cursorProvider } from '@locus/agent/providers/cursor';
import { deepseekProvider } from '@locus/agent/providers/deepseek';
import { fxProvider } from '@locus/agent/providers/fx';
import { grokProvider } from '@locus/agent/providers/grok';
import { kimiProvider } from '@locus/agent/providers/kimi';
import { ohmypiProvider } from '@locus/agent/providers/ohmypi';
import { opencodeProvider } from '@locus/agent/providers/opencode';
import { piProvider } from '@locus/agent/providers/pi';
import { AgentProviderRegistry } from '@locus/agent/server/provider-detection';

export const agentProviders = new AgentProviderRegistry([
  ampProvider,
  claudeProvider,
  codexProvider,
  cursorProvider,
  deepseekProvider,
  fxProvider,
  opencodeProvider,
  grokProvider,
  kimiProvider,
  ohmypiProvider,
  piProvider,
]);
