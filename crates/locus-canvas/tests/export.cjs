const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { JSDOM, VirtualConsole } = require('jsdom');
const runtime = readFileSync(join(__dirname, '../src/export.js'), 'utf8');
const templates = readFileSync(join(__dirname, '../src/templates.js'), 'utf8');

function page(bundle, source = '') {
    const dom = new JSDOM('<body></body>', {
        url: 'file:///Downloads/My%20Canvas/canvas.html',
        runScripts: 'dangerously', virtualConsole: new VirtualConsole(),
    });
    dom.window.Response = Response;
    dom.window.fetch = async url => { throw new Error('Unexpected network access: ' + url); };
    for (const text of [`(${runtime})(${JSON.stringify(bundle)});`, templates, source]) {
        const script = dom.window.document.createElement('script');
        script.textContent = text;
        dom.window.document.body.append(script);
    }
    return dom;
}

test('exported nested templates preserve data, repeated instances and event handlers offline', async () => {
    const dom = page({ templates: {
        parent: '<script>window.child = locus.loadCanvas("locus://child", document.currentScript.canvas.data);</script>',
        child: '<button>Increment</button><output></output><script>(() => { const c = document.currentScript.canvas; let count = c.data.count; c.root.querySelector("button").onclick = () => { c.root.querySelector("output").textContent = ++count; }; })();</script>',
    }, resources: {} });
    await dom.window.locus.loadCanvas('locus://parent', { count: 41 });
    await dom.window.child;
    dom.window.document.querySelector('button').click();
    assert.equal(dom.window.document.querySelector('output').textContent, '42');
    await dom.window.locus.loadCanvas('locus://parent', { count: 9 });
    await dom.window.child;
    dom.window.document.querySelectorAll('button')[1].click();
    assert.deepEqual(Array.from(dom.window.document.querySelectorAll('output'), node => node.textContent), ['42', '10']);
    dom.window.close();
});

test('exported local fetches work under file URLs with encoded names, query strings and binary bytes', async () => {
    const dom = page({ templates: {}, resources: { '数据.json': 'data:application/json;base64,' + Buffer.from('{"value":42}').toString('base64'), 'photo.png': 'data:image/png;base64,AP8q' } });
    const response = await dom.window.fetch('./%E6%95%B0%E6%8D%AE.json?v=2');
    assert.equal(response.headers.get('Content-Type'), 'application/json');
    assert.deepEqual(await response.json(), { value: 42 });
    assert.deepEqual(new Uint8Array(await (await dom.window.fetch('photo.png')).arrayBuffer()), new Uint8Array([0, 255, 42]));
    await assert.rejects(dom.window.fetch('../photo.png'), /Unexpected network/);
    await assert.rejects(dom.window.fetch('https://example.com/_locus/template?uri=locus://x'), /Unexpected network/);
    dom.window.close();
});

test('missing templates reject and encoded template addresses resolve without a host service', async () => {
    const dom = page({ templates: { '模版/hello world': '<p>Ready</p>' }, resources: {} });
    await dom.window.locus.loadCanvas('locus://%E6%A8%A1%E7%89%88/hello%20world');
    assert.equal(dom.window.document.querySelector('p').textContent, 'Ready');
    await assert.rejects(dom.window.locus.loadCanvas('locus://missing'), /Cannot load/);
    await assert.rejects(dom.window.locus.loadCanvas('locus://toString'), /Cannot load/);
    await assert.rejects(dom.window.locus.loadCanvas('locus://模版%2Fhello%20world'), /Cannot load/);
    await assert.rejects(dom.window.locus.loadCanvas('locus://模版/hello world/'), /Cannot load/);
    dom.window.close();
});

test('cross-note YAML and body snapshots are separate from original note bytes', async () => {
    const encode = text => 'data:text/plain;base64,' + Buffer.from(text).toString('base64');
    const dom = page({templates: {}, resources: {
        '_locus/resource/other/index.md?part=yaml': encode('title: Other'),
        '_locus/resource/other/index.md?part=body': encode('Body'),
        '_locus/resource/other/index.md': encode('---\ntitle: Other\n---\nBody'),
    }});
    assert.equal(await (await dom.window.fetch('locus://other/index.md?part=yaml')).text(), 'title: Other');
    assert.equal(await (await dom.window.fetch('locus://other/index.md?part=body')).text(), 'Body');
    assert.equal(await (await dom.window.fetch('locus://other/index.md')).text(), '---\ntitle: Other\n---\nBody');
    dom.window.close();
});
