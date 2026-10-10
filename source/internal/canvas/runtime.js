(() => {
  'use strict';
  const config = __LOCUS_CONFIG__;
  const root = new URL(config.root, location.href);
  const exported = !location.href.startsWith('locus-canvas:');
  function resourceUrl(uri) {
    if (!uri.startsWith('locus://')) throw new Error('Expected a locus:// resource address');
    const value = uri.slice(8);
    // The Go handler is the authority; reject traversal before URL normalization too.
    const [pathname, query = ''] = value.split('?');
    if (pathname.includes('#') || pathname.split('/').some(part => {
      try { const p = decodeURIComponent(part); return !p || p === '.' || p === '..' || /[/\\\0]/.test(p); }
      catch { return true; }
    })) throw new Error('Invalid resource address');
    let suffix = '';
    if (exported && query) {
      const part = new URLSearchParams(query).get('part');
      if (part !== 'yaml' && part !== 'body') throw new Error('Invalid note part');
      suffix = '.locus-' + part;
    }
    return new URL('files/' + pathname + suffix + (!exported && query ? '?' + query : ''), root).href;
  }
  Object.defineProperty(window, 'locus', { value: Object.freeze({ resourceUrl }) });
  const nativeFetch = window.fetch.bind(window);
  window.fetch = (input, init) => nativeFetch(typeof input === 'string' && input.startsWith('locus://') ? resourceUrl(input) : input, init);
  function resources(node) {
    if (!(node instanceof Element)) return;
    for (const el of [node, ...node.querySelectorAll('[src],[poster],link[href]')]) {
      for (const attr of ['src', 'poster', ...(el.tagName === 'LINK' ? ['href'] : [])]) {
        const value = el.getAttribute(attr);
        if (value && value.startsWith('locus://')) el.setAttribute(attr, resourceUrl(value));
      }
    }
  }
  new MutationObserver(records => {
    for (const r of records) { if (r.type === 'attributes') resources(r.target); else for (const n of r.addedNodes) resources(n); }
  }).observe(document.documentElement, { childList: true, subtree: true, attributes: true, attributeFilter: ['src', 'poster', 'href'] });
  function theme() { document.documentElement.classList.toggle('dark', matchMedia('(prefers-color-scheme: dark)').matches); }
  theme(); matchMedia('(prefers-color-scheme: dark)').addEventListener('change', theme);
  // Only trusted browser input may request native commands. No Go service or
  // filesystem-writing methods are bound to page JavaScript.
  document.addEventListener('keydown', e => {
    if (exported || !e.isTrusted || e.repeat || e.isComposing) return;
    if (e.key === 'F5' || ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'r')) {
      e.preventDefault(); location.reload();
    }
  }, true);
  window.addEventListener('load', async () => {
    try {
      const entry = await import(new URL(config.main, root).href);
      if (typeof entry.main !== 'function') throw new Error('main.js must export main(data)');
      await entry.main(config.data);
      document.documentElement.dataset.locusReady = 'true';
    } catch (error) {
      const notice = document.createElement('pre'); notice.className = 'locus-error';
      notice.textContent = 'Canvas failed: ' + String(error) + '\nFix the saved files and refresh to retry.';
      document.body.replaceChildren(notice);
      document.documentElement.dataset.locusError = String(error);
    }
  }, { once: true });
})();
