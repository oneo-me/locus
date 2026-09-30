const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const vm = require('node:vm');
const { JSDOM, VirtualConsole } = require('jsdom');
const resources = readFileSync(join(__dirname, '../src/runtime.js'), 'utf8');
const lifecycle = readFileSync(join(__dirname, '../src/main.js'), 'utf8');

function page(data) {
    const dom = new JSDOM('<base href="/_locus/resource/templates/example/canvas/"><button>Increment</button><output></output><script type="application/json" id="locus-canvas-data"></script>', {
        url: 'http://locus-canvas.localhost/canvas.html', runScripts: 'outside-only', virtualConsole: new VirtualConsole(),
    });
    dom.window.document.getElementById('locus-canvas-data').textContent = JSON.stringify(data);
    dom.window.fetch = async url => new Response(String(url));
    dom.window.eval(resources);
    dom.window.eval(lifecycle);
    return dom;
}

test('imports an ES module, passes data and awaits asynchronous main before completion', async () => {
    const dom = page({ count: 40 });
    try {
        const module = new vm.SourceTextModule(`const runCanvas = "canvas-owned";
        export async function main(data) {
            document.title = runCanvas;
            await Promise.resolve();
            let count = data.count;
            document.querySelector('button').onclick = () => document.querySelector('output').textContent = ++count;
            document.querySelector('output').textContent = count;
        }`, { context: dom.getInternalVMContext() });
        await module.link(() => { throw Error('Unexpected import'); });
        await module.evaluate();
        await dom.window.runCanvas(async () => module.namespace);
        assert.equal(dom.window.document.querySelector('output').textContent, '40');
        assert.equal(dom.window.document.title, 'canvas-owned');
        dom.window.document.querySelector('button').click();
        assert.equal(dom.window.document.querySelector('output').textContent, '41');
        assert.deepEqual(Object.keys(dom.window.locus), ['resourceUrl']);
    } finally { dom.window.close(); }
});

test('main receives null for a directly opened canvas', async () => {
    const dom = page(null);
    try {
        let input = 'uninitialized';
        await dom.window.runCanvas(async () => ({ main: async data => { input = data; } }));
        assert.equal(input, null);
    } finally { dom.window.close(); }
});

test('import failures, missing main and rejected main produce a readable error', async () => {
    for (const importer of [async () => { throw Error('Missing module'); }, async () => ({}), async () => ({ main: async () => { await Promise.resolve(); throw Error('Invalid data'); } })]) {
        const dom = page({});
        try {
            await dom.window.runCanvas(importer);
            assert.match(dom.window.document.querySelector('[role=alert]').textContent, /Missing module|must export main|Invalid data/);
            assert.equal(dom.window.document.querySelector('button'), null);
        } finally { dom.window.close(); }
    }
});

test('template relative URLs retain template ownership and library addresses retain library ownership', async () => {
    const dom = page({});
    try {
        const { document, locus } = dom.window;
        assert.equal(new URL('main.js', document.baseURI).pathname, '/_locus/resource/templates/example/canvas/main.js');
        const address = 'locus://caller/assets/photo.png';
        assert.equal(locus.resourceUrl(address), 'http://locus-canvas.localhost/_locus/resource/caller/assets/photo.png');
        assert.equal(await (await dom.window.fetch(address)).text(), locus.resourceUrl(address));
        const image = document.createElement('img');
        image.setAttribute('src', address);
        document.body.append(image);
        await Promise.resolve();
        assert.equal(image.src, locus.resourceUrl(address));
    } finally { dom.window.close(); }
});

test('note links retain their navigation addresses while resource links are mapped', async () => {
    const dom = page({});
    try {
        const { document, locus } = dom.window;
        document.body.innerHTML = `<a href="locus://diary/2026/entry"><span>Open note</span></a>
            <a href="locus://旅行%20计划/100%25" target="_blank">Encoded note</a>
            <a href="locus://diary/2026/assets/photo.png">Attachment</a>`;
        await Promise.resolve();
        const links = document.querySelectorAll('a');
        assert.equal(links[0].getAttribute('href'), 'locus://diary/2026/entry');
        assert.equal(links[1].getAttribute('href'), 'locus://旅行%20计划/100%25');
        assert.equal(links[2].href, locus.resourceUrl('locus://diary/2026/assets/photo.png'));
        links[0].href = 'locus://diary/2026/another';
        await Promise.resolve();
        assert.equal(links[0].getAttribute('href'), 'locus://diary/2026/another');
        const address = 'locus://diary/2026/entry/index.md?part=body';
        assert.equal(await (await dom.window.fetch(address)).text(), locus.resourceUrl(address));
    } finally { dom.window.close(); }
});
