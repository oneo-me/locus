const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const test = require('node:test');

function page() {
    const listeners = new Map();
    const messages = [];
    const window = {
        ipc: { postMessage(message) { messages.push(message); } },
        addEventListener(type, callback) { listeners.set(type, callback); },
    };
    const source = fs.readFileSync(path.join(__dirname, '../src/page.js'), 'utf8')
        .replace('__LOCUS_TERMINAL_MESSAGE__', 'terminal:test-token');
    vm.runInNewContext(source, { window });
    return {
        window,
        messages,
        press(overrides) {
            let prevented = false;
            listeners.get('keydown')({
                key: '`', metaKey: true, ctrlKey: false, altKey: false,
                shiftKey: false, isTrusted: true, repeat: false,
                preventDefault() { prevented = true; }, ...overrides,
            });
            return prevented;
        },
    };
}

test('only a genuine non-repeating shortcut launches the selected Agent', () => {
    const canvas = page();
    assert.equal(canvas.press({}), true);
    assert.deepEqual(canvas.messages, ['terminal:test-token']);
    canvas.press({ repeat: true });
    canvas.press({ isTrusted: false });
    canvas.press({ metaKey: false });
    canvas.press({ altKey: true });
    canvas.press({ shiftKey: true });
    assert.deepEqual(canvas.messages, ['terminal:test-token']);
    canvas.press({ metaKey: false, ctrlKey: true });
    assert.equal(canvas.messages.length, 2);
});

test('the page cannot replace the shortcut sender to obtain the launch message', () => {
    const canvas = page();
    let intercepted = false;
    canvas.window.ipc.postMessage = () => { intercepted = true; };
    canvas.press({});
    assert.equal(intercepted, false);
    assert.equal(canvas.window.terminalMessage, undefined);
    assert.deepEqual(canvas.messages, ['terminal:test-token']);
});

test('window, palette and settings shortcuts retain their application commands', () => {
    const canvas = page();
    for (const key of ['w', 'e', ',']) canvas.press({ key });
    assert.deepEqual(canvas.messages, ['w', 'e', ',']);
});
