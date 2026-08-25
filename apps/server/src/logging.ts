type LogFields = Record<string, unknown>;

function serializeError(error: unknown) {
  if (error instanceof Error) {
    return {
      name: error.name,
      message: error.message,
      stack: error.stack,
    };
  }
  return { message: String(error) };
}

function writeLog(
  level: 'info' | 'warn' | 'error',
  event: string,
  fields?: LogFields,
) {
  const message = JSON.stringify({
    timestamp: new Date().toISOString(),
    level,
    event,
    ...fields,
  });
  console[level](message);
}

export function logInfo(event: string, fields?: LogFields) {
  writeLog('info', event, fields);
}

export function logWarn(event: string, error: unknown, fields?: LogFields) {
  writeLog('warn', event, { ...fields, error: serializeError(error) });
}

export function logError(event: string, error: unknown, fields?: LogFields) {
  writeLog('error', event, { ...fields, error: serializeError(error) });
}
