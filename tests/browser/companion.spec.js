import { test, expect } from '@playwright/test';
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn } from 'node:child_process';
import net from 'node:net';
import http from 'node:http';
import dgram from 'node:dgram';
import { createInterface } from 'node:readline';

let directory, process, url, admin;
async function api(path, { token = admin, workspace, method = 'GET', body } = {}) {
  const response = await fetch(`${url}/api${path}`, {
    method,
    headers: { Authorization: `Bearer ${token}`, ...(workspace ? { 'x-atw-workspace': workspace } : {}), ...(body ? { 'Content-Type': 'application/json' } : {}) },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!response.ok) throw new Error(`API ${path} returned ${response.status}: ${await response.text()}`);
  return response.status === 204 || response.headers.get('content-length') === '0' ? null : response.json();
}
test.beforeAll(async () => {
  directory = await mkdtemp(join(tmpdir(), 'atw-browser-'));
  const port = await new Promise(resolve => { const s = net.createServer(); s.listen(0, '127.0.0.1', () => { const p = s.address().port; s.close(() => resolve(p)); }); });
  url = `http://127.0.0.1:${port}`;
  process = spawn(resolve('target/debug/atw'), ['--data-dir', directory, 'serve', '--port', String(port), '--no-open'], { stdio: 'ignore' });
  await expect.poll(async () => { try { return (await fetch(`${url}/health`)).ok; } catch { return false; } }).toBe(true);
  // Only test-generated local credentials are used, and are never logged.
  admin = (await readFile(join(directory, 'admin.token'), 'utf8')).trim();
});
async function openConversation(page, title) {
  await page.keyboard.press('Control+k');
  await page.getByRole('combobox', { name: 'Find a command or visual' }).fill(title);
  await page.getByRole('option').filter({ hasText: title }).filter({ hasText: 'Conversation' }).first().click();
  await expect(page.locator('#workspace-title')).toHaveText(title);
}
async function openFeedback(page) {
  const toggle = page.getByRole('button', { name: /^Feedback/ });
  if (await toggle.getAttribute('aria-expanded') !== 'true') await toggle.click();
}
test.afterAll(async () => { process?.kill(); if (directory) await rm(directory, { recursive: true, force: true }); });

test('a visual appears, revisions stay inspectable, and comments target the viewed revision', async ({ page }) => {
  const workspace = await api('/workspaces', { method: 'POST', body: { title: 'Architecture discussion' } });
  await page.goto(`${url}/#token=${encodeURIComponent(admin)}`);
  await openConversation(page, 'Architecture discussion');
  const publish = body => api('/visuals', { token: workspace.token, method: 'POST', body });
  const first = await publish({ title: 'The visual loop', format: 'mermaid', content: 'graph LR; Agent-->Companion; Companion-->Person; Person-->|Comment|Agent' });
  await expect(page.getByRole('heading', { name: 'The visual loop' })).toBeVisible();
  await expect.poll(async () => (await api(`/status/${first.id}/1`, { token: workspace.token })).render_status).toBe('rendered');
  const second = await publish({ id: first.id, expected_revision: 1, title: 'The visual loop', format: 'html', content: '<h1>Second version</h1><button onclick="this.textContent=\'Selected\'">Choose</button>' });
  await expect(page.frameLocator('#visual-frame').getByRole('heading', { name: 'Second version' })).toBeVisible();
  await page.frameLocator('#visual-frame').getByRole('button', { name: 'Choose' }).click();
  await expect(page.frameLocator('#visual-frame').getByRole('button', { name: 'Selected' })).toBeVisible();
  await page.getByLabel('Revision').selectOption('1');
  await publish({ id: first.id, expected_revision: second.revision, title: 'The visual loop', format: 'html', content: '<h1>Third version</h1>' });
  await expect(page.getByRole('button', { name: 'Show latest (revision 3)' })).toBeVisible();
  await expect(page.getByLabel('Revision')).toHaveValue('1');
  await openFeedback(page);
  await page.getByLabel('Comment').fill('Explain the feedback arrow.');
  await page.getByRole('button', { name: 'Add comment' }).click();
  await expect.poll(async () => (await api('/feedback', { token: workspace.token })).comments.length).toBe(1);
  const feedback = await api('/feedback', { token: workspace.token });
  expect(feedback.comments[0].revision).toBe(1);
  await expect(page.frameLocator('#visual-frame').locator('svg')).toBeVisible();
  await page.screenshot({ path: 'test-results/visual-companion.png', fullPage: true });
  await page.getByRole('button', { name: 'Show latest (revision 3)' }).click();
  await expect(page.frameLocator('#visual-frame').getByRole('heading', { name: 'Third version' })).toBeVisible();
});

test('interactive content cannot read owner state or reach external and control endpoints', async ({ page }) => {
  let requests = 0;
  const probe = http.createServer((_request, response) => { requests++; response.end('unexpected network'); });
  await new Promise(resolve => probe.listen(0, '127.0.0.1', resolve));
  try {
    const origin = `http://127.0.0.1:${probe.address().port}`;
    await writeFile(join(directory,'filesystem-probe.txt'),'This is an innocuous test fixture.');
    const w = await api('/workspaces', { method: 'POST', body: { title: 'Isolation checks' } });
    await page.goto(`${url}/#token=${encodeURIComponent(admin)}`);
    await openConversation(page, 'Isolation checks');
    const visual = await api('/visuals', { token: w.token, method: 'POST', body: {
      title: 'An isolated interaction', format: 'html', content: `<h1>Interactive, not privileged</h1><p id="access"></p><p id="file"></p><button onclick="this.textContent='It works'">Try it</button><script>
      try { parent.sessionStorage.getItem('atw.owner'); document.getElementById('access').textContent='Owner state exposed'; } catch { document.getElementById('access').textContent='Owner state inaccessible'; }
      fetch('${origin}/fetch').catch(()=>{}); new Image().src='${origin}/image';
      fetch('/api/workspaces').catch(()=>{});
      fetch('file://${directory}/filesystem-probe.txt').then(()=>{document.getElementById('file').textContent='Filesystem exposed'}).catch(()=>{document.getElementById('file').textContent='Filesystem access refused'});
      setTimeout(()=>{location.href='${origin}/navigation'},1000);
      </script>`
    } });
    const frame = page.frameLocator('#visual-frame');
    await expect(frame.getByText('Owner state inaccessible')).toBeVisible();
    await expect(frame.getByText('Filesystem access refused')).toBeVisible();
    await frame.getByRole('button', { name: 'Try it' }).click();
    await expect(frame.getByRole('button', { name: 'It works' })).toBeVisible();
    await expect.poll(async () => (await api(`/status/${visual.id}/1`, { token: w.token })).render_status).toBe('rendered');
    // The content navigates itself after 1s, leaving time to prove the click works first.
    await page.waitForTimeout(1300);
    expect(requests).toBe(0);
    // Chromium may replace a self-navigating frame with its blocked-navigation
    // page; the contract is that no request reaches the destination.
  } finally { await new Promise(resolve => probe.close(resolve)); }
});

test('all image formats render and malformed diagrams report failure rather than acceptance', async ({ page }) => {
  const w = await api('/workspaces', { method: 'POST', body: { title: 'Rendering checks' } });
  await page.goto(`${url}/#token=${encodeURIComponent(admin)}`);
  await openConversation(page, 'Rendering checks');
  const images = await page.evaluate(() => {
    const canvas = document.createElement('canvas'); canvas.width = 60; canvas.height = 40;
    const context = canvas.getContext('2d'); context.fillStyle = '#275ea8'; context.fillRect(0, 0, 60, 40);
    return ['png', 'jpeg', 'webp'].map(format => ({ format, content: canvas.toDataURL(`image/${format}`).split(',')[1] }));
  });
  images.push({ format: 'svg', content: '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect width="200" height="100" fill="#275ea8"/></svg>' });
  for (const image of images) {
    const visual = await api('/visuals', { token: w.token, method: 'POST', body: { title: `A ${image.format} drawing`, ...image } });
    await expect.poll(async () => (await api(`/status/${visual.id}/1`, { token: w.token })).render_status).toBe('rendered');
    await expect(page.frameLocator('#visual-frame').getByRole('img')).toBeVisible();
  }
  for (const invalid of [{ format: 'mermaid', content: 'not a diagram at all' }, { format: 'svg', content: '<svg not-valid' }]) {
    const visual = await api('/visuals', { token: w.token, method: 'POST', body: { title: 'Malformed source', ...invalid } });
    await expect.poll(async () => (await api(`/status/${visual.id}/1`, { token: w.token })).render_status).toBe('failed');
    expect((await api(`/status/${visual.id}/1`, { token: w.token })).error).toBeTruthy();
  }
});

test('another workspace stays unread without switching views and offline publication survives reconnect', async ({ page, context }) => {
  const a = await api('/workspaces', { method: 'POST', body: { title: 'Currently reading' } });
  const b = await api('/workspaces', { method: 'POST', body: { title: 'Another agent' } });
  await page.goto(`${url}/#token=${encodeURIComponent(admin)}`);
  await openConversation(page, 'Currently reading');
  await api('/visuals', { token: a.token, method: 'POST', body: { title: 'Keep this visible', format: 'html', content: '<h1>Reading this</h1>' } });
  await expect(page.frameLocator('#visual-frame').getByRole('heading', { name: 'Reading this' })).toBeVisible();
  const other = await api('/visuals', { token: b.token, method: 'POST', body: { title: 'Do not switch', format: 'html', content: '<h1>Another conversation</h1>' } });
  await expect(page.getByRole('button', { name: 'New visuals in Another agent' })).toBeVisible();
  await expect(page.frameLocator('#visual-frame').getByRole('heading', { name: 'Reading this' })).toBeVisible();
  expect((await api(`/status/${other.id}/1`, { token: b.token })).render_status).toBe('accepted');
  await page.close();
  await expect.poll(async () => (await api(`/status/${other.id}/1`, { token: b.token })).viewer_connected, { timeout: 7000 }).toBe(false);
  const offline = await api('/visuals', { token: b.token, method: 'POST', body: { id: other.id, expected_revision: 1, title: 'Updated while closed', format: 'html', content: '<h1>Latest after reconnect</h1>' } });
  expect(offline.presentation.viewer_connected).toBe(false);
  const reopened = await context.newPage();
  await reopened.goto(`${url}/#token=${encodeURIComponent(admin)}`);
  await openConversation(reopened, 'Another agent');
  await expect(reopened.frameLocator('#visual-frame').getByRole('heading', { name: 'Latest after reconnect' })).toBeVisible();
  await expect(reopened.getByLabel('Revision')).toHaveValue('2');
  await expect(reopened.getByLabel('Revision').locator('option')).toHaveCount(2);
});

test('a comment draft keeps its original revision when a live update arrives', async ({ page }) => {
  const w = await api('/workspaces', { method: 'POST', body: { title: 'Draft anchoring' } });
  await page.goto(`${url}/#token=${encodeURIComponent(admin)}`);
  await openConversation(page, 'Draft anchoring');
  const first = await api('/visuals', { token: w.token, method: 'POST', body: { title: 'Draft target', format: 'html', content: '<h1>Original</h1>' } });
  await expect(page.frameLocator('#visual-frame').getByRole('heading', { name: 'Original' })).toBeVisible();
  await openFeedback(page);
  await page.getByLabel('Comment').fill('This comment is about the original.');
  await api('/visuals', { token: w.token, method: 'POST', body: { id: first.id, expected_revision: 1, title: 'Draft target', format: 'html', content: '<h1>Updated</h1>' } });
  await expect(page.frameLocator('#visual-frame').getByRole('heading', { name: 'Updated' })).toBeVisible();
  await page.getByRole('button', { name: 'Add comment' }).click();
  await expect.poll(async () => (await api('/feedback', { token: w.token })).comments.length).toBe(1);
  expect((await api('/feedback', { token: w.token })).comments[0].revision).toBe(1);
});

test('deletion targets the displayed visual even while another publication is loading', async ({ page }) => {
  const w = await api('/workspaces', { method: 'POST', body: { title: 'Deletion identity' } });
  await page.goto(`${url}/#token=${encodeURIComponent(admin)}`);
  await openConversation(page, 'Deletion identity');
  const a = await api('/visuals', { token: w.token, method: 'POST', body: { title: 'Still displayed', format: 'html', content: '<h1>Still displayed</h1>' } });
  await expect(page.frameLocator('#visual-frame').getByRole('heading', { name: 'Still displayed' })).toBeVisible();
  let release, blocked = false;
  const gate = new Promise(resolve => { release = resolve; });
  await page.route('**/api/visuals/*', async route => {
    if (route.request().method() === 'GET' && !route.request().url().includes(a.id)) { blocked = true; await gate; }
    await route.continue();
  });
  const b = await api('/visuals', { token: w.token, method: 'POST', body: { title: 'Still loading', format: 'html', content: '<h1>Still loading</h1>' } });
  await expect.poll(() => blocked).toBe(true);
  page.once('dialog', dialog => dialog.accept());
  await page.keyboard.press('Control+k'); await page.keyboard.type('Delete this visual'); await page.keyboard.press('Enter');
  await expect.poll(async () => (await api('/visuals', { token: w.token })).some(v => v.id === a.id)).toBe(false);
  release();
  expect((await api('/visuals', { token: w.token })).some(v => v.id === b.id)).toBe(true);
});

test('the rendering boundary blocks WebRTC STUN traffic as well as HTTP', async ({ page }) => {
  const socket = dgram.createSocket('udp4'); let datagrams = 0;
  socket.on('message', () => { datagrams++; });
  await new Promise(resolve => socket.bind(0, '127.0.0.1', resolve));
  try {
    const w = await api('/workspaces', { method: 'POST', body: { title: 'WebRTC boundary' } });
    await page.goto(`${url}/#token=${encodeURIComponent(admin)}`);
    await openConversation(page, 'WebRTC boundary');
    await api('/visuals', { token: w.token, method: 'POST', body: { title: 'UDP probe', format: 'html', content: `<p id="probe">Starting probe</p><script>
      (async()=>{try { const pc=new RTCPeerConnection({iceServers:[{urls:'stun:127.0.0.1:${socket.address().port}'}]}); pc.createDataChannel('probe'); await pc.setLocalDescription(await pc.createOffer()); document.getElementById('probe').textContent='Probe attempted'; } catch { document.getElementById('probe').textContent='Probe blocked'; }})();
      </script>` } });
    await expect(page.frameLocator('#visual-frame').locator('#probe')).toHaveText(/Probe (attempted|blocked)/);
    await page.waitForTimeout(500);
    expect(datagrams).toBe(0);
  } finally { socket.close(); }
});

test('a lost rendering acknowledgment is retried without replacing the visual', async ({ page }) => {
  const w = await api('/workspaces', { method: 'POST', body: { title: 'Acknowledgment retry' } });
  await page.goto(`${url}/#token=${encodeURIComponent(admin)}`);
  await openConversation(page, 'Acknowledgment retry');
  let attempts = 0;
  await page.route('**/api/render', route => { attempts++; return attempts === 1 ? route.abort() : route.continue(); });
  const visual = await api('/visuals', { token: w.token, method: 'POST', body: { title: 'Retry confirmation', format: 'html', content: '<h1>Rendered once</h1>' } });
  await expect(page.frameLocator('#visual-frame').getByRole('heading', { name: 'Rendered once' })).toBeVisible();
  await expect.poll(async () => (await api(`/status/${visual.id}/1`, { token: w.token })).render_status).toBe('rendered');
  expect(attempts).toBeGreaterThan(1);
});

test('an MCP tool call reaches the viewer and the agent reads the person’s comment back', async ({ page }) => {
  const w = await api('/workspaces', { method: 'POST', body: { title: 'MCP round trip' } });
  const connection = join(directory, 'mcp-round-trip.json');
  await writeFile(connection, JSON.stringify({ url, workspace: w.workspace, token: w.token }), { mode: 0o600 });
  const child = spawn(resolve('target/debug/atw'), ['--connection', connection, 'mcp'], { stdio: ['pipe', 'pipe', 'ignore'] });
  const pending = new Map(); let next = 1;
  const lines = createInterface({ input: child.stdout });
  lines.on('line', line => { const response = JSON.parse(line); pending.get(response.id)?.(response); });
  const ask = (method, params = {}) => new Promise((resolve, reject) => {
    const id = next++;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`MCP ${method} timed out`)); }, 5000);
    pending.set(id, response => { clearTimeout(timer); pending.delete(id); response.error ? reject(new Error(response.error.message)) : resolve(response.result); });
    child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`);
  });
  const tool = (name, args = {}) => ask('tools/call', { name, arguments: args });
  try {
    await ask('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'atw-browser-integration', version: '1' } });
    child.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n');
    expect((await ask('tools/list')).tools.some(t => t.name === 'show_visual')).toBe(true);
    await page.goto(`${url}/#token=${encodeURIComponent(admin)}`);
    await openConversation(page, 'MCP round trip');
    const shown = await tool('show_visual', { title: 'From the agent’s actual tool', format: 'mermaid', local_file: resolve('examples/loop.mmd') });
    expect(shown.isError).toBe(false);
    const identity = { id: shown.structuredContent.id, revision: shown.structuredContent.revision };
    await expect(page.getByRole('heading', { name: 'From the agent’s actual tool', exact: true })).toBeVisible();
    await expect.poll(async () => (await tool('visual_status', identity)).structuredContent.render_status).toBe('rendered');
    await openFeedback(page);
    await page.getByLabel('Comment').fill('Please explain the return path.');
    await page.getByRole('button', { name: 'Add comment' }).click();
    const feedback = await tool('read_feedback', { after: 0, wait_ms: 1000 });
    expect(feedback.structuredContent.comments[0]).toMatchObject({ visual: identity.id, revision: 1, text: 'Please explain the return path.' });
  } finally { lines.close(); child.kill(); }
});

test('keyboard-only workflow switches conversations, inspects revisions, and comments', async ({ page }) => {
  const w=await api('/workspaces',{method:'POST',body:{title:'Keyboard discussion'}});
  await api('/workspaces',{method:'POST',body:{title:'Another keyboard conversation'}});
  await api('/visuals',{token:w.token,method:'POST',body:{title:'Other keyboard visual',format:'html',content:'<h1>Another visual</h1>'}});
  const first=await api('/visuals',{token:w.token,method:'POST',body:{title:'Keyboard diagram',format:'html',content:'<h1>Original keyboard visual</h1>'}});
  await api('/visuals',{token:w.token,method:'POST',body:{id:first.id,expected_revision:1,title:'Keyboard diagram',format:'html',content:'<h1>Latest keyboard visual</h1>'}});
  await page.goto(`${url}/#token=${encodeURIComponent(admin)}`);
  await expect(page.getByRole('button',{name:'Quick switcher'})).toBeVisible();
  const switchTo=async query=>{
    await page.keyboard.press('Control+k');
    await expect(page.getByRole('combobox',{name:'Find a command or visual'})).toBeFocused();
    await page.keyboard.type(query);
    await expect(page.getByRole('listbox',{name:'Matches'}).getByRole('option').filter({hasText:query}).first()).toBeVisible();
    await page.keyboard.press('Enter');
  };
  await switchTo('Keyboard discussion');
  await expect(page.frameLocator('#visual-frame').getByRole('heading',{name:'Latest keyboard visual'})).toBeVisible();
  await page.keyboard.press('Control+k'); await page.keyboard.type('keyboard');
  await page.keyboard.press('ArrowDown'); await page.keyboard.press('Enter');
  await expect(page.frameLocator('#visual-frame').getByRole('heading',{name:'Another visual'})).toBeVisible();
  for(let poll=0;poll<2;poll++)await page.waitForResponse(response=>response.url().endsWith('/api/visuals')&&response.request().method()==='GET');
  await expect(page.frameLocator('#visual-frame').getByRole('heading',{name:'Another visual'})).toBeVisible();
  await switchTo('Keyboard diagram');
  await page.keyboard.press('[');
  await expect(page.getByLabel('Revision')).toHaveValue('1');
  await page.keyboard.press('c');
  await expect(page.getByLabel('Comment')).toBeFocused();
  await page.keyboard.type('Explain the original version.'); await page.keyboard.press('Control+Enter');
  await expect.poll(async()=>(await api('/feedback',{token:w.token})).comments.length).toBe(1);
  expect((await api('/feedback',{token:w.token})).comments[0].revision).toBe(1);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('region',{name:'Visual surface'})).toBeFocused();
  await page.keyboard.press('g');
  await expect(page.getByLabel('Revision')).toHaveValue('2');
  await page.keyboard.press('?');
  await expect(page.getByRole('dialog',{name:'Keyboard shortcuts'})).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog',{name:'Keyboard shortcuts'})).not.toBeVisible();
  await switchTo('Another keyboard conversation');
  await expect(page.locator('#workspace-title')).toHaveText('Another keyboard conversation');
  await switchTo('Keyboard discussion');
  await switchTo('Keyboard diagram');
  await expect(page.frameLocator('#visual-frame').getByRole('heading',{name:'Latest keyboard visual'})).toBeVisible();
  await page.keyboard.press('c');
  await expect(page.getByLabel('Comment')).toBeFocused();
  await api('/visuals',{token:w.token,method:'POST',body:{id:first.id,expected_revision:2,title:'Keyboard diagram',format:'html',content:'<h1>Updated without stealing focus</h1>'}});
  await expect(page.frameLocator('#visual-frame').getByRole('heading',{name:'Updated without stealing focus'})).toBeVisible();
  await expect(page.getByLabel('Comment')).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('complementary',{name:'Visual feedback'})).toBeHidden();
});
