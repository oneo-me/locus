const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { JSDOM, VirtualConsole } = require('jsdom');
const runtime = readFileSync(join(__dirname, '../src/export.js'), 'utf8');
const resources = readFileSync(join(__dirname, '../src/runtime.js'), 'utf8');

function page(bundle, source = '') {
    const dom = new JSDOM('<body></body>', {
        url: 'file:///Downloads/My%20Canvas/canvas.html',
        runScripts: 'dangerously', virtualConsole: new VirtualConsole(),
    });
    dom.window.Response = Response;
    dom.window.fetch = async url => { throw new Error('Unexpected network access: ' + url); };
    for (const text of [`(${runtime})(${JSON.stringify(bundle)});`, resources, source]) {
        const script = dom.window.document.createElement('script');
        script.textContent = text;
        dom.window.document.body.append(script);
    }
    return dom;
}

test('exported local fetches work under file URLs with encoded names, query strings and binary bytes', async () => {
    const dom = page({ resources: { '数据.json': 'data:application/json;base64,' + Buffer.from('{"value":42}').toString('base64'), 'photo.png': 'data:image/png;base64,AP8q' } });
    const response = await dom.window.fetch('./%E6%95%B0%E6%8D%AE.json?v=2');
    assert.equal(response.headers.get('Content-Type'), 'application/json');
    assert.deepEqual(await response.json(), { value: 42 });
    assert.deepEqual(new Uint8Array(await (await dom.window.fetch('photo.png')).arrayBuffer()), new Uint8Array([0, 255, 42]));
    await assert.rejects(dom.window.fetch('../photo.png'), /Unexpected network/);
    await assert.rejects(dom.window.fetch('https://example.com/data'), /Unexpected network/);
    dom.window.close();
});

test('cross-note YAML and body snapshots are separate from original note bytes', async () => {
    const encode = text => 'data:text/plain;base64,' + Buffer.from(text).toString('base64');
    const dom = page({resources: {
        '_locus/resource/other/index.md?part=yaml': encode('title: Other'),
        '_locus/resource/other/index.md?part=body': encode('Body'),
        '_locus/resource/other/index.md': encode('---\ntitle: Other\n---\nBody'),
    }});
    assert.equal(await (await dom.window.fetch('locus://other/index.md?part=yaml')).text(), 'title: Other');
    assert.equal(await (await dom.window.fetch('locus://other/index.md?part=body')).text(), 'Body');
    assert.equal(await (await dom.window.fetch('locus://other/index.md')).text(), '---\ntitle: Other\n---\nBody');
    dom.window.close();
});
