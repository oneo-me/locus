bundle => {
    const nativeFetch = window.fetch.bind(window);
    const base = new URL('.', location.href);
    const decode = value => decodeURIComponent(value.replace(/%(?![0-9a-f]{2})/gi, '%25'));
    const templateKey = uri => {
        if (!uri?.startsWith('locus://') || /[?\\#\0]/.test(uri)) return null;
        const parts = uri.slice(8).split('/').map(decode);
        if (parts.some(part => !part || part === '.' || part === '..' || /[/\\\0]/.test(part))) return null;
        return parts.join('/');
    };
    const localName = value => {
        const url = new URL(value, location.href);
        const library = url.pathname.indexOf('/_locus/resource/');
        if (url.protocol === base.protocol && url.host === base.host && library >= 0) {
            const key = url.pathname.slice(library + 1);
            const exact = key + url.search;
            return Object.hasOwn(bundle.resources, exact) ? exact : key;
        }
        if (url.protocol !== base.protocol || url.host !== base.host || !url.pathname.startsWith(base.pathname)) return null;
        return decode(url.pathname.slice(base.pathname.length));
    };
    window.fetch = async (input, options) => {
        const value = typeof input === 'string' || input instanceof URL ? String(input) : input.url;
        const url = new URL(value, location.href);
        if (url.protocol === base.protocol && url.host === base.host && url.pathname === '/_locus/template') {
            const uri = url.searchParams.get('uri');
            const key = templateKey(uri);
            const source = key !== null && Object.hasOwn(bundle.templates, key) ? bundle.templates[key] : undefined;
            return new Response(source ?? '', { status: source === undefined ? 404 : 200 });
        }
        const name = localName(value);
        if (name !== null && Object.hasOwn(bundle.resources, name)) {
            const resource = bundle.resources[name];
            const [header, encoded] = resource.split(',');
            const bytes = Uint8Array.from(atob(encoded), character => character.charCodeAt(0));
            return new Response(bytes, { headers: { 'Content-Type': header.slice(5, -7) } });
        }
        return nativeFetch(input, options);
    };
}
