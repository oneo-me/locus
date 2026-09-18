import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, readdir, rm, symlink, copyFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { openLibrary, notePath } from '../library.mjs';
import { startServer } from '../server.mjs';

async function fixture(t) {
  const root = await mkdtemp(path.join(tmpdir(), 'locus-canvas-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  async function note(name, source) {
    const directory = path.join(root, name);
    await mkdir(path.join(directory, 'canvas/src'), { recursive: true });
    await writeFile(path.join(directory, 'index.md'), '---\ntitle: Example\n---\nBody');
    await writeFile(path.join(directory, 'canvas/package.json'), JSON.stringify({
      name: name.replaceAll('/', '-'), private: true, type: 'module', exports: './src/index.tsx',
      peerDependencies: { react: '^19.3.0' },
    }));
    await writeFile(path.join(directory, 'canvas/src/index.tsx'), source);
    return directory;
  }
  return { root, note };
}

test('note and resource resolution rejects traversal, symlinks and non-notes', async t => {
  const { root, note } = await fixture(t);
  await note('example', 'export default function Canvas() { return null; }');
  const library = await openLibrary(root);
  assert.match((await library.component('locus://example')).source, /index\.tsx$/);
  assert.equal((await library.resource('locus://example/index.md?part=body')).bytes.toString(), 'Body');
  assert.equal((await library.resource('locus://example/index.md?part=yaml')).bytes.toString(), 'title: Example');
  for (const address of ['locus://../example', 'locus://%2fexample', 'locus://example/canvas', 'locus://example?x']) {
    assert.throws(() => notePath(address));
  }
  await symlink(path.join(root, 'example'), path.join(root, 'linked'), 'dir');
  await assert.rejects(library.component('locus://linked'));
  await assert.rejects(library.resource('locus://example/canvas/../index.md'));
  await assert.rejects(library.resource('locus://example/index.md?invalid'));
  await assert.rejects(library.resource('locus://example/index.md?part=body?ignored'));
  for (const [source, yaml, body] of [
    ['---\n---\nBody\n', '', 'Body\n'],
    ['\uFEFF---\r\ntitle: Note\r\n---\r\nBody\r\n', 'title: Note', 'Body\r\n'],
    ['Body', '', 'Body'],
  ]) {
    await writeFile(path.join(root, 'example/index.md'), source);
    assert.equal((await library.resource('locus://example/index.md?part=yaml')).bytes.toString(), yaml);
    assert.equal((await library.resource('locus://example/index.md?part=body')).bytes.toString(), body);
  }
  const manifest = path.join(root, 'example/canvas/package.json');
  const incompatible = JSON.parse(await readFile(manifest, 'utf8'));
  incompatible.peerDependencies.react = '^18.0.0';
  await writeFile(manifest, JSON.stringify(incompatible));
  await assert.rejects(library.component('locus://example'), /incompatible/);
});

test('system npm installs package dependencies without lifecycle scripts or a second React', async t => {
  const { root, note } = await fixture(t);
  const directory = await note('dependency', 'import clsx from "clsx"; export default function Canvas() { return <p className={clsx("p-6")}>Dependency</p>; }');
  const file = path.join(directory, 'canvas/package.json');
  const manifest = JSON.parse(await readFile(file, 'utf8'));
  manifest.dependencies = { clsx: '2.1.1' };
  manifest.scripts = { postinstall: 'node -e "require(\'fs\').writeFileSync(\'unexpected\',\'script executed\')"' };
  await writeFile(file, JSON.stringify(manifest));
  const server = await startServer(root);
  t.after(() => server.close());
  const port = server.httpServer.address().port;
  const base = `http://127.0.0.1:${port}`;
  const entry = await fetch(`${base}/@id/locus:dependency`);
  assert.equal(entry.status, 200, await entry.clone().text());
  const code = await entry.text();
  const moduleUrl = code.match(/from "([^"]*clsx[^"]*)"/)?.[1];
  assert.ok(moduleUrl, code);
  assert.equal((await fetch(new URL(moduleUrl, base))).status, 200);
  assert.equal(JSON.parse(await readFile(path.join(directory, 'canvas/package-lock.json'), 'utf8')).packages['node_modules/clsx'].version, '2.1.1');
  await assert.rejects(readFile(path.join(directory, 'canvas/unexpected')));
  await assert.rejects(readFile(path.join(directory, 'canvas/node_modules/react/package.json')));
  const restarted = new Promise((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error('Manifest update did not restart Vite')), 10000);
    const original = server.restart.bind(server);
    server.restart = async (...args) => {
      try { await original(...args); clearTimeout(timeout); resolve(); }
      catch (error) { clearTimeout(timeout); reject(error); }
    };
  });
  manifest.description = 'Updated package';
  await writeFile(file, JSON.stringify(manifest));
  await restarted;
  assert.equal(server.httpServer.address().port, port);
  assert.equal((await fetch(`${base}/notes/dependency/`)).status, 200);
});

test('the system runtime bootstraps an empty host cache and exits with its owner', { timeout: 60000 }, async t => {
  const { root, note } = await fixture(t);
  await note('example', 'export default function Canvas() { return null; }');
  const host = path.join(root, 'host');
  await mkdir(host);
  for (const name of ['package.json', 'package-lock.json', 'launch.mjs', 'server.mjs', 'plugin.mjs', 'library.mjs', 'packages.mjs', 'host.tsx', 'host.css', 'sdk.tsx']) {
    await copyFile(new URL(`../${name}`, import.meta.url), path.join(host, name));
  }
  const child = spawn(process.execPath, [path.join(host, 'launch.mjs'), root], { stdio: ['pipe', 'pipe', 'pipe'] });
  t.after(() => child.kill());
  let errors = '';
  child.stderr.on('data', bytes => { errors += bytes; });
  const origin = await new Promise((resolve, reject) => {
    let output = '';
    const timeout = setTimeout(() => reject(new Error(`Service did not become ready: ${errors}`)), 45000);
    child.once('exit', code => { clearTimeout(timeout); reject(new Error(`Service exited (${code}): ${errors}`)); });
    child.stdout.on('data', bytes => {
      output += bytes;
      const match = output.match(/LOCUS_READY (http:\/\/127\.0\.0\.1:\d+)/);
      if (match) { clearTimeout(timeout); resolve(match[1]); }
    });
    child.once('error', error => { clearTimeout(timeout); reject(error); });
  });
  assert.equal((await fetch(`${origin}/notes/example/`)).status, 200);
  const entry = await fetch(`${origin}/@id/locus:example`);
  assert.equal(entry.status, 200, await entry.text());
  assert.ok(await readFile(path.join(host, 'node_modules/.locus-installed'), 'utf8'));
  const exited = once(child, 'exit');
  child.stdin.end();
  const [code] = await exited;
  assert.equal(code, 0);
  assert.deepEqual(await readdir(path.join(host, '.locus-packages')), []);
});

test('one Vite server serves routes, compiles React and cross-note imports, and produces Tailwind', async t => {
  const { root, note } = await fixture(t);
  await note('shared', 'export function Label() { return <strong className="p-6">Shared</strong>; }\nexport default Label;');
  await note('caller', 'import {Label} from "locus:shared"; export default function Canvas() { return <Label/>; }');
  await note('例子 100%#', 'export default function Canvas() { return <p>Encoded note</p>; }');
  const server = await startServer(root, { install: false });
  t.after(() => server.close());
  const base = `http://127.0.0.1:${server.httpServer.address().port}`;
  const page = await fetch(`${base}/notes/caller/`).then(r => r.text());
  assert.match(page, /data-note="locus:\/\/caller"/);
  assert.match(page, /@vite\/client/);
  assert.match(page, /@react-refresh/);
  const encodedNote = `locus://${encodeURIComponent('例子 100%#')}`;
  const encodedPage = await fetch(`${base}/notes/${encodeURIComponent('例子 100%#')}/`);
  assert.equal(encodedPage.status, 200);
  const encodedEntry = await fetch(`${base}/__locus/entry.tsx?note=${Buffer.from(encodedNote).toString('base64url')}&import`);
  assert.equal(encodedEntry.status, 200, await encodedEntry.clone().text());
  assert.match(await encodedEntry.text(), /Encoded note/);
  const pending = ['/host.tsx'];
  const fetched = new Set();
  while (pending.length) {
    const url = pending.pop();
    if (fetched.has(url)) continue;
    fetched.add(url);
    const response = await fetch(new URL(url, base));
    const code = await response.text();
    assert.equal(response.status, 200, `${url}: ${code.slice(0, 500)}`);
    for (const match of code.matchAll(/(?:from\s*|import\s*)["'](\/[^"']+)["']/g)) pending.push(match[1]);
  }
  const entry = await fetch(`${base}/__locus/entry.tsx?note=${Buffer.from('locus://caller').toString('base64url')}&import`).then(r => r.text());
  assert.match(entry, /react\/jsx-dev-runtime|react_jsx-dev-runtime/);
  const sharedUrl = entry.match(/from "([^"]*\.locus-packages[^"?]*index\.tsx[^"]*)"/)?.[1];
  assert.ok(sharedUrl, entry);
  const shared = await fetch(new URL(sharedUrl, base)).then(r => r.text());
  assert.match(shared, /Shared/);
  const cssUrl = shared.match(/import "([^"]*locus-style:[^"]*)"/)?.[1];
  assert.ok(cssUrl, shared);
  const css = await fetch(new URL(cssUrl, base)).then(r => r.text());
  assert.match(css, /\.p-6/);
  const socket = new WebSocket(`ws://127.0.0.1:${server.httpServer.address().port}/?token=${server.config.webSocketToken}`, 'vite-hmr');
  t.after(() => socket.close());
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  const update = new Promise((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error('HMR update did not arrive')), 10000);
    socket.addEventListener('message', event => {
      const message = JSON.parse(event.data);
      if (message.type === 'update' && message.updates.some(update => update.path.includes('index.tsx'))) {
        clearTimeout(timeout);
        resolve(message);
      }
    });
  });
  await writeFile(path.join(root, 'shared/canvas/src/index.tsx'), 'export function Label() { return <strong className="p-6">Changed</strong>; }\nexport default Label;');
  await update;
  assert.match(await fetch(new URL(sharedUrl, base)).then(r => r.text()), /Changed/);
  socket.close();
  const resource = await fetch(`${base}/__locus/resource?uri=${encodeURIComponent('locus://caller/index.md?part=body')}`);
  assert.equal(await resource.text(), 'Body');
  const denied = await fetch(`${base}/@fs${root}/caller/index.md`);
  assert.equal(denied.status, 403);
});
