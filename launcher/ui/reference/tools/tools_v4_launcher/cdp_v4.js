// 无头 Edge + CDP 发真实鼠标 / 键盘事件，验证 v4 新增交互
const { spawn } = require('child_process');
const EDGE = 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe';
const SCR = 'C:/Users/ADMINI~1/AppData/Local/Temp/claude/60c801a49415fe01/b35a5742-7dbc-4fe8-aa52-26645903e112/scratchpad';
const URL0 = 'file:///' + SCR + '/mockups/v4-A-title/index.html';
const PORT = 9341;
const sleep = ms => new Promise(r => setTimeout(r, ms));
(async () => {
  const p = spawn(EDGE, ['--headless=new', `--user-data-dir=${SCR}/edge_profile_v4_cdp`, `--remote-debugging-port=${PORT}`, '--window-size=1200,720', '--hide-scrollbars', '--no-first-run', 'about:blank'], { stdio: 'ignore' });
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
  const VK = { Escape: 27, ArrowDown: 40, ArrowUp: 38, ArrowLeft: 37, ArrowRight: 39, Enter: 13, r: 82 };
  const key = async k => { await send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: k, code: k, windowsVirtualKeyCode: VK[k] }); await send('Input.dispatchKeyEvent', { type: 'keyUp', key: k, code: k, windowsVirtualKeyCode: VK[k] }); await sleep(120); };
  await send('Page.enable'); await send('Runtime.enable');
  await send('Emulation.setDeviceMetricsOverride', { width: 1200, height: 720, deviceScaleFactor: 1, mobile: false });
  const load = async s => { await send('Page.navigate', { url: URL0 + '?state=' + s }); await sleep(1500); };
  const st = `JSON.stringify({layer,sub,tab,sel:sel[tab],row:rows()[sel[tab]]&&rows()[sel[tab]].t,D:D&&D.msg,lords:S.lords.join(''),f:LS.f,dn:S.dn,lv:S.lv,C:C.st,IMP:IMP.st,U:U.st,VER})`;
  // T1 首页单击「深夜与联机」：默认选中强制深夜；单击「图鉴」进占位页，Esc 回来光标在图鉴
  await load('home'); await mouse('mouseMoved', 600, 445); await sleep(150); await click(600, 445); await sleep(800);
  console.log('T1 点深夜与联机', await ev(st));
  await load('home'); await mouse('mouseMoved', 600, 407); await sleep(150); await click(600, 407); await sleep(800);
  console.log('T1 点图鉴', await ev(st)); await key('Escape'); await sleep(500); console.log('T1 图鉴 Esc', await ev('JSON.stringify({layer,hs})'));
  // T2 关闭强制深夜后深夜层数不可调，状态行变化
  await load('night'); await key('Enter'); await key('ArrowDown'); await key('ArrowRight'); await key('ArrowLeft');
  console.log('T2 关强制深夜 + 调层数', await ev(st), await ev(`document.querySelector('#olist .row[data-i="1"]').className`), await ev(`status.textContent`));
  // T3 永夜王：Enter 进入，Enter 取消三狼，→ ↓ 移动，R 全选，R 全不选，Esc 回列表
  await load('night'); await key('ArrowDown'); await key('ArrowDown'); await key('Enter'); await sleep(300);
  console.log('T3 进入', await ev(st));
  await key('Enter'); console.log('T3 Enter 取消三狼', await ev(st));
  await key('ArrowRight'); await key('ArrowDown'); console.log('T3 → ↓', await ev(st));
  await key('r'); console.log('T3 R', await ev(st)); await key('r'); console.log('T3 R 再按', await ev(st));
  await key('Escape'); await sleep(300); console.log('T3 Esc', await ev(st), await ev(`list.children[2].textContent`));
  // T4 ← → 预设：先挑出部分组合，再在列表行上切换
  await load('night'); await ev(`ssel(2)`); await key('Enter'); await sleep(600); await key('Enter'); await key('ArrowRight'); await key('Enter'); await key('Escape'); await sleep(600);
  const seq = [await ev('S.lords.join("")')];
  for (const k of ['ArrowLeft', 'ArrowLeft', 'ArrowRight', 'ArrowRight', 'ArrowRight', 'ArrowLeft']) { await key(k); seq.push(k.slice(5) + ':' + (await ev('S.lords.join("")') || '无')); }
  console.log('T4 预设序列', seq.join(' '));
  // T5 鼠标：悬停移动焦点；单击格子切换；双击只算一次
  await load('lords'); await sleep(600);
  const c = JSON.parse(await ev(`(()=>{const r=lgrid.children[2].getBoundingClientRect();return JSON.stringify({x:r.x+56,y:r.y+56})})()`));
  await mouse('mouseMoved', c.x, c.y); await sleep(150); console.log('T5 悬停', await ev(st));
  await click(c.x, c.y); await sleep(200); console.log('T5 单击', await ev(st));
  await sleep(600); await click(c.x, c.y, 1); await sleep(90); await click(c.x, c.y, 2); await sleep(200); console.log('T5 双击', await ev(st));
  // T6 更新：检查 → 有新版本；导入 → 校验 → 可安装 → 安装 → 已是最新
  await load('update'); await key('ArrowDown'); await key('Enter'); await sleep(300); console.log('T6 检查中', await ev(st));
  await sleep(2600); console.log('T6 检查完', await ev(st));
  await key('ArrowDown'); await key('ArrowDown'); await key('Enter'); await sleep(600); console.log('T6 选 zip 框', await ev(st)); await key('Enter'); await sleep(400);
  console.log('T6 校验中', await ev(st)); await sleep(3000); console.log('T6 校验完', await ev(st), await ev(`status.textContent`));
  await key('Enter'); await sleep(300); console.log('T6 安装中', await ev(st)); await sleep(6000); console.log('T6 装完', await ev(st), await ev(`status.textContent`), await ev(`$('#bang').hidden`));
  // T7 状态行「深夜 5 层」直达深夜层数
  await load('home'); await sleep(800);
  const s = JSON.parse(await ev(`(()=>{const r=status.children[1].getBoundingClientRect();return JSON.stringify({x:r.x+r.width/2,y:r.y+r.height/2,t:status.children[1].textContent})})()`));
  await mouse('mouseMoved', s.x, s.y); await click(s.x, s.y); await sleep(700); console.log('T7 点', s.t, await ev(st));
  console.log('T8 页面异常', errs.length ? errs : '无');
  ws.close(); p.kill(); process.exit(0);
})();
