// 首页与启动过场探测共用：打开 index.html 的假剧本、读界面状态、等对话框、存截图（probes\out）。
// 读状态的表达式都带括号，拼进 `${X} === ...` 时不受 ?? / && 优先级影响
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { sleep } from './cdp.mjs';

const OUT = join(dirname(fileURLToPath(import.meta.url)), 'out');

/** 剧本 URL；extra 如 '&dev=1' */
export const page = (script, extra = '') => `index.html?fake=${script}${extra}`;

export const MODE = `(document.getElementById('home')?.dataset.mode)`;
/** 菜单当前选中第几项（1 起）；选中横带动作为 'tail'；没有选中为 0 */
export const SELECTED = `(() => { const b = document.querySelector('.tmenu button.sel'); if (b) return +b.dataset.i + 1;
  return document.querySelector('.issue .opt.sel') ? 'tail' : 0; })()`;
export const DIALOG_OPEN = `(!!document.querySelector('.dlg.on'))`;
export const STEP_RUN = `(document.querySelector('#home .step.run .n')?.textContent ?? '')`;
export const STEP_ERR = `(document.querySelector('#home .step.err .n')?.textContent ?? '')`;
export const VEIL_ON = `(!!document.querySelector('#home .veil.on'))`;
/** 唤起时的那行字；不显示为 '' */
export const LMSG = `(() => { const m = document.querySelector('#home .lmsg'); return m && m.classList.contains('on') ? m.textContent : ''; })()`;
export const TOAST = `(() => { const t = document.querySelector('.toast.on'); return t ? t.textContent : ''; })()`;
export const STATUS = `[...document.querySelectorAll('#home .status > *')].map((s) => s.textContent)`;
export const BANNER = `(() => { const b = document.querySelector('#home .issue'); if (!b || b.hidden) return null;
  const a = b.querySelector('.opt'); return { msg: b.querySelector('b').textContent, action: a && !a.hidden ? a.textContent : null }; })()`;

/** 某个接口调用的全部记录 */
export const calls = (p, name) => p.eval(`(window.__calls || []).filter((c) => c.name === ${JSON.stringify(name)})`);

/** 打开的对话框：文案、选项、选中项、是否带失败图标；没有为 null */
export const dialog = (p) => p.eval(`(() => { const d = document.querySelector('.dlg.on'); if (!d) return null;
  const sel = d.querySelector('.opts .opt.sel');
  return { msg: d.querySelector('.msg').textContent, sub: d.querySelector('.sub')?.textContent ?? '', opts: [...d.querySelectorAll('.opts .opt')].map((b) => b.textContent),
    sel: sel ? +sel.dataset.i : -1, icon: !d.querySelector('.dico').hidden }; })()`);

export async function waitDialog(p, timeout = 4000, what = '对话框没有出现') {
  await p.waitFor(DIALOG_OPEN, timeout, what);
  return dialog(p);
}

/** 开着的对话框过了 300ms 回车冷却后按 key */
export async function answer(p, key) {
  await sleep(350);
  await p.key(key);
}

/** 像真鼠标一样先移过去再单击（CDP 的单击本身不带 pointermove，不移就测不到悬停与离开） */
export async function clickAt(p, selector) {
  const c = await p.center(selector);
  await p.mouseMove(c.x, c.y);
  await p.click(c.x, c.y);
}

/** 本机无头 Edge 报 prefers-reduced-motion: reduce；要看平时的动效时在 open 之前固定成 no-preference */
export const emulateMotion = (p, value = 'no-preference') =>
  p.send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value }] });

/** 首页状态（getHome）已画到状态行 */
export const waitHome = (p) => p.waitFor(`document.querySelectorAll('#home .status > *').length > 0`, 3000, '状态行没有画出来');

/** 等字体和入场动画（标志 1.6s、状态行延迟 1s 再淡入 1s）放完再截图 */
export async function settle(p) {
  await p.eval('document.fonts.ready.then(() => true)');
  await sleep(2300);
}

export const shot = (p, name) => p.screenshot(join(OUT, `${name}.png`));

/** 每项探测跑完再存一张收尾截图 end_<项目名>.png（任务书：每个探测都留截图；中途的关键画面另存） */
export const withEndShots = (tests) => Object.fromEntries(Object.entries(tests).map(([name, fn]) => [name, async (p, ctx) => {
  await fn(p, ctx);
  await shot(p, `end_${name}`);
}]));
