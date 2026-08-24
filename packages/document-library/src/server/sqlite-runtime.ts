import { DatabaseSync } from 'node:sqlite';

export type SqliteValue = string | number | bigint | Uint8Array | null;

export interface SqliteStatement {
  all(...parameters: SqliteValue[]): unknown[];
  get(...parameters: SqliteValue[]): unknown;
  run(...parameters: SqliteValue[]): unknown;
}

export interface SqliteDatabase {
  close(): void;
  exec(sql: string): unknown;
  prepare(sql: string): SqliteStatement;
}

export function openSqliteDatabase(path: string): SqliteDatabase {
  return new DatabaseSync(path);
}
