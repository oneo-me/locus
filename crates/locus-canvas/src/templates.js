(() => {
    async function loadCanvas(uri, data) {
        const target = document.body;
        if (typeof uri !== 'string' || !uri.startsWith('locus://')) {
            throw new TypeError('Canvas templates require a locus:// note address');
        }
        if (!(target instanceof HTMLElement) || !target.isConnected) {
            throw new TypeError('Canvas template target must be a connected HTML element');
        }
        const response = await fetch('/_locus/template?uri=' + encodeURIComponent(uri));
        if (!response.ok) throw new Error('Cannot load canvas template: ' + uri);
        const source = await response.text();
        if (!source.trim()) throw new Error('Canvas template is empty: ' + uri);
        const fragment = document.createElement('template');
        fragment.innerHTML = source;
        const scripts = Array.from(fragment.content.querySelectorAll('script'));
        const executable = [];
        for (const script of scripts) {
            const type = (script.getAttribute('type') || '').trim().toLowerCase();
            if (script.hasAttribute('src') || !['', 'text/javascript', 'application/javascript', 'application/json', 'application/ld+json'].includes(type)) {
                throw new Error('Canvas templates support inline classic scripts and JSON only');
            }
            if (!['application/json', 'application/ld+json'].includes(type)) executable.push(script);
        }
        if (!target.isConnected) throw new Error('Canvas template target was removed');
        const root = document.createElement('div');
        root.style.display = 'contents';
        const context = Object.freeze({ root, data });
        root.append(fragment.content);
        target.append(root);
        try {
            for (const original of executable) {
                const script = document.createElement('script');
                for (const attribute of original.attributes) script.setAttribute(attribute.name, attribute.value);
                script.canvas = context;
                script.textContent = original.textContent;
                let failure;
                const onError = event => { failure = event.error || new Error(event.message); };
                window.addEventListener('error', onError);
                try { original.replaceWith(script); }
                finally { window.removeEventListener('error', onError); }
                if (failure) throw failure;
            }
        } catch (error) {
            root.remove();
            throw error;
        }
    }
    window.locus = Object.freeze({ loadCanvas });
})();
