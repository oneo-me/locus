import {
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { join } from 'node:path';
import type {
  AgentConversation,
  AgentConversationSummary,
} from '../contracts/conversation.js';

export class AgentConversationStore {
  readonly #directory: string;
  readonly #conversations = new Map<string, AgentConversation>();

  constructor(dataDirectory: string) {
    this.#directory = join(dataDirectory, 'agent', 'conversations');
    mkdirSync(this.#directory, { recursive: true });
    for (const name of readdirSync(this.#directory)) {
      if (!name.endsWith('.json')) continue;
      try {
        const candidate = JSON.parse(
          readFileSync(join(this.#directory, name), 'utf8'),
        ) as AgentConversation;
        if (
          candidate.id &&
          typeof candidate.activityAt === 'string' &&
          (candidate.autoTitle === null ||
            typeof candidate.autoTitle === 'string') &&
          candidate.projectId &&
          candidate.projectName &&
          (candidate.providerSessionId === null ||
            typeof candidate.providerSessionId === 'string') &&
          Array.isArray(candidate.transcript)
        ) {
          const conversation: AgentConversation = {
            activityAt: candidate.activityAt,
            autoTitle: candidate.autoTitle,
            availableCommands: candidate.availableCommands,
            configOptions: candidate.configOptions,
            contextTokens: candidate.contextTokens,
            contextWindow: candidate.contextWindow,
            createdAt: candidate.createdAt,
            cwd: candidate.cwd,
            id: candidate.id,
            modes: candidate.modes,
            projectId: candidate.projectId,
            projectName: candidate.projectName,
            providerSessionId: candidate.providerSessionId,
            providerId: candidate.providerId,
            status: candidate.status,
            transcript: candidate.transcript,
            updatedAt: candidate.updatedAt,
          };
          if (conversation.status !== 'idle') conversation.status = 'idle';
          this.#conversations.set(conversation.id, conversation);
        }
      } catch {
        continue;
      }
    }
  }

  list(): AgentConversationSummary[] {
    return [...this.#conversations.values()]
      .sort((left, right) => right.activityAt.localeCompare(left.activityAt))
      .map(
        ({
          activityAt,
          autoTitle,
          createdAt,
          id,
          projectId,
          projectName,
          providerId,
          status,
          updatedAt,
        }) => ({
          activityAt,
          autoTitle,
          createdAt,
          id,
          projectId,
          projectName,
          providerId,
          status,
          updatedAt,
        }),
      );
  }

  get(id: string): AgentConversation | undefined {
    return this.#conversations.get(id);
  }

  save(conversation: AgentConversation): void {
    this.#conversations.set(conversation.id, conversation);
    const path = join(this.#directory, `${conversation.id}.json`);
    const temporaryPath = `${path}.tmp`;
    writeFileSync(temporaryPath, `${JSON.stringify(conversation, null, 2)}\n`);
    renameSync(temporaryPath, path);
  }

  delete(id: string): AgentConversation | undefined {
    const conversation = this.#conversations.get(id);
    if (!conversation) return undefined;
    this.#conversations.delete(id);
    unlinkSync(join(this.#directory, `${id}.json`));
    return conversation;
  }
}
