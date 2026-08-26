import {
  appendFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { join } from 'node:path';
import type {
  AgentConversation,
  AgentConversationSummary,
  AgentTranscriptEntry,
} from '../contracts/conversation.js';

type ConversationMetadata = Omit<AgentConversation, 'transcript'>;

interface ConversationRecord {
  conversation: AgentConversation | null;
  directory: string;
  metadata: ConversationMetadata;
  transcript: Map<string, StoredTranscriptEntry>;
}

interface StoredTranscriptEntry {
  entry: AgentTranscriptEntry;
  position: number;
}

type TranscriptEvent =
  | { content: string; id: string; op: 'append'; streaming: boolean }
  | { id: string; op: 'remove' }
  | { entry: AgentTranscriptEntry; op: 'put'; position: number };

const METADATA_FILENAME = 'metadata.json';
const TRANSCRIPT_FILENAME = 'transcript.jsonl';

export class AgentConversationStore {
  readonly #records = new Map<string, ConversationRecord>();
  readonly #root: string;

  constructor(dataDirectory: string) {
    this.#root = join(dataDirectory, 'projects');
    mkdirSync(this.#root, { recursive: true });
    this.#loadRecords();
  }

  list(): AgentConversationSummary[] {
    return [...this.#records.values()]
      .sort((left, right) =>
        right.metadata.activityAt.localeCompare(left.metadata.activityAt),
      )
      .map(({ metadata }) => summary(metadata));
  }

  get(id: string): AgentConversation | undefined {
    const record = this.#records.get(id);
    if (!record) return undefined;
    if (record.conversation) return record.conversation;
    const conversation: AgentConversation = {
      ...record.metadata,
      transcript: [...record.transcript.values()]
        .sort((left, right) => left.position - right.position)
        .map(({ entry }) => structuredClone(entry)),
    };
    record.conversation = conversation;
    return conversation;
  }

  save(conversation: AgentConversation): void {
    let record = this.#records.get(conversation.id);
    if (!record) {
      const directory = join(
        this.#root,
        localDateDirectory(conversation.createdAt),
        conversation.id,
      );
      mkdirSync(directory, { recursive: true });
      record = {
        conversation,
        directory,
        metadata: metadata(conversation),
        transcript: new Map(),
      };
      this.#records.set(conversation.id, record);
    }
    this.#saveTranscript(record, conversation.transcript);
    record.metadata = metadata(conversation);
    record.conversation = conversation;
    writeJsonAtomic(join(record.directory, METADATA_FILENAME), record.metadata);
  }

  delete(id: string): AgentConversation | undefined {
    const conversation = this.get(id);
    const record = this.#records.get(id);
    if (!conversation || !record) return undefined;
    this.#records.delete(id);
    rmSync(record.directory, { recursive: true });
    return conversation;
  }

  #loadRecords(): void {
    for (const dateEntry of readdirSync(this.#root, { withFileTypes: true })) {
      if (!dateEntry.isDirectory() || !isDateDirectory(dateEntry.name))
        continue;
      const dateDirectory = join(this.#root, dateEntry.name);
      for (const conversationEntry of readdirSync(dateDirectory, {
        withFileTypes: true,
      })) {
        if (!conversationEntry.isDirectory()) continue;
        const directory = join(dateDirectory, conversationEntry.name);
        const loaded = readMetadata(join(directory, METADATA_FILENAME));
        if (!loaded || loaded.id !== conversationEntry.name) continue;
        if (loaded.status !== 'idle') loaded.status = 'idle';
        this.#records.set(loaded.id, {
          conversation: null,
          directory,
          metadata: loaded,
          transcript: readTranscript(join(directory, TRANSCRIPT_FILENAME)),
        });
      }
    }
  }

  #saveTranscript(
    record: ConversationRecord,
    transcript: AgentTranscriptEntry[],
  ): void {
    const events: TranscriptEvent[] = [];
    const nextIds = new Set(transcript.map((entry) => entry.id));
    for (const id of record.transcript.keys()) {
      if (!nextIds.has(id)) events.push({ id, op: 'remove' });
    }
    transcript.forEach((entry, position) => {
      const previous = record.transcript.get(entry.id);
      if (
        previous &&
        previous.position === position &&
        (previous.entry.kind === 'message' ||
          previous.entry.kind === 'thought') &&
        entry.kind === previous.entry.kind &&
        canAppend(previous.entry, entry)
      ) {
        const content = entry.content.slice(previous.entry.content.length);
        if (content || previous.entry.streaming !== entry.streaming) {
          events.push({
            content,
            id: entry.id,
            op: 'append',
            streaming: entry.streaming,
          });
        }
      } else if (
        !previous ||
        previous.position !== position ||
        JSON.stringify(previous.entry) !== JSON.stringify(entry)
      ) {
        events.push({ entry, op: 'put', position });
      }
    });
    if (events.length) {
      appendFileSync(
        join(record.directory, TRANSCRIPT_FILENAME),
        `${events.map((event) => JSON.stringify(event)).join('\n')}\n`,
      );
    }
    record.transcript = new Map(
      transcript.map((entry, position) => [
        entry.id,
        { entry: structuredClone(entry), position },
      ]),
    );
  }
}

function metadata(conversation: AgentConversation): ConversationMetadata {
  const { transcript: _transcript, ...value } = conversation;
  return structuredClone(value);
}

function summary(value: ConversationMetadata): AgentConversationSummary {
  return {
    activityAt: value.activityAt,
    autoTitle: value.autoTitle,
    createdAt: value.createdAt,
    id: value.id,
    projectId: value.projectId,
    projectName: value.projectName,
    providerId: value.providerId,
    status: value.status,
    updatedAt: value.updatedAt,
  };
}

function readMetadata(path: string): ConversationMetadata | null {
  try {
    const value = JSON.parse(readFileSync(path, 'utf8')) as unknown;
    return isConversationMetadata(value) ? value : null;
  } catch {
    return null;
  }
}

function readTranscript(path: string): Map<string, StoredTranscriptEntry> {
  const transcript = new Map<string, StoredTranscriptEntry>();
  if (!existsSync(path)) return transcript;
  for (const line of readFileSync(path, 'utf8').split('\n')) {
    if (!line) continue;
    try {
      const event = JSON.parse(line) as TranscriptEvent;
      if (event.op === 'put' && isTranscriptEntry(event.entry)) {
        transcript.set(event.entry.id, {
          entry: event.entry,
          position: event.position,
        });
      } else if (event.op === 'remove') {
        transcript.delete(event.id);
      } else if (event.op === 'append') {
        const stored = transcript.get(event.id);
        if (
          stored &&
          (stored.entry.kind === 'message' || stored.entry.kind === 'thought')
        ) {
          stored.entry.content += event.content;
          stored.entry.streaming = event.streaming;
        }
      }
    } catch {
      continue;
    }
  }
  return transcript;
}

function isConversationMetadata(value: unknown): value is ConversationMetadata {
  if (!isRecord(value)) return false;
  return (
    typeof value.id === 'string' &&
    isSafeSegment(value.id) &&
    typeof value.activityAt === 'string' &&
    (value.autoTitle === null || typeof value.autoTitle === 'string') &&
    Array.isArray(value.availableCommands) &&
    Array.isArray(value.configOptions) &&
    typeof value.createdAt === 'string' &&
    typeof value.cwd === 'string' &&
    (value.modes === null || isRecord(value.modes)) &&
    typeof value.projectId === 'string' &&
    typeof value.projectName === 'string' &&
    (value.providerSessionId === null ||
      typeof value.providerSessionId === 'string') &&
    (value.providerSessionPath === null ||
      typeof value.providerSessionPath === 'string') &&
    typeof value.providerId === 'string' &&
    typeof value.status === 'string' &&
    typeof value.updatedAt === 'string'
  );
}

function isTranscriptEntry(value: unknown): value is AgentTranscriptEntry {
  return (
    isRecord(value) &&
    typeof value.id === 'string' &&
    typeof value.kind === 'string' &&
    typeof value.createdAt === 'string'
  );
}

function canAppend(
  previous: Extract<AgentTranscriptEntry, { kind: 'message' | 'thought' }>,
  next: Extract<AgentTranscriptEntry, { kind: 'message' | 'thought' }>,
): boolean {
  if (
    next.kind !== previous.kind ||
    !next.content.startsWith(previous.content)
  ) {
    return false;
  }
  const previousShape = { ...previous, content: '', streaming: false };
  const nextShape = { ...next, content: '', streaming: false };
  return JSON.stringify(previousShape) === JSON.stringify(nextShape);
}

function localDateDirectory(createdAt: string): string {
  const date = new Date(createdAt);
  if (Number.isNaN(date.getTime()))
    return new Date().toISOString().slice(0, 10);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

function isDateDirectory(value: string): boolean {
  return /^\d{4}-\d{2}-\d{2}$/.test(value);
}

function isSafeSegment(value: string): boolean {
  return (
    value.length > 0 && value !== '.' && value !== '..' && !/[\\/]/.test(value)
  );
}

function writeJsonAtomic(path: string, value: unknown): void {
  const temporaryPath = `${path}.tmp`;
  writeFileSync(temporaryPath, `${JSON.stringify(value, null, 2)}\n`);
  renameSync(temporaryPath, path);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
