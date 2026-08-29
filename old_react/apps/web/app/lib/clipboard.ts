export async function copyText(value: string): Promise<void> {
  const clipboard = globalThis.navigator?.clipboard;
  if (!clipboard) throw new Error('当前环境不支持剪贴板。');
  await clipboard.writeText(value);
}
