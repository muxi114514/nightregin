// 次级页探测共用：进页面、读选中行和值
import { sleep } from './cdp.mjs';
import { calls, clickAt, page, waitHome } from './screen-helpers.mjs';

export const MENU_ITEM = (n) => `.tmenu button[data-i="${n - 1}"]`;
export const NIGHT = 3;
export const OPTIONS = 4;
export const SAVES = 5;
export const SYS_ON = `(!!document.querySelector('#sys.on'))`;
export const SEL = `(document.querySelector('.olist .row.sel')?.dataset.id ?? null)`;
export const ROW = (id) => `.olist .row[data-id="${id}"]`;
export const ARROW = (id, side) => `${ROW(id)} .ar.${side}`;
export const VAL = (id) => `(document.querySelector('${ROW(id)} .val')?.textContent ?? '')`;
export const ROWS = `[...document.querySelectorAll('.olist .row')].map((r) => r.dataset.id)`;
export const LABELS = `[...document.querySelectorAll('.olist .row .lb')].map((r) => r.textContent)`;
export const DESC = `(document.querySelector('#sys .desc')?.textContent ?? '')`;
export const TITLE = `(document.querySelector('#sys .head h1')?.textContent ?? '')`;
export const OFF = (id) => `(document.querySelector('${ROW(id)}')?.classList.contains('off') ?? false)`;
export const TAB = `(document.querySelector('#sys .tab.on .tl')?.textContent ?? '')`;
export const MENU_SEL = `(() => { const b = document.querySelector('.tmenu button.sel'); return b ? +b.dataset.i + 1 : 0; })()`;

/** 切页后过了 500ms 点击防抖 */
export const settleClicks = () => sleep(560);

/** 打开首页，点菜单第 n 项进次级页，等行画出来并过了防抖 */
export async function openSys(p, base, n, extra = '', script = 'home-ok') {
  await p.open(base + page(script, extra));
  await waitHome(p);
  await settleClicks();
  await clickAt(p, MENU_ITEM(n));
  await p.waitFor(`${SYS_ON} && document.querySelectorAll('.olist .row').length > 0`, 3000, '次级页没有打开');
  await settleClicks();
}

export const setCalls = (p) => calls(p, 'setGameSetting');

/** 真实的双击：同一点按下松开两次 */
export async function doubleClickAt(p, selector) {
  const c = await p.center(selector);
  await p.mouseMove(c.x, c.y);
  for (const clickCount of [1, 2]) {
    const ev = { x: c.x, y: c.y, button: 'left', clickCount };
    await p.send('Input.dispatchMouseEvent', { ...ev, type: 'mousePressed', buttons: 1 });
    await p.send('Input.dispatchMouseEvent', { ...ev, type: 'mouseReleased', buttons: 0 });
  }
}

/** 把光标按 ↓ 移到某一行 */
export async function selectRow(p, id) {
  for (let i = 0; i < 30; i++) {
    if ((await p.eval(SEL)) === id) return;
    await p.key('ArrowDown');
  }
  throw new Error(`没能选中 ${id}`);
}
