export function providerPromptContent(
  content: string,
  commands: Array<{ name: string }>,
): string {
  const token = content.match(/^\S+/)?.[0];
  if (!token || !isAbsolutePathToken(token)) return content;
  const commandName = token.slice(1);
  if (commands.some((command) => command.name === commandName)) return content;
  return `\`${token}\`${content.slice(token.length)}`;
}

function isAbsolutePathToken(token: string): boolean {
  return (
    token.startsWith('/') && token.indexOf('/', 1) > 1 && !token.includes('`')
  );
}
