window.addEventListener('focus', function () {
    window.ipc.postMessage('focus');
});
window.addEventListener('contextmenu', function (event) {
    event.preventDefault();
    event.stopImmediatePropagation();
}, true);
window.addEventListener('keydown', function (event) {
    const key = event.key.toLowerCase();
    if (key === 'f5' || ((event.metaKey || event.ctrlKey) && key === 'r')) {
        event.preventDefault();
        event.stopImmediatePropagation();
        return;
    }
    if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) return;
    if (key === 'w' || key === 'e' || key === ',') {
        event.preventDefault();
        window.ipc.postMessage(key);
    }
}, true);
