function installKeyboard({ getWorkspaces, getVisuals, selectWorkspace, selectVisual, focusComment, stepRevision, followLatest }) {
  const byId = id => document.getElementById(id);
  const palette = byId('command-palette');
  const help = byId('shortcut-help');
  const query = byId('palette-query');
  let matches = [], active = 0, returnFocus = null;

  const focusCanvas = () => byId('canvas').focus();
  function rememberFocus() {
    returnFocus = document.activeElement?.tagName === 'IFRAME' ? byId('canvas') : document.activeElement;
  }
  function closeModal(dialog, action) {
    dialog.close();
    if (action) action();
    else if (returnFocus?.isConnected) returnFocus.focus();
  }
  for (const dialog of [palette, help]) dialog.addEventListener('cancel', event => { event.preventDefault(); closeModal(dialog); });

  function showHelp() {
    if (help.open) return;
    rememberFocus(); help.showModal(); byId('close-help').focus();
  }
  function entries() {
    const commands = [
      { label: 'Comment on this revision', hint: 'c', run: focusComment },
      { label: 'Follow latest revision', hint: 'g', run: followLatest },
      { label: 'Focus visual canvas', hint: 'Alt 3', run: focusCanvas },
      { label: 'Keyboard shortcuts', hint: '?', run: showHelp },
    ].map(item => ({ ...item, key: `action:${item.label}`, kind: 'Action' }));
    if (!byId('comment-form').hidden) commands.push({ key: 'action:delete-visual', label: 'Delete this visual', hint: 'Confirms first', kind: 'Action', run: () => byId('delete-visual').click() });
    if (!byId('delete-workspace').hidden) commands.push({ key: 'action:delete-workspace', label: 'Delete this workspace', hint: 'Confirms first', kind: 'Action', run: () => byId('delete-workspace').click() });
    return [
      ...getVisuals().map(v => ({ key: `visual:${v.id}`, label: v.title, kind: 'Visual', hint: `Revision ${v.revision}`, run: () => { selectVisual(v.id); focusCanvas(); } })),
      ...getWorkspaces().map(w => ({ key: `workspace:${w.id}`, label: w.title, kind: 'Conversation', hint: w.id.slice(0, 8), run: () => { selectWorkspace(w.id); focusCanvas(); } })),
      ...commands,
    ];
  }
  function choose(index) {
    const entry = matches[index];
    if (!entry) return;
    closeModal(palette, entry.run);
  }
  function highlight() {
    [...byId('palette-results').children].forEach((element, index) => element.setAttribute('aria-selected', String(index === active)));
    const selected = byId(`palette-option-${active}`);
    if (selected) { query.setAttribute('aria-activedescendant', selected.id); selected.scrollIntoView({ block: 'nearest' }); }
    else query.removeAttribute('aria-activedescendant');
  }
  function search(keepSelection = false) {
    const selected = keepSelection ? matches[active]?.key : null;
    const terms = query.value.toLowerCase().trim().split(/\s+/).filter(Boolean);
    matches = entries().filter(item => terms.every(term => `${item.label} ${item.kind}`.toLowerCase().includes(term))).slice(0, 50);
    active = Math.max(0, matches.findIndex(item => item.key === selected));
    byId('palette-results').replaceChildren(...matches.map((item, index) => {
      const element = document.createElement('li'); element.id = `palette-option-${index}`; element.setAttribute('role', 'option');
      const label = document.createElement('span'); label.textContent = item.label;
      const detail = document.createElement('small'); detail.textContent = `${item.kind} / ${item.hint}`;
      element.append(label, detail); element.onclick = () => choose(index); return element;
    }));
    byId('palette-empty').hidden = matches.length > 0;
    highlight();
  }
  function showSwitcher() {
    if (palette.open) { closeModal(palette); return; }
    if (help.open) closeModal(help);
    rememberFocus(); query.value = ''; search(); palette.showModal();
    query.setAttribute('aria-expanded', 'true'); query.focus();
  }
  palette.addEventListener('close', () => query.setAttribute('aria-expanded', 'false'));
  query.addEventListener('input', () => search());
  query.addEventListener('keydown', event => {
    if (['ArrowDown', 'ArrowUp'].includes(event.key)) {
      event.preventDefault(); if (!matches.length) return;
      active = (active + (event.key === 'ArrowDown' ? 1 : -1) + matches.length) % matches.length; highlight();
    } else if (event.key === 'Enter') { event.preventDefault(); choose(active); }
  });

  function paneTargets() {
    return [
      byId('workspaces').querySelector('[aria-current="true"]') || byId('workspaces').querySelector('button'),
      byId('visuals').querySelector('[aria-current="true"]') || byId('visuals').querySelector('button'),
      byId('canvas'),
      byId('comment-form').hidden ? null : byId('comment'),
    ];
  }
  function cyclePane(direction) {
    const targets = paneTargets().filter(Boolean);
    const current = document.activeElement;
    const containers = [byId('workspaces'), byId('visuals'), byId('canvas'), byId('comment-form')];
    const index = targets.findIndex(target => target === current || containers.find(container => container.contains(target))?.contains(current));
    const next = index < 0 ? (direction > 0 ? 0 : targets.length - 1) : (index + direction + targets.length) % targets.length;
    if (targets[next] === byId('comment')) focusComment(); else targets[next]?.focus();
  }
  function command(name) {
    if (name === 'switcher') { showSwitcher(); return; }
    if (name === 'escape') {
      if (palette.open) closeModal(palette); else if (help.open) closeModal(help); else focusCanvas();
      return;
    }
    if (palette.open || help.open) return;
    if (name === 'next-pane') cyclePane(1);
    else if (name === 'previous-pane') cyclePane(-1);
    else if (name === 'pane-4') focusComment();
    else if (name.startsWith('pane-')) paneTargets()[Number(name.slice(5)) - 1]?.focus();
  }
  // Electron routes these reserved app shortcuts here even when a sandboxed
  // interactive rendering owns keyboard focus. No privileged APIs are exposed.
  window.atwKeyboardCommand = command;
  byId('quick-switcher').onclick = showSwitcher;
  byId('keyboard-help').onclick = showHelp;
  byId('close-help').onclick = () => closeModal(help);

  document.addEventListener('keydown', event => {
    if (event.defaultPrevented || (event.repeat && !['j', 'k', 'ArrowUp', 'ArrowDown', '[', ']'].includes(event.key))) return;
    const modified = event.ctrlKey || event.metaKey;
    if (modified && event.key.toLowerCase() === 'k') { event.preventDefault(); command('switcher'); return; }
    if (event.key === 'Escape') { event.preventDefault(); command('escape'); return; }
    if (palette.open || help.open) return;
    if (event.key === 'F6') { event.preventDefault(); command(event.shiftKey ? 'previous-pane' : 'next-pane'); return; }
    if (event.altKey && /^[1-4]$/.test(event.key)) { event.preventDefault(); command(`pane-${event.key}`); return; }
    if (modified && event.key === 'Enter' && document.activeElement === byId('comment')) {
      event.preventDefault(); const submit = byId('comment-form').querySelector('button[type="submit"]');
      if (!submit.disabled) byId('comment-form').requestSubmit(submit); return;
    }
    const editing = event.target.closest?.('input, textarea, select, [contenteditable="true"]');
    if (editing || modified || event.altKey) return;
    const nav = document.activeElement?.closest('nav');
    if (nav && ['j', 'k', 'ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
      event.preventDefault(); const buttons = [...nav.querySelectorAll('button')]; const current = buttons.indexOf(document.activeElement);
      const next = event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1 : (current + (['j', 'ArrowDown'].includes(event.key) ? 1 : -1) + buttons.length) % buttons.length;
      buttons[next]?.focus(); return;
    }
    if (event.key === '?' ) { event.preventDefault(); showHelp(); }
    else if (event.key === '/') { event.preventDefault(); showSwitcher(); }
    else if (event.key === 'c') { event.preventDefault(); focusComment(); }
    else if (event.key === '[' || event.key === ']') { event.preventDefault(); stepRevision(event.key === '[' ? -1 : 1); }
    else if (event.key === 'g') { event.preventDefault(); followLatest(); }
    else if (event.key === 'Enter' && document.activeElement === byId('canvas')) { event.preventDefault(); byId('visual-frame')?.focus(); }
  });
  return { refresh: () => { if (palette.open) search(true); } };
}
