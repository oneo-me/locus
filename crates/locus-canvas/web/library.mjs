import { lstat, readFile, realpath } from 'node:fs/promises';
import path from 'node:path';
import semver from 'semver';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);

export function notePath(address) {
  if (!address.startsWith('locus://')) throw new Error('Expected a locus:// note address');
  if (/[?#]/.test(address.slice(8))) throw new Error('Encode note path characters before addressing a note');
  const parts = address.slice(8).split('/').map(decodeURIComponent);
  if (parts.some(part => !part || part.startsWith('.') || /[\\/:\0]/.test(part)
    || /[. ]$/.test(part) || ['assets', 'canvas', 'node_modules'].includes(part.toLowerCase()))) {
    throw new Error('Invalid note address');
  }
  return parts.join('/');
}

export async function regularFile(root, relative) {
  const parts = relative.split('/');
  if (parts.some(part => !part || part === '.' || part === '..' || /[\\:\0]/.test(part))) {
    throw new Error('Invalid resource path');
  }
  let file = root;
  for (const [index, part] of parts.entries()) {
    file = path.join(file, part);
    const stat = await lstat(file);
    if (stat.isSymbolicLink() || (index < parts.length - 1 ? !stat.isDirectory() : !stat.isFile())) {
      throw new Error('Resources must be ordinary files within the library');
    }
  }
  return file;
}

export class Library {
  constructor(root) { this.root = root; }

  async note(address) {
    const relative = notePath(address);
    await regularFile(this.root, `${relative}/index.md`);
    return path.join(this.root, relative);
  }

  async component(address, name = '.') {
    const directory = await this.note(address);
    const file = await regularFile(directory, 'canvas/package.json');
    const manifest = JSON.parse(await readFile(file, 'utf8'));
    const entry = typeof manifest.exports === 'string' && name === '.'
      ? manifest.exports : manifest.exports?.[name];
    if (typeof entry !== 'string' || !entry.startsWith('./')) {
      throw new Error(`${address} must export ${name} as a source file in canvas/package.json`);
    }
    if (manifest.dependencies?.react || manifest.dependencies?.['react-dom'] || manifest.devDependencies?.react || manifest.devDependencies?.['react-dom']) {
      throw new Error(`${address}: React and React DOM are supplied by Locus; use peerDependencies`);
    }
    for (const name of ['react', 'react-dom']) {
      const range = manifest.peerDependencies?.[name];
      if (range && !semver.satisfies(require(`${name}/package.json`).version, range)) {
        throw new Error(`${address}: ${name} ${range} is incompatible with the Locus runtime`);
      }
    }
    const source = await regularFile(path.dirname(file), entry.slice(2));
    return { directory: path.dirname(file), source, manifest };
  }

  async resource(address) {
    const [uri, query, extra] = address.split('?');
    if (extra !== undefined) throw new Error('Invalid resource query');
    if (!uri.startsWith('locus://')) throw new Error('Expected a locus:// resource');
    const parts = uri.slice(8).split('/');
    const boundary = parts.findIndex(part => ['assets', 'canvas', 'index.md'].includes(decodeURIComponent(part)));
    if (boundary < 1) throw new Error('Expected a note resource');
    const directory = await this.note(`locus://${parts.slice(0, boundary).join('/')}`);
    const relative = parts.slice(boundary).map(decodeURIComponent).join('/');
    if (relative.split('/').some(part => part === 'node_modules')) {
      throw new Error('Private package files are not library resources');
    }
    const file = await regularFile(directory, relative);
    const bytes = await readFile(file);
    if (!query) return { file, bytes };
    if (relative !== 'index.md' || !['part=yaml', 'part=body'].includes(query)) {
      throw new Error('Invalid note content part');
    }
    const text = bytes.toString('utf8').replace(/^\uFEFF/, '');
    const match = /^---\r?\n(?:([\s\S]*?)\r?\n)?(?:---|\.\.\.)(?:\r?\n|$)/.exec(text);
    return { file, bytes: Buffer.from(query === 'part=yaml' ? (match?.[1] ?? '') : (match ? text.slice(match[0].length) : text)) };
  }
}

export async function openLibrary(root) {
  return new Library(await realpath(root));
}
