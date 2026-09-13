async function runCanvas(importMain) {
    try {
        const { main } = await importMain();
        if (typeof main !== 'function') throw new TypeError('canvas/main.js must export main(data)');
        const data = JSON.parse(document.getElementById('locus-canvas-data').textContent);
        await main(data);
    } catch (error) {
        console.error(error);
        const message = document.createElement('pre');
        message.setAttribute('role', 'alert');
        message.style.cssText = 'white-space:pre-wrap;padding:1rem';
        message.textContent = 'Canvas: ' + (error?.message || String(error));
        document.body.replaceChildren(message);
    }
}
