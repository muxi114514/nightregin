// 无头 Edge + CDP 发真实鼠标 / 键盘事件，验证 v4 新增交互
const { spawn } = require('child_process');
const EDGE = 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe';
const SCR = 'C:/Users/ADMINI~1/AppData/Local/Temp/claude/60c801a49415fe01/b35a5742-7dbc-4fe8-aa52-26645903e112/scratchpad';
const URL0 = 'file:///' + SCR + '/mockups/v4-A-title/index.html';
const PORT = 9352;
const sleep = ms => new Promise(r => setTimeout(r, ms));
(async () => {
  const p = spawn(EDGE, ['--headless=new', `--user-data-dir=${SCR}/edge_profile_v4_cdpwiki`, `--remote-debugging-port=${PORT}`, '--window-size=1200,720', '--hide-scrollbars', '--no-first-run', 'about:blank'], { stdio: 'ignore' });
  let targets = [];
  for (let i = 0; i < 60; i++) { try { targets = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json(); if (targets.find(t => t.type === 'page')) break; } catch (e) {} await sleep(250); }
  const page = targets.find(t => t.type === 'page');
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise(r => ws.onopen = r);
  let id = 0; const pend = new Map(); const errs = [];
  ws.onmessage = e => { const m = JSON.parse(e.data); if (m.id && pend.has(m.id)) { pend.get(m.id)(m); pend.delete(m.id); }
    if (m.method === 'Runtime.exceptionThrown') errs.push(m.params.exceptionDetails.exception && m.params.exceptionDetails.exception.description || m.params.exceptionDetails.text); };
  const send = (method, params = {}) => new Promise(r => { const i = ++id; pend.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
  const ev = async expr => { const r = await send('Runtime.evaluate', { expression: expr, returnByValue: true, replMode: true }); return r.result && r.result.result ? r.result.result.value : JSON.stringify(r); };
  const mouse = (type, x, y, clickCount = 1) => send('Input.dispatchMouseEvent', { type, x, y, button: type === 'mouseMoved' ? 'none' : 'left', clickCount, buttons: type === 'mousePressed' ? 1 : 0 });
  const click = async (x, y, n = 1) => { await mouse('mousePressed', x, y, n); await mouse('mouseReleased', x, y, n); };
  const VK = { Escape: 27, ArrowDown: 40, ArrowUp: 38, ArrowLeft: 37, ArrowRight: 39, Enter: 13, r: 82, f: 70, e: 69, q: 81, Backspace: 8 };
  const key = async k => { await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: k, code: k, windowsVirtualKeyCode: VK[k] }); await send('Input.dispatchKeyEvent', { type: 'keyUp', key: k, code: k, windowsVirtualKeyCode: VK[k] }); await sleep(140); };
  const type = async t => { await send('Input.insertText', { text: t }); await sleep(150); };
  await send('Page.enable'); await send('Runtime.enable');
  await send('Emulation.setDeviceMetricsOverride', { width: 1200, height: 720, deviceScaleFactor: 1, mobile: false });
  const load = async s => { await send('Page.navigate', { url: URL0 + '?state=' + s }); await sleep(1500); };
  const ws_ = `JSON.stringify({layer,sub,c:CATS[WS.c].key,z:WS.z,q:WS.q,edit:WS.edit,cur:WL.length?wcur().id:null,n:WL.length,from:WS.from,lf:LS.f,lfrom:LS.from,tab,row:rows()[sel[tab]]&&rows()[sel[tab]].t,lords:S.lords.join('')})`;
  // W1 首页进图鉴：方向键在网格里移动，↑ 到查找行，Q / E 切分类，Esc 回首页
  await load('home'); await key('ArrowDown'); await key('Enter'); await sleep(500); console.log('W1 进图鉴', await ev(ws_));
  await key('ArrowRight'); await key('ArrowDown'); console.log('W1 → ↓', await ev(ws_));
  await key('ArrowDown'); console.log('W1 ↓ 到底', await ev(ws_));
  await key('ArrowUp'); await key('ArrowUp'); console.log('W1 ↑↑ 到查找行', await ev(ws_));
  await key('ArrowUp'); await key('ArrowRight'); console.log('W1 显示 →', await ev(ws_), await ev(`wrows.children[0].textContent`));
  await key('e'); console.log('W1 E', await ev(ws_)); await key('q'); await key('q'); console.log('W1 QQ', await ev(ws_));
  await key('Escape'); await sleep(500); console.log('W1 Esc', await ev('JSON.stringify({layer,hs})'));
  // W2 查找：Enter 开始输入，打字过滤，Enter 完成并落到结果，Backspace 清空
  await load('wiki-weapons'); await key('ArrowUp'); await key('Enter'); await type('大剑'); console.log('W2 输入', await ev(ws_), await ev(`document.activeElement.id`));
  await key('q'); console.log('W2 打字时 q 不切分类', await ev(ws_));
  await key('Enter'); console.log('W2 Enter 完成', await ev(ws_));
  await key('ArrowUp'); await key('Backspace'); console.log('W2 清空', await ev(ws_));
  await key('Enter'); await type('卡莉果'); console.log('W2 无结果', await ev(ws_), await ev(`[...wtabrow.children].map(b=>b.className).join('|')`), await ev(`wbox.textContent`));
  await key('Escape'); console.log('W2 Esc 取消', await ev(ws_));
  // W3 敌人 ⇄ 永夜王：Enter 去永夜王选择，Esc 回图鉴；永夜王选择里 F 回图鉴
  await load('wiki-enemies'); await key('ArrowRight'); await key('Enter'); await sleep(500); console.log('W3 Enter 去永夜王', await ev(ws_));
  await key('Enter'); console.log('W3 切换永夜', await ev(ws_)); await key('Escape'); await sleep(500); console.log('W3 Esc 回图鉴', await ev(ws_), await ev(`wdesc.querySelector('.ever').textContent`));
  await key('Escape'); await sleep(500); console.log('W3 再 Esc 回首页', await ev('JSON.stringify({layer,hs})'));
  await load('lords'); await key('ArrowRight'); await key('f'); await sleep(500); console.log('W3 永夜王 F 去图鉴', await ev(ws_));
  await key('ArrowRight'); await key('Enter'); await sleep(500); console.log('W3 图鉴 Enter 回永夜王', await ev(ws_));
  await key('Escape'); await sleep(500); console.log('W3 永夜王 Esc', await ev(ws_));
  // W4 更新页内容行 Enter 跳图鉴，Esc 回更新页同一行
  await load('update'); for (let i = 0; i < 4; i++) await key('ArrowDown'); console.log('W4 选中', await ev(ws_), await ev(`$('#skeys').textContent`));
  await key('Enter'); await sleep(500); console.log('W4 Enter', await ev(ws_)); await key('Escape'); await sleep(500); console.log('W4 Esc', await ev(ws_));
  // W5 鼠标：悬停选中格子，单击查找行开始输入，点格子结束输入
  await load('wiki-talismans'); await sleep(400);
  const cc = JSON.parse(await ev(`(()=>{const r=wbox.querySelectorAll('.wcell')[3].getBoundingClientRect();return JSON.stringify({x:r.x+40,y:r.y+40})})()`));
  await mouse('mouseMoved', cc.x, cc.y); await sleep(150); console.log('W5 悬停', await ev(ws_));
  await mouse('mouseMoved', 300, 244); await sleep(150); await click(300, 244); await sleep(200); console.log('W5 单击查找行', await ev(ws_));
  await type('徽章'); await click(cc.x - 300, cc.y); await sleep(200); console.log('W5 点别处结束输入', await ev(ws_));
  console.log('ERRORS', errs.length ? errs.join('\n') : 'none');
  ws.close(); p.kill(); process.exit(0);
})();
