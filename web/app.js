const $ = id => document.getElementById(id);
const launchOptions = new URLSearchParams(location.search);
const desktop = launchOptions.get('desktop') === '1';
const fragment = new URLSearchParams(location.hash.slice(1));
if (fragment.has('token')) { sessionStorage.setItem('atw.owner', fragment.get('token')); history.replaceState(null, '', location.pathname); }
const token = sessionStorage.getItem('atw.owner');
const viewer = crypto.randomUUID();
let workspace = launchOptions.get('workspace') || localStorage.getItem('atw.workspace');
let selected = null, revision = null, follow = true, render = null, historyKey = '', commentsKey = '', busy = false;
let seen = JSON.parse(localStorage.getItem('atw.seen') || '{}');
let workspaceSignature = '', visualsSignature = '';
let knownWorkspaces = [], knownVisuals = [];
let keyboard = null;
let refreshQueued = false, pendingCommentFocus = false, commentPosting = false;
let epoch = 0;
const pendingAcknowledgments = new Map();
let sendingAcknowledgments = false;
async function flushAcknowledgments() {
  if (sendingAcknowledgments) return;
  sendingAcknowledgments = true;
  try {
    for (const [key, acknowledgment] of pendingAcknowledgments) {
      try { await api('/render', { method: 'POST', body: acknowledgment }); }
      catch (value) { if (value.status !== 404) throw value; }
      if (pendingAcknowledgments.get(key) === acknowledgment) pendingAcknowledgments.delete(key);
    }
  } finally { sendingAcknowledgments = false; }
}
let draftTarget = null;
function commentAvailability() {
  const ready = render && render.id === selected && render.revision === revision;
  $('comment').disabled = !ready && !draftTarget;
  $('comment-form').querySelector('button[type="submit"]').disabled = commentPosting || $('comment').disabled;
  if (pendingCommentFocus && !$('comment').disabled) { pendingCommentFocus = false; $('comment-status').textContent = ''; $('comment').focus(); }
}
function requestCommentFocus() {
  if ($('comment-form').hidden) return;
  if ($('comment').disabled) { pendingCommentFocus = true; $('comment-status').textContent = 'Loading the selected revision…'; }
  else $('comment').focus();
}
function commentContext() {
  $('comment-context').textContent = draftTarget
    ? `Draft refers to “${draftTarget.title}”, revision ${draftTarget.revision}.`
    : render ? `Your comment will refer to revision ${render.revision}.` : 'Comments stay with the revision you viewed.';
}
$('comment').addEventListener('input', () => {
  if (!$('comment').value) draftTarget = null;
  else if (!draftTarget && render) draftTarget = { visual: render.id, revision: render.revision, title: render.title };
  commentContext();
  commentAvailability();
});

async function api(path, { method = 'GET', body, scoped = false, workspaceId = workspace, keepalive = false } = {}) {
  const response = await fetch(`/api${path}`, {
    method, keepalive,
    headers: { Authorization: `Bearer ${token}`, ...(scoped ? { 'x-atw-workspace': workspaceId } : {}), ...(body ? { 'Content-Type': 'application/json' } : {}) },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!response.ok) { const text = await response.text(); throw Object.assign(new Error(response.status === 401 ? 'Open the desktop companion with “atw open” to reconnect.' : text), { status: response.status }); }
  const text = await response.text();
  return text ? JSON.parse(text) : null;
}
function error(value) { $('error').hidden = !value; $('error').textContent = value || ''; }
function button(text, action, current, key = text) {
  const element = document.createElement('button'); element.textContent = text; element.setAttribute('aria-label', text);
  element.dataset.key = key;
  element.setAttribute('aria-current', String(current)); element.onclick = action; return element;
}
function navigation(id, buttons) {
  const nav = $(id);
  const key = nav.contains(document.activeElement) ? document.activeElement.dataset.key : null;
  nav.replaceChildren(...buttons);
  if (key) [...nav.children].find(element => element.dataset.key === key)?.focus({ preventScroll: true });
}
function selectVisual(id) { epoch++; pendingCommentFocus = false; selected = id; revision = null; follow = true; commentAvailability(); void refresh(); }
function stepRevision(delta) {
  const latest = knownVisuals.find(visual => visual.id === selected)?.revision;
  const target = (revision ?? latest) + delta;
  if (!latest || target < 1 || target > latest) return;
  epoch++; revision = target; follow = false; $('revision').value = String(target); commentAvailability(); void refresh();
}
function followLatest() { epoch++; follow = true; revision = knownVisuals.find(visual => visual.id === selected)?.revision ?? revision; commentAvailability(); void refresh(); }
function resetVisual() {
  epoch++;
  selected = null; revision = null; follow = true; render = null; historyKey = ''; commentsKey = ''; visualsSignature = '';
  knownVisuals = [];
  keyboard?.refresh();
  pendingCommentFocus = false;
  $('visual-frame')?.remove(); $('empty').hidden = false; $('visual-title').textContent = 'Select a visual';
  $('description').hidden = true; document.querySelector('.revision-controls').hidden = true; $('comment-form').hidden = true; $('comments').replaceChildren();
  $('render-status').textContent = 'Waiting for a visual'; $('revision-note').textContent = '';
  commentAvailability();
}
function selectWorkspace(id) {
  workspace = id; localStorage.setItem('atw.workspace', id); workspaceSignature = ''; resetVisual(); void refresh();
}
async function refresh() {
  if (busy) { refreshQueued = true; return; } busy = true;
  let marker = epoch;
  try {
    await api('/viewer', { method: 'POST', body: { viewer, desktop } });
    await flushAcknowledgments();
    if (marker !== epoch) return;
    const workspaces = await api('/workspaces');
    if (marker !== epoch) return;
    knownWorkspaces = workspaces;
    keyboard?.refresh();
    if (!workspaces.some(w => w.id === workspace)) { workspace = workspaces[0]?.id || null; resetVisual(); marker = epoch; }
    const active = workspaces.find(w => w.id === workspace);
    if (active) { seen[workspace] = active.sequence; localStorage.setItem('atw.seen', JSON.stringify(seen)); }
    const signature = JSON.stringify([workspaces, workspace, seen]);
    if (signature !== workspaceSignature) {
      workspaceSignature = signature;
      navigation('workspaces', workspaces.map(w => {
        const element = button(w.title, () => selectWorkspace(w.id), w.id === workspace, w.id);
        const identity = document.createElement('small'); identity.textContent = w.id.slice(0, 8); identity.className = 'workspace-identity'; element.append(identity);
        if (w.id !== workspace && w.sequence > (seen[w.id] || 0)) { const badge = document.createElement('span'); badge.className = 'unread'; badge.textContent = 'New visuals'; element.append(badge); }
        return element;
      }));
    }
    $('workspace-title').textContent = active?.title || 'Your visual companion';
    $('delete-workspace').hidden = !active;
    if (active) await refreshVisuals(marker);
    $('connection-status').textContent = 'Connected to your companion'; error(null);
  } catch (value) { $('connection-status').textContent = 'Disconnected · retrying'; error(value.message); }
  finally { busy = false; if (refreshQueued) { refreshQueued = false; queueMicrotask(refresh); } }
}
async function refreshVisuals(marker) {
  const visuals = await api('/visuals', { scoped: true });
  if (marker !== epoch) return;
  const previousSequence = knownVisuals[0]?.sequence || 0;
  knownVisuals = visuals;
  keyboard?.refresh();
  if (!visuals.some(v => v.id === selected)) { selected = visuals[0]?.id || null; revision = null; follow = true; }
  if (follow && visuals[0] && render && visuals[0].sequence > previousSequence) { selected = visuals[0].id; revision = null; }
  const signature = JSON.stringify([visuals, selected]);
  if (signature !== visualsSignature) {
    visualsSignature = signature;
    navigation('visuals', visuals.map(v => button(v.title, () => selectVisual(v.id), v.id === selected, v.id)));
  }
  const visual = visuals.find(v => v.id === selected);
  if (!visual) { if (render) resetVisual(); return; }
  if (follow || revision === null) revision = visual.revision;
  commentAvailability();
  const revisions = await api(`/visuals/${selected}`, { scoped: true });
  if (marker !== epoch) return;
  const newHistoryKey = JSON.stringify([selected, revision, revisions]);
  if (newHistoryKey !== historyKey) {
    historyKey = newHistoryKey;
    $('revision').replaceChildren(...revisions.map(r => { const option = document.createElement('option'); option.value = r.revision; option.textContent = r.revision; return option; }));
    $('revision').value = String(revision);
  }
  $('latest').hidden = revision === visual.revision;
  $('latest').textContent = `Show latest (revision ${visual.revision})`;
  document.querySelector('.revision-controls').hidden = false;
  $('comment-form').hidden = false;
  if (!render || render.id !== selected || render.revision !== revision) {
    const content = await api(`/visuals/${selected}/revisions/${revision}`, { scoped: true });
    if (marker !== epoch) return;
    show(content, visual.sequence);
  }
  await refreshComments();
}
function show(content, sequence) {
  render = { ...content, workspace, sequence, nonce: crypto.randomUUID(), sent: false };
  $('visual-title').textContent = content.title; $('description').textContent = content.description; $('description').hidden = !content.description;
  $('empty').hidden = true; $('visual-frame')?.remove();
  $('render-status').textContent = 'Rendering…'; $('revision-note').textContent = `Revision ${content.revision}`;
  commentContext();
  commentAvailability();
  const frame = document.createElement('iframe'); frame.id = 'visual-frame'; frame.title = content.title;
  frame.setAttribute('sandbox', 'allow-scripts'); frame.referrerPolicy = 'no-referrer'; frame.src = '/frame'; $('canvas').append(frame);
}
window.addEventListener('message', async event => {
  if (!render || event.source !== $('visual-frame')?.contentWindow) return;
  if (event.data?.type === 'atw:ready' && !render.sent) {
    render.sent = true; event.source.postMessage({ type: 'atw:content', nonce: render.nonce, format: render.format, content: render.content }, '*'); return;
  }
  if (event.data?.type !== 'atw:render' || event.data.nonce !== render.nonce) return;
  const status = event.data.error ? 'failed' : 'rendered';
  $('render-status').textContent = event.data.error ? `Rendering failed: ${event.data.error}` : 'Rendered in this viewer';
  pendingAcknowledgments.set(render.nonce, { visual: render.id, revision: render.revision, status, desktop, error: event.data.error || null });
  try { await flushAcknowledgments(); }
  catch (value) { error(value.message); }
});
async function refreshComments() {
  let after = 0, all = [];
  // Feedback is cursor-paged; retain the exact revision association in the UI.
  while (true) {
    const page = await api(`/feedback?after=${after}`, { scoped: true }); all.push(...page.comments); after = page.cursor;
    if (page.comments.length < 100) break;
  }
  const comments = all.filter(c => c.visual === selected && c.revision === revision);
  const key = JSON.stringify(comments);
  if (key !== commentsKey) {
    commentsKey = key;
    $('comments').replaceChildren(...comments.map(c => { const article = document.createElement('article'); const context = document.createElement('small'); context.textContent = `Revision ${c.revision}`; const text = document.createElement('p'); text.textContent = c.text; article.append(context, text); return article; }));
  }
}
$('revision').onchange = () => { epoch++; revision = Number($('revision').value); follow = false; commentAvailability(); void refresh(); };
$('latest').onclick = followLatest;
$('comment-form').onsubmit = async event => {
  event.preventDefault(); if (!render) return;
  // Capture identity before awaiting; concurrent publications cannot retarget a comment.
  const target = { visual: draftTarget?.visual || render.id, revision: draftTarget?.revision || render.revision, text: $('comment').value };
  commentPosting = true; commentAvailability();
  try { await api('/comments', { method: 'POST', body: target }); $('comment').value = ''; draftTarget = null; commentContext(); $('comment-status').textContent = `Added to revision ${target.revision}`; await refreshComments(); }
  catch (value) { error(value.message); } finally { commentPosting = false; commentAvailability(); }
};
$('delete-visual').onclick = async () => {
  const target = render && { id: render.id, workspace: render.workspace, title: render.title };
  if (!target || !confirm(`Delete “${target.title}”, every revision, and its comments?`)) return;
  try { await api(`/visuals/${target.id}`, { method: 'DELETE', scoped: true, workspaceId: target.workspace }); for (const [key,ack] of pendingAcknowledgments) if (ack.visual === target.id) pendingAcknowledgments.delete(key); resetVisual(); await refresh(); } catch (value) { error(value.message); }
};
$('delete-workspace').onclick = async () => {
  if (!workspace || !confirm('Delete this conversation workspace and all its visuals, revisions, and comments? Its agent connection will stop working.')) return;
  try { await api('/workspace', { method: 'DELETE', scoped: true }); workspace = null; resetVisual(); await refresh(); } catch (value) { error(value.message); }
};
window.atwDisconnectViewer = () => api('/viewer', { method: 'DELETE', body: { viewer, desktop }, keepalive: true });
window.addEventListener('pagehide', () => { void window.atwDisconnectViewer().catch(() => {}); });
if (!token) error('Open the desktop companion with “atw open” to connect.');
else { void refresh(); setInterval(refresh, 1000); }
keyboard = installKeyboard({ getWorkspaces: () => knownWorkspaces, getVisuals: () => knownVisuals, selectWorkspace, selectVisual, focusComment: requestCommentFocus, stepRevision, followLatest });
