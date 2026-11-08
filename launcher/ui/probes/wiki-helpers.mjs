// 图鉴探测共用：进页面、读当前分类和选中项
import { clickAt, page, waitHome } from './screen-helpers.mjs';
import { MENU_ITEM, settleClicks } from './sys-helpers.mjs';

export const ON = `(!!document.querySelector('#codex.on'))`;
export const TAB = `(document.querySelector('#codex .tab.on .tl')?.textContent ?? '')`;
export const NAME = `(document.querySelector('#wdesc h2')?.textContent ?? '')`;
export const PANEL = `(document.querySelector('#wdesc')?.textContent ?? '')`;
export const FOCUS = `(() => { const e = document.querySelector('#wbox .wcell.f, #wbox .row.sel'); return e ? +e.dataset.i : -1; })()`;
export const NONE = `(document.querySelector('.wnone')?.textContent ?? '')`;
export const ZONE = `(() => { if (document.querySelector('#wbox.on')) return 2; const r = document.querySelector('#wrows .row.sel'); return r ? +r.dataset.z : -1; })()`;
export const COUNT = `document.querySelectorAll('#wbox .wcell[data-i], #wbox .row[data-i]').length`;
export const QUERY = `(document.querySelector('#wq')?.value ?? '')`;
export const FILTER = `(document.querySelector('#wrows .row[data-z="0"] .val')?.textContent.trim() ?? '')`;
export const HINTS = `(document.querySelector('#ckeys')?.textContent ?? '')`;

/** 打开首页，点菜单第 2 项进图鉴，等内容画出来并过了点击防抖 */
export async function openWiki(p, base, extra = '') {
  await p.open(base + page('home-ok', extra));
  await waitHome(p);
  await settleClicks();
  await clickAt(p, MENU_ITEM(2));
  await p.waitFor(`${ON} && (document.querySelector('#wdesc h2') !== null || document.querySelector('.wnone') !== null)`, 3000, '图鉴没有打开');
  await settleClicks();
}

/** 用 Q / E 翻到某个分类 */
export async function gotoTab(p, name) {
  for (let i = 0; i < 16; i++) {
    if ((await p.eval(TAB)) === name) return;
    await p.key('e');
  }
  throw new Error(`翻不到分类 ${name}`);
}

/** 条目区按方向键走到第 i 条（下标从 0 数，每行 5 格） */
export async function gotoCell(p, i, cols = 5) {
  for (let n = 0; n < 4 && (await p.eval(ZONE)) !== 2; n++) await p.key('ArrowDown');
  for (let n = 0; n < 200; n++) {
    const at = await p.eval(FOCUS);
    if (at === i) return;
    if (Math.floor(at / cols) < Math.floor(i / cols)) await p.key('ArrowDown');
    else if (at < i) await p.key('ArrowRight');
    else if (Math.floor(at / cols) > Math.floor(i / cols)) await p.key('ArrowUp');
    else await p.key('ArrowLeft');
  }
  throw new Error(`走不到第 ${i} 条`);
}

/** 进查找行并开始输入 */
export async function startSearch(p) {
  for (let i = 0; i < 4 && (await p.eval(ZONE)) !== 1; i++) await p.key('ArrowUp');
  await p.key('Enter');
  await p.waitFor(`document.activeElement?.id === 'wq'`, 2000, '查找框没有拿到焦点');
}

export const typeText = (p, text) => p.send('Input.insertText', { text });
