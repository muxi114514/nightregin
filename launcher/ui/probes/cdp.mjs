// 探测用的最小 CDP 客户端：起无头 Edge（临时用户目录，端口读 DevToolsActivePort），开页，发真实键鼠，执行脚本，截图。
// 进程只按自己起的 pid 结束整棵树，绝不按映像名杀
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

const EDGE_PATHS = [
  process.env.NRGA_EDGE,
  'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
  'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
].filter(Boolean);
const DESIGN = { width: 1200, height: 720 };
const CALL_TIMEOUT_MS = 10000;
const CLEANUP_TIMEOUT_MS = 3000;
const VK = {
  Enter: 13, Escape: 27, Tab: 9, ' ': 32, ArrowLeft: 37, ArrowUp: 38, ArrowRight: 39, ArrowDown: 40,
  F3: 114, F5: 116, F7: 118, F12: 123,
};
const BUTTONS = { none: 0, left: 1, right: 2, middle: 4, back: 8, forward: 16 };

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** 按 pid 结束整棵进程树；已经退出时忽略 */
export function killTree(pid) {
  if (!pid) return;
  try {
    execFileSync('taskkill', ['/PID', String(pid), '/T', '/F'], { stdio: 'ignore', windowsHide: true });
  } catch {
    // 进程已不在
  }
}

/** 命令行里带着本次临时目录的进程 pid（只可能是这次起的 Edge） */
export function pidsUsing(profile) {
  const ps = `Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" | ` +
    `Where-Object { $_.CommandLine -and $_.CommandLine.Contains('${profile.replace(/'/g, "''")}') } | ` +
    `ForEach-Object { $_.ProcessId }`;
  try {
    const out = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', ps],
      { encoding: 'utf8', windowsHide: true, stdio: ['ignore', 'pipe', 'ignore'] });
    return out.split(/\s+/).filter(Boolean).map(Number);
  } catch {
    return [];
  }
}

/** 起过还没收尾的浏览器：spawn 的同一步就登记，连接阶段被打断也收得到 */
const live = new Set();

/** 同步收尾全部浏览器，返回每个的 { profile, swept, removed } */
export function closeAllBrowsers() {
  return [...live].map((b) => ({ profile: b.profile, ...b.close() }));
}

/** 给收尾用的等待加上限，卡住的端点不能拖住同步收尾 */
function within(promise, ms, what) {
  let timer;
  return Promise.race([
    promise,
    new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`${what} 超时`)), ms); }),
  ]).finally(() => clearTimeout(timer));
}

export async function launchEdge() {
  const exe = EDGE_PATHS.find((p) => existsSync(p));
  if (!exe) throw new Error('找不到 msedge.exe（可用环境变量 NRGA_EDGE 指定）');
  const profile = mkdtempSync(join(tmpdir(), 'nrga-probe-'));
  const proc = spawn(exe, [
    '--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`,
    '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--disable-sync',
    '--disable-background-networking', '--disable-component-update', '--mute-audio', '--hide-scrollbars',
    // 不把标签页登记到 Windows 的 WindowTabManager，否则任务视图 / Alt+Tab 里会留下一排探测页的残影
    '--disable-features=msWindowTabManagerPublic',
    `--window-size=${DESIGN.width},${DESIGN.height}`, 'about:blank',
  ], { stdio: 'ignore', windowsHide: true });
  const browser = new Browser(proc, profile);
  try {
    await browser.connect();
  } catch (e) {
    browser.close();
    throw e;
  }
  return browser;
}

class Browser {
  constructor(proc, profile) {
    this.proc = proc;
    this.profile = profile;
    this.port = 0;
    this.closed = false;
    this.exited = false;
    this.spawnError = null;
    proc.on('exit', () => { this.exited = true; });
    proc.on('error', (e) => { this.spawnError = e; });
    live.add(this);
  }

  /** 端口写在临时用户目录的 DevToolsActivePort 第一行 */
  async connect() {
    const file = join(this.profile, 'DevToolsActivePort');
    for (let i = 0; i < 200; i++) {
      if (this.spawnError) throw new Error(`Edge 起不来：${this.spawnError.message}`);
      if (this.exited) throw new Error('Edge 启动后立即退出');
      if (existsSync(file)) {
        const port = Number.parseInt(readFileSync(file, 'utf8').split(/\r?\n/)[0], 10);
        if (port > 0) {
          this.port = port;
          return;
        }
      }
      await sleep(100);
    }
    throw new Error('20 秒内没等到 DevToolsActivePort');
  }

  async newPage() {
    const res = await fetch(`http://127.0.0.1:${this.port}/json/new?about:blank`, { method: 'PUT' });
    if (!res.ok) throw new Error(`新建标签页失败：HTTP ${res.status}`);
    const page = new Page(await res.json(), this.port);
    await page.attach();
    return page;
  }

  /** 先让 Edge 自己退出（释放用户目录里的文件），最多等 5 秒 */
  async quit() {
    if (this.closed || this.exited || !this.port) return;
    let ws = null;
    try {
      const res = await fetch(`http://127.0.0.1:${this.port}/json/version`, { signal: AbortSignal.timeout(CLEANUP_TIMEOUT_MS) });
      const { webSocketDebuggerUrl } = await within(res.json(), CLEANUP_TIMEOUT_MS, '读 /json/version');
      ws = new WebSocket(webSocketDebuggerUrl);
      await within(new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; }), CLEANUP_TIMEOUT_MS, '连接浏览器');
      ws.send(JSON.stringify({ id: 1, method: 'Browser.close' }));
    } catch {
      ws?.close();
      // 连不上就交给 close() 结束进程树
    }
    for (let i = 0; i < 50 && !this.exited; i++) await sleep(100);
  }

  /**
   * 同步收尾，可在 exit / 信号处理里调用：结束进程树，等用到临时目录的进程都退出，再删目录。
   * 返回 { swept: 补结束的 pid, removed: 临时目录是否已删掉 }
   */
  close() {
    if (this.closed) return { swept: [], removed: true };
    this.closed = true;
    live.delete(this);
    // 已退出的进程 pid 可能被系统复用，不能再杀；残留子进程由下面按临时目录补扫
    if (!this.exited) killTree(this.proc.pid);
    const swept = new Set();
    // TerminateProcess 是异步的，子进程可能还没退干净
    for (let i = 0; i < 20; i++) {
      const left = pidsUsing(this.profile);
      if (!left.length) break;
      left.forEach((pid) => { swept.add(pid); killTree(pid); });
      sleepSync(250);
    }
    return { swept: [...swept], removed: removeDir(this.profile) };
  }
}

function sleepSync(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

/** 进程刚退出时文件句柄可能还没释放，删不掉就等一会儿再删 */
function removeDir(dir) {
  for (let i = 0; i < 40; i++) {
    try {
      rmSync(dir, { recursive: true, force: true });
    } catch {
      // 句柄还没放
    }
    if (!existsSync(dir)) return true;
    sleepSync(250);
  }
  return false;
}

class Page {
  constructor(target, port) {
    this.target = target;
    this.port = port;
    this.seq = 0;
    this.pending = new Map();
    /** 页面里未捕获的异常 */
    this.errors = [];
  }

  async attach() {
    this.ws = new WebSocket(this.target.webSocketDebuggerUrl);
    await new Promise((resolve, reject) => {
      this.ws.onopen = resolve;
      this.ws.onerror = () => reject(new Error('CDP 连接失败'));
    });
    this.ws.onmessage = (ev) => this.onMessage(JSON.parse(ev.data));
    this.ws.onclose = () => {
      for (const { reject } of this.pending.values()) reject(new Error('CDP 连接已断开'));
      this.pending.clear();
    };
    await this.send('Page.enable');
    await this.send('Runtime.enable');
    await this.send('Emulation.setDeviceMetricsOverride', { ...DESIGN, deviceScaleFactor: 1, mobile: false });
    await this.send('Emulation.setFocusEmulationEnabled', { enabled: true });
    await this.send('Page.bringToFront');
  }

  onMessage(m) {
    if (m.id && this.pending.has(m.id)) {
      const { resolve, reject } = this.pending.get(m.id);
      this.pending.delete(m.id);
      if (m.error) reject(new Error(`${m.error.message}（${m.error.code}）`));
      else resolve(m.result);
    } else if (m.method === 'Runtime.exceptionThrown') {
      const d = m.params.exceptionDetails;
      this.errors.push(d.exception?.description || d.text);
    }
  }

  send(method, params = {}) {
    const id = ++this.seq;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`${method} 超时`));
      }, CALL_TIMEOUT_MS);
      this.pending.set(id, {
        resolve: (v) => { clearTimeout(timer); resolve(v); },
        reject: (e) => { clearTimeout(timer); reject(e); },
      });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  /** 打开页面并等到 window.__ready */
  async open(url) {
    await this.send('Page.navigate', { url });
    await this.waitFor('document.readyState === "complete" && window.__ready === true', 8000, `页面没有就绪：${url}`);
  }

  async eval(expression) {
    const r = await this.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) {
      throw new Error(`页面脚本出错：${r.exceptionDetails.exception?.description || r.exceptionDetails.text}`);
    }
    return r.result.value;
  }

  async waitFor(expression, timeoutMs = 3000, message = `等待超时：${expression}`) {
    const end = Date.now() + timeoutMs;
    for (;;) {
      try {
        if (await this.eval(expression)) return;
      } catch {
        // 页面还在加载
      }
      if (Date.now() > end) throw new Error(message);
      await sleep(50);
    }
  }

  /** 按一次键；o: { ctrl, shift, alt, repeat } */
  async key(k, o = {}) {
    let key = k;
    let code = k;
    let vk = VK[k];
    if (k.length === 1 && k !== ' ') {
      const up = k.toUpperCase();
      vk = up.charCodeAt(0);
      code = /[A-Z]/.test(up) ? `Key${up}` : /[0-9]/.test(k) ? `Digit${k}` : k;
      key = o.shift ? up : k.toLowerCase();
    }
    const base = {
      key, code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk,
      modifiers: (o.alt ? 1 : 0) | (o.ctrl ? 2 : 0) | (o.shift ? 8 : 0),
    };
    await this.send('Input.dispatchKeyEvent', { ...base, type: 'rawKeyDown', autoRepeat: !!o.repeat });
    await this.send('Input.dispatchKeyEvent', { ...base, type: 'keyUp' });
  }

  mouseMove(x, y) {
    return this.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0 });
  }

  /** 按下再松开；button 可为 left / right / middle / back / forward */
  async click(x, y, { button = 'left' } = {}) {
    const ev = { x, y, button, clickCount: 1 };
    await this.send('Input.dispatchMouseEvent', { ...ev, type: 'mousePressed', buttons: BUTTONS[button] });
    await this.send('Input.dispatchMouseEvent', { ...ev, type: 'mouseReleased', buttons: 0 });
  }

  wheel(x, y, deltaY, { ctrl = false } = {}) {
    return this.send('Input.dispatchMouseEvent', {
      type: 'mouseWheel', x, y, deltaX: 0, deltaY, modifiers: ctrl ? 2 : 0,
    });
  }

  /** 元素中心坐标（设计尺寸 1200×720，缩放为 1） */
  async center(selector) {
    const c = await this.eval(`(() => { const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return null; const r = el.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`);
    if (!c) throw new Error(`找不到元素：${selector}`);
    return c;
  }

  async screenshot(file) {
    const { data } = await this.send('Page.captureScreenshot', { format: 'png' });
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, Buffer.from(data, 'base64'));
  }

  async close() {
    try {
      this.ws.close();
      const res = await fetch(`http://127.0.0.1:${this.port}/json/close/${this.target.id}`, { signal: AbortSignal.timeout(CLEANUP_TIMEOUT_MS) });
      await res.body?.cancel();
    } catch {
      // 浏览器已关
    }
  }
}
