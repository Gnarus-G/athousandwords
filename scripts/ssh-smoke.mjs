// A real loopback SSH server, reverse port forward, and SSH-executed adapter.
// Uses only temporary test keys and connection files; never prints their values.
import { spawn, execFileSync } from 'node:child_process';
import { mkdtemp, mkdir, writeFile, readFile, copyFile, rm } from 'node:fs/promises';
import { tmpdir, userInfo } from 'node:os';
import { join, resolve } from 'node:path';
import net from 'node:net';
import assert from 'node:assert/strict';

const temporary = await mkdtemp(join(tmpdir(), 'atw-ssh-'));
const children = [];
const diagnostics = [];
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const binary = resolve('target/debug/atw');
const quote = value => `'${String(value).replaceAll("'", "'\\''")}'`;
async function port() {
  return new Promise(resolve => { const server = net.createServer(); server.listen(0, '127.0.0.1', () => { const p = server.address().port; server.close(() => resolve(p)); }); });
}
function launch(command, args) {
  const child = spawn(command, args, { stdio: ['ignore', 'ignore', 'pipe'] });
  let errors = ''; child.stderr.on('data', chunk => { errors += chunk; });
  child.on('error', error => { errors += error.message; });
  children.push(child); diagnostics.push(() => errors); return { child, errors: () => errors };
}
async function waitFor(check, service) {
  for (let attempt = 0; attempt < 100; attempt++) {
    if (await check()) return;
    if (service.child.exitCode !== null) throw new Error(service.errors());
    await sleep(50);
  }
  throw new Error(`Service did not become ready: ${service.errors()}`);
}
try {
  const companionPort = await port(), sshPort = await port(), forwardedPort = await port();
  const url = `http://127.0.0.1:${companionPort}`;
  const data = join(temporary, 'companion');
  const remote = join(temporary, 'remote-agent'); await mkdir(remote, { mode: 0o700 });
  const companion = launch(binary, ['--data-dir', data, 'serve', '--port', String(companionPort), '--no-open']);
  await waitFor(async () => { try { return (await fetch(`${url}/health`)).ok; } catch { return false; } }, companion);
  const localConnection = join(temporary, 'conversation.json');
  execFileSync(binary, ['--data-dir', data, '--url', url, 'workspace', 'create', '--title', 'SSH verification', '--out', localConnection], { stdio: ['ignore', 'ignore', 'pipe'] });
  const remoteConnection = join(remote, 'conversation.json'); await copyFile(localConnection, remoteConnection);
  const image = '<svg xmlns="http://www.w3.org/2000/svg" width="300" height="100"><text x="20" y="50">Delivered through SSH</text></svg>';
  const imagePath = join(remote, 'drawing.svg'); await writeFile(imagePath, image);
  const hostKey = join(temporary, 'host-key'), clientKey = join(temporary, 'client-key');
  execFileSync('ssh-keygen', ['-q', '-t', 'ed25519', '-N', '', '-f', hostKey]);
  execFileSync('ssh-keygen', ['-q', '-t', 'ed25519', '-N', '', '-f', clientKey]);
  const config = join(temporary, 'sshd.conf');
  // This disposable loopback server reads its own key under /tmp, whose parent
  // permissions deliberately do not meet production authorized_keys StrictModes.
  await writeFile(config, `Port ${sshPort}\nListenAddress 127.0.0.1\nHostKey ${hostKey}\nPidFile ${temporary}/sshd.pid\nAuthorizedKeysFile ${clientKey}.pub\nStrictModes no\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nLogLevel ERROR\nAllowUsers ${userInfo().username}\n`, { mode: 0o600 });
  const sshdPath = execFileSync('which', ['sshd'], { encoding: 'utf8' }).trim();
  const sshd = launch(sshdPath, ['-D', '-e', '-f', config]);
  await waitFor(() => new Promise(resolve => {
    const socket = net.connect(sshPort, '127.0.0.1'); socket.on('connect', () => { socket.destroy(); resolve(true); }); socket.on('error', () => resolve(false));
  }), sshd);
  const command = [binary, '--connection', remoteConnection, '--url', `http://127.0.0.1:${forwardedPort}`, 'show', '--title', 'From an SSH agent', '--format', 'svg', '--file', imagePath].map(quote).join(' ');
  const output = execFileSync('ssh', ['-i', clientKey, '-p', String(sshPort), '-o', 'BatchMode=yes', '-o', 'IdentitiesOnly=yes', '-o', 'StrictHostKeyChecking=no', '-o', 'UserKnownHostsFile=/dev/null', '-o', 'LogLevel=ERROR', '-o', 'ExitOnForwardFailure=yes', '-R', `127.0.0.1:${forwardedPort}:127.0.0.1:${companionPort}`, `${userInfo().username}@127.0.0.1`, command], { encoding: 'utf8', timeout: 15_000 });
  const shown = JSON.parse(output); assert.equal(shown.revision, 1);
  const connection = JSON.parse(await readFile(localConnection, 'utf8'));
  const response = await fetch(`${url}/api/visuals/${shown.id}/revisions/1`, { headers: { Authorization: `Bearer ${connection.token}` } });
  assert.equal(response.status, 200); assert.equal((await response.json()).content, image);
  console.log('PASS: SSH-executed CLI transferred SVG bytes through a real reverse SSH forward.');
  console.log('Topology: isolated temporary sshd on loopback, not a separate physical machine.');
} catch (error) {
  await sleep(50);
  for (const diagnostic of diagnostics) if (diagnostic()) console.error(diagnostic());
  throw error;
} finally {
  for (const child of children.reverse()) child.kill();
  await Promise.all(children.map(child => child.exitCode !== null ? Promise.resolve() : new Promise(resolve => child.once('exit', resolve))));
  await rm(temporary, { recursive: true, force: true });
}
