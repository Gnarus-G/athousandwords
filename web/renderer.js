import mermaid from 'mermaid';

let nonce, failed = false;
function report(error) {
  if (error) failed = true;
  parent.postMessage({ type: 'atw:render', nonce, error: error ? String(error).slice(0, 4000) : null }, '*');
}
window.addEventListener('error', event => { if (nonce) report(event.message || 'Script failed'); });
window.addEventListener('unhandledrejection', event => { if (nonce) report(event.reason?.message || 'Script promise rejected'); });
window.addEventListener('message', async event => {
  if (event.source !== parent || event.data?.type !== 'atw:content' || nonce) return;
  nonce = event.data.nonce;
  const { format, content } = event.data;
  const root = document.getElementById('render-root');
  try {
    const chromium = /(?:Chrome|Chromium)\/(\d+)/.exec(navigator.userAgent);
    if (!chromium || Number(chromium[1]) < 152) throw new Error('Use Chromium 152 or newer: this viewer requires Connection-Allowlist network isolation.');
    if (format === 'html') {
      root.innerHTML = content;
      // innerHTML does not execute scripts. Activate them inside this opaque sandbox.
      for (const previous of root.querySelectorAll('script')) {
        const script = document.createElement('script');
        for (const attribute of previous.attributes) script.setAttribute(attribute.name, attribute.value);
        script.textContent = previous.textContent; previous.replaceWith(script);
      }
    } else if (format === 'mermaid') {
      mermaid.initialize({ startOnLoad: false, securityLevel: 'strict', theme: 'neutral', fontFamily: 'Noto Sans, sans-serif', maxTextSize: 100_000 });
      const result = await mermaid.render('diagram', content); root.innerHTML = result.svg;
    } else {
      const image = document.createElement('img'); image.alt = 'Agent-supplied visual';
      const ready = new Promise((resolve, reject) => { image.onload = resolve; image.onerror = () => reject(new Error(`Invalid ${format.toUpperCase()} image`)); });
      image.src = format === 'svg' ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(content)}` : `data:image/${format};base64,${content}`;
      root.append(image); await ready;
    }
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    if (!failed) report(null);
  } catch (error) { root.textContent = `Could not render: ${error.message}`; report(error.message); }
});
parent.postMessage({ type: 'atw:ready' }, '*');
