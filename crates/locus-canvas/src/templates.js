(() => {
    const resourceUrl = uri => typeof uri === 'string' && uri.startsWith('locus://')
        ? new URL('_locus/resource/' + uri.slice(8), location.href).href : uri;
    const nativeFetch = window.fetch.bind(window);
    window.fetch = (input, options) => nativeFetch(
        typeof input === 'string' || input instanceof URL ? resourceUrl(String(input)) : input, options);

    const rewriteElement = element => {
        for (const attribute of ['src', 'href', 'poster', 'srcset', 'style']) {
            const value = element.getAttribute(attribute);
            if (value?.includes('locus://')) element.setAttribute(attribute,
                value.replace(/locus:\/\/[^\s"'<>),]+/g, resourceUrl));
        }
    };
    new MutationObserver(records => {
        for (const record of records) {
            if (record.type === 'attributes') rewriteElement(record.target);
            for (const node of record.addedNodes) {
                if (node.nodeType !== 1) continue;
                rewriteElement(node);
                node.querySelectorAll('*').forEach(rewriteElement);
            }
        }
    }).observe(document.documentElement, { subtree: true, childList: true, attributes: true,
        attributeFilter: ['src', 'href', 'poster', 'srcset', 'style'] });
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
        for (const element of fragment.content.querySelectorAll('*')) {
            for (const attribute of ['src', 'href', 'poster']) {
                const value = element.getAttribute(attribute);
                if (value?.startsWith('locus://')) element.setAttribute(attribute, resourceUrl(value));
            }
        }
        const scripts = Array.from(fragment.content.querySelectorAll('script'));
        const executable = [];
        for (const script of scripts) {
            const type = (script.getAttribute('type') || '').trim().toLowerCase();
            if (!['', 'text/javascript', 'application/javascript', 'module', 'application/json', 'application/ld+json'].includes(type)) {
                throw new Error('Unsupported canvas script type');
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
                const external = script.hasAttribute('src') || script.type === 'module';
                if (!external) script.textContent += '\n;document.currentScript.dispatchEvent(new Event("locus:script-executed"));';
                let onError;
                try {
                    await new Promise((resolve, reject) => {
                        onError = event => reject(event.error || new Error(event.message));
                        window.addEventListener('error', onError);
                        script.addEventListener(external ? 'load' : 'locus:script-executed', resolve, { once: true });
                        script.onerror = () => reject(new Error('Cannot load canvas script: ' + script.src));
                        original.replaceWith(script);
                    });
                } finally { window.removeEventListener('error', onError); }
            }
        } catch (error) {
            root.remove();
            throw error;
        }
    }
    window.locus = Object.freeze({ loadCanvas, resourceUrl });
})();
