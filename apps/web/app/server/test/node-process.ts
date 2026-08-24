import { spawnSync } from 'node:child_process';

export function execFileSync(
  executable: string,
  args: string[],
  options: { cwd: string; encoding?: 'utf8' },
): string {
  const result = spawnSync(executable, args, {
    cwd: options.cwd,
    encoding: 'buffer',
    env: process.env,
  });
  if (result.status !== 0) {
    throw new Error(
      `${executable} exited with code ${result.status}: ${new TextDecoder().decode(result.stderr ?? Buffer.alloc(0))}`,
    );
  }
  return new TextDecoder().decode(result.stdout ?? Buffer.alloc(0));
}
