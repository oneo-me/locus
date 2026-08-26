import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';

export function readOpenCodeSessionTitle(
  sessionId: string,
  dataDirectory = openCodeDataDirectory(),
): string | null {
  const databasePath = join(dataDirectory, 'opencode.db');
  if (!existsSync(databasePath)) return null;

  let database: DatabaseSync | null = null;
  try {
    database = new DatabaseSync(databasePath, { readOnly: true });
    const row = database
      .prepare('SELECT title FROM session WHERE id = ?')
      .get(sessionId) as { title?: unknown } | undefined;
    const title = typeof row?.title === 'string' ? row.title.trim() : '';
    return title && !title.startsWith('New session - ') ? title : null;
  } catch {
    return null;
  } finally {
    try {
      database?.close();
    } catch {}
  }
}

function openCodeDataDirectory(): string {
  const dataHome = process.env.XDG_DATA_HOME?.trim();
  const defaultDataHome =
    process.platform === 'win32' && process.env.LOCALAPPDATA
      ? process.env.LOCALAPPDATA
      : join(homedir(), '.local', 'share');
  return join(dataHome || defaultDataHome, 'opencode');
}
