const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { JSDOM, VirtualConsole } = require('jsdom');
const runtime = readFileSync(require('node:path').join(__dirname, '../src/templates.js'), 'utf8');

function page(source) {
    const dom = new JSDOM('<main></main>', {
        url: 'http://locus-canvas.localhost/canvas.html',
        runScripts: 'dangerously', virtualConsole: new VirtualConsole(),
    });
    dom.window.fetch = async url => {
        assert.match(url, /^\/_locus\/template\?uri=locus%3A%2F%2F/);
        return { ok: true, text: async () => source };
    };
    const script = dom.window.document.createElement('script');
    script.textContent = runtime;
    dom.window.document.head.append(script);
    return dom;
}

test('loads markup and ordered scripts with input data, without returning a template API', async () => {
    const dom = page(`<p></p><script type="application/json" id="data">{"value":2}</script>
        <script>(() => { const c = document.currentScript.canvas; c.root.querySelector('p').textContent = JSON.parse(c.root.querySelector('#data').textContent).value; })();</script>
        <script>(() => { const c = document.currentScript.canvas; c.root.querySelector('p').textContent += c.data.name; })();</script>`);
    const { document, locus } = dom.window;
    assert.deepEqual(Object.keys(locus), ['loadCanvas']);
    assert.equal(await locus.loadCanvas('locus://旅行', { name: 'Tokyo' }), undefined);
    assert.equal(await locus.loadCanvas('locus://旅行', { name: 'Kyoto' }), undefined);
    assert.deepEqual(Array.from(document.querySelectorAll('p'), p => p.textContent), ['2Tokyo', '2Kyoto']);
    dom.window.close();
});

test('rejects non-locus addresses, failed loads, empty sources and unsupported scripts', async () => {
    for (const source of [' ', '<script src="local.js"></script>', '<script type="module"></script>', '<script>throw new Error("broken")</script>']) {
        const dom = page(source);
        const target = dom.window.document.body;
        await assert.rejects(dom.window.locus.loadCanvas('locus://travel', {}));
        assert.equal(target.querySelector('div'), null);
        dom.window.close();
    }
    const dom = page('<p>Ready</p>');
    const target = dom.window.document.body;
    for (const uri of ['file:///tmp/template', 'https://example.com', '../travel', null]) {
        await assert.rejects(dom.window.locus.loadCanvas(uri, {}));
    }
    dom.window.fetch = async () => ({ ok: false });
    await assert.rejects(dom.window.locus.loadCanvas('locus://missing', {}));
    assert.equal(target.querySelector('div'), null);
    dom.window.close();
});

test('templates can pass input to another template and detached pages do not execute scripts', async () => {
    const dom = page('<script>window.childLoaded = locus.loadCanvas("locus://child", document.currentScript.canvas.data);</script>');
    const { document, locus } = dom.window;
    const target = document.body;
    const initialFetch = dom.window.fetch;
    dom.window.fetch = async url => {
        if (url.includes('child')) return { ok: true, text: async () => '<p></p><script>(() => { const c = document.currentScript.canvas; c.root.querySelector("p").textContent = c.data.answer; })();</script>' };
        return initialFetch(url);
    };
    await locus.loadCanvas('locus://parent', { answer: 42 });
    await dom.window.childLoaded;
    assert.equal(document.querySelector('p').textContent, '42');
    dom.window.fetch = async () => {
        target.remove();
        return { ok: true, text: async () => '<script>window.unwanted = true;</script>' };
    };
    await assert.rejects(locus.loadCanvas('locus://late', {}));
    assert.equal(dom.window.unwanted, undefined);
    dom.window.close();
});
