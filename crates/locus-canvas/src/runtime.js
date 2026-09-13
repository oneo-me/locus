(() => {
    const resourceUrl = uri => typeof uri === 'string' && uri.startsWith('locus://')
        ? new URL('./_locus/resource/' + uri.slice(8), location.href).href : uri;
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
    window.locus = Object.freeze({ resourceUrl });
})();
