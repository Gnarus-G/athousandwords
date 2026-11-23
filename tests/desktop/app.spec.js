import { test, expect } from '@playwright/test';
import { _electron } from 'playwright';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn } from 'node:child_process';
import net from 'node:net';
import dgram from 'node:dgram';

test('the dedicated desktop window renders a diagram without exposing Node to content', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'atw-desktop-'));
  const port = await new Promise(resolve => { const s=net.createServer(); s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>resolve(p));}); });
  const url=`http://127.0.0.1:${port}`;
  const companion=spawn(resolve('target/debug/atw'),['--data-dir',directory,'serve','--port',String(port),'--no-open'],{stdio:'ignore'});
  let desktop;
  try {
    await expect.poll(async()=>{try{return (await fetch(`${url}/health`)).ok;}catch{return false;}}).toBe(true);
    const admin=(await readFile(join(directory,'admin.token'),'utf8')).trim();
    const w=await (await fetch(`${url}/api/workspaces`,{method:'POST',headers:{Authorization:`Bearer ${admin}`,'Content-Type':'application/json'},body:JSON.stringify({title:'Desktop verification'})})).json();
    const shown=await (await fetch(`${url}/api/visuals`,{method:'POST',headers:{Authorization:`Bearer ${w.token}`,'Content-Type':'application/json'},body:JSON.stringify({title:'A real app window',format:'mermaid',content:'graph LR; Agent-->Companion; Companion-->Person'})})).json();
    const env={...process.env}; delete env.ELECTRON_RUN_AS_NODE;
    desktop=await _electron.launch({args:[resolve('desktop/main.cjs'),'--data-dir',directory,'--url',url,'--workspace',w.workspace],env});
    const page=await desktop.firstWindow();
    // CDP's page.keyboard bypasses Electron's native before-input-event hook.
    const nativeKey=(keyCode,modifiers=[])=>desktop.evaluate(({BrowserWindow},input)=>{
      const contents=BrowserWindow.getAllWindows()[0].webContents;
      contents.sendInputEvent({type:'keyDown',...input}); contents.sendInputEvent({type:'keyUp',...input});
    },{keyCode,modifiers});
    await expect(page.getByRole('heading',{name:'A real app window',exact:true})).toBeVisible();
    await expect(page.frameLocator('#visual-frame').locator('svg')).toBeVisible();
    expect(await page.evaluate(()=>typeof process)).toBe('undefined');
    expect(await desktop.evaluate(({app})=>app.getName())).toBe('A Thousand Words');
    expect(await desktop.evaluate(({BrowserWindow})=>BrowserWindow.getAllWindows().length)).toBe(1);
    await expect.poll(async()=>{const response=await fetch(`${url}/api/status/${shown.id}/1`,{headers:{Authorization:`Bearer ${w.token}`}});return (await response.json()).render_status;}).toBe('rendered');
    await page.screenshot({path:'test-results/desktop-companion.png'});
    const socket=dgram.createSocket('udp4'); let datagrams=0;
    socket.on('message',()=>{datagrams++;});
    await new Promise(resolve=>socket.bind(0,'127.0.0.1',resolve));
    try {
      await fetch(`${url}/api/visuals`,{method:'POST',headers:{Authorization:`Bearer ${w.token}`,'Content-Type':'application/json'},body:JSON.stringify({title:'Desktop isolation check',format:'html',content:`<p id="probe">Starting</p><button onclick="this.textContent='Selected'">Choose</button><script>(async()=>{try{const pc=new RTCPeerConnection({iceServers:[{urls:'stun:127.0.0.1:${socket.address().port}'}]});pc.createDataChannel('probe');await pc.setLocalDescription(await pc.createOffer());document.getElementById('probe').textContent='Probe attempted';}catch{document.getElementById('probe').textContent='Probe blocked';}})();</script>`})});
      await expect(page.frameLocator('#visual-frame').locator('#probe')).toHaveText(/Probe (attempted|blocked)/);
      expect(await page.frameLocator('#visual-frame').locator('body').evaluate(()=>typeof require)).toBe('undefined');
      await page.frameLocator('#visual-frame').getByRole('button',{name:'Choose'}).click();
      await expect(page.frameLocator('#visual-frame').getByRole('button',{name:'Selected'})).toBeVisible();
      await page.waitForTimeout(500);
      expect(datagrams).toBe(0);
    } finally {socket.close();}
    await nativeKey('K',['control']);
    await expect(page.getByRole('combobox',{name:'Find a command or visual'})).toBeFocused();
    await nativeKey('Escape');
    await expect(page.getByRole('region',{name:'Visual surface'})).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(page.locator('#visual-frame')).toBeFocused();
    await nativeKey('F6');
    await expect(page.getByLabel('Comment')).toBeFocused();
    await nativeKey('Escape');
    await page.route('**/api/viewer',async route=>{
      if(route.request().method()==='DELETE')await new Promise(resolve=>setTimeout(resolve,250));
      await route.continue().catch(()=>{});
    });
    const exited=new Promise(resolve=>desktop.process().once('exit',resolve));
    await nativeKey('W',['control']); await exited; desktop=null;
    const afterClose=await(await fetch(`${url}/api/status/${shown.id}/1`,{headers:{Authorization:`Bearer ${w.token}`}})).json();
    expect(afterClose.viewer_connected).toBe(false);
  } finally {
    await desktop?.close(); companion.kill();
    await rm(directory,{recursive:true,force:true});
  }
});
