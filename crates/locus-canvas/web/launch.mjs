import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { runNpm } from './packages.mjs';

const directory = path.dirname(fileURLToPath(import.meta.url));
const [major, minor] = process.versions.node.split('.').map(Number);
if (!(major === 20 && minor >= 19 || major === 22 && minor >= 12 || major > 22)) {
  throw new Error('Canvas requires Node.js 20.19+ or 22.12+. Install a supported Node.js and npm, then reload.');
}
let server;
const shutdown = async () => {
  if (server) await server.close();
  process.exit(0);
};
process.stdin.resume();
process.stdin.on('end', shutdown);
process.on('SIGTERM', shutdown);
process.on('SIGINT', shutdown);
const fingerprint = createHash('sha256').update(await readFile(path.join(directory, 'package-lock.json'))).digest('hex');
const marker = path.join(directory, 'node_modules/.locus-installed');
if (await readFile(marker, 'utf8').catch(() => '') !== fingerprint) {
  await runNpm(directory, ['ci', '--ignore-scripts', '--no-audit', '--no-fund']);
  await writeFile(marker, fingerprint);
}
const { startServer } = await import('./server.mjs');
server = await startServer(process.argv[2]);
process.stdout.write(`LOCUS_READY http://127.0.0.1:${server.httpServer.address().port}\n`);
