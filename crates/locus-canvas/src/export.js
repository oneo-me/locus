bundle => {
    const nativeFetch = window.fetch.bind(window);
    const base = new URL('.', location.href);
    const decode = value => decodeURIComponent(value.replace(/%(?![0-9a-f]{2})/gi, '%25'));
    const localName = value => {
        const url = new URL(value, document.baseURI);
        if (url.protocol !== base.protocol || url.host !== base.host || !url.pathname.startsWith(base.pathname)) return null;
        const path = url.pathname.slice(base.pathname.length);
        const name = path.startsWith('_locus/resource/') ? path : decode(path);
        return Object.hasOwn(bundle.resources, name + url.search) ? name + url.search : name;
    };
    window.fetch = async (input, options) => {
        const value = typeof input === 'string' || input instanceof URL ? String(input) : input.url;
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
