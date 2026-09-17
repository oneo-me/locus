import { spawn } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';

export class Packages {
  #pending = Promise.resolve();

  ensure(directory) {
    const task = this.#pending.then(async () => {
      const source = await readFile(path.join(directory, 'package.json'), 'utf8');
      const manifest = JSON.parse(source);
      const lock = await readFile(path.join(directory, 'package-lock.json'), 'utf8').catch(() => '');
      const fingerprint = source + lock;
      const stamp = path.join(directory, 'node_modules/.locus-installed');
      const hash = createHash('sha256').update(fingerprint).digest('hex');
      if (await readFile(stamp, 'utf8').catch(() => '') === hash) return;
      if (lock || Object.keys({ ...manifest.dependencies, ...manifest.devDependencies }).length) {
        await runNpm(directory, ['install', '--ignore-scripts', '--legacy-peer-deps', '--no-audit', '--no-fund']);
        const updated = source + await readFile(path.join(directory, 'package-lock.json'), 'utf8').catch(() => '');
        await writeFile(stamp, createHash('sha256').update(updated).digest('hex'));
      }
    });
    this.#pending = task.catch(() => {});
    return task;
  }
}

export function runNpm(directory, args) {
  return new Promise((resolve, reject) => {
    const command = process.platform === 'win32' ? 'npm.cmd' : 'npm';
    const child = spawn(command, args, {
      cwd: directory, stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true,
      shell: process.platform === 'win32',
    });
    const stop = () => child.kill();
    process.once('exit', stop);
    let output = '';
    for (const stream of [child.stdout, child.stderr]) stream.on('data', bytes => {
      output = (output + bytes.toString()).slice(-16000);
      process.stderr.write(bytes);
    });
    child.on('error', error => { process.off('exit', stop); reject(error); });
    child.on('exit', code => {
      process.off('exit', stop);
      code === 0 ? resolve() : reject(new Error(`npm failed (${code}): ${output}`));
    });
  });
}
