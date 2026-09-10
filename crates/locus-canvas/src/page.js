(() => {
    const send = window.ipc.postMessage.bind(window.ipc);
    const terminalMessage = '__LOCUS_TERMINAL_MESSAGE__';
    window.addEventListener('focus', function () {
        send('focus');
    });
    window.addEventListener('contextmenu', function (event) {
        event.preventDefault();
        event.stopImmediatePropagation();
    }, true);
    window.addEventListener('keydown', function (event) {
        const key = event.key.toLowerCase();
        if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) return;
        if (key === '`') {
            event.preventDefault();
            if (event.isTrusted && !event.repeat) send(terminalMessage);
        } else if (key === 'w' || key === 'e' || key === ',') {
            event.preventDefault();
            if (!event.repeat) send(key);
        }
    }, true);
})();
