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
        const conversation = JSON.parse(
          readFileSync(join(this.#directory, name), 'utf8'),
        ) as AgentConversation;
        if (
          conversation.id &&
          typeof conversation.activityAt === 'string' &&
          (conversation.autoTitle === null ||
            typeof conversation.autoTitle === 'string') &&
          conversation.projectId &&
          conversation.projectName &&
          Array.isArray(conversation.transcript)
        ) {
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
          title,
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
          title,
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
