const { app, BrowserWindow, Menu, dialog, session, nativeTheme } = require('electron');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');

const args = process.argv.slice(2);
const option = (name, fallback) => { const index = args.indexOf(name); return index < 0 ? fallback : args[index + 1]; };
const data = path.resolve(option('--data-dir', path.join(process.env.XDG_DATA_HOME || path.join(os.homedir(), '.local/share'), 'athousandwords')));
const endpoint = new URL(option('--url', 'http://127.0.0.1:7331'));
if (endpoint.protocol !== 'http:' || !['127.0.0.1', 'localhost', '[::1]'].includes(endpoint.hostname) || endpoint.username || endpoint.password || endpoint.pathname !== '/' || endpoint.search || endpoint.hash) {
  throw new Error('The desktop app requires an HTTP loopback companion URL.');
}
const profile = path.join(data, 'desktop');
fs.mkdirSync(profile, { recursive: true, mode: 0o700 });
app.setName('A Thousand Words');
app.setPath('userData', profile);
app.setDesktopName('athousandwords.desktop');
app.commandLine.appendSwitch('class', 'athousandwords');
let window;

function createWindow() {
  const token = fs.readFileSync(path.join(data, 'admin.token'), 'utf8').trim();
  const target = new URL(endpoint);
  target.searchParams.set('desktop', '1');
  const workspace = option('--workspace');
  if (workspace) target.searchParams.set('workspace', workspace);
  target.hash = new URLSearchParams({ token }).toString();
  window = new BrowserWindow({
    title: 'A Thousand Words', width: 1280, height: 900, minWidth: 800, minHeight: 600,
    backgroundColor: '#181923', show: false, autoHideMenuBar: true,
    webPreferences: { nodeIntegration: false, nodeIntegrationInSubFrames: false, contextIsolation: true, sandbox: true, webviewTag: false, backgroundThrottling: false },
  });
  window.setMenu(null);
  window.once('ready-to-show', () => args.includes('--background') ? window.showInactive() : window.show());
  window.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
  window.webContents.on('before-input-event', (event, input) => {
    if (input.type !== 'keyDown' || input.isAutoRepeat) return;
    const modified = input.control || input.meta;
    const key = input.key.toLowerCase();
    if (modified && ['w', 'q'].includes(key)) { event.preventDefault(); window.close(); return; }
    let command;
    if (modified && key === 'k') command = 'switcher';
    else if (input.key === 'Escape') command = 'escape';
    else if (input.key === 'F6') command = input.shift ? 'previous-pane' : 'next-pane';
    else if (input.alt && /^[1-4]$/.test(input.key)) command = `pane-${input.key}`;
    if (command) {
      event.preventDefault();
      window.webContents.executeJavaScript(`window.atwKeyboardCommand?.(${JSON.stringify(command)})`).catch(() => {});
    }
  });
  window.webContents.on('will-navigate', (event, url) => {
    const next = new URL(url);
    if (next.origin !== endpoint.origin || next.pathname !== '/') event.preventDefault();
  });
  let closing = false;
  window.on('close', event => {
    event.preventDefault();
    if (closing) return;
    closing = true;
    const target = window;
    // Complete normal deregistration before Chromium exits. Otherwise an
    // immediately following publication could mistake the old lease for an app.
    Promise.race([
      target.webContents.executeJavaScript('window.atwDisconnectViewer?.()').catch(() => {}),
      new Promise(resolve => setTimeout(resolve, 1000)),
    ]).finally(() => { if (!target.isDestroyed()) target.destroy(); });
  });
  window.on('closed', () => { window = null; });
  // The owner credential stays in trusted main-process memory and a URL fragment;
  // it is neither a command-line argument nor exposed through a preload bridge.
  window.loadURL(target.toString()).catch(() => {
    dialog.showErrorBox('A Thousand Words', 'Could not connect to the local companion. Check that its service is running.');
    app.quit();
  });
}

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on('second-instance', (_event, commandLine) => {
    if (!window) { if (app.isReady()) createWindow(); return; }
    if (!commandLine.includes('--background')) { if (window.isMinimized()) window.restore(); window.show(); window.focus(); }
  });
  app.whenReady().then(() => {
    nativeTheme.themeSource = 'dark';
    Menu.setApplicationMenu(null);
    session.defaultSession.setPermissionRequestHandler((_contents, _permission, callback) => callback(false));
    session.defaultSession.setPermissionCheckHandler(() => false);
    session.defaultSession.on('will-download', event => event.preventDefault());
    session.defaultSession.webRequest.onBeforeRequest({ urls: ['<all_urls>'] }, (details, callback) => {
      const url = new URL(details.url);
      callback({ cancel: !['data:', 'blob:', 'devtools:'].includes(url.protocol) && url.origin !== endpoint.origin });
    });
    createWindow();
  }).catch(() => {
    dialog.showErrorBox('A Thousand Words', 'Could not initialize the desktop app. Start the companion first.');
    app.quit();
  });
}
app.on('window-all-closed', () => app.quit());
