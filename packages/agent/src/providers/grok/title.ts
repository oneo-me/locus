import { readFileSync, readdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

const UUID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export function readGrokSessionTitle(
  sessionId: string,
  grokHome = defaultGrokHome(),
): string | null {
  if (!UUID_PATTERN.test(sessionId)) return null;
  try {
    const sessionsDirectory = join(grokHome, 'sessions');
    for (const workspace of readdirSync(sessionsDirectory, {
      withFileTypes: true,
    })) {
      if (!workspace.isDirectory()) continue;
      try {
        const summary = JSON.parse(
          readFileSync(
            join(sessionsDirectory, workspace.name, sessionId, 'summary.json'),
            'utf8',
          ),
        ) as { generated_title?: unknown };
        const title =
          typeof summary.generated_title === 'string'
            ? summary.generated_title.trim()
            : '';
        if (title) return title;
      } catch {
        continue;
      }
    }
  } catch {}
  return null;
}

function defaultGrokHome(): string {
  return process.env.GROK_HOME?.trim() || join(homedir(), '.grok');
}
