import { ampProvider } from '@locus/agent-provider-amp';
import { claudeProvider } from '@locus/agent-provider-claude';
import { codexProvider } from '@locus/agent-provider-codex';
import { cursorProvider } from '@locus/agent-provider-cursor';
import { deepseekProvider } from '@locus/agent-provider-deepseek';
import { fxProvider } from '@locus/agent-provider-fx';
import { grokProvider } from '@locus/agent-provider-grok';
import { kimiProvider } from '@locus/agent-provider-kimi';
import { ohmypiProvider } from '@locus/agent-provider-ohmypi';
import { opencodeProvider } from '@locus/agent-provider-opencode';
import { piProvider } from '@locus/agent-provider-pi';
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
